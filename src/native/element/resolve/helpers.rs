// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared helpers for element resolve (CDP frame/object/hit-test).

use super::super::js::*;
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use serde_json::Value;

/// Object handle for the `<iframe>` element that owns a frame, resolved on the
/// parent session. Works for same-process frames where no dedicated CDP
/// session exists.
pub(crate) async fn frame_owner_object_id(
    client: &CdpClient,
    session_id: &str,
    frame_id: &str,
) -> Result<String, String> {
    let owner = client
        .send_command(
            "DOM.getFrameOwner",
            Some(serde_json::json!({ "frameId": frame_id })),
            Some(session_id),
        )
        .await?;
    let backend_node_id = owner
        .get("backendNodeId")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| format!("Could not resolve the owner element of frame {frame_id}"))?;
    let result: DomResolveNodeResult = client
        .send_command_typed(
            "DOM.resolveNode",
            &DomResolveNodeParams {
                backend_node_id: Some(backend_node_id),
                node_id: None,
                object_group: Some("browser-automation-cli".to_string()),
            },
            Some(session_id),
        )
        .await?;
    result
        .object
        .object_id
        .ok_or_else(|| format!("No objectId for the owner element of frame {frame_id}"))
}

/// Hit-test a ref-resolved node at its computed click point and error if an
/// unrelated element (overlay, banner, sticky header) would receive the input
/// instead. Best effort: resolution failures skip the check rather than block
/// the interaction.
pub(crate) async fn check_node_interception(
    client: &CdpClient,
    session_id: &str,
    backend_node_id: i64,
    target: &str,
    x: f64,
    y: f64,
) -> Result<(), String> {
    let resolved: Result<DomResolveNodeResult, String> = client
        .send_command_typed(
            "DOM.resolveNode",
            &DomResolveNodeParams {
                backend_node_id: Some(backend_node_id),
                node_id: None,
                object_group: Some("browser-automation-cli".to_string()),
            },
            Some(session_id),
        )
        .await;
    let Ok(resolved) = resolved else {
        return Ok(());
    };
    let Some(object_id) = resolved.object.object_id else {
        return Ok(());
    };
    // Box-model coordinates are in the top-level viewport space, so the
    // hit-test starts from the top document. For an OOPIF node the
    // frameElement walk stops at the process boundary, where the frame's own
    // document and session-local coordinates are already consistent.
    let function = format!(
        r#"function(x, y) {{
            let topDoc = this.ownerDocument || document;
            while (topDoc.defaultView && topDoc.defaultView.frameElement) {{
                topDoc = topDoc.defaultView.frameElement.ownerDocument;
            }}
            const blockerAt = {BLOCKER_AT_JS};
            return blockerAt(topDoc, this, x, y);
        }}"#,
    );
    let result = client
        .send_command(
            "Runtime.callFunctionOn",
            Some(serde_json::json!({
                "objectId": object_id,
                "functionDeclaration": function,
                "arguments": [{ "value": x }, { "value": y }],
                "returnByValue": true,
            })),
            Some(session_id),
        )
        .await;
    if let Ok(value) = result {
        if let Some(blocker) = value
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
        {
            return Err(intercepted_error(target, blocker));
        }
    }
    Ok(())
}

/// Coordinates from DOM.getBoxModel are viewport-relative, and input events
/// only land inside the viewport, so make sure the node is visible first.
/// Best effort: a node that cannot be scrolled (display:none, detached) will
/// fail in DOM.getBoxModel with a clearer error anyway.
pub(crate) async fn scroll_node_into_view(
    client: &CdpClient,
    session_id: &str,
    backend_node_id: i64,
) {
    let _ = client
        .send_command(
            "DOM.scrollIntoViewIfNeeded",
            Some(serde_json::json!({ "backendNodeId": backend_node_id })),
            Some(session_id),
        )
        .await;
}

/// Resolve the effective CDP session for an element's frame.
/// If the element's frame_id has a dedicated cross-origin iframe session, return it.
/// Otherwise, return the parent session.
pub(crate) fn resolve_frame_session<'a>(
    frame_id: Option<&str>,
    session_id: &'a str,
    iframe_sessions: &'a rustc_hash::FxHashMap<String, String>,
) -> &'a str {
    frame_id
        .and_then(|fid| iframe_sessions.get(fid))
        .map(|s| s.as_str())
        .unwrap_or(session_id)
}

pub(crate) async fn resolve_by_selector(
    client: &CdpClient,
    session_id: &str,
    selector: &str,
) -> Result<(f64, f64), String> {
    let js = build_selector_js(selector);

    let result: EvaluateResult = client
        .send_command_typed(
            "Runtime.evaluate",
            &EvaluateParams {
                expression: js,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(session_id),
        )
        .await?;

    if let Some(exc) = result.exception_details {
        let detail = exc
            .exception
            .as_ref()
            .and_then(|e| e.description.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or(exc.text);
        return Err(format!("Element not found: {selector} ({detail})"));
    }

    let val = result.result.value.unwrap_or(Value::Null);
    if let Some(blocker) = val.get("blocker").and_then(|v| v.as_str()) {
        return Err(intercepted_error(selector, blocker));
    }
    let x = val.get("x").and_then(|v| v.as_f64());
    let y = val.get("y").and_then(|v| v.as_f64());

    match (x, y) {
        (Some(x), Some(y)) => Ok((x, y)),
        _ => Err(format!("Element not found: {selector}")),
    }
}

pub(crate) fn intercepted_error(target: &str, blocker: &str) -> String {
    format!(
        "Element '{target}' is covered by <{blocker}> at its click point, so the input would land on that element instead. Dismiss or interact with the covering element first (it is often a dialog, banner, or sticky header)."
    )
}

pub(crate) fn box_model_center(model: &BoxModel) -> (f64, f64) {
    // content quad: [x1,y1, x2,y2, x3,y3, x4,y4]
    if model.content.len() >= 8 {
        let x = (model.content[0] + model.content[2] + model.content[4] + model.content[6]) / 4.0;
        let y = (model.content[1] + model.content[3] + model.content[5] + model.content[7]) / 4.0;
        (x, y)
    } else {
        (0.0, 0.0)
    }
}
