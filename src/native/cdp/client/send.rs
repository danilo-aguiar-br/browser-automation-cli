// SPDX-License-Identifier: MIT OR Apache-2.0

//! CDP send helpers (typed / no-params / no-wait).

use chromiumoxide::error::CdpError;
use chromiumoxide::page::Page;
use serde_json::Value;
use tokio::sync::broadcast;

use super::super::types::CdpEvent;
use super::raw::RawCdpCommand;
use super::types::CdpClient;

impl CdpClient {
    /// Send one CDP command and await its response.
    ///
    /// `session_id` routes to an attached target; `None` addresses the browser.
    ///
    /// # Errors
    ///
    /// Returns the message of `page_for_session` when `session_id` names a
    /// target with no live chromiumoxide `Page`, or the
    /// `format_cdp_err` rendering of the `CdpError` raised by
    /// `Page::execute` / `Browser::execute` — a closed connection, a protocol
    /// error reported by the browser, or a per-command timeout. Callers in
    /// `src/browser` lift this `String` into
    /// [`ErrorKind::Protocol`](crate::error::ErrorKind::Protocol).
    pub async fn send_command(
        &self,
        method: &str,
        params: Option<Value>,
        session_id: Option<&str>,
    ) -> Result<Value, String> {
        let cmd = RawCdpCommand {
            method: method.to_string(),
            params: params.unwrap_or(Value::Null),
        };

        let result = if let Some(sid) = session_id.filter(|s| !s.is_empty()) {
            let page = self.page_for_session(sid).await?;
            page.execute(cmd)
                .await
                .map_err(|e| format_cdp_err(method, &e))?
        } else {
            let browser = self.browser.lock().await;
            browser
                .execute(cmd)
                .await
                .map_err(|e| format_cdp_err(method, &e))?
        };

        Ok(result.result)
    }

    /// Subscribe to the CDP event stream.
    ///
    /// A broadcast receiver only sees events sent AFTER it subscribes, so a
    /// subscriber that starts late misses what already fired.
    pub fn subscribe(&self) -> broadcast::Receiver<CdpEvent> {
        self.event_tx.subscribe()
    }

    /// [`send_command`](Self::send_command) with typed params and result.
    ///
    /// Preferred over the `Value` form: a shape mismatch surfaces here instead
    /// of as a `None` several layers away.
    ///
    /// # Errors
    ///
    /// Fails when `params` does not serialize to JSON, when the underlying
    /// [`send_command`](Self::send_command) fails, or when the CDP result does
    /// not deserialize into `R` — the shape mismatch this wrapper exists to
    /// surface.
    pub async fn send_command_typed<P: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: &P,
        session_id: Option<&str>,
    ) -> Result<R, String> {
        let params_value =
            serde_json::to_value(params).map_err(|e| format!("Failed to serialize params: {e}"))?;
        let result = self
            .send_command(method, Some(params_value), session_id)
            .await?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to deserialize CDP response for {method}: {e}"))
    }

    /// [`send_command`](Self::send_command) for commands that take no arguments.
    ///
    /// # Errors
    ///
    /// Propagates [`send_command`](Self::send_command) unchanged: no live
    /// `Page` for `session_id`, or a `CdpError` from the browser.
    pub async fn send_command_no_params(
        &self,
        method: &str,
        session_id: Option<&str>,
    ) -> Result<Value, String> {
        self.send_command(method, None, session_id).await
    }

    /// Best-effort command (still awaits oxide execute).
    ///
    /// # Errors
    ///
    /// Never returns `Err`. The inner
    /// [`send_command`](Self::send_command) result is discarded on purpose so
    /// a teardown or fire-and-forget command cannot fail the caller; the
    /// `Result` is kept only so call sites read like the awaiting variants.
    pub async fn send_command_no_wait(
        &self,
        method: &str,
        params: Option<Value>,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        let _ = self.send_command(method, params, session_id).await;
        Ok(())
    }

    async fn page_for_session(&self, session_id: &str) -> Result<Page, String> {
        let browser = self.browser.lock().await;
        // Single Browser::pages() list (latency: avoid second CDP round-trip on fallback).
        let pages = browser
            .pages()
            .await
            .map_err(|e| format!("Browser::pages failed: {e}"))?;
        let n = pages.len();
        for page in &pages {
            if page.session_id().as_ref() == session_id {
                return Ok(page.clone());
            }
        }
        // Fallback: first page if only one exists (session id mismatch after attach).
        if n == 1 {
            if let Some(page) = pages.into_iter().next() {
                return Ok(page);
            }
        }
        Err(format!(
            "No chromiumoxide Page for session_id={session_id} (pages={n})"
        ))
    }
}

/// Format a chromiumoxide error for agent-facing messages (Display only → borrow).
pub(crate) fn format_cdp_err(method: &str, e: &CdpError) -> String {
    format!("CDP error ({method}): {e}")
}
