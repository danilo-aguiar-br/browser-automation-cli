// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cursor-interactive discovery and hidden-input promotion.

use super::tree::{CursorElementInfo, HiddenInputKind, TreeNode};
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::types::*;
use rustc_hash::FxHashMap;
use serde_json::Value;

pub(super) async fn find_cursor_interactive_elements(
    client: &CdpClient,
    session_id: &str,
) -> Result<FxHashMap<i64, CursorElementInfo>, String> {
    // Single JS evaluation that matches the v0.19.0 Node.js findCursorInteractiveElements():
    // - Uses querySelectorAll('*') to walk all elements
    // - Checks getComputedStyle(el).cursor === 'pointer'
    // - Checks onclick attribute/handler and tabindex
    // - Skips interactiveTags (a, button, input, select, textarea, details, summary)
    // - Skips elements with interactive ARIA roles
    // - Deduplicates inherited cursor:pointer from parent
    // - Skips empty text and zero-size elements
    // - Tags each matched element with data-__ab-ci for batch backendNodeId resolution
    let js = r#"
(function() {
    var results = [];
    if (!document.body) return results;

    var interactiveRoles = {
        'button':1, 'link':1, 'textbox':1, 'checkbox':1, 'radio':1, 'combobox':1, 'listbox':1,
        'menuitem':1, 'menuitemcheckbox':1, 'menuitemradio':1, 'option':1, 'searchbox':1,
        'slider':1, 'spinbutton':1, 'switch':1, 'tab':1, 'treeitem':1
    };
    var interactiveTags = {
        'a':1, 'button':1, 'input':1, 'select':1, 'textarea':1, 'details':1, 'summary':1
    };

    var allElements = document.body.querySelectorAll('*');
    for (var i = 0; i < allElements.length; i++) {
        var el = allElements[i];

        if (el.closest && el.closest('[hidden], [aria-hidden="true"]')) continue;

        var tagName = el.tagName.toLowerCase();
        if (interactiveTags[tagName]) continue;

        var role = el.getAttribute('role');
        if (role && interactiveRoles[role.toLowerCase()]) continue;

        var computedStyle = getComputedStyle(el);
        var hasCursorPointer = computedStyle.cursor === 'pointer';
        var hasOnClick = el.hasAttribute('onclick') || el.onclick !== null;
        var tabIndex = el.getAttribute('tabindex');
        var hasTabIndex = tabIndex !== null && tabIndex !== '-1';
        var ce = el.getAttribute('contenteditable');
        var isEditable = ce === '' || ce === 'true';

        if (!hasCursorPointer && !hasOnClick && !hasTabIndex && !isEditable) continue;

        // Skip elements that only inherit cursor:pointer from an ancestor
        if (hasCursorPointer && !hasOnClick && !hasTabIndex && !isEditable) {
            var parent = el.parentElement;
            if (parent && getComputedStyle(parent).cursor === 'pointer') continue;
        }

        var text = (el.textContent || '').trim().slice(0, 100);

        var rect = el.getBoundingClientRect();
        if (rect.width === 0 || rect.height === 0) continue;

        // Detect hidden radio/checkbox inputs inside this element (common pattern:
        // <label> wrapping a display:none <input type="radio"> styled as a card).
        // Note: we only check display/visibility/hidden, NOT opacity:0 or sr-only,
        // because those inputs remain in Chrome's AX tree and already appear as
        // role="radio" without promotion.
        var hiddenInputType = null;
        var hiddenInputChecked = null;
        var hiddenInput = el.querySelector('input[type="radio"], input[type="checkbox"]');
        if (hiddenInput) {
            var hiddenInputStyle = getComputedStyle(hiddenInput);
            var isInputHidden = hiddenInputStyle.display === 'none' || hiddenInputStyle.visibility === 'hidden' || hiddenInput.hidden;
            if (isInputHidden) {
                hiddenInputType = hiddenInput.type;
                hiddenInputChecked = hiddenInput.indeterminate ? 'mixed' : String(hiddenInput.checked);
            }
        }

        el.setAttribute('data-__ab-ci', String(results.length));
        results.push({
            text: text,
            tagName: tagName,
            hasOnClick: hasOnClick,
            hasCursorPointer: hasCursorPointer,
            hasTabIndex: hasTabIndex,
            isEditable: isEditable,
            hiddenInputType: hiddenInputType,
            hiddenInputChecked: hiddenInputChecked
        });
    }
    return results;
})()
"#;

    let result: EvaluateResult = client
        .send_command_typed(
            "Runtime.evaluate",
            &EvaluateParams {
                expression: js.to_string(),
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(session_id),
        )
        .await?;

    let elements: Vec<Value> = result
        .result
        .value
        .and_then(|v| serde_json::from_value::<Vec<Value>>(v).ok())
        .unwrap_or_default();

    if elements.is_empty() {
        return Ok(FxHashMap::default());
    }

    // Batch-resolve backendNodeIds: use DOM.getDocument to get the root nodeId,
    // then DOM.querySelectorAll to get all tagged elements in a single call.
    let doc: Value = client
        .send_command(
            "DOM.getDocument",
            Some(serde_json::json!({ "depth": 0 })),
            Some(session_id),
        )
        .await?;

    let root_node_id = doc
        .get("root")
        .and_then(|r| r.get("nodeId"))
        .and_then(|v| v.as_i64())
        .ok_or("DOM.getDocument did not return root nodeId")?;

    let query_result: Value = client
        .send_command(
            "DOM.querySelectorAll",
            Some(serde_json::json!({
                "nodeId": root_node_id,
                "selector": "[data-__ab-ci]"
            })),
            Some(session_id),
        )
        .await?;

    let node_ids: Vec<i64> = query_result
        .get("nodeIds")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
        .unwrap_or_default();

    // Resolve backendNodeIds for each DOM node (bounded concurrent CDP).
    let cdp_limit = crate::concurrency::effective_limit_capped(crate::concurrency::CDP_FANOUT_CAP);
    let describe_futures: Vec<_> = node_ids
        .iter()
        .map(|&node_id| {
            client.send_command(
                "DOM.describeNode",
                Some(serde_json::json!({ "nodeId": node_id })),
                Some(session_id),
            )
        })
        .collect();

    let describe_results =
        crate::concurrency::join_bounded_ordered(describe_futures, cdp_limit).await;

    // Build a map from data-__ab-ci index to backendNodeId.
    let mut idx_to_backend: FxHashMap<usize, i64> = FxHashMap::default();
    for desc in describe_results.into_iter().flatten() {
        let backend_id = desc
            .get("node")
            .and_then(|n| n.get("backendNodeId"))
            .and_then(|v| v.as_i64());
        let ci_attr = desc
            .get("node")
            .and_then(|n| n.get("attributes"))
            .and_then(|a| a.as_array())
            .and_then(|attrs| {
                // attributes is a flat array: [name, value, name, value, ...]
                attrs
                    .iter()
                    .enumerate()
                    .find(|(_, v)| v.as_str() == Some("data-__ab-ci"))
                    .and_then(|(i, _)| attrs.get(i + 1))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<usize>().ok())
            });
        if let (Some(bid), Some(idx)) = (backend_id, ci_attr) {
            idx_to_backend.insert(idx, bid);
        }
    }

    // Clean up the data attributes we injected for backendNodeId resolution.
    let cleanup_js =
        r#"(function(){ var els = document.querySelectorAll('[data-__ab-ci]'); for (var i = 0; i < els.length; i++) els[i].removeAttribute('data-__ab-ci'); return els.length; })()"#.to_string();
    if let Err(e) = client
        .send_command_typed::<EvaluateParams, EvaluateResult>(
            "Runtime.evaluate",
            &EvaluateParams {
                expression: cleanup_js,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(session_id),
        )
        .await
    {
        tracing::warn!(
            target: "browser_automation_cli::native::snapshot",
            error = %e,
            "failed to clean up data-__ab-ci attributes"
        );
    }

    // Build the map
    let mut map: FxHashMap<i64, CursorElementInfo> = FxHashMap::default();
    for (i, elem) in elements.iter().enumerate() {
        let backend_node_id = idx_to_backend.get(&i).copied();

        // Role differentiation: v0.19.0 uses 'clickable' for cursor:pointer or onclick,
        // 'focusable' for tabindex-only elements.
        let has_cursor_pointer = elem
            .get("hasCursorPointer")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let has_on_click = elem
            .get("hasOnClick")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let has_tab_index = elem
            .get("hasTabIndex")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let is_editable = elem
            .get("isEditable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let kind = if has_cursor_pointer || has_on_click {
            "clickable"
        } else if is_editable {
            "editable"
        } else {
            "focusable"
        };

        let mut hints: Vec<String> = Vec::new();
        if has_cursor_pointer {
            hints.push("cursor:pointer".to_string());
        }
        if has_on_click {
            hints.push("onclick".to_string());
        }
        if has_tab_index {
            hints.push("tabindex".to_string());
        }
        if is_editable {
            hints.push("contenteditable".to_string());
        }

        let text = elem
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        let hidden_input_kind = elem
            .get("hiddenInputType")
            .and_then(|v| v.as_str())
            .and_then(HiddenInputKind::parse);
        let hidden_input_checked = elem
            .get("hiddenInputChecked")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(bid) = backend_node_id {
            map.insert(
                bid,
                CursorElementInfo {
                    kind: kind.to_string(),
                    hints,
                    text,
                    hidden_input_kind,
                    hidden_input_checked,
                },
            );
        }
    }

    Ok(map)
}

/// Promote LabelText/generic nodes that wrap a hidden radio/checkbox input.
/// When a `<label>` contains a `display:none` `<input type="radio">`, Chrome excludes
/// the input from the AX tree entirely, leaving only the label with role="LabelText"
/// and an empty name. We detect these via cursor-interactive scanning and promote
/// the label to the correct input role so consumers see role="radio" in data.refs.
pub(super) fn promote_hidden_inputs(
    tree_nodes: &mut [TreeNode],
    cursor_elements: &FxHashMap<i64, CursorElementInfo>,
) {
    for node in tree_nodes.iter_mut() {
        if !matches!(node.role.as_str(), "LabelText" | "generic") {
            continue;
        }
        let cursor_info = match node
            .backend_node_id
            .and_then(|bid| cursor_elements.get(&bid))
        {
            Some(info) => info,
            None => continue,
        };
        if let Some(input_kind) = cursor_info.hidden_input_kind {
            node.role = input_kind.as_role().to_string();
            if node.name.is_empty() && !cursor_info.text.is_empty() {
                node.name = cursor_info.text.clone();
            }
            if let Some(ref checked) = cursor_info.hidden_input_checked {
                node.checked = Some(checked.clone());
            }
        }
    }
}
