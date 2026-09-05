// SPDX-License-Identifier: MIT OR Apache-2.0

//! Immediate dominator tree (sequential; N-142).

use rustc_hash::FxHashSet;
use serde_json::Value;

use super::types::SnapshotGraph;

impl SnapshotGraph {
    pub(crate) fn compute_idom(&self) -> Vec<Option<usize>> {
        let n = self.nodes.len();
        if n == 0 {
            return Vec::new();
        }

        // Prefer synthetic/root-like nodes; else first node with no retainers; else 0.
        let mut roots: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                &*node.type_name == "synthetic"
                    || node.name.contains("GC roots")
                    || &*node.name == "(GC roots)"
            })
            .map(|(i, _)| i)
            .collect();
        if roots.is_empty() {
            roots = self
                .nodes
                .iter()
                .enumerate()
                .filter(|(i, _)| self.in_edges[*i].is_empty())
                .map(|(i, _)| i)
                .collect();
        }
        if roots.is_empty() {
            roots.push(0);
        }
        let root = roots[0];

        // Build predecessor lists from reverse edges; ensure root has no preds.
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (to, edges) in self.in_edges.iter().enumerate() {
            if to == root {
                continue;
            }
            for e in edges {
                if e.from < n {
                    preds[to].push(e.from);
                }
            }
        }

        // RPO via iterative DFS on forward graph.
        let mut rpo = Vec::with_capacity(n);
        let mut visited = vec![false; n];
        let mut stack = vec![(root, false)];
        while let Some((u, expanded)) = stack.pop() {
            if expanded {
                rpo.push(u);
                continue;
            }
            if visited[u] {
                continue;
            }
            visited[u] = true;
            stack.push((u, true));
            for e in &self.out_edges[u] {
                if e.to < n && !visited[e.to] {
                    stack.push((e.to, false));
                }
            }
        }
        // Orphans not reachable from root still get an entry.
        for (i, was_visited) in visited.iter().enumerate() {
            if !was_visited {
                rpo.push(i);
            }
        }
        rpo.reverse(); // reverse postorder

        let mut idom: Vec<Option<usize>> = vec![None; n];
        idom[root] = Some(root);

        // Map index in rpo for semi-order compare.
        let mut rpo_index = vec![0usize; n];
        for (i, &u) in rpo.iter().enumerate() {
            rpo_index[u] = i;
        }

        let intersect =
            |mut b1: usize, mut b2: usize, idom: &[Option<usize>], rpo_index: &[usize]| {
                while b1 != b2 {
                    while rpo_index[b1] > rpo_index[b2] {
                        b1 = idom[b1].unwrap_or(b1);
                    }
                    while rpo_index[b2] > rpo_index[b1] {
                        b2 = idom[b2].unwrap_or(b2);
                    }
                }
                b1
            };

        let mut changed = true;
        let mut iterations = 0usize;
        while changed && iterations < n.saturating_mul(2).max(8) {
            changed = false;
            iterations += 1;
            for &u in &rpo {
                if u == root {
                    continue;
                }
                let mut new_idom: Option<usize> = None;
                for &p in &preds[u] {
                    if idom[p].is_none() {
                        continue;
                    }
                    new_idom = Some(match new_idom {
                        None => p,
                        Some(cur) => intersect(p, cur, &idom, &rpo_index),
                    });
                }
                if new_idom.is_some() && new_idom != idom[u] {
                    idom[u] = new_idom;
                    changed = true;
                }
            }
        }
        idom
    }

    pub(crate) fn dominator_chain(&self, idx: usize) -> Vec<Value> {
        let idom = self.compute_idom();
        let mut chain = Vec::new();
        let mut seen: FxHashSet<usize> = FxHashSet::default();
        let mut cur = idx;
        for _ in 0..self.nodes.len().saturating_add(1) {
            if !seen.insert(cur) {
                break;
            }
            chain.push(self.node_json(cur));
            match idom.get(cur).copied().flatten() {
                Some(d) if d != cur => cur = d,
                _ => break,
            }
        }
        chain.reverse(); // root → … → node
        chain
    }
}
