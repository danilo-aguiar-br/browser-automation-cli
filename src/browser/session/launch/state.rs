// SPDX-License-Identifier: MIT OR Apache-2.0
//! Post-launch session state: capture domains, dialog tracking, event pump.

use serde_json::{json, Value};
use tokio::sync::broadcast;

use crate::error::{CliError, ErrorKind};
use crate::native::cdp::types::CdpEvent;
use crate::native::network;

use super::super::{is_noise_network_url, CaptureOpts, OneShotSession};

impl OneShotSession {
    /// OS process id of the launched Chrome, when known.
    pub fn chrome_pid(&self) -> Option<u32> {
        self.chrome_pid
    }

    /// Capture toggles used when this session was launched.
    pub fn capture(&self) -> CaptureOpts {
        self.capture
    }

    pub(super) async fn enable_capture_domains(&mut self) -> Result<(), CliError> {
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();

        // Always enable Page domain for dialogs/screencast and attach page-session listeners
        // (heap chunks, screencast frames, JS dialogs are target-scoped).
        let _ = self
            .manager
            .client
            .send_command_no_params("Page.enable", Some(&session_id))
            .await;
        let _ = self.manager.client.attach_page_session_forwarders().await;

        if self.capture.console {
            self.manager
                .client
                .send_command_no_params("Runtime.enable", Some(&session_id))
                .await
                .map_err(|e| CliError::new(ErrorKind::Protocol, format!("Runtime.enable: {e}")))?;
            // Page-level console listeners (context7): complements browser-level forwarder.
            let _ = self.manager.client.attach_page_console_forwarders().await;
        }
        if self.capture.network {
            self.manager
                .client
                .send_command_no_params("Network.enable", Some(&session_id))
                .await
                .map_err(|e| CliError::new(ErrorKind::Protocol, format!("Network.enable: {e}")))?;
            // Also enable at browser scope (no session) when available.
            let _ = self
                .manager
                .client
                .send_command_no_params("Network.enable", None)
                .await;
            let _ = self.manager.client.attach_page_network_forwarders().await;
        }
        Ok(())
    }

    /// Dialog-map key for an event, falling back to the active page.
    ///
    /// Browser-scope events arrive without a session id; attributing them to the
    /// active page is the only reading that keeps the guard useful, and it is the
    /// conservative one (it blocks rather than silently allows).
    fn dialog_key_for(&self, session_id: Option<&str>) -> String {
        dialog_map_key(session_id, self.manager.active_session_id().ok())
    }

    /// True when a JavaScript dialog is open on the active page (GAP-041).
    ///
    /// Drains pending events first so the answer reflects dialogs that opened
    /// since the last command.
    pub fn dialog_open_on_active_page(&mut self) -> bool {
        self.drain_events();
        let key = self.dialog_key_for(None);
        self.dialog_open.get(&key).copied().unwrap_or(false)
    }

    /// Clear the dialog flag for the active page (navigation dismisses dialogs).
    pub fn clear_active_page_dialog(&mut self) {
        let key = self.dialog_key_for(None);
        self.dialog_open.remove(&key);
        self.dialog_suppress_open.remove(&key);
    }

    /// Record that a dialog is open on the active page.
    ///
    /// Used when an interaction result reports the dialog synchronously, so the
    /// guard does not have to race `Page.javascriptDialogOpening`.
    pub fn mark_active_page_dialog_open(&mut self) {
        let key = self.dialog_key_for(None);
        // A new dialog must re-arm the guard even if a prior answer was settling.
        self.dialog_suppress_open.remove(&key);
        self.dialog_open.insert(key, true);
    }

    /// After a successful `Page.handleJavaScriptDialog`, clear the open flag and
    /// suppress stale `javascriptDialogOpening` until Closed (GAP-054).
    ///
    /// Returns whether `Page.javascriptDialogClosed` was observed within the
    /// settle budget (agent-visible `dialog_settled`).
    pub async fn settle_after_dialog_answer(&mut self) -> bool {
        let key = self.dialog_key_for(None);
        self.dialog_open.remove(&key);
        self.dialog_suppress_open.insert(key.clone(), true);

        let budget = std::time::Duration::from_millis(crate::xdg::resolve_dialog_settle_ms());
        let slice = std::time::Duration::from_millis(crate::xdg::resolve_event_pump_slice_ms());
        let deadline = std::time::Instant::now() + budget;
        loop {
            self.pump_events().await;
            // Closed clears suppress in ingest_event.
            if !self.dialog_suppress_open.contains_key(&key) {
                return true;
            }
            if std::time::Instant::now() >= deadline {
                // Keep suppress so late Opening cannot re-block the next step.
                self.dialog_open.remove(&key);
                return false;
            }
            tokio::time::sleep(slice).await;
        }
    }

    /// Merge console/network buffers into a result JSON when capture flags are on.
    pub fn with_capture_fields(&mut self, mut data: Value) -> Value {
        self.drain_events();
        if let Some(obj) = data.as_object_mut() {
            if self.capture.console {
                obj.insert("console".to_string(), json!(self.console_log.clone()));
                obj.insert("console_count".to_string(), json!(self.console_log.len()));
            }
            if self.capture.network {
                obj.insert("network".to_string(), json!(self.network_log.clone()));
                obj.insert("network_count".to_string(), json!(self.network_log.len()));
            }
        }
        data
    }

    /// Drain pending CDP events into local buffers (non-blocking).
    pub fn drain_events(&mut self) {
        loop {
            match self.event_rx.try_recv() {
                Ok(evt) => self.ingest_event(&evt),
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
                Err(broadcast::error::TryRecvError::Closed) => break,
            }
        }
    }

    /// Drain events and ack screencast frames (required or Chrome stops sending).
    pub async fn pump_events(&mut self) {
        self.drain_events();
        let acks: Vec<i64> = self.screencast_ack_ids.drain(..).collect();
        if acks.is_empty() {
            return;
        }
        let session_id = self.manager.active_session_id().ok().map(|s| s.to_string());
        for sid in acks {
            let _ = self
                .manager
                .client
                .send_command(
                    "Page.screencastFrameAck",
                    Some(json!({ "sessionId": sid })),
                    session_id.as_deref(),
                )
                .await;
        }
    }

    /// Ingest one CDP event by shared reference (fields are only read / selectively cloned).
    fn ingest_event(&mut self, evt: &CdpEvent) {
        match evt.method.as_str() {
            "Runtime.consoleAPICalled" if self.capture.console => {
                let level = evt
                    .params
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("log")
                    .to_string();
                let raw_args: Vec<Value> = evt
                    .params
                    .get("args")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let text = network::format_console_args(&raw_args);
                self.console_log.push(json!({
                    "type": level,
                    "text": text,
                    "args": raw_args,
                }));
            }
            // GAP-030: the page's own DataTransfer, captured while
            // Input.setInterceptDrags is armed. Kept unconditionally: it is only
            // ever produced by a drag this process started.
            "Input.dragIntercepted" => {
                self.drag_intercepted = Some(evt.params.clone());
            }
            // GAP-032: network quiet tracking. Counted regardless of
            // `--capture-network`, which only governs the request *log*.
            "Network.loadingFinished" | "Network.loadingFailed" => {
                self.net_inflight = self.net_inflight.saturating_sub(1).max(0);
                self.net_last_activity = Some(std::time::Instant::now());
            }
            "Network.requestWillBeSent" => {
                self.net_inflight += 1;
                self.net_started += 1;
                self.net_last_activity = Some(std::time::Instant::now());
                if !self.capture.network {
                    return;
                }
                let request = evt.params.get("request").cloned().unwrap_or(Value::Null);
                let method = request
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("GET");
                let url = request.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if is_noise_network_url(url) {
                    return;
                }
                let request_id = evt
                    .params
                    .get("requestId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.network_log.push(json!({
                    "requestId": request_id,
                    "method": method,
                    "url": url,
                }));
            }
            "HeapProfiler.addHeapSnapshotChunk" => {
                if let Some(chunk) = evt.params.get("chunk").and_then(|v| v.as_str()) {
                    self.heap_chunks.push(chunk.to_string());
                }
            }
            "HeapProfiler.reportHeapSnapshotProgress" => {
                if evt
                    .params
                    .get("finished")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    self.heap_snapshot_finished = true;
                }
            }
            "Tracing.dataCollected" => {
                if let Some(value) = evt.params.get("value") {
                    // CDP sends an array of events; store as one NDJSON line (or expand).
                    if let Some(arr) = value.as_array() {
                        for item in arr {
                            self.trace_chunks
                                .push(serde_json::to_string(item).unwrap_or_default());
                        }
                    } else {
                        self.trace_chunks
                            .push(serde_json::to_string(value).unwrap_or_default());
                    }
                }
            }
            "Tracing.tracingComplete" => {
                self.tracing_complete = true;
            }
            "Page.screencastFrame" => {
                if let Some(data) = evt.params.get("data").and_then(|v| v.as_str()) {
                    // Cap buffer to avoid unbounded memory in long screencasts.
                    if self.screencast_frames.len() < 600 {
                        self.screencast_frames.push(data.to_string());
                    }
                }
                if let Some(sid) = evt.params.get("sessionId").and_then(|v| v.as_i64()) {
                    self.screencast_ack_ids.push(sid);
                }
            }
            // GAP-041: track dialogs per page. `CdpEvent::session_id` is the CDP
            // session, which maps 1:1 to a target, so a dialog on one tab no
            // longer blocks commands on another.
            "Page.javascriptDialogOpening" => {
                let key = self.dialog_key_for(evt.session_id.as_deref());
                // GAP-054: after handleJavaScriptDialog, Opening may still be in
                // the broadcast queue; re-arming would block the next step.
                if self
                    .dialog_suppress_open
                    .get(&key)
                    .copied()
                    .unwrap_or(false)
                {
                    return;
                }
                self.dialog_open.insert(key, true);
            }
            "Page.javascriptDialogClosed" => {
                let key = self.dialog_key_for(evt.session_id.as_deref());
                self.dialog_open.remove(&key);
                self.dialog_suppress_open.remove(&key);
            }
            // GAP-A012: unknown / extra CDP events (e.g. *ExtraInfo on modern Chrome) are
            // intentionally ignored so network/console capture is not aborted.
            _ => {}
        }
    }
}

/// Per-page dialog map key (GAP-041 multi-tab).
///
/// CDP event bodies for `Page.javascriptDialogOpening` / `Closed` do not carry
/// `sessionId` (docs.rs chromiumoxide_cdp). The page-scoped forwarder stamps
/// `CdpEvent.session_id` from `Page::session_id`; browser-scope events pass
/// `None` and fall back to the active session (conservative: block rather than
/// silently allow).
pub(crate) fn dialog_map_key(
    event_session_id: Option<&str>,
    active_session_id: Option<&str>,
) -> String {
    event_session_id
        .map(str::to_string)
        .or_else(|| active_session_id.map(str::to_string))
        .unwrap_or_default()
}

#[cfg(test)]
mod dialog_map_key_tests {
    use super::dialog_map_key;
    use std::collections::HashMap;

    #[test]
    fn two_session_ids_produce_distinct_keys() {
        let k0 = dialog_map_key(Some("sess-tab-a"), Some("sess-active"));
        let k1 = dialog_map_key(Some("sess-tab-b"), Some("sess-active"));
        assert_eq!(k0, "sess-tab-a");
        assert_eq!(k1, "sess-tab-b");
        assert_ne!(
            k0, k1,
            "multi-tab isolation requires distinct keys per session"
        );
    }

    #[test]
    fn missing_event_session_falls_back_to_active() {
        assert_eq!(dialog_map_key(None, Some("sess-active")), "sess-active");
    }

    #[test]
    fn both_none_yields_empty_key() {
        assert_eq!(dialog_map_key(None, None), "");
    }

    /// Harness: dialog open on S0 must not report open when active is S1.
    #[test]
    fn open_map_isolation_two_session_ids() {
        let mut dialog_open: HashMap<String, bool> = HashMap::new();
        let s0 = dialog_map_key(Some("s0"), Some("s0"));
        let s1 = dialog_map_key(Some("s1"), Some("s1"));
        dialog_open.insert(s0, true);
        let active_key = dialog_map_key(None, Some("s1"));
        assert_eq!(active_key, s1);
        assert!(
            !dialog_open.get(&active_key).copied().unwrap_or(false),
            "dialog on s0 must not block active s1"
        );
        assert!(dialog_open.get("s0").copied().unwrap_or(false));
    }
}
