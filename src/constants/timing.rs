// SPDX-License-Identifier: MIT OR Apache-2.0
//! Event-pump slices, UI settle delays, perf trace polling, and state collection.

/// Default event-pump / wait slice (milliseconds) for screencast-aware waits.
///
/// Operator override: XDG `config set event_pump_slice_ms <n>` (`> 0`).
pub const DEFAULT_EVENT_PUMP_SLICE_MS: u64 = 50;

/// Eval drain slice (milliseconds) while waiting for Runtime.evaluate results.
///
/// Capped by [`DEFAULT_EVENT_PUMP_SLICE_MS`] / XDG `event_pump_slice_ms` at use sites.
pub const DEFAULT_EVAL_DRAIN_SLICE_MS: u64 = 40;

/// UI interact settle delay after click/type/extension load (milliseconds).
///
/// Operator override: XDG `config set interact_settle_ms <n>` (`> 0`).
pub const DEFAULT_INTERACT_SETTLE_MS: u64 = 200;

/// Short support-thread settle (milliseconds) for sync helpers.
pub const DEFAULT_SUPPORT_SETTLE_MS: u64 = 80;

/// Navigation micro-settle (milliseconds) after some page transitions.
pub const DEFAULT_NAV_MICRO_SETTLE_MS: u64 = 100;

/// Perf auto-stop settle after load/reload (milliseconds).
pub const DEFAULT_PERF_AUTOSTOP_SETTLE_MS: u64 = 500;

/// Perf trace poll inner slice (milliseconds).
pub const DEFAULT_PERF_TRACE_INNER_SLICE_MS: u64 = 20;

/// Perf trace outer poll interval (milliseconds).
pub const DEFAULT_PERF_TRACE_OUTER_SLICE_MS: u64 = 50;

/// Perf trace outer poll max iterations (~5s at default outer slice).
pub const DEFAULT_PERF_TRACE_OUTER_ITERS: u32 = 100;

/// Perf trace inner drain iterations after complete.
pub const DEFAULT_PERF_TRACE_INNER_ITERS: u32 = 5;

/// Budget for `Input.dragIntercepted` to arrive after a real mouse drag gesture
/// (milliseconds). Exceeding it means the browser never armed drag interception,
/// which downgrades the drag to a synthetic mouse gesture with an explicit
/// warning rather than a silent false positive (GAP-030).
pub const DRAG_INTERCEPT_BUDGET_MS: u64 = 1_500;

/// Default quiet window for `wait --network-idle` (milliseconds): how long the
/// in-flight request count must stay at zero before the page counts as idle.
pub const DEFAULT_NETWORK_IDLE_WINDOW_MS: u64 = 500;

/// Default quiet window for `wait --dom-stable-ms` (milliseconds).
pub const DEFAULT_DOM_STABLE_WINDOW_MS: u64 = 500;

/// Max wait after `Page.handleJavaScriptDialog` for `Page.javascriptDialogClosed`
/// (milliseconds). GAP-054: clear optimistically but suppress stale Opening until
/// Closed (or this budget). Override: XDG `config set dialog_settle_ms <n>`.
pub const DEFAULT_DIALOG_SETTLE_MS: u64 = 2_000;

/// Budget for `Page.enable` / domain prep during tab switch (milliseconds).
///
/// A page-modal JS dialog can stall domain enable on the owner target; tab switch
/// treats domain enable as best-effort under this budget so
/// `handleJavaScriptDialog` can still target the newly active session (GAP-041).
pub const TAB_SWITCH_DOMAIN_ENABLE_BUDGET_MS: u64 = 2_000;

/// CDP storage collect outer deadline (seconds).
pub const STATE_COLLECT_DEADLINE_SECS: u64 = 5;
/// CDP storage event recv slice (seconds).
pub const STATE_EVENT_RECV_SECS: u64 = 2;
/// Settle delay after load_state navigation (milliseconds).
pub const STATE_LOAD_SETTLE_MS: u64 = 500;
