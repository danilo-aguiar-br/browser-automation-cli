// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot CDP browser session (`OneShotSession`) and capture options.
//!
//! Component map (Pass 25 — stdin/stdout agent audit / SRP):
//! - `launch` — Chrome launch + capture domain enable + event pump
//! - `nav` — navigation / init scripts
//! - `record` — interaction recording into `run --script` steps
//! - `content/` — scrape/view, input, eval, PDF/grab, extract (Tier-3 SRP)
//! - `assert_net` — cookies, asserts, console/network buffers
//! - `interact` — input, history, multi-page
//! - `wait_emulate` — waits, pick, emulate, resize
//! - `media` — perf, screencast, heap
//! - `extensions` — extensions, devtools3p, webmcp, shutdown

mod assert_net;
mod content;
mod extensions;
mod interact;
mod launch;
mod media;
mod nav;
mod record;
mod storage;
mod wait_emulate;

use rustc_hash::FxHashMap;

use serde_json::Value;
use tokio::sync::broadcast;

use crate::native::browser::BrowserManager;
use crate::native::cdp::types::CdpEvent;
use crate::native::element::RefMap;

pub use wait_emulate::WaitRequest;

/// Capture toggles for process-local console/network buffers.
#[derive(Debug, Clone, Copy, Default)]
pub struct CaptureOpts {
    /// Buffer `Runtime.consoleAPICalled` into the session console log.
    pub console: bool,
    /// Buffer `Network.requestWillBeSent` into the session network log.
    pub network: bool,
}

/// Drop Chrome-internal schemes from capture-network (agent-ready envelope).
pub(crate) fn is_internal_browser_url(url: &str) -> bool {
    url.starts_with("chrome:")
        || url.starts_with("chrome-extension:")
        || url.starts_with("devtools:")
}

/// Drop non-document noise from capture-network (internal + data/blob embeds).
pub(crate) fn is_noise_network_url(url: &str) -> bool {
    is_internal_browser_url(url) || url.starts_with("data:") || url.starts_with("blob:")
}

/// Headless Chrome session owned by a single CLI invocation (or one `run` script).
pub struct OneShotSession {
    manager: BrowserManager,
    ref_map: RefMap,
    /// Frame id → CDP session id (process-minted keys → FxHashMap).
    iframe_sessions: FxHashMap<String, String>,
    chrome_pid: Option<u32>,
    capture: CaptureOpts,
    event_rx: broadcast::Receiver<CdpEvent>,
    console_log: Vec<Value>,
    network_log: Vec<Value>,
    /// Oldest console entries dropped to hold the capture ring cap.
    console_dropped: u64,
    /// Oldest network entries dropped to hold the capture ring cap.
    network_dropped: u64,
    /// Oldest trace chunks dropped to hold the tracing ring cap.
    ///
    /// Separate from the console and network counters because `Tracing.
    /// dataCollected` arrives in a different order of magnitude — tens of
    /// thousands of events for a few seconds of recording.
    trace_dropped: u64,
    /// Bytes accumulated across `heap_chunks` while a snapshot streams in.
    ///
    /// Tracked rather than derived so the budget can be enforced as the chunks
    /// arrive, instead of after the process has already paid for the memory.
    heap_bytes: usize,
    /// Set when `heap_bytes` crossed the `heap_snapshot_max_bytes` budget.
    ///
    /// A heap snapshot is ONE JSON document that CDP delivers in slices, so it
    /// CANNOT be capped by dropping the oldest chunk the way the console,
    /// network and trace rings are: the surviving text would be syntactically
    /// invalid JSON, written to disk with `ok: true` and a healthy-looking
    /// `bytes`. The buffer therefore stops growing and the read fails closed.
    heap_overflow: bool,
    perf_active: bool,
    screencast_active: bool,
    heap_chunks: Vec<String>,
    trace_chunks: Vec<String>,
    /// Last written trace path from `perf stop` (for offline insight).
    last_trace_path: Option<std::path::PathBuf>,
    /// In-memory NDJSON of last trace (cleared after stop unless kept for insight).
    last_trace_body: Option<String>,
    /// PNG base64 frames from Page.screencastFrame.
    screencast_frames: Vec<String>,
    /// Oldest screencast frames dropped to hold the frame ring cap.
    ///
    /// This buffer was capped long before the console, network and trace rings
    /// were, and the audit plan cited it as "the correct pattern" to copy when
    /// they got their ceilings. Measured 2026-08-30: it violated both halves of
    /// the rule that wave added. It kept the OLDEST frames and discarded every
    /// frame after the cap, and it reported neither fact — so a recording past
    /// the ceiling ended early while the envelope answered `ok: true` with a
    /// healthy `frames_buffered`. This counter is the half that was missing.
    screencast_dropped: u64,
    /// Output directory for screencast frames (set on start).
    screencast_dir: Option<std::path::PathBuf>,
    /// Pending screencast frame sessionIds awaiting ack.
    screencast_ack_ids: Vec<i64>,
    /// CDP session ids with a JS dialog currently open (alert/confirm/prompt).
    ///
    /// **Per page, not per session (GAP-041).** A CDP session id maps 1:1 to a
    /// page/target, so a dialog on one tab must not block commands on another.
    /// The previous session-wide boolean conflated the two.
    dialog_open: FxHashMap<String, bool>,
    /// After a successful answer, ignore late `javascriptDialogOpening` until
    /// `javascriptDialogClosed` (or settle budget) so the next step is not
    /// blocked by a stale Opening still in the broadcast queue (GAP-054).
    dialog_suppress_open: FxHashMap<String, bool>,
    /// HeapProfiler.reportHeapSnapshotProgress finished=true observed.
    heap_snapshot_finished: bool,
    /// Tracing.tracingComplete observed after perf stop.
    tracing_complete: bool,
    /// Ring of console buffers from prior navigations in this process (max 3).
    console_preserved: Vec<Vec<Value>>,
    /// Ring of network buffers from prior navigations in this process (max 3).
    network_preserved: Vec<Vec<Value>>,
    /// Extension ids loaded via --load-extension in this session (for uninstall effect).
    loaded_extension_ids: Vec<String>,
    /// Named BrowserContext ids (tool-ref isolatedContext string names; GAP-004).
    /// Named isolated world → context id (process-minted → FxHashMap).
    named_contexts: FxHashMap<String, String>,
    /// Last `Input.dragIntercepted` payload: the DataTransfer the page built in
    /// its own `dragstart` handler (GAP-030).
    drag_intercepted: Option<Value>,
    /// In-flight network requests, tracked for `wait --network-idle` (GAP-032).
    /// Counted unconditionally: `--capture-network` governs the request *log*,
    /// not whether the browser is busy.
    net_inflight: i64,
    /// Total requests started in this process; lets a caller detect "a request
    /// happened" rather than only "nothing is in flight right now".
    net_started: u64,
    /// Monotonic tick of the last network start/finish/failure.
    net_last_activity: Option<std::time::Instant>,
    /// Last device metrics applied in this one-shot (width, height, scale, mobile).
    ///
    /// `emulate` with only `screen` reuses this pair so CDP can attach
    /// `screenWidth`/`screenHeight` without resetting a prior resize.
    last_device_metrics: (i32, i32, f64, bool),
}
