// SPDX-License-Identifier: MIT OR Apache-2.0
//! History navigation and reload with dialog handling.

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Navigate back in history for the active page.
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Browser`] —
    /// `"back failed: …"` — when the `history.back()` evaluation is refused,
    /// which is what no active page or a destroyed execution context produces.
    ///
    /// An empty history is **not** an error: `history.back()` is a no-op, the
    /// settle passes, and the envelope reports the unchanged URL. Robots are
    /// not consulted on this path, because the destination was already visited.
    pub async fn back(&mut self) -> Result<Value, CliError> {
        self.history_nav("back").await
    }

    /// Navigate forward in history for the active page.
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Browser`] —
    /// `"forward failed: …"` — when the `history.forward()` evaluation is
    /// refused. Nothing to go forward to is not an error; it is a no-op.
    pub async fn forward(&mut self) -> Result<Value, CliError> {
        self.history_nav("forward").await
    }

    /// Reload via CDP `Page.reload` with optional `ignoreCache` (GAP-A005).
    ///
    /// Optional `init_script` is registered for this reload only and removed after
    /// (GAP-A006). `handle_before_unload` arms dialog auto-accept during reload
    /// without injecting a permanent beforeunload listener (GAP-A009).
    /// Reload the active page with optional cache bypass and beforeunload handling.
    /// Reload the active page; `ignore_cache` maps to CDP ignoreCache.
    ///
    /// # Errors
    ///
    /// Propagates [`add_init_script`](Self::add_init_script) when
    /// `init_script` cannot be registered, and fails with
    /// [`ErrorKind::Browser`] when no page
    /// is active or `Page.reload` is refused (`"reload failed: …"`).
    ///
    /// A `handle_before_unload` value other than `accept` or `dismiss` reads
    /// as "off" rather than failing. The init script is removed after the
    /// reload attempt on both paths, and the URL and title read-back is
    /// best-effort.
    ///
    /// Robots are **not** re-checked: the page was already fetched once, and a
    /// reload does not reach a new resource.
    pub async fn reload_with_options(
        &mut self,
        ignore_cache: bool,
        init_script: Option<&str>,
        handle_before_unload: Option<&str>,
    ) -> Result<Value, CliError> {
        self.drain_events();
        self.preserve_capture_snapshot();

        let mut init_script_id: Option<String> = None;
        if let Some(js) = init_script {
            let id = self.add_init_script(js).await?;
            if !id.is_empty() {
                init_script_id = Some(id);
            }
        }

        let dialog_action = match handle_before_unload {
            Some(a) if a.eq_ignore_ascii_case("accept") => Some("accept"),
            Some(a) if a.eq_ignore_ascii_case("dismiss") => Some("dismiss"),
            _ => None,
        };

        let reload_result = self
            .reload_with_dialog_pump(ignore_cache, dialog_action)
            .await;

        if let Some(id) = init_script_id.as_deref() {
            let _ = self.manager.remove_script_to_evaluate(id).await;
        }

        reload_result?;
        tokio::time::sleep(std::time::Duration::from_millis(
            crate::xdg::resolve_interact_settle_ms(),
        ))
        .await;
        self.drain_events();
        let url = self.manager.get_url().await.unwrap_or_default();
        let title = self.manager.get_title().await.unwrap_or_default();
        Ok(json!({
            "reloaded": true,
            "ignore_cache": ignore_cache,
            "init_script_applied": init_script.is_some(),
            "handle_before_unload": handle_before_unload,
            "url": url,
            "title": title,
        }))
    }

    /// Reload current page via CDP `Page.reload` (GAP-A005).
    ///
    /// # Errors
    ///
    /// Propagates [`reload_with_options`](Self::reload_with_options) with no
    /// init script and no beforeunload handling: no active page, or a refused
    /// `Page.reload`. With a `beforeunload` dialog armed by the page and no
    /// handling requested, the reload can stall on that dialog.
    pub async fn reload(&mut self, ignore_cache: bool) -> Result<Value, CliError> {
        self.reload_with_options(ignore_cache, None, None).await
    }

    async fn reload_with_dialog_pump(
        &mut self,
        ignore_cache: bool,
        dialog_action: Option<&str>,
    ) -> Result<(), CliError> {
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        let client = std::sync::Arc::clone(&self.manager.client);

        // Named binding, not `let _ = ...`: the guard must outlive the reload
        // below, and a bare underscore would abort the pump immediately. See
        // the same pattern, and the reason for it, in `nav.rs`.
        let _dialog_pump = if let Some(action) = dialog_action {
            let accept = !action.eq_ignore_ascii_case("dismiss");
            let client_d = std::sync::Arc::clone(&client);
            let sid = session_id.clone();
            Some(crate::runtime_util::AbortOnDrop::new(tokio::spawn(
                async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            crate::xdg::resolve_event_pump_slice_ms(),
                        ))
                        .await;
                        let _ = client_d
                            .send_command(
                                "Page.handleJavaScriptDialog",
                                Some(json!({ "accept": accept })),
                                Some(&sid),
                            )
                            .await;
                    }
                },
            )))
        } else {
            None
        };

        let res = client
            .send_command(
                "Page.reload",
                Some(json!({ "ignoreCache": ignore_cache })),
                Some(&session_id),
            )
            .await
            .map(|_| ())
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("reload failed: {e}")));

        self.clear_active_page_dialog();
        res
    }

    async fn history_nav(&mut self, direction: &str) -> Result<Value, CliError> {
        self.drain_events();
        let script = match direction {
            "back" => "history.back(); 'ok'",
            "forward" => "history.forward(); 'ok'",
            _ => "null",
        };
        self.manager
            .evaluate(script, None)
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("{direction} failed: {e}")))?;
        tokio::time::sleep(std::time::Duration::from_millis(
            crate::xdg::resolve_interact_settle_ms(),
        ))
        .await;
        self.drain_events();
        let url = self.manager.get_url().await.unwrap_or_default();
        let title = self.manager.get_title().await.unwrap_or_default();
        Ok(json!({
            "navigation": direction,
            "url": url,
            "title": title,
        }))
    }
}
