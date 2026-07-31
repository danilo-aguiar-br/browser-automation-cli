// SPDX-License-Identifier: MIT OR Apache-2.0

//! Retaining paths over the snapshot graph.

use std::collections::VecDeque;

use serde_json::{json, Value};

use super::types::SnapshotGraph;

impl SnapshotGraph {
    pub(crate) fn retaining_paths(
        &self,
        idx: usize,
        max_depth: usize,
        max_paths: usize,
    ) -> (Vec<Value>, bool) {
        // BFS upward on reverse edges toward roots (nodes with no retainers or synthetic).
        let mut paths: Vec<Value> = Vec::new();
        let mut limits = false;
        // state: (node, path_of_node_indices from target upward)
        let mut q: VecDeque<(usize, Vec<usize>)> = VecDeque::new();
        q.push_back((idx, vec![idx]));
        let mut visited_states = 0usize;
        let max_states = super::super::limits::MAX_STATES;

        while let Some((u, path)) = q.pop_front() {
            visited_states += 1;
            if visited_states > max_states {
                limits = true;
                break;
            }
            if paths.len() >= max_paths {
                limits = true;
                break;
            }
            let is_root = self.in_edges[u].is_empty()
                || self.nodes[u].type_name == "synthetic"
                || self.nodes[u].name.contains("GC roots");
            if (is_root && path.len() > 1) || path.len() > max_depth {
                let nodes_json: Vec<Value> =
                    path.iter().rev().map(|&i| self.node_json(i)).collect();
                // path was target→…→ancestor; reverse to root→…→target
                if path.len() > max_depth && !is_root {
                    // depth limit without root
                    paths.push(json!({
                        "nodes": nodes_json,
                        "depth": path.len().saturating_sub(1),
                        "reached_root": false,
                    }));
                } else {
                    paths.push(json!({
                        "nodes": nodes_json,
                        "depth": path.len().saturating_sub(1),
                        "reached_root": is_root,
                    }));
                }
                continue;
            }
            if self.in_edges[u].is_empty() {
                let nodes_json: Vec<Value> =
                    path.iter().rev().map(|&i| self.node_json(i)).collect();
                paths.push(json!({
                    "nodes": nodes_json,
                    "depth": path.len().saturating_sub(1),
                    "reached_root": true,
                }));
                continue;
            }
            for e in &self.in_edges[u] {
                if path.contains(&e.from) {
                    continue;
                }
                if path.len() > max_depth {
                    limits = true;
                    continue;
                }
                let mut next = path.clone();
                next.push(e.from);
                q.push_back((e.from, next));
            }
        }
        (paths, limits)
    }
}
