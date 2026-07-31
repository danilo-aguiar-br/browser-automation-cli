// SPDX-License-Identifier: MIT OR Apache-2.0

use rustc_hash::FxHashMap;

use super::cursor::promote_hidden_inputs;
use super::options::*;
use super::tree::*;
use crate::native::element::{resolve_ax_session, RefMap};

fn build_dedup_set(ref_map: &RefMap) -> std::collections::HashSet<String> {
    ref_map
        .entries_sorted()
        .into_iter()
        .filter(|(_, entry)| !entry.name.is_empty())
        .map(|(_, entry)| entry.name.to_lowercase())
        .collect()
}

/// Recursively collect all `backendNodeId` values from a CDP DOM node tree
/// (as returned by `DOM.describeNode` with `depth: -1`).

#[test]
fn test_interactive_roles() {
    assert!(INTERACTIVE_ROLES.contains(&"button"));
    assert!(INTERACTIVE_ROLES.contains(&"textbox"));
    assert!(!INTERACTIVE_ROLES.contains(&"heading"));
}

#[test]
fn test_content_roles() {
    assert!(CONTENT_ROLES.contains(&"heading"));
    assert!(!CONTENT_ROLES.contains(&"button"));
}

#[test]
fn test_compact_tree_basic() {
    let tree = "- navigation\n  - link \"Home\" [ref=e1]\n  - link \"About\" [ref=e2]\n- main\n  - heading \"Title\"\n  - paragraph\n    - text: Hello\n";
    let result = compact_tree(tree, false);
    assert!(result.contains("[ref=e1]"));
    assert!(result.contains("[ref=e2]"));
    assert!(result.contains("Hello"));
}

#[test]
fn test_compact_tree_radio_checkbox() {
    // Radio/checkbox lines have attributes before ref (e.g. [checked=false, ref=e1])
    // so "ref=" appears without a leading "[" — compact_tree must still keep them.
    let tree = "- form\n  - radio \"Single unit\" [checked=false, ref=e1]\n  - checkbox \"I agree\" [checked=false, ref=e2]\n  - button \"Submit\" [ref=e3]\n";
    let result = compact_tree(tree, true);
    assert!(
        result.contains("radio \"Single unit\""),
        "radio should be kept"
    );
    assert!(
        result.contains("checkbox \"I agree\""),
        "checkbox should be kept"
    );
    assert!(
        result.contains("button \"Submit\""),
        "button should be kept"
    );
}

#[test]
fn test_compact_tree_empty_interactive() {
    let result = compact_tree("- generic\n", true);
    assert_eq!(result, "(no interactive elements)");
}

#[test]
fn test_count_indent() {
    assert_eq!(count_indent("- heading"), 0);
    assert_eq!(count_indent("  - link"), 1);
    assert_eq!(count_indent("    - text"), 2);
}

#[test]
fn test_role_name_tracker() {
    let mut tracker = RoleNameTracker::new();
    assert_eq!(tracker.track("button", "Submit", 0), 0);
    assert_eq!(tracker.track("button", "Submit", 1), 1);
    assert_eq!(tracker.track("button", "Cancel", 2), 0);

    let dups = tracker.get_duplicates();
    assert!(dups.contains_key("button:Submit"));
    assert!(!dups.contains_key("button:Cancel"));
}

// -----------------------------------------------------------------------
// Cursor-interactive text dedup (Issue #841 regression guard)
// -----------------------------------------------------------------------

#[test]
fn test_dedup_set_from_ref_map_names() {
    let mut ref_map = RefMap::new();
    ref_map.add("e1".to_string(), Some(1), "link", "Example Link", None);
    ref_map.add("e2".to_string(), Some(2), "button", "Submit", None);

    let set = build_dedup_set(&ref_map);
    assert!(set.contains("example link"));
    assert!(set.contains("submit"));
    assert!(!set.contains("other text"));
}

#[test]
fn test_dedup_set_case_insensitive() {
    let mut ref_map = RefMap::new();
    ref_map.add("e1".to_string(), Some(1), "button", "Submit Form", None);

    let set = build_dedup_set(&ref_map);
    assert!(set.contains("submit form"));
    assert!(!set.contains("Submit Form"));
}

#[test]
fn test_dedup_set_empty_inputs() {
    let ref_map = RefMap::new();
    let set = build_dedup_set(&ref_map);
    assert!(set.is_empty());
}

#[test]
fn test_dedup_set_skips_empty_names() {
    let mut ref_map = RefMap::new();
    ref_map.add("e1".to_string(), Some(1), "generic", "", None);
    ref_map.add("e2".to_string(), Some(2), "button", "OK", None);

    let set = build_dedup_set(&ref_map);
    assert_eq!(set.len(), 1);
    assert!(set.contains("ok"));
}

// -----------------------------------------------------------------------
// resolve_ax_session tests (Issue #925 regression guard)
// Cross-origin iframes must use a dedicated session without frameId.
// Same-origin iframes must use the parent session with frameId.
// -----------------------------------------------------------------------

#[test]
fn test_cross_origin_iframe_uses_dedicated_session() {
    let parent_session = "parent-session";
    let iframe_frame_id = "cross-origin-iframe-frame";
    let iframe_session = "cross-origin-iframe-session";

    let mut iframe_sessions = FxHashMap::default();
    iframe_sessions.insert(iframe_frame_id.to_string(), iframe_session.to_string());

    let (params, session) =
        resolve_ax_session(Some(iframe_frame_id), parent_session, &iframe_sessions);

    assert_eq!(session, iframe_session);
    assert_eq!(params, serde_json::json!({}));
}

#[test]
fn test_same_origin_iframe_uses_parent_session_with_frame_id() {
    let parent_session = "parent-session";
    let iframe_frame_id = "same-origin-iframe-frame";
    let iframe_sessions = FxHashMap::default();

    let (params, session) =
        resolve_ax_session(Some(iframe_frame_id), parent_session, &iframe_sessions);

    assert_eq!(session, parent_session);
    assert_eq!(params, serde_json::json!({ "frameId": iframe_frame_id }));
}

#[test]
fn test_main_frame_uses_parent_session() {
    let parent_session = "parent-session";
    let iframe_sessions = FxHashMap::default();

    let (params, session) = resolve_ax_session(None, parent_session, &iframe_sessions);

    assert_eq!(session, parent_session);
    assert_eq!(params, serde_json::json!({}));
}

// -----------------------------------------------------------------------
// promote_hidden_inputs
// -----------------------------------------------------------------------

fn make_node(role: &str, name: &str, backend_node_id: Option<i64>) -> TreeNode {
    let mut node = TreeNode::empty();
    node.role = role.to_string();
    node.name = name.to_string();
    node.backend_node_id = backend_node_id;
    node
}

fn make_cursor_info(
    hidden_kind: Option<HiddenInputKind>,
    hidden_checked: Option<&str>,
    text: &str,
) -> CursorElementInfo {
    CursorElementInfo {
        kind: "clickable".to_string(),
        hints: vec!["cursor:pointer".to_string()],
        text: text.to_string(),
        hidden_input_kind: hidden_kind,
        hidden_input_checked: hidden_checked.map(|s| s.to_string()),
    }
}

#[test]
fn test_promote_label_with_hidden_radio() {
    let mut nodes = vec![
        make_node("LabelText", "", Some(1)),
        make_node("LabelText", "", Some(2)),
        make_node("button", "Submit", Some(3)),
    ];
    let mut cursor_elements = FxHashMap::default();
    cursor_elements.insert(
        1,
        make_cursor_info(Some(HiddenInputKind::Radio), Some("false"), "Option A"),
    );
    cursor_elements.insert(
        2,
        make_cursor_info(Some(HiddenInputKind::Radio), Some("true"), "Option B"),
    );

    promote_hidden_inputs(&mut nodes, &cursor_elements);

    assert_eq!(nodes[0].role, "radio");
    assert_eq!(nodes[0].name, "Option A");
    assert_eq!(nodes[0].checked, Some("false".to_string()));
    assert_eq!(nodes[1].role, "radio");
    assert_eq!(nodes[1].name, "Option B");
    assert_eq!(nodes[1].checked, Some("true".to_string()));
    // button should be untouched
    assert_eq!(nodes[2].role, "button");
}

#[test]
fn test_promote_preserves_existing_name() {
    // If AX tree already has a name, don't overwrite with textContent
    let mut nodes = vec![make_node("LabelText", "AX Name", Some(1))];
    let mut cursor_elements = FxHashMap::default();
    cursor_elements.insert(
        1,
        make_cursor_info(Some(HiddenInputKind::Radio), Some("false"), "Text Content"),
    );

    promote_hidden_inputs(&mut nodes, &cursor_elements);

    assert_eq!(nodes[0].role, "radio");
    assert_eq!(nodes[0].name, "AX Name"); // preserved, not overwritten
}

#[test]
fn test_promote_skips_without_hidden_input() {
    // Cursor-interactive label WITHOUT a hidden input should not be promoted
    let mut nodes = vec![make_node("LabelText", "", Some(1))];
    let mut cursor_elements = FxHashMap::default();
    cursor_elements.insert(1, make_cursor_info(None, None, "Click me"));

    promote_hidden_inputs(&mut nodes, &cursor_elements);

    assert_eq!(nodes[0].role, "LabelText"); // unchanged
}
