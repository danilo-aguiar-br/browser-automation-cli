// SPDX-License-Identifier: MIT OR Apache-2.0
//! Emulation, dialogs, uploads, and page scripts.

use serde_json::{json, Value};

use crate::native::element::{resolve_element_object_id, RefMap};

use super::BrowserManager;

impl BrowserManager {
    /// Override viewport size, scale factor, mobile and touch emulation.
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, or with the CDP
    /// error raised by `Emulation.setDeviceMetricsOverride`. The follow-up
    /// `Browser.getWindowForTarget` / `Browser.setContentsSize` pair is
    /// best-effort: both are experimental CDP, and a refusal is logged at
    /// `warn` rather than returned, because the CSS viewport override — the
    /// part the caller asked for — already landed.
    pub async fn set_viewport(
        &self,
        width: i32,
        height: i32,
        device_scale_factor: f64,
        mobile: bool,
    ) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        self.client
            .send_command(
                "Emulation.setDeviceMetricsOverride",
                Some(crate::native::stealth::device_metrics_override(
                    width,
                    height,
                    device_scale_factor,
                    mobile,
                )),
                Some(session_id),
            )
            .await?;

        // Screencast captures the actual content area, not the emulated CSS
        // viewport, so resize the content area to match.
        if let Ok(target_id) = self.active_target_id() {
            if let Ok(window_info) = self
                .client
                .send_command(
                    "Browser.getWindowForTarget",
                    Some(json!({ "targetId": target_id })),
                    None,
                )
                .await
            {
                if let Some(window_id) = window_info.get("windowId").and_then(|v| v.as_i64()) {
                    if let Err(e) = self
                        .client
                        .send_command(
                            "Browser.setContentsSize",
                            Some(json!({
                                "windowId": window_id,
                                "width": width,
                                "height": height,
                            })),
                            None,
                        )
                        .await
                    {
                        tracing::warn!(
                            target: "browser_automation_cli::native::browser",
                            error = %e,
                            "Browser.setContentsSize failed (experimental CDP)"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Override the user agent string, and nothing else, for this session.
    ///
    /// # What this deliberately does not do
    ///
    /// `Emulation.setUserAgentOverride` also accepts `userAgentMetadata`, which
    /// is what drives the `sec-ch-ua`, `sec-ch-ua-mobile` and
    /// `sec-ch-ua-platform` Client Hints. This call omits it, so the UA string
    /// changes and the Client Hints keep whatever Chrome was already sending.
    ///
    /// That split is a detectable contradiction: a bot check reads the UA and
    /// the hints side by side, and a header saying Windows next to a UA saying
    /// macOS answers its question for it. The name says so out loud because the
    /// short name `set_user_agent` invited call sites to assume the identity
    /// moved as one piece.
    ///
    /// Use this only for `emulate --user-agent`, where the operator asked for
    /// exactly this string and nothing more. Identity changes that must stay
    /// coherent go through [`crate::native::stealth::user_agent_override`],
    /// which carries the matching hints.
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, or with the CDP
    /// error raised by `Emulation.setUserAgentOverride`.
    pub async fn set_user_agent_without_client_hints(
        &self,
        user_agent: &str,
    ) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        self.client
            .send_command(
                "Emulation.setUserAgentOverride",
                Some(json!({ "userAgent": user_agent })),
                Some(session_id),
            )
            .await?;
        Ok(())
    }

    /// Emulate a media type or feature, such as `print` or `prefers-color-scheme`.
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, or with the CDP
    /// error raised by `Emulation.setEmulatedMedia` — which is also what
    /// rejects an unknown media type or feature name.
    pub async fn set_emulated_media(
        &self,
        media: Option<&str>,
        features: Option<Vec<(String, String)>>,
    ) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        let mut params = json!({});
        if let Some(m) = media {
            params["media"] = Value::String(m.to_string());
        }
        if let Some(feats) = features {
            let features_arr: Vec<Value> = feats
                .iter()
                .map(|(name, value)| json!({ "name": name, "value": value }))
                .collect();
            params["features"] = Value::Array(features_arr);
        }
        self.client
            .send_command("Emulation.setEmulatedMedia", Some(params), Some(session_id))
            .await?;
        Ok(())
    }

    /// Raise this tab to the foreground.
    ///
    /// Matters even headless: a backgrounded tab is throttled, which changes
    /// timing-sensitive results.
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, or with the CDP
    /// error raised by `Page.bringToFront`.
    pub async fn bring_to_front(&self) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        self.client
            .send_command("Page.bringToFront", None, Some(session_id))
            .await?;
        Ok(())
    }

    /// Override the timezone reported to the page, by IANA id.
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, or with the CDP
    /// error raised by `Emulation.setTimezoneOverride` — which is what rejects
    /// a `timezone_id` Chrome does not recognize as an IANA zone.
    pub async fn set_timezone(&self, timezone_id: &str) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        self.client
            .send_command(
                "Emulation.setTimezoneOverride",
                Some(json!({ "timezoneId": timezone_id })),
                Some(session_id),
            )
            .await?;
        Ok(())
    }

    /// Override the locale reported to the page.
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, or with the CDP
    /// error raised by `Emulation.setLocaleOverride`, which rejects a malformed
    /// locale tag.
    pub async fn set_locale(&self, locale: &str) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        self.client
            .send_command(
                "Emulation.setLocaleOverride",
                Some(json!({ "locale": locale })),
                Some(session_id),
            )
            .await?;
        Ok(())
    }

    /// Override the geolocation the page receives.
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, or with the CDP
    /// error raised by `Emulation.setGeolocationOverride`, which rejects
    /// coordinates outside the valid latitude/longitude ranges.
    pub async fn set_geolocation(
        &self,
        latitude: f64,
        longitude: f64,
        accuracy: Option<f64>,
    ) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        self.client
            .send_command(
                "Emulation.setGeolocationOverride",
                Some(json!({
                    "latitude": latitude,
                    "longitude": longitude,
                    "accuracy": accuracy.unwrap_or(1.0),
                })),
                Some(session_id),
            )
            .await?;
        Ok(())
    }

    /// Grant browser permissions up front so the page never prompts.
    ///
    /// A prompt in a one-shot run is a deadlock: nobody is there to answer it.
    ///
    /// # Errors
    ///
    /// Fails with the CDP error raised by `Browser.grantPermissions`, which
    /// rejects any name outside the protocol's `PermissionType` enum. Browser
    /// scoped, so it does not require an active page.
    pub async fn grant_permissions(&self, permissions: &[String]) -> Result<(), String> {
        self.client
            .send_command(
                "Browser.grantPermissions",
                Some(json!({ "permissions": permissions })),
                None,
            )
            .await?;
        Ok(())
    }

    /// Accept or dismiss the open JavaScript dialog.
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, or with the CDP
    /// error raised by `Page.handleJavaScriptDialog` — which is what reports
    /// "No dialog is showing" when the caller answered a dialog that is not
    /// open. Callers surface that as
    /// [`ErrorKind::Precondition`](crate::error::ErrorKind::Precondition).
    pub async fn handle_dialog(
        &self,
        accept: bool,
        prompt_text: Option<&str>,
    ) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        let mut params = json!({ "accept": accept });
        if let Some(text) = prompt_text {
            params["promptText"] = Value::String(text.to_string());
        }
        self.client
            .send_command(
                "Page.handleJavaScriptDialog",
                Some(params),
                Some(session_id),
            )
            .await?;
        Ok(())
    }

    /// Set the files of a file input without a native picker.
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, when
    /// `resolve_element_object_id` cannot resolve `selector` (unknown `@eN`
    /// ref, no DOM match, or a detached node), when `DOM.describeNode` is
    /// refused, when the described node carries no `backendNodeId`
    /// (`"Could not get backendNodeId for file input"`), or when
    /// `DOM.setFileInputFiles` is refused — which is what rejects a target
    /// that is not a file input.
    ///
    /// The paths in `files` are resolved by the browser, not here, so a
    /// missing file surfaces as a CDP error rather than an I/O error.
    pub async fn upload_files(
        &self,
        selector: &str,
        files: &[String],
        ref_map: &RefMap,
        iframe_sessions: &rustc_hash::FxHashMap<String, String>,
    ) -> Result<(), String> {
        let session_id = self.active_session_id()?;

        let (object_id, effective_session_id) =
            resolve_element_object_id(&self.client, session_id, ref_map, selector, iframe_sessions)
                .await?;

        let describe: Value = self
            .client
            .send_command(
                "DOM.describeNode",
                Some(json!({ "objectId": object_id })),
                Some(&effective_session_id),
            )
            .await?;

        let backend_node_id = describe
            .get("node")
            .and_then(|n| n.get("backendNodeId"))
            .and_then(|v| v.as_i64())
            .ok_or("Could not get backendNodeId for file input")?;

        self.client
            .send_command(
                "DOM.setFileInputFiles",
                Some(json!({
                    "files": files,
                    "backendNodeId": backend_node_id,
                })),
                Some(&effective_session_id),
            )
            .await?;

        Ok(())
    }

    /// Register a script to run on every new document; returns its identifier.
    ///
    /// Runs BEFORE page script, which is what makes it usable for patching
    /// globals the page will read.
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, or with the CDP
    /// error raised by `Page.addScriptToEvaluateOnNewDocument`. A response
    /// carrying no `identifier` is **not** an error: the empty string is
    /// returned, which
    /// [`remove_script_to_evaluate`](Self::remove_script_to_evaluate) will
    /// later fail to unregister.
    pub async fn add_script_to_evaluate(&self, source: &str) -> Result<String, String> {
        let session_id = self.active_session_id()?;
        let result = self
            .client
            .send_command(
                "Page.addScriptToEvaluateOnNewDocument",
                Some(json!({ "source": source })),
                Some(session_id),
            )
            .await?;
        Ok(result
            .get("identifier")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    /// Unregister a script previously added by [`add_script_to_evaluate`](Self::add_script_to_evaluate).
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, or with the CDP
    /// error raised by `Page.removeScriptToEvaluateOnNewDocument`, which
    /// rejects an `identifier` this session never registered.
    pub async fn remove_script_to_evaluate(&self, identifier: &str) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        self.client
            .send_command(
                "Page.removeScriptToEvaluateOnNewDocument",
                Some(json!({ "identifier": identifier })),
                Some(session_id),
            )
            .await?;
        Ok(())
    }
}
