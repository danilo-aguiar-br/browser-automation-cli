// SPDX-License-Identifier: MIT OR Apache-2.0
//! Navigation, lifecycle waits, and evaluation.

use rustc_hash::FxHashSet;
use serde_json::{json, Value};
use tokio::sync::broadcast;

use crate::native::cdp::types::*;

use super::types::*;
use super::BrowserManager;

impl BrowserManager {
    /// Navigate the active tab and wait according to `wait_until`.
    pub async fn navigate(&mut self, url: &str, wait_until: WaitUntil) -> Result<Value, String> {
        let session_id = self.active_session_id()?.to_string();
        let mut lifecycle_rx = self.client.subscribe();

        let nav_result: PageNavigateResult = self
            .client
            .send_command_typed(
                "Page.navigate",
                &PageNavigateParams {
                    url: url.to_string(),
                    referrer: None,
                },
                Some(&session_id),
            )
            .await?;

        if let Some(ref error_text) = nav_result.error_text {
            return Err(format!("Navigation failed: {error_text}"));
        }

        // Only wait for lifecycle events if Chrome created a new loader (full navigation).
        // If loader_id is None, it was a same-document navigation (e.g., hash routing)
        // which does not fire Page.loadEventFired or Page.domContentEventFired.
        if nav_result.loader_id.is_some() && wait_until != WaitUntil::None {
            self.wait_for_lifecycle(wait_until, &session_id, &mut lifecycle_rx)
                .await?;
        }

        let page_url = self.get_url().await.unwrap_or_else(|_| url.to_string());
        let title = self.get_title().await.unwrap_or_default();

        // Track visited origin for cross-origin localStorage collection in save_state
        if let Ok(parsed) = url::Url::parse(&page_url) {
            let origin = parsed.origin().ascii_serialization();
            if origin != "null" {
                self.visited_origins.insert(origin);
            }
        }

        if let Some(page) = self.pages.get_mut(self.active_page_index) {
            page.url = page_url.clone();
            page.title = title.clone();
        }

        Ok(json!({ "url": page_url, "title": title }))
    }

    async fn wait_for_lifecycle(
        &self,
        wait_until: WaitUntil,
        session_id: &str,
        rx: &mut broadcast::Receiver<CdpEvent>,
    ) -> Result<(), String> {
        let (event_name, ready_states): (&str, &[&str]) = match wait_until {
            WaitUntil::Load => ("Page.loadEventFired", &["complete"]),
            WaitUntil::DomContentLoaded => {
                ("Page.domContentEventFired", &["interactive", "complete"])
            }
            WaitUntil::NetworkIdle => return self.wait_for_network_idle(session_id, rx).await,
            WaitUntil::None => return Ok(()),
        };

        let timeout = tokio::time::Duration::from_millis(self.default_timeout_ms);

        tokio::time::timeout(timeout, async {
            loop {
                // Poll readyState: oxide browser-level listeners may miss session events
                // (and about:blank often completes before the subscription is armed).
                if let Ok(state) = self
                    .client
                    .send_command(
                        "Runtime.evaluate",
                        Some(json!({
                            "expression": "document.readyState",
                            "returnByValue": true
                        })),
                        Some(session_id),
                    )
                    .await
                {
                    let rs = state
                        .get("result")
                        .and_then(|r| r.get("value"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if ready_states.contains(&rs) {
                        return Ok(());
                    }
                }

                match tokio::time::timeout(
                    tokio::time::Duration::from_millis(crate::xdg::policy::policy_u64(
                        crate::xdg::policy::key::CDP_EVENT_DRAIN_POLL_MS,
                    )),
                    rx.recv(),
                )
                .await
                {
                    Ok(Ok(event)) => {
                        let sid_ok = event.session_id.is_none()
                            || event.session_id.as_deref() == Some(session_id);
                        if event.method == event_name && sid_ok {
                            return Ok(());
                        }
                    }
                    Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
                    Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => {
                        // Keep polling readyState until outer timeout.
                    }
                    Err(_) => {
                        // recv timed out — loop and re-check readyState
                    }
                }
            }
        })
        .await
        .map_err(|_| format!("Timeout waiting for {event_name}"))?
    }

    async fn wait_for_network_idle(
        &self,
        session_id: &str,
        rx: &mut broadcast::Receiver<CdpEvent>,
    ) -> Result<(), String> {
        let timeout = tokio::time::Duration::from_millis(self.default_timeout_ms);
        poll_network_idle(session_id, rx, timeout).await
    }

    /// Current URL of the active tab, read from the live page.
    pub async fn get_url(&self) -> Result<String, String> {
        let result = self.evaluate_simple("location.href").await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// Current title of the active tab, read from the live page.
    pub async fn get_title(&self) -> Result<String, String> {
        let result = self.evaluate_simple("document.title").await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// Serialized HTML of the active document.
    pub async fn get_content(&self) -> Result<String, String> {
        let result = self
            .evaluate_simple("document.documentElement.outerHTML")
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// Evaluate JavaScript in the active page and return its value.
    pub async fn evaluate(&self, script: &str, _args: Option<Value>) -> Result<Value, String> {
        let session_id = self.active_session_id()?.to_string();

        let result: EvaluateResult = self
            .client
            .send_command_typed(
                "Runtime.evaluate",
                &EvaluateParams {
                    expression: script.to_string(),
                    return_by_value: Some(true),
                    await_promise: Some(true),
                },
                Some(&session_id),
            )
            .await?;

        if let Some(ref details) = result.exception_details {
            let msg = details
                .exception
                .as_ref()
                .and_then(|e| e.description.as_deref())
                .unwrap_or(&details.text);
            return Err(format!("Evaluation error: {msg}"));
        }

        Ok(result.result.value.unwrap_or(Value::Null))
    }

    async fn evaluate_simple(&self, expression: &str) -> Result<Value, String> {
        self.evaluate(expression, None).await
    }

    /// Wait for a lifecycle milestone on a session the caller names.
    ///
    /// The external form exists for tabs this manager is not currently
    /// focused on, where the active-tab helpers would wait on the wrong page.
    pub async fn wait_for_lifecycle_external(
        &self,
        wait_until: WaitUntil,
        session_id: &str,
    ) -> Result<(), String> {
        let mut rx = self.client.subscribe();
        self.wait_for_lifecycle(wait_until, session_id, &mut rx)
            .await
    }
}
pub(crate) async fn poll_network_idle(
    session_id: &str,
    rx: &mut broadcast::Receiver<CdpEvent>,
    overall_timeout: tokio::time::Duration,
) -> Result<(), String> {
    // Single-task state: no Arc/Mutex (rules: do not wrap local exclusive state
    // in interior mutability — plain `mut FxHashSet` is enough).
    // Trusted request ids from CDP → FxHash (not SipHash DoS surface).
    let mut pending = FxHashSet::<String>::default();

    tokio::time::timeout(overall_timeout, async {
        let mut idle_start: Option<tokio::time::Instant> = None;

        loop {
            let recv_result = tokio::time::timeout(
                tokio::time::Duration::from_millis(crate::xdg::policy::policy_u64(
                    crate::xdg::policy::key::CDP_TARGET_EVENT_WAIT_MS,
                )),
                rx.recv(),
            )
            .await;

            match recv_result {
                Ok(Ok(event))
                    if event.session_id.is_none()
                        || event.session_id.as_deref() == Some(session_id) =>
                {
                    match event.method.as_str() {
                        "Network.requestWillBeSent" => {
                            if let Some(id) = event.params.get("requestId").and_then(|v| v.as_str())
                            {
                                pending.insert(id.to_string());
                                idle_start = None;
                            }
                        }
                        "Network.loadingFinished" | "Network.loadingFailed" => {
                            if let Some(id) = event.params.get("requestId").and_then(|v| v.as_str())
                            {
                                pending.remove(id);
                                if pending.is_empty() {
                                    idle_start = Some(tokio::time::Instant::now());
                                }
                            }
                        }
                        "Page.loadEventFired" if pending.is_empty() => {
                            idle_start = Some(tokio::time::Instant::now());
                        }
                        _ => {}
                    }
                }
                Ok(Ok(_)) => {}
                Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(_)) => break,
                Err(_) => {
                    // Timeout on recv -- if no pending requests, start (or
                    // continue) the idle timer instead of returning
                    // immediately.  This prevents false-positive idle
                    // detection when the subscription starts after the page
                    // has already loaded (e.g. cached pages).
                    if pending.is_empty() && idle_start.is_none() {
                        idle_start = Some(tokio::time::Instant::now());
                    }
                }
            }

            if let Some(start) = idle_start {
                if start.elapsed()
                    >= tokio::time::Duration::from_millis(crate::xdg::policy::policy_u64(
                        crate::xdg::policy::key::CDP_NETWORK_IDLE_SETTLE_MS,
                    ))
                {
                    return Ok(());
                }
            }
        }

        Ok(())
    })
    .await
    .map_err(|_| "Timeout waiting for networkidle".to_string())?
}
