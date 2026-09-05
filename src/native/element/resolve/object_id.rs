// SPDX-License-Identifier: MIT OR Apache-2.0
//! Resolve element remote object id over CDP.

use super::super::js::*;
use super::super::refs::{active_frame, parse_ref, RefMap};
use super::ax::find_node_id_by_role_name;
use super::helpers::{frame_owner_object_id, resolve_frame_session};
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;

/// Find a selector inside a same-process iframe and return its object handle.
pub(crate) async fn resolve_object_in_same_process_frame(
    client: &CdpClient,
    session_id: &str,
    frame_id: &str,
    selector: &str,
) -> Result<String, String> {
    let owner_object_id = frame_owner_object_id(client, session_id, frame_id).await?;
    let find_expr = build_find_element_js_in("doc", selector);
    let function = format!(
        "function() {{ const doc = this.contentDocument; if (!doc) return null; return {find_expr}; }}",
    );
    let result = client
        .send_command(
            "Runtime.callFunctionOn",
            Some(serde_json::json!({
                "objectId": owner_object_id,
                "functionDeclaration": function,
                "returnByValue": false,
            })),
            Some(session_id),
        )
        .await?;
    result
        .get("result")
        .and_then(|r| r.get("objectId"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| format!("Element not found in the selected frame: {selector}"))
}

/// JavaScript object handle for the element, plus the session that owns it.
///
/// The handle is what `Runtime.callFunctionOn` needs, so this is the entry point
/// for every query that runs a function against an element. Accepts a `@eN` ref,
/// a CSS selector, or a durable `role=…[name="…"]` locator.
///
/// The returned session id can differ from the one passed in when the element
/// lives in an iframe.
///
/// # Errors
///
/// For a durable `role=…[name="…"]` locator, fails with
/// `"durable locator <loc> did not resolve: …"` when the live accessibility
/// tree carries no such node, and with `"durable locator <loc> resolved to no
/// object"` when `DOM.resolveNode` returns no handle.
///
/// For a `@eN` ref, fails with `"Unknown ref: <id>"` when `ref_map` never
/// recorded it — the usual cause is a snapshot invalidated by an intervening
/// `eval` or navigation — and with `"No objectId for ref <id>"` when the
/// accessibility re-query succeeds but the node no longer resolves. A stale
/// `backend_node_id` is not an error: the fast path is abandoned and the
/// re-query runs.
///
/// For a CSS or XPath selector, fails with
/// `"Element not found: <selector> (<detail>)"` when nothing matches or the
/// query itself throws — a malformed XPath lands here — and with
/// `"Element not found in the selected frame: <selector>"` under an active
/// same-process `frame` selection. Any underlying `Runtime.evaluate`,
/// `Runtime.callFunctionOn` or `DOM.resolveNode` refusal is propagated as-is.
pub async fn resolve_element_object_id(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<(String, String), String> {
    // GAP-034 pillar 2: a durable `role=…[name="…"]` locator is resolved against
    // the LIVE accessibility tree, never against this process's ref map. That is
    // what makes it work in a process that never took the original snapshot.
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
        let object_id = result
            .object
            .object_id
            .ok_or_else(|| format!("durable locator {selector_or_ref} resolved to no object"))?;
        return Ok((object_id, session_id.to_string()));
    }

    if let Some(ref_id) = parse_ref(selector_or_ref) {
        let entry = ref_map
            .get(&ref_id)
            .ok_or_else(|| format!("Unknown ref: {ref_id}"))?;

        let effective_session_id =
            resolve_frame_session(entry.frame_id.as_deref(), session_id, iframe_sessions);

        // Try cached backend_node_id first (fast path)
        if let Some(backend_node_id) = entry.backend_node_id {
            let result: Result<DomResolveNodeResult, String> = client
                .send_command_typed(
                    "DOM.resolveNode",
                    &DomResolveNodeParams {
                        backend_node_id: Some(backend_node_id),
                        node_id: None,
                        object_group: Some("browser-automation-cli".to_string()),
                    },
                    Some(effective_session_id),
                )
                .await;

            if let Ok(r) = result {
                if let Some(object_id) = r.object.object_id {
                    return Ok((object_id, effective_session_id.to_string()));
                }
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
        let result: DomResolveNodeResult = client
            .send_command_typed(
                "DOM.resolveNode",
                &DomResolveNodeParams {
                    backend_node_id: Some(fresh_id),
                    node_id: None,
                    object_group: Some("browser-automation-cli".to_string()),
                },
                Some(effective_session_id),
            )
            .await?;
        let object_id = result
            .object
            .object_id
            .ok_or_else(|| format!("No objectId for ref {ref_id}"))?;
        return Ok((object_id, effective_session_id.to_string()));
    }

    // Selector fallback (CSS or XPath): honor an active `frame <sel>` selection.
    if let Some(frame_id) = active_frame() {
        if let Some(frame_session) = iframe_sessions.get(&frame_id) {
            let js = build_find_element_js(selector_or_ref);
            let result: EvaluateResult = client
                .send_command_typed(
                    "Runtime.evaluate",
                    &EvaluateParams {
                        expression: js,
                        return_by_value: Some(false),
                        await_promise: Some(false),
                    },
                    Some(frame_session.as_str()),
                )
                .await?;
            let object_id = object_id_from_evaluate(result, selector_or_ref)?;
            return Ok((object_id, frame_session.clone()));
        }
        let object_id =
            resolve_object_in_same_process_frame(client, session_id, &frame_id, selector_or_ref)
                .await?;
        return Ok((object_id, session_id.to_string()));
    }

    let js = build_find_element_js(selector_or_ref);
    let result: EvaluateResult = client
        .send_command_typed(
            "Runtime.evaluate",
            &EvaluateParams {
                expression: js,
                return_by_value: Some(false),
                await_promise: Some(false),
            },
            Some(session_id),
        )
        .await?;

    let object_id = object_id_from_evaluate(result, selector_or_ref)?;
    Ok((object_id, session_id.to_string()))
}
