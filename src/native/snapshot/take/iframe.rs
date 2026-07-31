// SPDX-License-Identifier: MIT OR Apache-2.0
//! Iframe frame-id resolution and backend node id collection.
//!
//! Split out of `take_snapshot` so the recursion helpers can be read without
//! scrolling through the main snapshot pass.

use crate::native::cdp::client::CdpClient;
use serde_json::Value;

/// Resolve the child frame ID for an iframe element given its backendNodeId.
pub(super) async fn resolve_iframe_frame_id(
    client: &CdpClient,
    session_id: &str,
    backend_node_id: i64,
) -> Result<String, String> {
    // depth: 1 ensures contentDocument is included in the response
    let describe: Value = client
        .send_command(
            "DOM.describeNode",
            Some(serde_json::json!({ "backendNodeId": backend_node_id, "depth": 1 })),
            Some(session_id),
        )
        .await?;

    // Try contentDocument.frameId first (standard for iframes)
    if let Some(frame_id) = describe
        .get("node")
        .and_then(|n| n.get("contentDocument"))
        .and_then(|cd| cd.get("frameId"))
        .and_then(|v| v.as_str())
    {
        return Ok(frame_id.to_string());
    }

    // Fallback: the node itself may have a frameId
    describe
        .get("node")
        .and_then(|n| n.get("frameId"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Could not resolve iframe frame ID".to_string())
}

pub(super) fn collect_backend_node_ids(node: &Value, ids: &mut rustc_hash::FxHashSet<i64>) {
    if let Some(id) = node.get("backendNodeId").and_then(|v| v.as_i64()) {
        ids.insert(id);
    }
    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for child in children {
            collect_backend_node_ids(child, ids);
        }
    }
    // Shadow DOM and content documents
    if let Some(shadow) = node.get("shadowRoots").and_then(|v| v.as_array()) {
        for child in shadow {
            collect_backend_node_ids(child, ids);
        }
    }
    if let Some(doc) = node.get("contentDocument") {
        collect_backend_node_ids(doc, ids);
    }
}
