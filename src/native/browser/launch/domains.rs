// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP domain enablement and debugger resume.

use serde_json::json;

use crate::native::browser::BrowserManager;

impl BrowserManager {
    /// Enable the CDP domains this CLI relies on for a session.
    pub async fn enable_domains_pub(&self, session_id: &str) -> Result<(), String> {
        self.enable_domains(session_id).await
    }

    /// Enable domains AND apply the per-session setup that must precede navigation.
    pub async fn prepare_domains_pub(&self, session_id: &str) -> Result<(), String> {
        self.prepare_domains(session_id).await
    }

    /// Release a target that auto-attach paused before its first script ran.
    ///
    /// Auto-attached targets start suspended; a session that is never resumed
    /// looks like a page that simply never loads.
    pub async fn resume_if_waiting_pub(&self, session_id: &str) -> Result<(), String> {
        self.resume_if_waiting(session_id).await
    }

    /// Auto-attach to targets created from now on, so new tabs are tracked.
    pub async fn enable_browser_auto_attach_pub(&self) -> Result<(), String> {
        self.client
            .send_command(
                "Target.setAutoAttach",
                Some(json!({
                    "autoAttach": true,
                    "waitForDebuggerOnStart": true,
                    "flatten": true
                })),
                None,
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn enable_domains(&self, session_id: &str) -> Result<(), String> {
        self.prepare_domains(session_id).await?;
        self.resume_if_waiting(session_id).await?;
        Ok(())
    }

    pub(crate) async fn prepare_domains(&self, session_id: &str) -> Result<(), String> {
        self.client
            .send_command_no_params("Page.enable", Some(session_id))
            .await?;
        self.client
            .send_command_no_params("Runtime.enable", Some(session_id))
            .await?;
        self.client
            .send_command_no_params("Network.enable", Some(session_id))
            .await?;
        // Enable auto-attach for cross-origin iframe support.
        // flatten: true gives each iframe its own session_id.
        // waitForDebuggerOnStart keeps child targets paused until the one-shot session
        // installs any required network controls and explicitly resumes them.
        // Ignored on engines that don't support it (e.g. Lightpanda).
        let _ = self
            .client
            .send_command(
                "Target.setAutoAttach",
                Some(json!({
                    "autoAttach": true,
                    "waitForDebuggerOnStart": true,
                    "flatten": true
                })),
                Some(session_id),
            )
            .await;
        Ok(())
    }

    pub(crate) async fn resume_if_waiting(&self, session_id: &str) -> Result<(), String> {
        // Needed for real browser sessions (Chrome 144+) where targets are
        // paused after attach until explicitly resumed. No-op otherwise.
        let _ = self
            .client
            .send_command_no_wait("Runtime.runIfWaitingForDebugger", None, Some(session_id))
            .await;
        Ok(())
    }
}
