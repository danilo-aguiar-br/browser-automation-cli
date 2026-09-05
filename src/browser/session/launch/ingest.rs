// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP event ingestion into the one-shot capture buffers.
//!
//! Split out of `state` when `state` crossed the 300-line production ceiling.
//! The seam is the one the file already described: `state` arms the CDP
//! domains and pumps the channel, while everything here turns one event into
//! buffer content. The cut follows that responsibility, not the line number
//! that forced it.

use serde_json::{json, Value};

use crate::native::cdp::types::CdpEvent;
use crate::native::network;

use super::super::{is_noise_network_url, OneShotSession};

mod rings;

#[cfg(test)]
mod tests;

impl OneShotSession {
    /// Copy `status` and `mimeType` from a CDP response onto the newest capture
    /// record carrying this `requestId`.
    ///
    /// Shared by two arms because CDP answers a request in two different
    /// places: an ordinary request through `Network.responseReceived`, and a
    /// redirect hop through the `redirectResponse` of the NEXT
    /// `requestWillBeSent` for the same id. Writing the copy twice would let
    /// the two paths drift, and a drifted path here is invisible — the record
    /// simply lacks a field, with `ok: true`.
    fn enrich_with_response(&mut self, request_id: &str, response: &Value) {
        let status = response.get("status").and_then(Value::as_u64);
        let mime = response
            .get("mimeType")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        if status.is_none() && mime.is_none() {
            return;
        }
        // A response lands close behind its request, so scan from the end.
        let Some(entry) = self
            .network_log
            .iter_mut()
            .rev()
            .find(|e| e.get("requestId").and_then(|v| v.as_str()) == Some(request_id))
        else {
            return;
        };
        let Some(obj) = entry.as_object_mut() else {
            return;
        };
        if let Some(status) = status {
            obj.insert("status".to_string(), json!(status));
        }
        if let Some(mime) = mime {
            obj.insert("mimeType".to_string(), json!(mime));
        }
    }

    /// Ingest one CDP event by shared reference (fields are only read / selectively cloned).
    pub(super) fn ingest_event(&mut self, evt: &CdpEvent) {
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
                Self::push_capped(
                    &mut self.console_log,
                    &mut self.console_dropped,
                    json!({
                        "type": level,
                        "text": text,
                        "args": raw_args,
                    }),
                );
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
                // A redirect hop is answered inside the NEXT requestWillBeSent
                // for the SAME requestId, as `redirectResponse`; the hop never
                // gets a `responseReceived` of its own. Measured 2026-08-28 on
                // `https://www.rust-lang.org`: without this the first leg of
                // the redirect was the single record of 25 with no `status`,
                // and it is the row a caller inspects first.
                if let Some(redirect) = evt.params.get("redirectResponse") {
                    let redirect = redirect.clone();
                    self.enrich_with_response(request_id, &redirect);
                }
                // The resource type sits at the TOP level of
                // `Network.requestWillBeSent`, a sibling of `request` rather
                // than a field inside it. Reading it from `request` is why
                // `--resource-types` filtered on a key nothing ever wrote and
                // answered zero for every value on every page, with ok:true.
                //
                // The protocol types it `Option<ResourceType>`, so absence is
                // ordinary. It lands as `Other` and never as a missing key: a
                // record without the key is dropped silently by any filter, and
                // that silent drop is the whole defect being closed here.
                let resource_type = evt
                    .params
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or(crate::constants::DEFAULT_RESOURCE_TYPE);
                Self::push_capped(
                    &mut self.network_log,
                    &mut self.network_dropped,
                    json!({
                        "requestId": request_id,
                        "method": method,
                        "url": url,
                        "resourceType": resource_type,
                    }),
                );
            }
            // `status` and `mimeType` belong to the RESPONSE, and the request
            // event cannot carry them. The mitm consumer read `status` off the
            // request log for the whole life of the feature and nothing ever
            // wrote it, so `mitm capture-url` emitted `status: null` with
            // ok:true. Enriching the existing record by `requestId` keeps one
            // record per exchange, which `net get` and the HAR writer both
            // assume, instead of adding a second parallel log that would drift.
            "Network.responseReceived" => {
                if !self.capture.network {
                    return;
                }
                let Some(request_id) = evt.params.get("requestId").and_then(|v| v.as_str()) else {
                    return;
                };
                if let Some(response) = evt.params.get("response") {
                    let response = response.clone();
                    self.enrich_with_response(request_id, &response);
                }
            }
            "HeapProfiler.addHeapSnapshotChunk" => {
                if let Some(chunk) = evt.params.get("chunk").and_then(|v| v.as_str()) {
                    // Budget the snapshot as it streams, and REFUSE past it —
                    // never truncate. The chunks concatenate into a single JSON
                    // document (`heap.rs` joins with ""), so dropping the oldest
                    // would emit syntactically invalid JSON to disk with
                    // `ok: true` and a plausible byte count. `take` turns the
                    // flag into an explicit error instead.
                    //
                    // The knob is `heap_snapshot_max_bytes`, which already
                    // existed and was enforced only when READING a snapshot off
                    // disk — the ceiling sat on the side that cannot exhaust
                    // memory, while the side that can had none.
                    if self.heap_overflow {
                        return;
                    }
                    let budget = usize::try_from(crate::xdg::policy::policy_u64(
                        crate::xdg::policy::key::HEAP_SNAPSHOT_MAX_BYTES,
                    ))
                    .unwrap_or(usize::MAX);
                    let next = self.heap_bytes.saturating_add(chunk.len());
                    if budget > 0 && next > budget {
                        self.heap_overflow = true;
                        // Release what was accumulated: it can no longer be
                        // completed, so holding it only keeps the memory this
                        // ceiling exists to bound.
                        self.heap_chunks = Vec::new();
                        self.heap_bytes = 0;
                        return;
                    }
                    self.heap_bytes = next;
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
                            Self::push_capped_str(
                                &mut self.trace_chunks,
                                &mut self.trace_dropped,
                                serde_json::to_string(item).unwrap_or_default(),
                            );
                        }
                    } else {
                        Self::push_capped_str(
                            &mut self.trace_chunks,
                            &mut self.trace_dropped,
                            serde_json::to_string(value).unwrap_or_default(),
                        );
                    }
                }
            }
            "Tracing.tracingComplete" => {
                self.tracing_complete = true;
            }
            "Page.screencastFrame" => {
                if let Some(data) = evt.params.get("data").and_then(|v| v.as_str()) {
                    // A RING, not a wall. This was `if len < CAP { push }`, which
                    // kept the OLDEST frames and silently discarded every frame
                    // after the ceiling: a long recording ended early while the
                    // envelope reported success, which is indistinguishable from
                    // a page that simply stopped changing.
                    //
                    // The audit plan cited this very site as "the correct pattern
                    // already in the SAME file" for the console and network rings,
                    // and the wave that copied it added the rule those rings now
                    // follow: drop the OLDEST, and DECLARE the truncation. The
                    // model violated both halves, because it was written before
                    // the rule existed and nothing re-read it against the rule.
                    // It is now the same `cap_ring` its own imitators use.
                    self.screencast_frames.push(data.to_string());
                    Self::cap_ring(
                        &mut self.screencast_frames,
                        &mut self.screencast_dropped,
                        crate::constants::SCREENCAST_FRAME_BUFFER_CAP,
                    );
                }
                if let Some(sid) = evt.params.get("sessionId").and_then(|v| v.as_i64()) {
                    // NEVER capped, and that is not an oversight. `Page.
                    // screencastFrameAck` is what keeps Chrome sending, so a
                    // dropped ack stalls the stream instead of bounding memory —
                    // the ceiling would cost the whole feature to save a few
                    // kilobytes. `pump_events` drains this list IN FULL on every
                    // call, so it holds only the ids that arrived between two
                    // pumps, never one per frame of the session. An ack is still
                    // queued for a frame the ring above dropped: Chrome is owed
                    // the acknowledgement whether or not this process kept the
                    // bytes.
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
