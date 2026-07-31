// SPDX-License-Identifier: MIT OR Apache-2.0
//! Link `href` resolution for ref-bearing nodes (`--urls`).
//!
//! CDP has no batch resolve API, so both phases fan out per node under the
//! process concurrency budget; never unbounded `join_all`.

use crate::native::cdp::client::CdpClient;

use super::TreeNode;

/// Fill `url` on every ref-bearing `link` node, in two bounded CDP phases.
pub(super) async fn attach_link_urls(
    client: &CdpClient,
    session_id: &str,
    tree_nodes: &mut [TreeNode],
) {
    let link_nodes: Vec<(usize, i64)> = tree_nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.role == "link" && n.has_ref && n.backend_node_id.is_some())
        .filter_map(|(i, n)| n.backend_node_id.map(|bid| (i, bid)))
        .collect();

    if !link_nodes.is_empty() {
        // CDP has no batch resolve API, so we parallelize individual calls.
        // Phase 1: resolve all backend node IDs to JS object IDs in parallel.
        // Bounded CDP fan-out (rules_rust_paralelismo: never unbounded join_all).
        let cdp_limit = crate::concurrency::effective_limit_capped(32);
        let resolve_futs: Vec<_> = link_nodes
            .iter()
            .map(|&(idx, bid)| async move {
                let resolved = client
                    .send_command(
                        "DOM.resolveNode",
                        Some(serde_json::json!({ "backendNodeId": bid })),
                        Some(session_id),
                    )
                    .await;
                let obj_id = resolved.ok().and_then(|r| {
                    r.get("object")
                        .and_then(|o| o.get("objectId"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                });
                (idx, obj_id)
            })
            .collect();
        let resolved: Vec<(usize, Option<String>)> =
            crate::concurrency::join_bounded(resolve_futs, cdp_limit).await;

        // Phase 2: fetch hrefs for all resolved objects (bounded).
        let href_futs: Vec<_> = resolved
            .iter()
            .filter_map(|(idx, obj_id)| {
                let oid = obj_id.as_ref()?;
                Some(async move {
                    let result = client
                        .send_command(
                            "Runtime.callFunctionOn",
                            Some(serde_json::json!({
                                "objectId": oid,
                                "functionDeclaration": "function() { return this.href || ''; }",
                                "returnByValue": true,
                            })),
                            Some(session_id),
                        )
                        .await;
                    let href = result.ok().and_then(|r| {
                        r.get("result")
                            .and_then(|r| r.get("value"))
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                    });
                    (*idx, href)
                })
            })
            .collect();
        let hrefs: Vec<(usize, Option<String>)> =
            crate::concurrency::join_bounded(href_futs, cdp_limit).await;

        for (idx, href) in hrefs {
            if let Some(url) = href {
                tree_nodes[idx].url = Some(url);
            }
        }
    }
}
