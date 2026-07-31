// SPDX-License-Identifier: MIT OR Apache-2.0
//! Browser close, process status, and download behavior.

use serde_json::json;

use super::BrowserManager;

impl BrowserManager {
    /// FINALIZE: close the browser, then reap whatever this manager owns.
    pub async fn close(&mut self) -> Result<(), String> {
        // Chrome one-shot: chromiumoxide FINALIZE (close + wait + kill fallback).
        if self.owns_oxide_browser {
            self.owns_oxide_browser = false;
            let res = crate::native::cdp::oxide::finalize_browser(self.client.browser()).await;
            if let Some(dir) = self.temp_user_data_dir.take() {
                let _ = std::fs::remove_dir_all(&dir);
            }
            return res;
        }

        if self.browser_process.is_some() {
            // Lightpanda (or other process we spawned): ask browser to close, then reap.
            let _ = self
                .client
                .send_command_no_params("Browser.close", None)
                .await;
        }

        if let Some(mut process) = self.browser_process.take() {
            let timeout = std::time::Duration::from_secs(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::BROWSER_CLOSE_WAIT_SECS,
            ));
            let _ = tokio::task::spawn_blocking(move || {
                process.wait_or_kill(timeout);
            })
            .await;
        }

        if let Some(dir) = self.temp_user_data_dir.take() {
            let _ = std::fs::remove_dir_all(&dir);
        }

        Ok(())
    }

    /// Whether any tab is currently tracked.
    pub fn has_pages(&self) -> bool {
        !self.pages.is_empty()
    }

    /// Per-operation timeout in milliseconds, as resolved from config.
    pub fn default_timeout_ms(&self) -> u64 {
        self.default_timeout_ms
    }

    /// Checks if the CDP connection is alive by sending a simple command.
    /// Returns false if the command times out or fails.
    pub async fn is_connection_alive(&self) -> bool {
        let timeout = tokio::time::Duration::from_secs(
            crate::xdg::resolve_cdp_connection_probe_timeout_secs(),
        );
        let result = tokio::time::timeout(
            timeout,
            self.client
                .send_command_no_params("Browser.getVersion", None),
        )
        .await;

        match result {
            Ok(Ok(_)) => true,
            Ok(Err(_)) | Err(_) => false,
        }
    }

    /// Non-blocking check whether the locally-launched browser process has exited
    /// (crashed or terminated). Also reaps the zombie if it has exited.
    /// Returns false for external CDP connections (no child process to monitor).
    pub fn has_process_exited(&mut self) -> bool {
        if let Some(ref mut process) = self.browser_process {
            process.has_exited()
        } else {
            false
        }
    }

    /// DevTools websocket URL this manager is connected to.
    pub fn get_cdp_url(&self) -> &str {
        &self.ws_url
    }

    /// Returns the Chrome debug server address as "host:port".
    pub fn chrome_host_port(&self) -> &str {
        let stripped = self
            .ws_url
            .strip_prefix("ws://")
            .or_else(|| self.ws_url.strip_prefix("wss://"))
            .unwrap_or(&self.ws_url);
        stripped.split('/').next().unwrap_or(stripped)
    }

    /// Chrome target id of the active tab, or an error when there is none.
    pub fn active_target_id(&self) -> Result<&str, String> {
        self.pages
            .get(self.active_page_index)
            .map(|p| p.target_id.as_str())
            .ok_or_else(|| "No active page".to_string())
    }

    /// Returns true if this manager was connected via CDP (as opposed to local launch).
    pub fn is_cdp_connection(&self) -> bool {
        self.browser_process.is_none()
    }

    /// Whether the connection was made straight to a page endpoint.
    ///
    /// A direct page connection has no browser-level session, so target and
    /// tab commands are not available on it.
    pub fn is_direct_page_connection(&self) -> bool {
        self.direct_page
    }

    /// Point downloads at `download_path` and allow them.
    pub async fn set_download_behavior(&self, download_path: &str) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        self.client
            .send_command(
                "Browser.setDownloadBehavior",
                Some(json!({
                    "behavior": "allowAndName",
                    "downloadPath": download_path,
                    "eventsEnabled": true,
                })),
                Some(session_id),
            )
            .await?;
        Ok(())
    }
}
