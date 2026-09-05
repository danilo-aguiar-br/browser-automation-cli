// SPDX-License-Identifier: MIT OR Apache-2.0
//! Native wheel gesture: aim, dispatch, and converge on the requested delta.
//!
//! Split from [`mod@super::scroll`] so the scripted `scrollBy` path and the real
//! `mouseWheel` path cannot share helpers by accident. That sharing is how the
//! original defect survived: measuring and scrolling lived in one JS function,
//! so making it also scroll was one line and no caller could tell.

use super::kinematics;
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use serde_json::Value;

use super::scroll::{call_on, MEASURE_FN};

/// Centre of the **visible** part of the scrolling box, in viewport coordinates.
///
/// A wheel event is delivered to whatever sits under the cursor, so the tick has
/// to be aimed at the target container rather than at the origin.
///
/// The rect is intersected with the viewport first, and that step is the whole
/// point. `getBoundingClientRect()` on the document scrolling element reports the
/// **document** box, so on a 4000 px page its centre is y≈2000 against a 720 px
/// viewport: every tick was dispatched 1300 px below anything visible. Chrome
/// then routed part of the gesture straight to the compositor and dropped the
/// rest, so the page saw zero `wheel` events while `scrollTop` still moved --
/// a partial scroll that reported `ok: true`. Any container taller than the
/// viewport reproduced it. Measured 2026-08-06: `--delta-y 400` moved 300 px
/// and `window` received 0 `wheel` events.
const CENTER_FN: &str = r#"function() {
    const vw = window.innerWidth || 0;
    const vh = window.innerHeight || 0;
    const r = this.getBoundingClientRect ? this.getBoundingClientRect() : null;
    if (r && r.width > 0 && r.height > 0) {
        const left = Math.max(r.left, 0);
        const top = Math.max(r.top, 0);
        const right = Math.min(r.right, vw);
        const bottom = Math.min(r.bottom, vh);
        if (right > left && bottom > top) {
            return { x: (left + right) / 2, y: (top + bottom) / 2 };
        }
    }
    return { x: vw / 2, y: vh / 2 };
}"#;

/// [`MEASURE_FN`] deferred to after the next produced frame.
///
/// A plain `Runtime.callFunctionOn` can land between the wheel dispatch and the
/// compositor commit, reporting the pre-scroll offset. The settle loop then reads
/// a shortfall that does not exist and re-sends the delta, so a 100 px request
/// scrolls 200 px. Two nested `requestAnimationFrame` callbacks put the read
/// after a committed frame, which fixes the read side of the same race the retry
/// loop fixes on the write side. The `setTimeout` arm is not a pacing delay: a
/// backgrounded document never runs rAF at all, and without it the promise would
/// never resolve. The bound reuses `nav_micro_settle_ms` rather than adding a
/// knob: it is the same question -- how long to wait on a renderer that may
/// produce nothing -- and a second key for it would drift from the first.
const MEASURE_SETTLED_FN: &str = r#"function(fallbackMs) {
    const snap = () => ({
        x: this.scrollLeft,
        y: this.scrollTop,
        scrollHeight: this.scrollHeight,
        clientHeight: this.clientHeight,
        scrollWidth: this.scrollWidth,
        clientWidth: this.clientWidth,
    });
    return new Promise((resolve) => {
        let frames = 0;
        const tick = () => {
            if (++frames >= 2) { resolve(snap()); return; }
            requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
        setTimeout(() => resolve(snap()), fallbackMs);
    });
}"#;

/// Read the offset only once the renderer has committed a frame.
async fn measure_settled(
    client: &CdpClient,
    session_id: &str,
    object_id: &str,
) -> Result<Value, String> {
    let fallback_ms =
        crate::xdg::policy::policy_u64(crate::xdg::policy::key::DEFAULT_NAV_MICRO_SETTLE_MS);
    let result: Value = client
        .send_command_typed(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: MEASURE_SETTLED_FN.to_string(),
                object_id: Some(object_id.to_string()),
                arguments: Some(vec![CallArgument {
                    value: Some(serde_json::json!(fallback_ms)),
                    object_id: None,
                }]),
                return_by_value: Some(true),
                await_promise: Some(true),
            },
            Some(session_id),
        )
        .await?;
    if let Some(text) = result
        .pointer("/exceptionDetails/text")
        .and_then(|v| v.as_str())
    {
        return Err(format!("scroll failed in page: {text}"));
    }
    result
        .pointer("/result/value")
        .cloned()
        .ok_or_else(|| "scroll helper returned no value".to_string())
}

/// Read `x` / `y` out of a [`MEASURE_FN`] result.
fn xy(v: &Value) -> (f64, f64) {
    (
        v.get("x").and_then(Value::as_f64).unwrap_or_default(),
        v.get("y").and_then(Value::as_f64).unwrap_or_default(),
    )
}

/// Send one round of wheel ticks at a fixed viewport point.
async fn send_ticks(
    client: &CdpClient,
    session_id: &str,
    k: &mut kinematics::Kinematics,
    at: (f64, f64),
    ticks: Vec<(f64, f64)>,
) -> Result<(), String> {
    for (dx, dy) in ticks {
        client
            .send_command_typed::<_, Value>(
                "Input.dispatchMouseEvent",
                &DispatchMouseEventParams {
                    event_type: "mouseWheel".to_string(),
                    x: at.0,
                    y: at.1,
                    button: None,
                    buttons: None,
                    click_count: None,
                    // Both axes travel on every tick: CDP drops a mouseWheel that
                    // supplies only one delta, so omitting the idle axis makes the
                    // whole gesture a no-op.
                    delta_x: Some(dx),
                    delta_y: Some(dy),
                    modifiers: None,
                    screen_x: None,
                    screen_y: None,
                },
                Some(session_id),
            )
            .await?;
        let gap = k.move_gap_ms();
        if gap > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(gap)).await;
        }
    }
    Ok(())
}

/// Scroll by dispatching real `mouseWheel` events, one tick at a time.
///
/// Returns the position measured **after** the gesture, converging on the
/// requested delta rather than trusting the dispatch acks: a wheel event sent
/// before the renderer is ready is acknowledged and then dropped, so a single
/// pass silently under-scrolls. Each extra round re-sends only the shortfall and
/// stops as soon as a round moves nothing, which is how "the event was dropped"
/// stays distinguishable from "the container is already at its end".
pub(super) async fn wheel_scroll(
    client: &CdpClient,
    session_id: &str,
    object_id: &str,
    delta_x: f64,
    delta_y: f64,
) -> Result<Value, String> {
    let mut k = kinematics::active();
    let ticks = k.wheel_ticks(delta_x, delta_y);
    if ticks.is_empty() {
        return call_on(client, session_id, object_id, MEASURE_FN).await;
    }

    let at = xy(&call_on(client, session_id, object_id, CENTER_FN).await?);
    let start = xy(&call_on(client, session_id, object_id, MEASURE_FN).await?);
    send_ticks(client, session_id, &mut k, at, ticks).await?;
    let mut measured = measure_settled(client, session_id, object_id).await?;

    let rounds =
        crate::xdg::policy::policy_u64(crate::xdg::policy::key::INPUT_SCROLL_SETTLE_ROUNDS);
    for _ in 0..rounds {
        let now = xy(&measured);
        let (rem_x, rem_y) = (delta_x - (now.0 - start.0), delta_y - (now.1 - start.1));
        if rem_x.abs() < 1.0 && rem_y.abs() < 1.0 {
            break;
        }
        let retry = k.wheel_ticks(rem_x, rem_y);
        send_ticks(client, session_id, &mut k, at, retry).await?;
        measured = measure_settled(client, session_id, object_id).await?;
        let after = xy(&measured);
        if (after.0 - now.0).abs() < 1.0 && (after.1 - now.1).abs() < 1.0 {
            break;
        }
    }
    Ok(measured)
}
