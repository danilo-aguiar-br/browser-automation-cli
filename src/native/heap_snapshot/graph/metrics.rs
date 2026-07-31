// SPDX-License-Identifier: MIT OR Apache-2.0

//! Graph metrics: resolve, BFS distances, retained sizes, detachedness.

use std::collections::VecDeque;

use super::types::{NodeRec, SnapshotGraph};

impl SnapshotGraph {
    pub(crate) fn resolve_node(&self, node_id_or_index: u64) -> Result<usize, String> {
        if let Some(&idx) = self.id_to_index.get(&node_id_or_index) {
            return Ok(idx);
        }
        let idx = node_id_or_index as usize;
        if idx < self.nodes.len() {
            return Ok(idx);
        }
        Err(format!(
            "node id/index {node_id_or_index} not found (node_count={})",
            self.nodes.len()
        ))
    }

    pub(crate) fn pick_root(&self) -> usize {
        // Prefer synthetic/(GC roots); else first node with no retainers; else 0.
        if let Some((i, _)) = self.nodes.iter().enumerate().find(|(_, n)| {
            n.name.contains("GC roots") || n.type_name == "synthetic" || n.name == "(GC roots)"
        }) {
            return i;
        }
        self.nodes
            .iter()
            .enumerate()
            .find(|(i, _)| self.in_edges[*i].is_empty())
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// BFS distance from the graph root along outgoing edges (`None` if unreachable).
    pub(crate) fn distances_from_root(&self) -> Vec<Option<u64>> {
        let n = self.nodes.len();
        let mut dist = vec![None; n];
        if n == 0 {
            return dist;
        }
        let root = self.pick_root();
        let mut q = VecDeque::new();
        dist[root] = Some(0);
        q.push_back(root);
        while let Some(u) = q.pop_front() {
            let d = dist[u].unwrap_or(0);
            for e in &self.out_edges[u] {
                if dist[e.to].is_none() {
                    dist[e.to] = Some(d + 1);
                    q.push_back(e.to);
                }
            }
        }
        dist
    }

    /// Retained size per node: self_size of the node plus all nodes it dominates.
    pub(crate) fn retained_sizes(&self) -> Vec<u64> {
        let n = self.nodes.len();
        let mut retained = vec![0u64; n];
        if n == 0 {
            return retained;
        }
        let idom = self.compute_idom();
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, &dom) in idom.iter().enumerate() {
            if let Some(d) = dom {
                if d != i {
                    children[d].push(i);
                }
            }
        }
        // Post-order DFS from each root of the dominator forest.
        fn dfs(
            u: usize,
            children: &[Vec<usize>],
            nodes: &[NodeRec],
            retained: &mut [u64],
            seen: &mut [bool],
        ) {
            if seen[u] {
                return;
            }
            seen[u] = true;
            let mut sum = nodes[u].self_size;
            for &c in &children[u] {
                dfs(c, children, nodes, retained, seen);
                sum = sum.saturating_add(retained[c]);
            }
            retained[u] = sum;
        }
        let mut seen = vec![false; n];
        for i in 0..n {
            if !seen[i] {
                // Climb to dominator-tree root.
                let mut r = i;
                let mut guard = 0;
                while let Some(d) = idom[r] {
                    if d == r || guard > n {
                        break;
                    }
                    r = d;
                    guard += 1;
                }
                dfs(r, &children, &self.nodes, &mut retained, &mut seen);
            }
        }
        for (i, ret) in retained.iter_mut().enumerate() {
            if *ret == 0 {
                *ret = self.nodes[i].self_size;
            }
        }
        retained
    }

    pub(crate) fn detachedness_label(raw: Option<u64>) -> String {
        match raw {
            None => "unknown".into(),
            Some(0) => "attached".into(),
            Some(1) => "detached".into(),
            Some(2) => "unknown".into(),
            Some(v) => format!("code_{v}"),
        }
    }
}
