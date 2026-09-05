// SPDX-License-Identifier: MIT OR Apache-2.0
//! Browser close, process status, and download behavior.

use serde_json::json;

use super::BrowserManager;

/// Delete an ephemeral profile, and the `/tmp` directory Chrome tied to it.
///
/// # Why the profile alone is not the whole footprint
///
/// A unix socket path is capped near 108 bytes and this product's profile path
/// — an XDG cache dir plus a UUID — exceeds it. Chrome reacts by putting the
/// real `SingletonSocket` in a short `/tmp/org.chromium.Chromium.*` directory
/// and leaving a symlink to it in the profile. `remove_dir_all` on the profile
/// takes the symlink and leaves the directory, so every launch leaked one.
///
/// Measured before this helper existed: three launches, three directories, and
/// `doctor` reporting `chromium_tmp_singleton_orphans` climbing with each one.
/// A cross-run collector eventually swept them, but only after an age floor
/// and only if another launch happened to follow — so the last launch of any
/// session always left one behind.
///
/// The symlink must be read BEFORE the profile is deleted: the wipe destroys
/// the only proof of which directory belongs to this launch.
fn wipe_profile_and_its_tmp_singleton(dir: &std::path::Path) {
    let tmp_singleton = crate::residual::owned_chromium_tmp_dir_via_profile(dir);
    let _ = std::fs::remove_dir_all(dir);
    if let Some(tmp) = tmp_singleton {
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

impl BrowserManager {
    /// FINALIZE: close the browser, then reap whatever this manager owns.
    ///
    /// # Errors
    ///
    /// On the chromiumoxide-owned path, propagates `finalize_browser`: a
    /// refused `Browser.close` or a failing `Browser::wait`. Every other path
    /// returns `Ok(())` by construction — `Browser.close` on a self-spawned
    /// child is sent best-effort, the child is reaped by
    /// `wait_or_kill`, and the temp profile wipe (plus its
    /// `/tmp/org.chromium.Chromium.*` singleton directory) ignores I/O errors,
    /// so FINALIZE cannot fail after the browser is already gone.
    pub async fn close(&mut self) -> Result<(), String> {
        // Chrome one-shot: chromiumoxide FINALIZE (close + wait + kill fallback).
        if self.owns_oxide_browser {
            self.owns_oxide_browser = false;
            let res = crate::native::cdp::oxide::finalize_browser(self.client.browser()).await;
            if let Some(dir) = self.temp_user_data_dir.take() {
                wipe_profile_and_its_tmp_singleton(&dir);
            }
            return res;
        }

        if self.browser_process.is_some() {
            // A process we spawned (Chrome self-spawn or Lightpanda): ask the
            // browser to close, then reap.
            let _ = self
                .client
                .send_command_no_params("Browser.close", None)
                .await;
            // Stop the event pump BEFORE the socket dies. `Browser.close` drops
            // the WebSocket without a closing handshake, and chromiumoxide logs
            // that at ERROR from inside its handler — on a run that succeeded.
            // Ordering the abort here means the reset is never observed, rather
            // than observed and filtered.
            self.client.stop_event_pump();
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
            wipe_profile_and_its_tmp_singleton(&dir);
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
    ///
    /// # Errors
    ///
    /// Returns `"No active page"` when `active_page_index` addresses no
    /// tracked page — before the first attach, or after every tab was closed.
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
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, or with the CDP
    /// error raised by `Browser.setDownloadBehavior` — an engine that does not
    /// implement it, or a `download_path` the browser cannot use.
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
