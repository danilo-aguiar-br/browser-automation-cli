// SPDX-License-Identifier: MIT OR Apache-2.0

use super::dispatch::dispatch_click;
use super::types::ClickResult;
use crate::native::cdp::client::CdpClient;
use crate::native::element::{resolve_element_center, RefMap};

/// Real mouse click at the element's centre, scrolling it into view first.
pub async fn click(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    button: &str,
    click_count: i32,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<ClickResult, String> {
    let (x, y, effective_session_id) = resolve_element_center(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;
    // A click-triggered dialog can fire on the frame's own session (OOPIF) or
    // on the top-level page session; both count as "ours". A dialog on any
    // other session belongs to a background tab and must not abort this click.
    dispatch_click(
        client,
        &effective_session_id,
        &[effective_session_id.as_str(), session_id],
        x,
        y,
        button,
        click_count,
    )
    .await
}

/// Two clicks close enough in time and space to register as a double-click.
pub async fn dblclick(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<ClickResult, String> {
    click(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        "left",
        2,
        iframe_sessions,
    )
    .await
}

/// Move the pointer over an element without pressing anything.
///
/// Under `human` the pointer travels there; a single jump to the centre reports
/// the same final position but produces one `mousemove` where a hand produces a
/// stream, and never crosses whatever lies between origin and target.
pub async fn hover(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<(), String> {
    let (x, y, effective_session_id) = resolve_element_center(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;
    let (x, y) = super::kinematics::active().jitter_target(x, y);
    super::dispatch::travel_to(
        client,
        &effective_session_id,
        &[effective_session_id.as_str(), session_id],
        x,
        y,
    )
    .await?;
    Ok(())
}

/// Drag from one element center to another via CDP mouse events.
pub async fn drag(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    from: &str,
    to: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<(), String> {
    let (x1, y1, sid1) =
        resolve_element_center(client, session_id, ref_map, from, iframe_sessions).await?;
    let (x2, y2, sid2) =
        resolve_element_center(client, session_id, ref_map, to, iframe_sessions).await?;
    if sid1 != sid2 {
        return Err("drag endpoints must share the same frame/session".to_string());
    }
    let sid = sid1;

    // One interpolation for both drag routes (GAP: `drag_html5` had the only
    // correct implementation and this one jumped A→B in a single move, so the
    // same product gesture behaved differently depending on which path ran).
    let mut k = super::kinematics::active();
    let send = super::drag_html5::mouse;

    send(client, &sid, "mouseMoved", x1, y1, 0, None).await?;
    send(client, &sid, "mousePressed", x1, y1, 1, Some(1)).await?;
    for (px, py) in k.path((x1, y1), (x2, y2)) {
        send(client, &sid, "mouseMoved", px, py, 1, None).await?;
        let gap = k.move_gap_ms();
        if gap > 0 && k.profile().is_human() {
            tokio::time::sleep(std::time::Duration::from_millis(gap)).await;
        }
    }
    send(client, &sid, "mouseReleased", x2, y2, 0, Some(1)).await?;
    Ok(())
}

/// Click at page coordinates (vision / experimental path).
pub async fn click_at(
    client: &CdpClient,
    session_id: &str,
    x: f64,
    y: f64,
    dblclick: bool,
) -> Result<ClickResult, String> {
    let click_count = if dblclick { 2 } else { 1 };
    dispatch_click(client, session_id, &[session_id], x, y, "left", click_count).await
}

/// Touch tap, for pages that listen for touch rather than mouse events.
///
/// Requires touch emulation to be on; without it the page sees nothing.
pub async fn tap_touch(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<(), String> {
    let (x, y, effective_session_id) = resolve_element_center(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    client
        .send_command(
            "Input.dispatchTouchEvent",
            Some(serde_json::json!({
                "type": "touchStart",
                "touchPoints": [{ "x": x, "y": y }],
            })),
            Some(&effective_session_id),
        )
        .await?;

    client
        .send_command(
            "Input.dispatchTouchEvent",
            Some(serde_json::json!({
                "type": "touchEnd",
                "touchPoints": [],
            })),
            Some(&effective_session_id),
        )
        .await?;

    Ok(())
}
