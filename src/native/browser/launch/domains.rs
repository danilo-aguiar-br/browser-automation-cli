// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP domain enablement and debugger resume.

use serde_json::json;

use crate::native::browser::BrowserManager;

impl BrowserManager {
    /// Enable the CDP domains this CLI relies on for a session.
    ///
    /// # Errors
    ///
    /// Fails with the CDP error raised by `Page.enable`, `Runtime.enable` or
    /// `Network.enable` on `session_id` — an unknown session, or a detached
    /// target. `Target.setAutoAttach` and the debugger resume are best-effort
    /// and cannot fail the call, because engines such as Lightpanda do not
    /// implement them.
    pub async fn enable_domains_pub(&self, session_id: &str) -> Result<(), String> {
        self.enable_domains(session_id).await
    }

    /// Enable domains AND apply the per-session setup that must precede navigation.
    ///
    /// # Errors
    ///
    /// Fails with the CDP error raised by `Page.enable`, `Runtime.enable` or
    /// `Network.enable` on `session_id`. The `Target.setAutoAttach` that
    /// follows is best-effort and its refusal is discarded.
    pub async fn prepare_domains_pub(&self, session_id: &str) -> Result<(), String> {
        self.prepare_domains(session_id).await
    }

    /// Release a target that auto-attach paused before its first script ran.
    ///
    /// Auto-attached targets start suspended; a session that is never resumed
    /// looks like a page that simply never loads.
    ///
    /// # Errors
    ///
    /// Never returns `Err`. `Runtime.runIfWaitingForDebugger` is sent through
    /// the no-wait path and its outcome is discarded, because a target that
    /// was never suspended — every engine below Chrome 144 — legitimately
    /// refuses it.
    pub async fn resume_if_waiting_pub(&self, session_id: &str) -> Result<(), String> {
        self.resume_if_waiting(session_id).await
    }

    /// Auto-attach to targets created from now on, so new tabs are tracked.
    ///
    /// # Errors
    ///
    /// Fails with the CDP error raised by the browser-scoped
    /// `Target.setAutoAttach` — the engine does not implement the `Target`
    /// domain, or rejects `flatten`. Unlike the per-session call inside
    /// `prepare_domains`, this refusal **is** surfaced: without it no new tab
    /// is ever tracked.
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
        // Three independent domains on one session, so they go out together
        // rather than paying three serial round trips at every launch. CDP has
        // no ordering relation between `Page`, `Runtime` and `Network`: none
        // reads state the others install, and the events each turns on are
        // delivered to forwarders that subscribe on their own.
        //
        // `Target.setAutoAttach` below is deliberately NOT part of this: it
        // governs the child targets those domains will report, and it stays
        // after them.
        //
        // `Runtime` is CONDITIONAL and the other two are not. Enabling it is a
        // cheap fingerprint — it installs console and execution-context
        // machinery a plain page never asks for — and measured 2026-09-04 the
        // one-shot path had no consumer for the events it turns on. Evaluation
        // does not need it: `Runtime.evaluate` and `Runtime.callFunctionOn` are
        // commands and answer with the domain disabled, and no call site in
        // this crate targets an evaluation by `executionContextId`.
        //
        // `browser_policy::runtime_events_needed` carries the answer, published
        // once from dispatch, and the envelope reports it as
        // `runtime_enable_used` so the claim is checkable from outside.
        tokio::try_join!(
            self.client
                .send_command_no_params("Page.enable", Some(session_id)),
            self.client
                .send_command_no_params("Network.enable", Some(session_id)),
        )?;
        if crate::browser_policy::runtime_events_needed() {
            self.client
                .send_command_no_params("Runtime.enable", Some(session_id))
                .await?;
        }
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
