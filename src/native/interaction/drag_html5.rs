// SPDX-License-Identifier: MIT OR Apache-2.0
//! HTML5 drag-and-drop primitives over raw CDP (GAP-030).
//!
//! # Why not `Input.dispatchDragEvent` alone
//!
//! The vendored `cdp-protocol/browser_protocol.json` restricts
//! `Input.dispatchDragEvent` to `dragEnter`, `dragOver`, `drop` and `dragCancel`.
//! There is no `dragstart`. Dispatching those events with a payload we invent
//! therefore skips the page's own `dragstart` handler and injects **our**
//! `DataTransfer` instead of the one the page would have produced — a false
//! positive for any test that claims the page's drag handler ran.
//!
//! The honest route is `Input.setInterceptDrags(true)`, a real mouse gesture so
//! the page fires its own `dragstart`, capturing the resulting
//! `Input.dragIntercepted` event, and only then dispatching `dragEnter` /
//! `dragOver` / `drop` on the destination with **that** intercepted payload.
//!
//! `chromiumoxide` 0.9 exposes no typed drag API, so every call here is raw CDP.

use serde_json::{json, Value};

use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use crate::native::element::{resolve_element_center, resolve_element_object_id, RefMap};

/// Where inside the destination rect the drop point lands.
///
/// `Before` / `After` exist for ordered lists, where dropping on the exact centre
/// of a sibling is ambiguous about which side of it the item lands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DropAnchor {
    /// Geometric centre of the destination rect.
    #[default]
    Center,
    /// Just inside the leading edge, so the item lands BEFORE the target.
    Before,
    /// Just inside the trailing edge, so the item lands AFTER the target.
    After,
}

impl DropAnchor {
    /// Parse an anchor token, accepting the spatial synonyms an agent may use
    /// (`top`/`start` for `before`, `bottom`/`end` for `after`).
    pub fn parse(token: &str) -> Result<Self, String> {
        match token.trim().to_ascii_lowercase().as_str() {
            "center" | "middle" | "" => Ok(Self::Center),
            "before" | "top" | "start" => Ok(Self::Before),
            "after" | "bottom" | "end" => Ok(Self::After),
            other => Err(format!(
                "unknown drop anchor `{other}`; use center | before | after"
            )),
        }
    }

    /// Canonical token, for echoing the resolved anchor in the envelope.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Center => "center",
            Self::Before => "before",
            Self::After => "after",
        }
    }
}

/// One drag request. `to` (element) and `to_x`/`to_y` (page coordinates) are
/// alternatives; when both are given the coordinates win per axis.
#[derive(Debug, Clone, Default)]
pub struct DragRequest {
    /// Source element, as a `@eN` ref or a selector.
    pub from: String,
    /// Destination element. Ignored on an axis where an explicit coordinate is given.
    pub to: Option<String>,
    /// Destination X in page coordinates, overriding `to` on that axis.
    pub to_x: Option<f64>,
    /// Destination Y in page coordinates, overriding `to` on that axis.
    pub to_y: Option<f64>,
    /// Where inside the destination rect to drop. Ignored when coordinates are explicit.
    pub anchor: DropAnchor,
    /// Explicit `DragData` to inject instead of the page's own `DataTransfer`.
    /// Opt-in only: it bypasses the page's `dragstart` handler.
    pub synthetic_payload: Option<Value>,
}

/// Which CDP route actually produced the drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragRoute {
    /// Real `dragstart` from the page, captured via `Input.dragIntercepted`.
    Intercepted,
    /// Caller supplied the `DataTransfer` payload explicitly; the page's
    /// `dragstart` was deliberately bypassed.
    SyntheticPayload,
    /// `setInterceptDrags` / `dragIntercepted` unavailable on this browser;
    /// degraded to a plain mouse gesture that never proves the drag handler ran.
    SyntheticMouse,
}

impl DragRoute {
    /// Canonical token for the envelope, so the caller can tell which route ran.
    ///
    /// This distinction is the whole point of the enum: `intercepted` proves the
    /// page's own `dragstart` fired, while the synthetic routes do not.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Intercepted => "intercepted",
            Self::SyntheticPayload => "synthetic_payload",
            Self::SyntheticMouse => "synthetic_mouse",
        }
    }
}

/// Number of intermediate mouse moves between press and destination.
///
/// One single jump can land under Chrome`s drag threshold heuristics; a short
/// interpolation makes the gesture indistinguishable from a human drag.
///
/// Operator override: XDG `config set drag_move_steps <n>`.
fn drag_move_steps() -> usize {
    crate::xdg::policy::policy_usize(crate::xdg::policy::key::DRAG_MOVE_STEPS)
}

/// Pause between interpolated moves so the renderer can act on each one.
///
/// Operator override: XDG `config set drag_move_gap_ms <n>`.
fn drag_move_gap_ms() -> u64 {
    crate::xdg::policy::policy_u64(crate::xdg::policy::key::DRAG_MOVE_GAP_MS)
}

/// Enable or disable drag interception. Errors when the browser lacks the command.
pub async fn set_intercept_drags(
    client: &CdpClient,
    session_id: &str,
    enabled: bool,
) -> Result<(), String> {
    client
        .send_command(
            "Input.setInterceptDrags",
            Some(json!({ "enabled": enabled })),
            Some(session_id),
        )
        .await
        .map(|_| ())
}

/// Dispatch one left-button mouse event.
///
/// `pub(super)` so [`super::pointer::drag`] shares it: the two drag routes used
/// to carry separate dispatch code, which is how one of them ended up
/// interpolating and the other jumping A→B in a single move.
pub(super) async fn mouse(
    client: &CdpClient,
    session_id: &str,
    event_type: &str,
    x: f64,
    y: f64,
    buttons: i32,
    click_count: Option<i32>,
) -> Result<(), String> {
    client
        .send_command_typed::<_, Value>(
            "Input.dispatchMouseEvent",
            &DispatchMouseEventParams {
                event_type: event_type.to_string(),
                x,
                y,
                button: Some("left".to_string()),
                buttons: Some(buttons),
                click_count,
                delta_x: None,
                delta_y: None,
                modifiers: None,
            },
            Some(session_id),
        )
        .await
        .map(|_| ())
}

/// Press at the source and move toward the destination so the page fires its own
/// `dragstart`. Leaves the left button **down**; the caller must release it.
pub async fn start_drag_gesture(
    client: &CdpClient,
    session_id: &str,
    from: (f64, f64),
    to: (f64, f64),
) -> Result<(), String> {
    mouse(client, session_id, "mouseMoved", from.0, from.1, 0, None).await?;
    mouse(
        client,
        session_id,
        "mousePressed",
        from.0,
        from.1,
        1,
        Some(1),
    )
    .await?;
    // G11 step 6: ONE trajectory generator for both drag routes.
    //
    // This loop used to interpolate `t = step / steps` on its own while
    // `pointer::drag` had already moved to `kinematics`. Two code paths for the
    // same gesture is how they drift: a straight line at constant speed is a
    // synthetic signature by itself, so the HTML5 route was still emitting the
    // exact pattern the other route had been fixed to stop emitting.
    //
    // `Kinematics::path` keeps the property this route actually needs -- several
    // intermediate moves rather than one hop, so the gesture clears Chrome's
    // drag threshold -- and adds the bowed Bezier and eased pacing for free.
    // Under `--input-profile direct` it yields the destination alone, which is
    // the documented byte-for-byte escape hatch.
    // Route-local budget: `drag_move_*` predate `input_move_*`, are still
    // documented and still settable, so they keep steering THIS route.
    let mut kin =
        super::kinematics::active().with_move_budget(drag_move_steps().max(1), drag_move_gap_ms());
    for (x, y) in kin.path(from, to) {
        mouse(client, session_id, "mouseMoved", x, y, 1, None).await?;
        // The renderer starts a drag from its own input pipeline, not from the
        // CDP call return. Back-to-back moves get coalesced into one hop that
        // can skip the drag threshold entirely.
        tokio::time::sleep(std::time::Duration::from_millis(kin.move_gap_ms())).await;
    }
    Ok(())
}

/// Release the left button at `at`.
pub async fn release_drag_gesture(
    client: &CdpClient,
    session_id: &str,
    at: (f64, f64),
) -> Result<(), String> {
    mouse(client, session_id, "mouseReleased", at.0, at.1, 0, Some(1)).await
}

/// Dispatch one drag event carrying `data` (a CDP `DragData` object).
///
/// `event_type` must be one of `dragEnter`, `dragOver`, `drop`, `dragCancel` —
/// the only values the protocol accepts.
pub async fn dispatch_drag_event(
    client: &CdpClient,
    session_id: &str,
    event_type: &str,
    x: f64,
    y: f64,
    data: &Value,
) -> Result<(), String> {
    client
        .send_command(
            "Input.dispatchDragEvent",
            Some(json!({
                "type": event_type,
                "x": x,
                "y": y,
                "data": data,
            })),
            Some(session_id),
        )
        .await
        .map(|_| ())
}

/// Complete a drop at `to` using `data` as the `DataTransfer` payload.
pub async fn complete_drop(
    client: &CdpClient,
    session_id: &str,
    to: (f64, f64),
    data: &Value,
) -> Result<(), String> {
    for event_type in ["dragEnter", "dragOver", "drop"] {
        dispatch_drag_event(client, session_id, event_type, to.0, to.1, data).await?;
    }
    Ok(())
}

/// Normalise an intercepted payload into a `DragData` the protocol accepts.
///
/// `Input.dragIntercepted` wraps the payload in `{"data": {...}}`; older builds
/// have been seen emitting the `DragData` fields at the top level. `items` is
/// mandatory on the way back in, so a payload without it is rejected rather than
/// silently padded with an empty array (that would drop the page's real data).
pub fn normalize_drag_data(event_params: &Value) -> Result<Value, String> {
    let candidate = event_params
        .get("data")
        .filter(|v| v.is_object())
        .unwrap_or(event_params);
    let items = candidate
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "dragIntercepted payload has no `items` array".to_string())?;
    let mut out = json!({
        "items": items,
        "dragOperationsMask": candidate
            .get("dragOperationsMask")
            .and_then(Value::as_i64)
            .unwrap_or(1),
    });
    if let Some(files) = candidate.get("files").filter(|v| v.is_array()) {
        out["files"] = files.clone();
    }
    Ok(out)
}

/// Validate a caller-supplied synthetic payload before it reaches the browser.
pub fn validate_synthetic_payload(payload: &Value) -> Result<Value, String> {
    if !payload.is_object() {
        return Err("synthetic drag payload must be a JSON object".to_string());
    }
    normalize_drag_data(payload)
}

/// Viewport rect of a resolved element, used to place anchored drop points.
#[derive(Debug, Clone, Copy)]
/// Destination rectangle in page coordinates, used to resolve an anchor.
pub struct ElementRect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width in CSS pixels.
    pub width: f64,
    /// Height in CSS pixels.
    pub height: f64,
}

impl ElementRect {
    /// Drop point for an anchor, kept 1px inside the rect.
    ///
    /// An exact edge coordinate hit-tests to the NEIGHBOUR, so `Before` and
    /// `After` computed on the boundary would drop on the wrong element.
    pub fn anchor_point(&self, anchor: DropAnchor) -> (f64, f64) {
        let cx = self.x + self.width / 2.0;
        // Stay 1px inside the rect: an exact edge coordinate hit-tests to the
        // neighbouring element on fractional device pixel ratios.
        // NOT `clamp`: on a NaN height (degenerate rect) `f64::min`/`max` fold
        // to the 4.0 bound, while `clamp` would propagate NaN into a CDP
        // coordinate. Keeping the fold preserves the existing behaviour.
        #[allow(clippy::manual_clamp)]
        let inset = (self.height / 4.0).min(4.0).max(1.0);
        let y = match anchor {
            DropAnchor::Center => self.y + self.height / 2.0,
            DropAnchor::Before => self.y + inset,
            DropAnchor::After => self.y + self.height - inset,
        };
        (cx, y)
    }
}

/// Read the viewport rect of a selector or `@eN` ref.
pub async fn element_rect(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<(ElementRect, String), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;
    let result: Value = client
        .send_command_typed(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: r#"function() {
                    const r = this.getBoundingClientRect();
                    return { x: r.x, y: r.y, width: r.width, height: r.height };
                }"#
                .to_string(),
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;
    let v = result
        .pointer("/result/value")
        .ok_or_else(|| format!("could not read rect of `{selector_or_ref}`"))?;
    let num = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    Ok((
        ElementRect {
            x: num("x"),
            y: num("y"),
            width: num("width"),
            height: num("height"),
        },
        effective_session_id,
    ))
}

/// Resolve the source point of a drag.
pub async fn source_point(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    from: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<((f64, f64), String), String> {
    let (x, y, sid) =
        resolve_element_center(client, session_id, ref_map, from, iframe_sessions).await?;
    Ok(((x, y), sid))
}
