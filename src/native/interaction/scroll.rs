// SPDX-License-Identifier: MIT OR Apache-2.0

use super::kinematics;
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use crate::native::element::{resolve_element_object_id, RefMap};
use serde_json::Value;

/// One scroll request against the viewport or a scrollable container (GAP-031).
///
/// `to_x` / `to_y` request an absolute scroll offset. When either is set the
/// absolute path wins for that axis and the matching delta is ignored; an unset
/// axis keeps its current offset.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScrollRequest<'a> {
    /// CSS selector or `@eN` ref of a scrollable container; `None` = viewport.
    pub target: Option<&'a str>,
    /// Horizontal scroll amount in CSS pixels. Ignored when `to_x` is set.
    pub delta_x: f64,
    /// Vertical scroll amount in CSS pixels. Ignored when `to_y` is set.
    pub delta_y: f64,
    /// Absolute horizontal offset to scroll to.
    pub to_x: Option<f64>,
    /// Absolute vertical offset to scroll to.
    pub to_y: Option<f64>,
}

/// Read the scroll position and container metrics without moving anything.
///
/// Split out from [`SCROLL_FN`] so the `human` path can measure before and after
/// a real wheel gesture instead of scrolling from JavaScript. Measuring and
/// acting used to be the same function, which is precisely why `scrollBy` became
/// the mechanism: once the JS was there to read `scrollHeight`, making it also
/// scroll was one more line, and no test could tell the difference because none
/// looked at which events the page received.
pub(super) const MEASURE_FN: &str = r#"function() {
    return {
        x: this.scrollLeft,
        y: this.scrollTop,
        scrollHeight: this.scrollHeight,
        clientHeight: this.clientHeight,
        scrollWidth: this.scrollWidth,
        clientWidth: this.clientWidth,
    };
}"#;

/// JS body shared by the container and viewport paths.
///
/// `this` is the scrolling element in both cases, so the same source reports
/// `scrollHeight` / `clientHeight` for an overflow container and for the
/// document scrolling element. No stylesheet or attribute is injected.
///
/// Used by `--input-profile direct` and by absolute (`to_x` / `to_y`) requests,
/// which have no wheel equivalent: a wheel carries a delta, never a destination.
const SCROLL_FN: &str = r#"function(dx, dy, tx, ty) {
    const before = { x: this.scrollLeft, y: this.scrollTop };
    if (tx !== null || ty !== null) {
        this.scrollTo({
            left: tx !== null ? tx : this.scrollLeft,
            top: ty !== null ? ty : this.scrollTop,
            behavior: 'instant',
        });
    } else {
        this.scrollBy({ left: dx, top: dy, behavior: 'instant' });
    }
    return {
        before,
        after: { x: this.scrollLeft, y: this.scrollTop },
        scrollHeight: this.scrollHeight,
        clientHeight: this.clientHeight,
        scrollWidth: this.scrollWidth,
        clientWidth: this.clientWidth,
        scrollable: this.scrollHeight > this.clientHeight
            || this.scrollWidth > this.clientWidth,
    };
}"#;

pub(super) fn scroll_arguments(req: &ScrollRequest<'_>) -> Vec<CallArgument> {
    [
        serde_json::json!(req.delta_x),
        serde_json::json!(req.delta_y),
        req.to_x.map_or(Value::Null, |v| serde_json::json!(v)),
        req.to_y.map_or(Value::Null, |v| serde_json::json!(v)),
    ]
    .into_iter()
    .map(|value| CallArgument {
        value: Some(value),
        object_id: None,
    })
    .collect()
}

/// Call a zero-argument JS helper on an already-resolved object.
pub(super) async fn call_on(
    client: &CdpClient,
    session_id: &str,
    object_id: &str,
    body: &str,
) -> Result<Value, String> {
    let result: Value = client
        .send_command_typed(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: body.to_string(),
                object_id: Some(object_id.to_string()),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
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

/// Shape the envelope the way [`SCROLL_FN`] does, from two measurements.
fn metrics_envelope(before: &Value, after: &Value) -> Value {
    let pos = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64).unwrap_or_default();
    let scroll_height = pos(after, "scrollHeight");
    let client_height = pos(after, "clientHeight");
    let scroll_width = pos(after, "scrollWidth");
    let client_width = pos(after, "clientWidth");
    serde_json::json!({
        "before": { "x": pos(before, "x"), "y": pos(before, "y") },
        "after": { "x": pos(after, "x"), "y": pos(after, "y") },
        "scrollHeight": scroll_height,
        "clientHeight": client_height,
        "scrollWidth": scroll_width,
        "clientWidth": client_width,
        "scrollable": scroll_height > client_height || scroll_width > client_width,
    })
}

/// Scroll the viewport or a container that owns its own scrollbar (GAP-031).
///
/// Returns the position before and after plus the container metrics, so a caller
/// can tell "scrolled to the end" apart from "container is not scrollable".
///
/// Under `--input-profile human` a relative scroll is delivered as a sequence of
/// real `mouseWheel` events, so a page that listens for `wheel` -- lazy-load,
/// infinite scroll, carousels -- actually reacts. Absolute requests and the
/// `direct` profile keep the JavaScript path, which emits no `wheel` at all.
///
/// # Errors
///
/// With `req.target` set, propagates
/// [`resolve_element_object_id`].
/// Without it, fails with the CDP error raised by the `Runtime.evaluate` that
/// resolves the scrolling element, or with
/// `"scroll: no document scrolling element"` when that evaluation yields no
/// object handle — a document with no `documentElement`.
///
/// Then fails with the CDP error raised by the measuring or scrolling
/// `Runtime.callFunctionOn`, with `"scroll helper returned no value"` when the
/// helper answers `undefined`, and on the human profile with any refused
/// `Input.dispatchMouseEvent` wheel tick. A container that cannot scroll is
/// **not** an error: it is reported as `scrollable: false` with equal
/// before/after positions.
pub async fn scroll(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    req: ScrollRequest<'_>,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<Value, String> {
    let (object_id, effective_session_id) = match req.target {
        Some(sel) => {
            let (object_id, sid) =
                resolve_element_object_id(client, session_id, ref_map, sel, iframe_sessions)
                    .await?;
            (object_id, sid)
        }
        None => {
            // Resolve the document scrolling element as a RemoteObject so the
            // viewport path runs the exact same function body as the container path.
            let resolved: Value = client
                .send_command_typed(
                    "Runtime.evaluate",
                    &EvaluateParams {
                        expression: "document.scrollingElement || document.documentElement"
                            .to_string(),
                        return_by_value: Some(false),
                        await_promise: Some(false),
                    },
                    Some(session_id),
                )
                .await?;
            let object_id = resolved
                .pointer("/result/objectId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "scroll: no document scrolling element".to_string())?
                .to_string();
            (object_id, session_id.to_string())
        }
    };

    // A wheel carries a delta, so an absolute request has no wheel equivalent and
    // keeps the scripted path. `direct` keeps it too, byte for byte.
    let absolute = req.to_x.is_some() || req.to_y.is_some();
    if !absolute && kinematics::active_profile().is_human() {
        let before = call_on(client, &effective_session_id, &object_id, MEASURE_FN).await?;
        let after = super::wheel::wheel_scroll(
            client,
            &effective_session_id,
            &object_id,
            req.delta_x,
            req.delta_y,
        )
        .await?;
        return Ok(metrics_envelope(&before, &after));
    }

    let result: Value = client
        .send_command_typed(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: SCROLL_FN.to_string(),
                object_id: Some(object_id),
                arguments: Some(scroll_arguments(&req)),
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
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
        .ok_or_else(|| "scroll returned no metrics".to_string())
}
