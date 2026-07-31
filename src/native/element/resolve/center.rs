// SPDX-License-Identifier: MIT OR Apache-2.0
//! Resolve element click center (x,y) over CDP.

use super::super::js::*;
use super::super::refs::{active_frame, parse_ref, RefMap};
use super::ax::find_node_id_by_role_name;
use super::helpers::{
    box_model_center, check_node_interception, frame_owner_object_id, intercepted_error,
    resolve_by_selector, resolve_frame_session, scroll_node_into_view,
};
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;

/// Find a selector inside a same-process iframe and return its center in
/// top-level viewport coordinates (input events dispatch in that space).
/// Same-origin access to contentDocument is what makes this possible; a
/// cross-origin frame never takes this path because it has its own session.
pub(crate) async fn resolve_center_in_same_process_frame(
    client: &CdpClient,
    session_id: &str,
    frame_id: &str,
    selector: &str,
) -> Result<(f64, f64), String> {
    let owner_object_id = frame_owner_object_id(client, session_id, frame_id).await?;
    let find_expr = build_find_element_js_in("doc", selector);
    let function = format!(
        r#"function() {{
            const doc = this.contentDocument;
            if (!doc) return null;
            const el = {find_expr};
            if (!el) return null;
            if (el.scrollIntoViewIfNeeded) el.scrollIntoViewIfNeeded(true);
            else el.scrollIntoView({{ block: 'center', inline: 'center' }});
            const rect = el.getBoundingClientRect();
            let x = rect.x + rect.width / 2;
            let y = rect.y + rect.height / 2;
            let win = doc.defaultView;
            while (win && win.frameElement) {{
                const frameRect = win.frameElement.getBoundingClientRect();
                x += frameRect.x + win.frameElement.clientLeft;
                y += frameRect.y + win.frameElement.clientTop;
                win = win.parent;
            }}
            const blockerAt = {BLOCKER_AT_JS};
            const topDoc = win ? win.document : doc;
            return {{ x: x, y: y, blocker: blockerAt(topDoc, el, x, y) }};
        }}"#,
    );
    let result = client
        .send_command(
            "Runtime.callFunctionOn",
            Some(serde_json::json!({
                "objectId": owner_object_id,
                "functionDeclaration": function,
                "returnByValue": true,
            })),
            Some(session_id),
        )
        .await?;
    let value = result.get("result").and_then(|r| r.get("value"));
    if let Some(blocker) = value
        .and_then(|v| v.get("blocker"))
        .and_then(|v| v.as_str())
    {
        return Err(intercepted_error(selector, blocker));
    }
    let x = value.and_then(|v| v.get("x")).and_then(|v| v.as_f64());
    let y = value.and_then(|v| v.get("y")).and_then(|v| v.as_f64());
    match (x, y) {
        (Some(x), Some(y)) => Ok((x, y)),
        _ => Err(format!(
            "Element not found in the selected frame: {selector}"
        )),
    }
}

/// Viewport coordinates of the element's centre, plus the session that owns it.
///
/// Accepts all three locator forms — a `@eN` ref, a CSS selector, or a durable
/// `role=…[name="…"]` — and scrolls the node into view first, because a centre
/// computed for an off-screen element points somewhere a click cannot land.
///
/// The returned session id is not always the one passed in: an element inside
/// an iframe resolves in the iframe's session, and the caller must dispatch the
/// interaction there.
pub async fn resolve_element_center(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<(f64, f64, String), String> {
    // GAP-034 pillar 2: durable locators resolve against the LIVE accessibility
    // tree, so they work in a process that never took the original snapshot.
    if let Some(locator) = crate::native::element::DurableLocator::parse(selector_or_ref) {
        let backend_node_id = find_node_id_by_role_name(
            client,
            session_id,
            &locator.role,
            &locator.name,
            // The wire form is 1-based for humans; the AX walker counts from 0.
            Some(locator.candidate_index()),
            None,
            iframe_sessions,
        )
        .await
        .map_err(|e| format!("durable locator {selector_or_ref} did not resolve: {e}"))?;
        scroll_node_into_view(client, session_id, backend_node_id).await;
        let result: DomGetBoxModelResult = client
            .send_command_typed(
                "DOM.getBoxModel",
                &DomGetBoxModelParams {
                    backend_node_id: Some(backend_node_id),
                    node_id: None,
                    object_id: None,
                },
                Some(session_id),
            )
            .await?;
        let (x, y) = box_model_center(&result.model);
        check_node_interception(client, session_id, backend_node_id, selector_or_ref, x, y).await?;
        return Ok((x, y, session_id.to_string()));
    }

    if let Some(ref_id) = parse_ref(selector_or_ref) {
        let entry = ref_map
            .get(&ref_id)
            .ok_or_else(|| format!("Unknown ref: {ref_id}"))?;

        let effective_session_id =
            resolve_frame_session(entry.frame_id.as_deref(), session_id, iframe_sessions);

        // Try cached backend_node_id first (fast path)
        if let Some(backend_node_id) = entry.backend_node_id {
            scroll_node_into_view(client, effective_session_id, backend_node_id).await;
            let result: Result<DomGetBoxModelResult, String> = client
                .send_command_typed(
                    "DOM.getBoxModel",
                    &DomGetBoxModelParams {
                        backend_node_id: Some(backend_node_id),
                        node_id: None,
                        object_id: None,
                    },
                    Some(effective_session_id),
                )
                .await;

            if let Ok(r) = result {
                let (x, y) = box_model_center(&r.model);
                check_node_interception(
                    client,
                    effective_session_id,
                    backend_node_id,
                    selector_or_ref,
                    x,
                    y,
                )
                .await?;
                return Ok((x, y, effective_session_id.to_string()));
            }
            // backend_node_id is stale; re-query the accessibility tree below
        }

        // Fallback: re-query the accessibility tree to find a fresh node by role/name
        let fresh_id = find_node_id_by_role_name(
            client,
            session_id,
            &entry.role,
            &entry.name,
            entry.nth,
            entry.frame_id.as_deref(),
            iframe_sessions,
        )
        .await?;
        scroll_node_into_view(client, effective_session_id, fresh_id).await;
        let result: DomGetBoxModelResult = client
            .send_command_typed(
                "DOM.getBoxModel",
                &DomGetBoxModelParams {
                    backend_node_id: Some(fresh_id),
                    node_id: None,
                    object_id: None,
                },
                Some(effective_session_id),
            )
            .await?;
        let (x, y) = box_model_center(&result.model);
        check_node_interception(
            client,
            effective_session_id,
            fresh_id,
            selector_or_ref,
            x,
            y,
        )
        .await?;
        return Ok((x, y, effective_session_id.to_string()));
    }

    // CSS selector: honor an active `frame <sel>` selection.
    if let Some(frame_id) = active_frame() {
        // Cross-process iframe: its dedicated session's main frame IS the
        // iframe, so plain document-rooted resolution works there.
        if let Some(frame_session) = iframe_sessions.get(&frame_id) {
            let (x, y) = resolve_by_selector(client, frame_session, selector_or_ref).await?;
            return Ok((x, y, frame_session.clone()));
        }
        let (x, y) =
            resolve_center_in_same_process_frame(client, session_id, &frame_id, selector_or_ref)
                .await?;
        return Ok((x, y, session_id.to_string()));
    }
    let (x, y) = resolve_by_selector(client, session_id, selector_or_ref).await?;
    Ok((x, y, session_id.to_string()))
}
