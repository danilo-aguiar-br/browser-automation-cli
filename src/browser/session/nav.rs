// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::browser::WaitUntil;

use super::OneShotSession;

impl OneShotSession {
    /// Navigate and wait for load (same process). Honors robots when policy is Honor.
    pub async fn goto(
        &mut self,
        url: &str,
        robots: crate::robots::RobotsPolicy,
    ) -> Result<Value, CliError> {
        self.goto_with_options(url, robots, None, None, None).await
    }

    /// Navigate with tool-ref options: init script, beforeunload, navigation timeout.
    ///
    /// `init_script` is registered for the next document only and removed in `finally`
    /// (parity with tool-ref navigate_page; GAP-A006).
    ///
    /// `handle_before_unload` arms CDP dialog auto-accept/dismiss during navigation only
    /// (GAP-A009 / GAP-003). It does **not** inject a permanent `beforeunload` listener.
    /// Pass `Some("accept")`, `Some("dismiss")`, or `None` (off).
    pub async fn goto_with_options(
        &mut self,
        url: &str,
        robots: crate::robots::RobotsPolicy,
        init_script: Option<&str>,
        handle_before_unload: Option<&str>,
        navigation_timeout_ms: Option<u64>,
    ) -> Result<Value, CliError> {
        // GAP-026: contain the local scheme before the browser ever sees it.
        crate::fs_roots::ensure_file_url_allowed_default(url)?;
        // Same user-agent the http engine sends (`scrape_local::http`,
        // `scrape_local::sitemap`). A literal here meant the two engines matched
        // robots rules under different identities, so the same site could be
        // allowed on one path and denied on the other.
        crate::robots::enforce_robots(url, robots, &crate::robots::robots_user_agent()).await?;
        self.ref_map.clear();

        // Snapshot console/net before navigation so include_preserved can keep history.
        self.preserve_capture_snapshot();

        let mut init_script_id: Option<String> = None;
        if let Some(js) = init_script {
            let id = self.manager.add_script_to_evaluate(js).await.map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("init_script registration failed: {e}"),
                    crate::i18n::suggestion_key("init_script_javascript", None),
                )
            })?;
            if !id.is_empty() {
                init_script_id = Some(id);
            }
        }

        // GAP-A009 / GAP-003: auto-handle beforeunload via CDP accept|dismiss.
        let dialog_action = match handle_before_unload {
            Some(a) if a.eq_ignore_ascii_case("accept") => Some("accept"),
            Some(a) if a.eq_ignore_ascii_case("dismiss") => Some("dismiss"),
            _ => None,
        };

        let nav_result = self
            .navigate_with_dialog_pump(url, navigation_timeout_ms, dialog_action)
            .await;

        // GAP-A006: always remove one-shot init script after the navigation attempt.
        if let Some(id) = init_script_id.as_deref() {
            let _ = self.manager.remove_script_to_evaluate(id).await;
        }

        nav_result?;

        // Give console/network a brief window after load.
        tokio::time::sleep(std::time::Duration::from_millis(
            crate::xdg::policy::policy_u64(crate::xdg::policy::key::DEFAULT_NAV_MICRO_SETTLE_MS),
        ))
        .await;
        self.drain_events();
        let page_url = self
            .manager
            .get_url()
            .await
            .unwrap_or_else(|_| url.to_string());
        let title = self.manager.get_title().await.unwrap_or_default();
        let data = json!({
            "url": page_url,
            "title": title,
            "robots_policy": robots.as_str(),
            "init_script_applied": init_script.is_some(),
            "handle_before_unload": handle_before_unload,
            "navigation_timeout_ms": navigation_timeout_ms,
        });
        Ok(self.with_capture_fields(data))
    }

    /// Navigate while optionally auto-accepting/dismissing JS dialogs (beforeunload).
    ///
    /// Dialog pump runs on a cloned CDP client so it does not borrow `manager` across
    /// the navigate future (GAP-A009).
    async fn navigate_with_dialog_pump(
        &mut self,
        url: &str,
        navigation_timeout_ms: Option<u64>,
        dialog_action: Option<&str>,
    ) -> Result<(), CliError> {
        let dialog_task = if let Some(action) = dialog_action {
            let accept = !action.eq_ignore_ascii_case("dismiss");
            let client = std::sync::Arc::clone(&self.manager.client);
            let session_id = self
                .manager
                .active_session_id()
                .map_err(|e| CliError::new(ErrorKind::Browser, e))?
                .to_string();
            Some(tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        crate::xdg::resolve_event_pump_slice_ms(),
                    ))
                    .await;
                    let _ = client
                        .send_command(
                            "Page.handleJavaScriptDialog",
                            Some(json!({ "accept": accept })),
                            Some(&session_id),
                        )
                        .await;
                }
            }))
        } else {
            None
        };

        let nav_fut = self.manager.navigate(url, WaitUntil::Load);
        let nav_res = if let Some(ms) = navigation_timeout_ms {
            match tokio::time::timeout(std::time::Duration::from_millis(ms), nav_fut).await {
                Ok(r) => r,
                Err(_) => {
                    if let Some(t) = dialog_task {
                        t.abort();
                    }
                    return Err(CliError::with_suggestion(
                        ErrorKind::Unavailable,
                        format!("Navigation timed out after {ms}ms"),
                        crate::i18n::suggestion_key("raise_timeout", None),
                    ));
                }
            }
        } else {
            nav_fut.await
        };

        if let Some(t) = dialog_task {
            t.abort();
        }
        self.clear_active_page_dialog();

        nav_res.map(|_| ()).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("Navigation failed: {e}"),
                "Check URL scheme and network; try about:blank for smoke",
            )
        })
    }

    /// Temp Chrome profile path for ledger residual wipe.
    pub fn temp_user_data_dir(&self) -> Option<std::path::PathBuf> {
        self.manager.temp_user_data_dir().map(|p| p.to_path_buf())
    }

    /// Register a CDP init script for subsequent navigations in this process.
    pub async fn add_init_script(&self, source: &str) -> Result<String, CliError> {
        self.manager
            .add_script_to_evaluate(source)
            .await
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Browser,
                    format!("init_script registration failed: {e}"),
                    crate::i18n::suggestion_key("init_script_javascript", None),
                )
            })
    }

    /// Active stable tab id (`t1`, …) for tool-ref get_tab_id.
    pub fn active_tab_id_string(&self) -> Option<String> {
        self.manager
            .active_tab_id()
            .map(crate::native::browser::format_tab_id)
    }

    /// Mark current capture buffers as a navigation boundary for include_preserved.
    pub(crate) fn preserve_capture_snapshot(&mut self) {
        self.drain_events();
        if !self.console_log.is_empty() {
            self.console_preserved.push(self.console_log.clone());
            if self.console_preserved.len() > 3 {
                let drop_n = self.console_preserved.len() - 3;
                self.console_preserved.drain(0..drop_n);
            }
        }
        if !self.network_log.is_empty() {
            self.network_preserved.push(self.network_log.clone());
            if self.network_preserved.len() > 3 {
                let drop_n = self.network_preserved.len() - 3;
                self.network_preserved.drain(0..drop_n);
            }
        }
        // Current navigation starts a fresh "live" buffer; preserved holds prior rings.
        self.console_log.clear();
        self.network_log.clear();
    }
}
