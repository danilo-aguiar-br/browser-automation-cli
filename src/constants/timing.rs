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

/// Intermediate mouse positions synthesized for one HTML5 drag gesture.
///
/// Lived next to its use site while its sibling `DRAG_INTERCEPT_BUDGET_MS` was
/// promoted here, which left one half of the same gesture tunable and the other
/// half frozen. Both belong to the same budget family.
pub const DRAG_MOVE_STEPS: u64 = 6;

/// Delay between synthesized drag positions (milliseconds).
///
/// Roughly one animation frame. Sites that throttle `dragover` need a wider gap,
/// which is why it is a knob and not a literal.
pub const DRAG_MOVE_GAP_MS: u64 = 16;

/// Intermediate pointer positions synthesized for one move under `human`.
///
/// Generalizes [`DRAG_MOVE_STEPS`], which tuned the same behaviour for `drag`
/// alone. Higher than the drag default because a click travels farther than a
/// drag hand-off and the renderer coalesces back-to-back moves into one hop.
pub const INPUT_MOVE_STEPS: u64 = 24;

/// Delay between synthesized pointer positions (milliseconds).
///
/// Generalizes [`DRAG_MOVE_GAP_MS`]. Below one animation frame the renderer
/// coalesces the moves and the interpolation is discarded before the page sees
/// it, which defeats the whole point of interpolating.
pub const INPUT_MOVE_GAP_MS: u64 = 12;

/// Hold time between `mousePressed` and `mouseReleased` (milliseconds).
///
/// Zero dwell puts press and release in the same millisecond at the same pixel,
/// which no hand produces.
pub const INPUT_CLICK_DWELL_MS: u64 = 65;

/// Hold time between `keyDown` and `keyUp` (milliseconds).
///
/// Also fixes keys that need a real hold: repeat, game shortcuts, and handlers
/// that measure press duration.
pub const INPUT_KEY_DWELL_MS: u64 = 45;

/// Delay between characters while typing (milliseconds).
///
/// The `delay_ms` parameter of `type_text` has always existed and defaulted to
/// zero with no flag able to reach it; this is the value that finally feeds it.
pub const INPUT_TYPE_DELAY_MS: u64 = 95;

/// Scroll distance carried by one synthesized wheel tick (CSS pixels).
///
/// A wheel notch, not the whole delta: one giant tick is as unlike a human as no
/// wheel event at all.
pub const INPUT_SCROLL_TICK_PX: u64 = 100;

/// Ceiling on the number of wheel ticks one scroll gesture may synthesize.
///
/// Without a ceiling the tick count is `delta / INPUT_SCROLL_TICK_PX`, and each
/// tick costs one CDP round trip, so the cost of a scroll grows linearly with
/// the distance asked for. A `--delta-y 100000` became 1000 round trips and
/// exhausted the command timeout — the caller reads a timeout and concludes the
/// page is slow, when the cost was self-inflicted.
///
/// Past the ceiling the ticks simply carry more pixels each. The split already
/// distributes the delta proportionally, so capping the count changes the
/// texture of the gesture and never its total travel.
///
/// Override: XDG `input_scroll_max_ticks`. Raise it when a page needs finer
/// granularity than one long gesture provides.
pub const INPUT_SCROLL_MAX_TICKS: u64 = 40;

/// Radius of the random offset applied to a click target (CSS pixels).
///
/// Without it every click on the same element lands on the exact geometric
/// centre, so N clicks produce N pixel-identical coordinates.
pub const INPUT_TARGET_JITTER_PX: u64 = 3;

/// Extra rounds allowed to deliver a wheel delta the renderer dropped.
///
/// A `mouseWheel` dispatched before the renderer's hit-test tree is ready is
/// acknowledged and then discarded, so the ack proves nothing (crbug 444929150).
/// Measured 2026-08-06 on a freshly navigated page: `--delta-y 400` landed the
/// full 400 px in 3 of 5 runs and 300 px in the other 2, with the loss always
/// one whole tick. Re-reading the offset and re-sending only the shortfall is
/// what turns that into a deterministic result; a fixed sleep would trade the
/// same race for a slower one.
pub const INPUT_SCROLL_SETTLE_ROUNDS: u64 = 3;

/// Default per-operation timeout for the Chrome engine (milliseconds).
///
/// The Lightpanda engine has always exposed `lightpanda_session_timeout_secs`
/// while Chrome — the default engine — had this frozen at its call site. Same
/// class of budget, same right to be tuned.
pub const CHROME_DEFAULT_TIMEOUT_MS: u64 = 25_000;

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
