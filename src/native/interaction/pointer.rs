// SPDX-License-Identifier: MIT OR Apache-2.0

use super::dispatch::dispatch_click;
use super::types::ClickResult;
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use crate::native::element::{resolve_element_center, RefMap};
use serde_json::Value;

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
    client
        .send_command_typed::<_, Value>(
            "Input.dispatchMouseEvent",
            &DispatchMouseEventParams {
                event_type: "mouseMoved".to_string(),
                x,
                y,
                button: None,
                buttons: None,
                click_count: None,
                delta_x: None,
                delta_y: None,
                modifiers: None,
            },
            Some(&effective_session_id),
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
    for (event_type, x, y, button, buttons, click_count) in [
        ("mouseMoved", x1, y1, None, None, None),
        (
            "mousePressed",
            x1,
            y1,
            Some("left".to_string()),
            Some(1),
            Some(1),
        ),
        (
            "mouseMoved",
            x2,
            y2,
            Some("left".to_string()),
            Some(1),
            None,
        ),
        (
            "mouseReleased",
            x2,
            y2,
            Some("left".to_string()),
            Some(0),
            Some(1),
        ),
    ] {
        client
            .send_command_typed::<_, Value>(
                "Input.dispatchMouseEvent",
                &DispatchMouseEventParams {
                    event_type: event_type.to_string(),
                    x,
                    y,
                    button,
                    buttons,
                    click_count,
                    delta_x: None,
                    delta_y: None,
                    modifiers: None,
                },
                Some(&sid),
            )
            .await?;
    }
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
