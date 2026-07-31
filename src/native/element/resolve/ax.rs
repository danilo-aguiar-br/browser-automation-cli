// SPDX-License-Identifier: MIT OR Apache-2.0
//! Accessibility-tree session and node helpers for resolve.

use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use serde_json::Value;

/// Determine which CDP session and parameters to use for an AX tree query.
/// Cross-origin iframes have a dedicated session (no frameId needed);
/// same-origin iframes use the parent session with a frameId parameter.
pub(crate) fn resolve_ax_session<'a>(
    frame_id: Option<&str>,
    session_id: &'a str,
    iframe_sessions: &'a rustc_hash::FxHashMap<String, String>,
) -> (serde_json::Value, &'a str) {
    if let Some(frame_id) = frame_id {
        if let Some(iframe_sid) = iframe_sessions.get(frame_id) {
            (serde_json::json!({}), iframe_sid.as_str())
        } else {
            (serde_json::json!({ "frameId": frame_id }), session_id)
        }
    } else {
        (serde_json::json!({}), session_id)
    }
}

/// Re-query the accessibility tree to find a node matching role+name+nth,
/// returning its fresh backendDOMNodeId. This uses the same data source
/// (Accessibility.getFullAXTree) that built the ref map during snapshot,
/// so role/name matching is guaranteed to be consistent.
pub(crate) async fn find_node_id_by_role_name(
    client: &CdpClient,
    session_id: &str,
    role: &str,
    name: &str,
    nth: Option<usize>,
    frame_id: Option<&str>,
    iframe_sessions: &rustc_hash::FxHashMap<String, String>,
) -> Result<i64, String> {
    let (ax_params, effective_session_id) =
        resolve_ax_session(frame_id, session_id, iframe_sessions);
    let ax_tree: GetFullAXTreeResult = client
        .send_command_typed(
            "Accessibility.getFullAXTree",
            &ax_params,
            Some(effective_session_id),
        )
        .await?;

    let nth_index = nth.unwrap_or(0);
    let mut match_count: usize = 0;

    for node in &ax_tree.nodes {
        if node.ignored.unwrap_or(false) {
            continue;
        }
        let node_role = extract_ax_string(&node.role);
        let node_name = extract_ax_string(&node.name);
        if node_role == role && node_name == name {
            if match_count == nth_index {
                return node.backend_d_o_m_node_id.ok_or_else(|| {
                    format!("AX node has no backendDOMNodeId for role={role} name={name}")
                });
            }
            match_count += 1;
        }
    }

    Err(format!(
        "Could not locate element with role={role} name={name}"
    ))
}

pub(crate) fn extract_ax_string(value: &Option<AXValue>) -> String {
    match value {
        Some(v) => match &v.value {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            _ => String::new(),
        },
        None => String::new(),
    }
}
