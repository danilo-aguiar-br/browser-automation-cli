// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::browser::WaitUntil;

use super::OneShotSession;

impl OneShotSession {
    /// Navigate and wait for load (same process). Honors robots when policy is Honor.
    ///
    /// # Errors
    ///
    /// Propagates [`goto_with_options`](Self::goto_with_options) with no init
    /// script, no beforeunload handling and the default navigation ceiling: a
    /// `file:` URL outside the allowed roots, a robots refusal, a navigation
    /// timeout, or a browser-level navigation failure.
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
    ///
    /// # Errors
    ///
    /// Fails before the browser sees the URL when a `file:` target lies
    /// outside the allowed roots, and when
    /// [`enforce_robots`](crate::robots::enforce_robots) refuses it under
    /// [`RobotsPolicy::Honor`](crate::robots::RobotsPolicy).
    ///
    /// Then fails with
    /// [`ErrorKind::Browser`] —
    /// `"init_script registration failed: …"` — when `init_script` cannot be
    /// registered, when no page is active, and on
    /// `"Navigation failed: …"` for a browser-reported navigation error such
    /// as `net::ERR_NAME_NOT_RESOLVED`.
    ///
    /// Fails with
    /// [`ErrorKind::Unavailable`] —
    /// `"Navigation timed out after Nms"` — when `navigation_timeout_ms`, or
    /// the `chrome_default_timeout_ms` ceiling in its absence, elapses first.
    ///
    /// A `handle_before_unload` value other than `accept` or `dismiss` is
    /// **not** an error: it reads as "off". The init script is removed after
    /// the navigation attempt whether it succeeded or failed, and the URL and
    /// title read-back afterwards is best-effort.
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
        // GAP-026 twin for the network side. The `http` engine has bounded
        // operator URLs since `scrape_local::http`, and this one did not, so
        // every target `--engine http` refuses stayed reachable by asking for
        // `--engine browser` instead. A defence a flag turns off is not one.
        //
        // Scoped to http(s) deliberately: `assert_safe_http_url` answers every
        // other scheme with "unsupported scheme for HTTP client", which is
        // right for an HTTP client and wrong for a browser that legitimately
        // navigates `about:blank`, `data:` and `file://` — the last already
        // bounded on the line above, by the check built for local paths.
        let scheme_is_http = {
            let lower = url.trim().to_ascii_lowercase();
            lower.starts_with("http://") || lower.starts_with("https://")
        };
        if scheme_is_http {
            crate::net::assert_safe_http_url(url)?;
        }
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
            // Witness fields are merged below rather than written here, so a
            // new policy key reaches every browser envelope at once.
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
        // Bound to a NAMED variable so the guard lives to the end of this
        // function. `let _ = ...` would drop it on the spot and abort the pump
        // before the navigation it exists to serve ever begins.
        //
        // The guard replaces the two hand-written `abort()` calls that used to
        // sit on the timeout path and the success path. Those covered three of
        // the four exits; the fourth — this future being DROPPED when SIGINT
        // cancels the run — reached neither, and left the loop detached,
        // issuing CDP commands at a browser already being torn down.
        let _dialog_pump = if let Some(action) = dialog_action {
            let accept = !action.eq_ignore_ascii_case("dismiss");
            let client = std::sync::Arc::clone(&self.manager.client);
            let session_id = self
                .manager
                .active_session_id()
                .map_err(|e| CliError::new(ErrorKind::Browser, e))?
                .to_string();
            Some(crate::runtime_util::AbortOnDrop::new(tokio::spawn(
                async move {
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
                },
            )))
        } else {
            None
        };

        let nav_fut = self.manager.navigate(url, WaitUntil::Load);
        // No explicit `--navigation-timeout-ms` still gets a ceiling: the same
        // per-operation budget every other Chrome call honours. Awaiting
        // unbounded here left the process hanging on a server that accepts the
        // connection and never answers, and the only brake was the global
        // `--timeout`, which defaults to 0 — that is, to none at all.
        let ms = navigation_timeout_ms.unwrap_or_else(|| {
            crate::xdg::policy::policy_u64(crate::xdg::policy::key::CHROME_DEFAULT_TIMEOUT_MS)
        });
        let nav_res =
            match tokio::time::timeout(std::time::Duration::from_millis(ms), nav_fut).await {
                Ok(r) => r,
                Err(_) => {
                    // The pump stops here because `_dialog_pump` is dropped by
                    // this return, not because anyone remembered to abort it.
                    return Err(CliError::with_suggestion(
                        ErrorKind::Unavailable,
                        format!("Navigation timed out after {ms}ms"),
                        // The navigation ceiling, NOT the global or per-step
                        // one. The generic hint used to be sent here, and it
                        // named two flags that do not govern this timeout: the
                        // caller raised `--step-timeout`, hit the identical
                        // error, and went looking at the network and the site.
                        crate::i18n::suggestion_key("raise_navigation_timeout", None),
                    ));
                }
            };

        self.clear_active_page_dialog();

        nav_res.map(|_| ()).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("Navigation failed: {e}"),
                crate::i18n::suggestion_key("navigation_failed_check", None),
            )
        })
    }

    /// Temp Chrome profile path for ledger residual wipe.
    pub fn temp_user_data_dir(&self) -> Option<std::path::PathBuf> {
        self.manager.temp_user_data_dir().map(|p| p.to_path_buf())
    }

    /// Register a CDP init script for subsequent navigations in this process.
    ///
    /// # Errors
    ///
    /// Fails with
    /// [`ErrorKind::Browser`] —
    /// `"init_script registration failed: …"`, carrying the
    /// `init_script_javascript` suggestion — when no page is active or
    /// `Page.addScriptToEvaluateOnNewDocument` is refused. Invalid JavaScript
    /// is not rejected here: it is only parsed when the next document loads.
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
        // `take` rather than `clone`: the live buffers are cleared at the end of
        // this same function, so copying every record only to drop the original
        // two statements later paid for a full duplicate of the navigation.
        let rings =
            crate::xdg::policy::policy_usize(crate::xdg::policy::key::CAPTURE_PRESERVED_RINGS);
        if !self.console_log.is_empty() {
            self.console_preserved
                .push(std::mem::take(&mut self.console_log));
            if self.console_preserved.len() > rings {
                let drop_n = self.console_preserved.len() - rings;
                self.console_preserved.drain(0..drop_n);
            }
        }
        if !self.network_log.is_empty() {
            self.network_preserved
                .push(std::mem::take(&mut self.network_log));
            if self.network_preserved.len() > rings {
                let drop_n = self.network_preserved.len() - rings;
                self.network_preserved.drain(0..drop_n);
            }
        }
        // Current navigation starts a fresh "live" buffer; preserved holds prior
        // rings. `take` already emptied whichever buffer was non-empty; these
        // clears cover the branch that was skipped.
        self.console_log.clear();
        self.network_log.clear();
    }
}
