// SPDX-License-Identifier: MIT OR Apache-2.0
//! Emulation, dialogs, uploads, and page scripts.

use serde_json::{json, Value};

use crate::native::element::{resolve_element_object_id, RefMap};

use super::BrowserManager;

impl BrowserManager {
    /// Override viewport size, scale factor, mobile and touch emulation.
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
                Some(json!({
                    "width": width,
                    "height": height,
                    "deviceScaleFactor": device_scale_factor,
                    "mobile": mobile,
                })),
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

    /// Override the user agent for this session only.
    pub async fn set_user_agent(&self, user_agent: &str) -> Result<(), String> {
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
    pub async fn bring_to_front(&self) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        self.client
            .send_command("Page.bringToFront", None, Some(session_id))
            .await?;
        Ok(())
    }

    /// Override the timezone reported to the page, by IANA id.
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
