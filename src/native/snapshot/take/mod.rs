// SPDX-License-Identifier: MIT OR Apache-2.0
//! `take_snapshot` orchestration.

mod iframe;

mod urls;

use iframe::{collect_backend_node_ids, resolve_iframe_frame_id};
use urls::attach_link_urls;

use super::cursor::{find_cursor_interactive_elements, promote_hidden_inputs};
use super::options::*;
use super::tree::*;
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::{EvaluateParams, EvaluateResult, GetFullAXTreeResult};
use crate::native::element::{resolve_ax_session, RefMap};
use rustc_hash::FxHashMap;
use serde_json::Value;

/// Walk the accessibility tree and mint the `@eN` refs an agent acts on.
///
/// This is the read half of every interaction: a ref only exists because a
/// snapshot created it, and it is valid only within this process.
pub async fn take_snapshot(
    client: &CdpClient,
    session_id: &str,
    options: &SnapshotOptions,
    ref_map: &mut RefMap,
    frame_id: Option<&str>,
    iframe_sessions: &FxHashMap<String, String>,
) -> Result<String, String> {
    client
        .send_command_no_params("DOM.enable", Some(session_id))
        .await?;
    client
        .send_command_no_params("Accessibility.enable", Some(session_id))
        .await?;

    // If a CSS selector is provided, resolve the set of backendNodeIds that
    // belong to the DOM subtree rooted at the matched element.  We use this
    // set to pick the right AX subtree root(s) later.
    let selector_backend_ids: Option<rustc_hash::FxHashSet<i64>> =
        if let Some(ref selector) = options.selector {
            let js = format!(
                "document.querySelector({})",
                serde_json::to_string(selector).unwrap_or_default()
            );
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

            // A throwing evaluation (e.g. an invalid CSS selector like a
            // snapshot ref "@e1") still yields an objectId — for the
            // exception object. Passing that to DOM.describeNode produces the
            // cryptic "Object id doesn't reference a Node"; fail clearly
            // instead, and only accept results that are actually DOM nodes.
            if let Some(exception) = result.exception_details {
                let detail = exception
                    .exception
                    .and_then(|e| e.description)
                    .unwrap_or(exception.text);
                return Err(format!("Invalid selector '{selector}': {detail}"));
            }
            if result.result.subtype.as_deref() != Some("node") {
                return Err(format!("Selector '{selector}' did not match any element"));
            }
            let object_id = result
                .result
                .object_id
                .ok_or_else(|| format!("Selector '{selector}' did not match any element"))?;

            // Request the full DOM subtree (depth: -1) so we can collect all
            // backendNodeIds that live under the matched element.
            let describe: Value = client
                .send_command(
                    "DOM.describeNode",
                    Some(serde_json::json!({ "objectId": object_id, "depth": -1 })),
                    Some(session_id),
                )
                .await?;

            let root_node = describe
                .get("node")
                .ok_or_else(|| format!("Could not resolve DOM node for selector '{selector}'"))?;

            let mut ids = rustc_hash::FxHashSet::default();
            collect_backend_node_ids(root_node, &mut ids);

            if ids.is_empty() {
                return Err(format!(
                    "Could not resolve backendNodeId for selector '{selector}'"
                ));
            }

            Some(ids)
        } else {
            None
        };

    let (ax_params, effective_session_id) =
        resolve_ax_session(frame_id, session_id, iframe_sessions);
    // Ensure domains are enabled on the iframe session (defensive fallback
    // in case the attach-time enable in execute_command was missed).
    if effective_session_id != session_id {
        let _ = client
            .send_command_no_params("DOM.enable", Some(effective_session_id))
            .await;
        let _ = client
            .send_command_no_params("Accessibility.enable", Some(effective_session_id))
            .await;
    }
    let ax_tree: GetFullAXTreeResult = client
        .send_command_typed(
            "Accessibility.getFullAXTree",
            &ax_params,
            Some(effective_session_id),
        )
        .await?;

    let (mut tree_nodes, root_indices) = build_tree(&ax_tree.nodes);

    // When a selector is given, find AX nodes whose backendDOMNodeId falls
    // within the target DOM subtree and pick the top-level ones as roots.
    let effective_roots = if let Some(ref id_set) = selector_backend_ids {
        // Mark which tree_nodes belong to the target DOM subtree.
        let in_subtree: Vec<bool> = tree_nodes
            .iter()
            .map(|n| n.backend_node_id.is_some_and(|bid| id_set.contains(&bid)))
            .collect();

        // An AX node is a "top-level" match if it is in the subtree but its
        // parent (in the AX tree) is not.
        let mut roots = Vec::new();
        for (idx, node) in tree_nodes.iter().enumerate() {
            if !in_subtree[idx] {
                continue;
            }
            let parent_in_subtree = node.parent_idx.is_some_and(|pidx| in_subtree[pidx]);
            if !parent_in_subtree {
                roots.push(idx);
            }
        }

        if roots.is_empty() {
            return Err(format!(
                "No accessibility node found for selector '{}'",
                options.selector.as_deref().unwrap_or("")
            ));
        }
        roots
    } else {
        root_indices
    };

    let mut tracker = RoleNameTracker::new();
    let mut next_ref: usize = ref_map.next_ref_num();

    let mut nodes_with_refs: Vec<(usize, usize)> = Vec::new();

    // Pre-collect cursor-interactive elements so we can mark them with refs during tree building
    let cursor_elements: FxHashMap<i64, CursorElementInfo> =
        find_cursor_interactive_elements(client, session_id)
            .await
            .unwrap_or_default();

    promote_hidden_inputs(&mut tree_nodes, &cursor_elements);

    for (idx, node) in tree_nodes.iter().enumerate() {
        let role = node.role.as_str();
        let mut should_ref = if INTERACTIVE_ROLES.contains(&role) {
            true
        } else if CONTENT_ROLES.contains(&role) {
            !node.name.is_empty()
        } else {
            false
        };

        if node
            .backend_node_id
            .is_some_and(|bid| cursor_elements.contains_key(&bid))
        {
            // ref elements that are cursor-interactive
            should_ref = true;
        }

        if should_ref {
            let nth = tracker.track(role, &node.name, idx);
            nodes_with_refs.push((idx, nth));
        }
    }

    let duplicates = tracker.get_duplicates();

    for (idx, nth) in &nodes_with_refs {
        let node = &tree_nodes[*idx];
        let key = format!("{}:{}", node.role, node.name);
        let actual_nth = if duplicates.contains_key(&key) {
            Some(*nth)
        } else {
            None
        };

        let ref_id = format!("e{next_ref}");
        next_ref += 1;

        ref_map.add_with_frame(
            ref_id.clone(),
            tree_nodes[*idx].backend_node_id,
            &tree_nodes[*idx].role,
            &tree_nodes[*idx].name,
            actual_nth,
            frame_id,
        );

        tree_nodes[*idx].has_ref = true;
        tree_nodes[*idx].ref_id = Some(ref_id);
    }

    // Populate cursor_info for ref-bearing nodes
    for (idx, _) in &nodes_with_refs {
        if let Some(bid) = tree_nodes[*idx].backend_node_id {
            if let Some(cursor_info) = cursor_elements.get(&bid) {
                tree_nodes[*idx].cursor_info = Some((*cursor_info).clone());
            }
        }
    }

    ref_map.set_next_ref_num(next_ref);

    if options.urls {
        attach_link_urls(client, session_id, &mut tree_nodes).await;
    }

    let mut output = String::new();
    for &root_idx in &effective_roots {
        render_tree(&tree_nodes, root_idx, 0, &mut output, options);
    }

    // Recurse into child iframes: for each Iframe node with a backend_node_id,
    // resolve the child frame ID and take a snapshot of its content.
    // We only recurse from the main frame (frame_id == None) to avoid
    // unbounded depth; nested iframes within iframes are not expanded.
    if frame_id.is_none() {
        let mut iframe_snapshots: Vec<(String, String)> = Vec::new(); // (ref_id, child_snapshot)
        for node in tree_nodes.iter() {
            if node.role != "Iframe" || !node.has_ref {
                continue;
            }
            let Some(bid) = node.backend_node_id else {
                continue;
            };
            let ref_id = node.ref_id.as_deref().unwrap_or("");
            if let Ok(child_fid) = resolve_iframe_frame_id(client, session_id, bid).await {
                // Snapshot the child frame; errors are silently ignored
                // (e.g. cross-origin iframes)
                if let Ok(child_text) = Box::pin(take_snapshot(
                    client,
                    session_id,
                    options,
                    ref_map,
                    Some(&child_fid),
                    iframe_sessions,
                ))
                .await
                {
                    if !child_text.is_empty()
                        && child_text != "(empty page)"
                        && child_text != "(no interactive elements)"
                    {
                        iframe_snapshots.push((ref_id.to_string(), child_text));
                    }
                }
            }
        }

        // Insert each child snapshot after its Iframe line in the output
        for (ref_id, child_text) in iframe_snapshots {
            let marker = format!("[ref={ref_id}]");
            if let Some(pos) = output.find(&marker) {
                // Find the end of the Iframe line
                let line_end = output[pos..]
                    .find('\n')
                    .map(|i| pos + i)
                    .unwrap_or(output.len());
                // Determine the indent of the Iframe line
                let line_start = output[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let iframe_line = &output[line_start..line_end];
                let iframe_indent = iframe_line.len() - iframe_line.trim_start().len();
                let child_indent = iframe_indent + 2; // one level deeper
                let prefix = " ".repeat(child_indent);

                let indented_child: String = child_text
                    .lines()
                    .map(|line| format!("{prefix}{line}\n"))
                    .collect();

                // Ensure there's a newline to insert after
                if line_end == output.len() {
                    output.push('\n');
                    output.push_str(&indented_child);
                } else {
                    output.insert_str(line_end + 1, &indented_child);
                }
            }
        }
    }

    if options.compact {
        output = compact_tree(&output, options.interactive);
    }

    let trimmed = output.trim().to_string();

    if trimmed.is_empty() {
        if options.interactive {
            return Ok("(no interactive elements)".to_string());
        }
        return Ok("(empty page)".to_string());
    }

    Ok(trimmed)
}
