// SPDX-License-Identifier: MIT OR Apache-2.0
//! AX node list to `TreeNode` tree, with depth assignment.

use super::super::ax::{extract_ax_string, extract_ax_string_opt, extract_properties};
use crate::native::cdp::types::AXNode;
use rustc_hash::FxHashMap;

use super::types::*;

pub(crate) fn build_tree(nodes: &[AXNode]) -> (Vec<TreeNode>, Vec<usize>) {
    let mut tree_nodes: Vec<TreeNode> = Vec::with_capacity(nodes.len());
    // Known size → avoid rehash while indexing a11y nodes (perf rules: with_capacity).
    let mut id_to_idx: FxHashMap<String, usize> =
        FxHashMap::with_capacity_and_hasher(nodes.len(), Default::default());

    for (i, node) in nodes.iter().enumerate() {
        let role = extract_ax_string(&node.role);
        let name = extract_ax_string(&node.name);
        let value_text = extract_ax_string_opt(&node.value);

        let (level, checked, expanded, selected, disabled, required) =
            extract_properties(&node.properties);

        if (node.ignored.unwrap_or(false) && role != "RootWebArea") || role == "InlineTextBox" {
            tree_nodes.push(TreeNode::empty());
            id_to_idx.insert(node.node_id.clone(), i);
            continue;
        }

        tree_nodes.push(TreeNode {
            role,
            name,
            level,
            checked,
            expanded,
            selected,
            disabled,
            required,
            value_text,
            backend_node_id: node.backend_d_o_m_node_id,
            children: Vec::new(),
            parent_idx: None,
            has_ref: false,
            ref_id: None,
            depth: 0,
            cursor_info: None,
            url: None,
        });
        id_to_idx.insert(node.node_id.clone(), i);
    }

    // Build parent-child relationships
    for (i, node) in nodes.iter().enumerate() {
        if let Some(ref child_ids) = node.child_ids {
            for cid in child_ids {
                if let Some(&child_idx) = id_to_idx.get(cid) {
                    tree_nodes[i].children.push(child_idx);
                    tree_nodes[child_idx].parent_idx = Some(i);
                }
            }
        }
    }

    // Process StaticText aggregation
    for i in 0..tree_nodes.len() {
        if tree_nodes[i].role.is_empty() || tree_nodes[i].children.is_empty() {
            continue;
        }

        let children_indices: Vec<usize> = tree_nodes[i].children.clone();

        // Continuous StaticText nodes at the same level are an artifact of HTML structure rather than semantic meaning.
        // They typically represent a single continuous piece of text on the page that was split due to inline elements, formatting tags, or other structural reasons.
        // Thus, continuous StaticText children are aggregated into the first one.
        let mut start = 0;
        while start < children_indices.len() {
            // Skip non-StaticText nodes
            if tree_nodes[children_indices[start]].role != "StaticText" {
                start += 1;
                continue;
            }

            // Find the end of the current StaticText sequence
            let mut end = start + 1;
            while end < children_indices.len()
                && tree_nodes[children_indices[end]].role == "StaticText"
            {
                end += 1;
            }

            // If we have a sequence of at least two StaticText
            if end > start + 1 {
                // Collect and aggregate all names from the sequence
                let aggregated_name: String = (start..end)
                    .map(|idx| tree_nodes[children_indices[idx]].name.clone())
                    .collect();
                // Always aggregate into the first node of the sequence
                tree_nodes[children_indices[start]].name = aggregated_name;
                // Clear the rest of the nodes in the sequence (from start+1 to end-1)
                for j in (start + 1)..end {
                    tree_nodes[children_indices[j]].clear();
                }
            }
            start = end;
        }

        // Deduplicate redundant StaticText
        if children_indices.len() == 1
            && tree_nodes[children_indices[0]].role == "StaticText"
            && tree_nodes[i].name == tree_nodes[children_indices[0]].name
        {
            tree_nodes[children_indices[0]].clear();
        }
    }

    // Set depths
    let mut root_indices = Vec::new();
    let children_exist: Vec<bool> = nodes.iter().map(|_| false).collect();
    let mut is_child = children_exist;
    for node in &tree_nodes {
        for &child in &node.children {
            is_child[child] = true;
        }
    }
    for (i, &is_c) in is_child.iter().enumerate() {
        if !is_c {
            root_indices.push(i);
        }
    }

    fn set_depth(nodes: &mut [TreeNode], idx: usize, depth: usize) {
        nodes[idx].depth = depth;
        let children: Vec<usize> = nodes[idx].children.clone();
        for child_idx in children {
            set_depth(nodes, child_idx, depth + 1);
        }
    }

    for &root in &root_indices {
        set_depth(&mut tree_nodes, root, 0);
    }

    (tree_nodes, root_indices)
}
