// SPDX-License-Identifier: MIT OR Apache-2.0
//! Offline V8 `.heapsnapshot` analysis for `browser-automation-cli heap *`.
#![allow(missing_docs)]
//!
//! Parses the Chrome heap snapshot JSON format and rebuilds a real object graph:
//! outgoing edges, retainers (reverse edges), dominator chains, retaining paths,
//! and per-node object details (distance, retained size, detachedness).
//!
//! # Workload / parallelism (rules_rust_paralelismo)
//!
//! **CPU-bound** graph algorithms (dominators, retained size) over a single
//! shared adjacency structure.
//!
//! - **PAR-93:** node materialize from flat arrays uses Rayon when
//!   `n ≥ CPU_MAP_THRESHOLD`; class maps merge sequentially after.
//! - **Dominator phases stay sequential** (N-142 / N-152): RPO → idom → retained
//!   have data dependencies; blind `par_iter` races or multiplies RSS.
//! - Independent post-idom maps (filter/score node lists, dups) use
//!   [`crate::concurrency::map_cpu`] / [`crate::concurrency::sort_by_cpu`] when
//!   length ≥ threshold (PAR-65 / PAR-107).
//! - Entry point is **sync CLI** (`spawn_blocking` if ever called from async).
//! - File size is hard-capped by [`MAX_HEAP_SNAPSHOT_BYTES`].

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use serde_json::{json, Value};

/// Default caps to keep multi-GB snapshots agent-usable.
const DEFAULT_MAX_RETAINERS: usize = 200;
const DEFAULT_MAX_EDGES: usize = 200;
const DEFAULT_MAX_PATHS: usize = 32;
const DEFAULT_MAX_PATH_DEPTH: usize = 8;
const DEFAULT_MAX_CLASS_NODES: usize = 500;
/// Hard file-size budget before reading a `.heapsnapshot` into RAM
/// (rules: never `read_to_string` untrusted/huge input without a ceiling).
const MAX_HEAP_SNAPSHOT_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone)]
struct NodeRec {
    index: usize,
    type_name: String,
    name: String,
    id: u64,
    self_size: u64,
    edge_count: usize,
    /// V8 detachedness enum when present in `node_fields`; else `None`.
    detachedness: Option<u64>,
}

#[derive(Debug, Clone)]
struct EdgeRec {
    from: usize,
    to: usize,
    type_name: String,
    name: String,
}

#[derive(Debug)]
struct SnapshotGraph {
    path: String,
    bytes: u64,
    nodes: Vec<NodeRec>,
    /// Outgoing edges by node index.
    out_edges: Vec<Vec<EdgeRec>>,
    /// Incoming edges by node index (retainers).
    in_edges: Vec<Vec<EdgeRec>>,
    /// node id field → node index
    id_to_index: HashMap<u64, usize>,
    class_counts: HashMap<String, u64>,
    class_self_sizes: HashMap<String, u64>,
    /// class name → node indices
    class_to_nodes: HashMap<String, Vec<usize>>,
    node_fields: Vec<String>,
    edge_fields: Vec<String>,
    node_types: Vec<String>,
    edge_types: Vec<String>,
    string_count: u64,
    strings: Vec<String>,
}

impl SnapshotGraph {
    fn load(path: &Path) -> Result<Self, String> {
        let meta = std::fs::metadata(path).map_err(|e| format!("heap file: {e}"))?;
        if meta.len() > MAX_HEAP_SNAPSHOT_BYTES {
            return Err(format!(
                "heap snapshot too large: {} bytes > {} budget (use a smaller capture)",
                meta.len(),
                MAX_HEAP_SNAPSHOT_BYTES
            ));
        }
        // Capacity known from metadata → try_reserve before full read (OOM → Result).
        let mut raw = String::new();
        raw.try_reserve_exact(meta.len() as usize)
            .map_err(|e| format!("heap allocate failed ({e}); file may exceed host RAM"))?;
        let file = std::fs::File::open(path).map_err(|e| format!("heap open: {e}"))?;
        use std::io::Read;
        std::io::BufReader::new(file)
            .read_to_string(&mut raw)
            .map_err(|e| format!("heap read: {e}"))?;
        let v: Value = crate::json_util::from_str(&raw)
            .map_err(|e| format!("heap parse JSON: {e}"))?;
        // Drop the raw string early so peak RSS does not hold JSON text + Value.
        drop(raw);

        let snapshot = v.get("snapshot").cloned().unwrap_or(Value::Null);
        let meta_obj = snapshot.get("meta").cloned().unwrap_or(Value::Null);

        let node_fields = string_list(&meta_obj, "node_fields");
        let edge_fields = string_list(&meta_obj, "edge_fields");
        let node_types = nested_string_list(&meta_obj, "node_types");
        let edge_types = nested_string_list(&meta_obj, "edge_types");

        let nodes_flat = i64_list(&v, "nodes");
        let edges_flat = i64_list(&v, "edges");
        let strings = string_array(&v, "strings");

        let node_stride = node_fields.len().max(1);
        let edge_stride = edge_fields.len().max(1);

        let type_idx = field_index(&node_fields, "type").unwrap_or(0);
        let name_idx = field_index(&node_fields, "name");
        let id_idx = field_index(&node_fields, "id");
        let self_idx = field_index(&node_fields, "self_size");
        let edge_count_idx = field_index(&node_fields, "edge_count");
        let detached_idx = field_index(&node_fields, "detachedness");

        let edge_type_idx = field_index(&edge_fields, "type").unwrap_or(0);
        let edge_name_idx = field_index(&edge_fields, "name_or_index");
        let to_node_idx =
            field_index(&edge_fields, "to_node").unwrap_or(edge_fields.len().saturating_sub(1));

        // Pre-size when the node count is known; fail closed on OOM (untrusted snapshot).
        let approx_nodes = nodes_flat.len() / node_stride.max(1);
        // PAR-93: materialize NodeRec in parallel when large; merge class maps sequentially
        // (HashMap shared mutation is not Rayon-safe). idom/RPO remain sequential (N-142).
        let n_full = approx_nodes;
        let materialize = |index: usize| -> Option<NodeRec> {
            let base = index * node_stride;
            if base + node_stride > nodes_flat.len() {
                return None;
            }
            let chunk = &nodes_flat[base..base + node_stride];
            let type_id = chunk[type_idx].max(0) as usize;
            let type_name = node_types
                .get(type_id)
                .cloned()
                .unwrap_or_else(|| format!("type_{type_id}"));
            let name = name_idx
                .and_then(|ni| {
                    let sid = chunk[ni].max(0) as usize;
                    strings.get(sid).cloned().filter(|s| !s.is_empty())
                })
                .unwrap_or_else(|| type_name.clone());
            let id = id_idx
                .map(|i| chunk[i].max(0) as u64)
                .unwrap_or(index as u64);
            let self_size = self_idx.map(|i| chunk[i].max(0) as u64).unwrap_or(0);
            let edge_count = edge_count_idx
                .map(|i| chunk[i].max(0) as usize)
                .unwrap_or(0);
            let detachedness = detached_idx.map(|i| chunk[i].max(0) as u64);
            Some(NodeRec {
                index,
                type_name,
                name,
                id,
                self_size,
                edge_count,
                detachedness,
            })
        };
        let nodes: Vec<NodeRec> = if n_full < crate::concurrency::CPU_MAP_THRESHOLD {
            (0..n_full).filter_map(materialize).collect()
        } else {
            crate::concurrency::install_rayon_pool_once();
            use rayon::prelude::*;
            (0..n_full)
                .into_par_iter()
                .filter_map(materialize)
                .collect()
        };
        let mut class_counts: HashMap<String, u64> = HashMap::with_capacity(64);
        let mut class_self_sizes: HashMap<String, u64> = HashMap::with_capacity(64);
        let mut class_to_nodes: HashMap<String, Vec<usize>> = HashMap::with_capacity(64);
        let mut id_to_index: HashMap<u64, usize> = HashMap::new();
        id_to_index
            .try_reserve(nodes.len())
            .map_err(|e| format!("heap id map reserve failed: {e}"))?;
        for node in &nodes {
            *class_counts.entry(node.name.clone()).or_insert(0) += 1;
            *class_self_sizes.entry(node.name.clone()).or_insert(0) += node.self_size;
            class_to_nodes
                .entry(node.name.clone())
                .or_default()
                .push(node.index);
            id_to_index.insert(node.id, node.index);
        }

        let n = nodes.len();
        let mut out_edges: Vec<Vec<EdgeRec>> = vec![Vec::new(); n];
        let mut in_edges: Vec<Vec<EdgeRec>> = vec![Vec::new(); n];

        let mut edge_cursor = 0usize;
        for (from, node) in nodes.iter().enumerate() {
            for _ in 0..node.edge_count {
                let base = edge_cursor * edge_stride;
                if base + edge_stride > edges_flat.len() {
                    break;
                }
                let etype_id = edges_flat[base + edge_type_idx].max(0) as usize;
                let type_name = edge_types
                    .get(etype_id)
                    .cloned()
                    .unwrap_or_else(|| format!("edge_type_{etype_id}"));
                let ename = edge_name_idx
                    .map(|ni| {
                        let raw = edges_flat[base + ni];
                        // element/property edges store string index; others may store numeric index
                        if raw >= 0 {
                            let sid = raw as usize;
                            strings
                                .get(sid)
                                .cloned()
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| raw.to_string())
                        } else {
                            raw.to_string()
                        }
                    })
                    .unwrap_or_default();
                let to_flat = edges_flat[base + to_node_idx].max(0) as usize;
                let to = to_flat / node_stride;
                if to < n {
                    let e = EdgeRec {
                        from,
                        to,
                        type_name,
                        name: ename,
                    };
                    out_edges[from].push(e.clone());
                    in_edges[to].push(e);
                }
                edge_cursor += 1;
            }
        }

        Ok(Self {
            path: path.to_string_lossy().into_owned(),
            bytes: meta.len(),
            nodes,
            out_edges,
            in_edges,
            id_to_index,
            class_counts,
            class_self_sizes,
            class_to_nodes,
            node_fields,
            edge_fields,
            node_types,
            edge_types,
            string_count: strings.len() as u64,
            strings,
        })
    }

    fn resolve_node(&self, node_id_or_index: u64) -> Result<usize, String> {
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

    fn node_json(&self, idx: usize) -> Value {
        let n = &self.nodes[idx];
        json!({
            "index": n.index,
            "id": n.id,
            "name": n.name,
            "type": n.type_name,
            "self_size": n.self_size,
            "edge_count": n.edge_count,
            "retainer_count": self.in_edges[idx].len(),
        })
    }

    fn pick_root(&self) -> usize {
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
    fn distances_from_root(&self) -> Vec<Option<u64>> {
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
    fn retained_sizes(&self) -> Vec<u64> {
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

    fn detachedness_label(raw: Option<u64>) -> String {
        match raw {
            None => "unknown".into(),
            Some(0) => "attached".into(),
            Some(1) => "detached".into(),
            Some(2) => "unknown".into(),
            Some(v) => format!("code_{v}"),
        }
    }

    /// Full object details for one node (official object_details tool surface).
    fn object_info_json(&self, idx: usize) -> Value {
        let n = &self.nodes[idx];
        let distances = self.distances_from_root();
        let retained = self.retained_sizes();
        let distance = distances[idx];
        json!({
            "index": n.index,
            "id": n.id,
            "name": n.name,
            "type": n.type_name,
            "self_size": n.self_size,
            "retained_size": retained[idx],
            "distance": distance,
            "edge_count": n.edge_count,
            "retainer_count": self.in_edges[idx].len(),
            "detachedness": Self::detachedness_label(n.detachedness),
        })
    }

    fn edge_json(&self, e: &EdgeRec) -> Value {
        let from = &self.nodes[e.from];
        let to = &self.nodes[e.to];
        json!({
            "type": e.type_name,
            "name": e.name,
            "from_id": from.id,
            "from_name": from.name,
            "to_id": to.id,
            "to_name": to.name,
        })
    }

    /// Immediate dominator tree via iterative data-flow (Cooper/Harvey/Kennedy style).
    fn compute_idom(&self) -> Vec<Option<usize>> {
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
                node.type_name == "synthetic"
                    || node.name.contains("GC roots")
                    || node.name == "(GC roots)"
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

    fn dominator_chain(&self, idx: usize) -> Vec<Value> {
        let idom = self.compute_idom();
        let mut chain = Vec::new();
        let mut seen = HashSet::new();
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

    fn retaining_paths(
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
        const MAX_STATES: usize = 50_000;

        while let Some((u, path)) = q.pop_front() {
            visited_states += 1;
            if visited_states > MAX_STATES {
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

fn field_index(fields: &[String], name: &str) -> Option<usize> {
    fields.iter().position(|f| f == name)
}

fn string_list(meta: &Value, key: &str) -> Vec<String> {
    meta.get(key)
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn nested_string_list(meta: &Value, key: &str) -> Vec<String> {
    meta.get(key)
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_else(|| string_list(meta, key))
}

fn i64_list(root: &Value, key: &str) -> Vec<i64> {
    root.get(key)
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_i64()).collect())
        .unwrap_or_default()
}

fn string_array(root: &Value, key: &str) -> Vec<String> {
    root.get(key)
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .map(|x| x.as_str().unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default()
}

pub fn summarize(path: &Path) -> Result<Value, String> {
    let s = SnapshotGraph::load(path)?;
    let mut top: Vec<(String, u64)> = s.class_counts.into_iter().collect();
    // PAR-107: large class lists sort on Rayon budget.
    crate::concurrency::sort_by_key_cpu(&mut top, |b| std::cmp::Reverse(b.1));
    top.truncate(20);
    Ok(json!({
        "path": s.path,
        "bytes": s.bytes,
        "exists": true,
        "node_count": s.nodes.len() as u64,
        "edge_count": s.out_edges.iter().map(|e| e.len() as u64).sum::<u64>(),
        "string_count": s.string_count,
        "top_classes": top.into_iter().map(|(name, count)| json!({
            "name": name,
            "count": count,
        })).collect::<Vec<_>>(),
        "offline": true,
    }))
}

pub fn details(path: &Path) -> Result<Value, String> {
    let s = SnapshotGraph::load(path)?;
    let mut classes: Vec<Value> = s
        .class_counts
        .iter()
        .map(|(name, count)| {
            json!({
                "name": name,
                "count": count,
                "self_size": s.class_self_sizes.get(name).copied().unwrap_or(0),
            })
        })
        .collect();
    crate::concurrency::sort_by_cpu(&mut classes, |a, b| {
        b.get("count")
            .and_then(|v| v.as_u64())
            .cmp(&a.get("count").and_then(|v| v.as_u64()))
    });
    Ok(json!({
        "path": s.path,
        "bytes": s.bytes,
        "node_count": s.nodes.len() as u64,
        "edge_count": s.out_edges.iter().map(|e| e.len() as u64).sum::<u64>(),
        "string_count": s.string_count,
        "node_fields": s.node_fields,
        "edge_fields": s.edge_fields,
        "node_types": s.node_types,
        "edge_types": s.edge_types,
        "classes": classes,
        "offline": true,
    }))
}

pub fn compare(base: &Path, current: &Path) -> Result<Value, String> {
    let b = SnapshotGraph::load(base)?;
    let c = SnapshotGraph::load(current)?;
    let b_edges: u64 = b.out_edges.iter().map(|e| e.len() as u64).sum();
    let c_edges: u64 = c.out_edges.iter().map(|e| e.len() as u64).sum();
    Ok(json!({
        "base": {
            "path": b.path,
            "bytes": b.bytes,
            "node_count": b.nodes.len() as u64,
            "edge_count": b_edges,
            "string_count": b.string_count,
        },
        "current": {
            "path": c.path,
            "bytes": c.bytes,
            "node_count": c.nodes.len() as u64,
            "edge_count": c_edges,
            "string_count": c.string_count,
        },
        "delta_bytes": (c.bytes as i64) - (b.bytes as i64),
        "delta_nodes": (c.nodes.len() as i64) - (b.nodes.len() as i64),
        "delta_edges": (c_edges as i64) - (b_edges as i64),
        "delta_strings": (c.string_count as i64) - (b.string_count as i64),
        "offline": true,
    }))
}

pub fn duplicate_strings(path: &Path) -> Result<Value, String> {
    let s = SnapshotGraph::load(path)?;
    let mut freq: HashMap<&str, u64> = HashMap::new();
    for st in &s.strings {
        if st.is_empty() {
            continue;
        }
        *freq.entry(st.as_str()).or_insert(0) += 1;
    }
    // PAR-65: independent string→json map after sequential freq count.
    let pairs: Vec<(&str, u64)> = freq.into_iter().filter(|(_, c)| *c > 1).collect();
    let mut dups: Vec<Value> = crate::concurrency::map_cpu(&pairs, |(s, c)| {
        json!({
            "string": if s.len() > 120 { format!("{}…", &s[..120]) } else { s.to_string() },
            "count": c,
            "bytes_est": (s.len() as u64) * c,
        })
    });
    crate::concurrency::sort_by_cpu(&mut dups, |a, b| {
        b.get("count")
            .and_then(|v| v.as_u64())
            .cmp(&a.get("count").and_then(|v| v.as_u64()))
    });
    let total = dups.len();
    dups.truncate(50);
    Ok(json!({
        "path": s.path,
        "duplicate_groups": total,
        "top_duplicates": dups,
        "offline": true,
    }))
}

/// `id` is 1-based rank into top classes by instance count.
pub fn class_nodes(path: &Path, id: u64) -> Result<Value, String> {
    let s = SnapshotGraph::load(path)?;
    let mut top: Vec<(String, u64)> = s
        .class_counts
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    crate::concurrency::sort_by_key_cpu(&mut top, |b| std::cmp::Reverse(b.1));
    let idx = id.saturating_sub(1) as usize;
    let (name, count) = top.get(idx).cloned().ok_or_else(|| {
        format!(
            "class id {id} out of range (have {} classes; use 1-based rank)",
            top.len()
        )
    })?;
    let indices = s.class_to_nodes.get(&name).cloned().unwrap_or_default();
    let truncated = indices.len() > DEFAULT_MAX_CLASS_NODES;
    let node_ids: Vec<Value> = indices
        .iter()
        .take(DEFAULT_MAX_CLASS_NODES)
        .map(|&i| s.node_json(i))
        .collect();
    Ok(json!({
        "path": s.path,
        "class_id": id,
        "name": name,
        "count": count,
        "self_size": s.class_self_sizes.get(&name).copied().unwrap_or(0),
        "nodes": node_ids,
        "truncated": truncated,
        "offline": true,
    }))
}

/// Detailed information about one heap object by node id (offline).
///
/// Returns id, name, type, self_size, retained_size, distance, edge_count,
/// retainer_count, and detachedness — matching the official object-details surface.
pub fn object_details(path: &Path, node: u64) -> Result<Value, String> {
    let s = SnapshotGraph::load(path)?;
    let idx = s.resolve_node(node)?;
    let object = s.object_info_json(idx);
    Ok(json!({
        "path": s.path,
        "op": "object-details",
        "object": object,
        "offline": true,
    }))
}

pub fn node_op(path: &Path, node: u64, op: &str) -> Result<Value, String> {
    if op == "object-details" || op == "object_details" {
        return object_details(path, node);
    }
    node_op_with_limits(
        path,
        node,
        op,
        DEFAULT_MAX_PATH_DEPTH,
        DEFAULT_MAX_PATHS,
        DEFAULT_MAX_RETAINERS,
        DEFAULT_MAX_EDGES,
    )
}

pub fn node_op_with_limits(
    path: &Path,
    node: u64,
    op: &str,
    max_depth: usize,
    max_paths: usize,
    max_retainers: usize,
    max_edges: usize,
) -> Result<Value, String> {
    let s = SnapshotGraph::load(path)?;
    let idx = s.resolve_node(node)?;
    let node_info = s.node_json(idx);

    match op {
        "edges" => {
            let edges = &s.out_edges[idx];
            let truncated = edges.len() > max_edges;
            let list: Vec<Value> = edges
                .iter()
                .take(max_edges)
                .map(|e| s.edge_json(e))
                .collect();
            Ok(json!({
                "path": s.path,
                "op": "edges",
                "node": node_info,
                "edges": list,
                "edge_count": edges.len(),
                "truncated": truncated,
                "offline": true,
            }))
        }
        "retainers" => {
            let edges = &s.in_edges[idx];
            let truncated = edges.len() > max_retainers;
            let list: Vec<Value> = edges
                .iter()
                .take(max_retainers)
                .map(|e| s.edge_json(e))
                .collect();
            Ok(json!({
                "path": s.path,
                "op": "retainers",
                "node": node_info,
                "retainers": list,
                "retainer_count": edges.len(),
                "truncated": truncated,
                "offline": true,
            }))
        }
        "dominators" => {
            let chain = s.dominator_chain(idx);
            Ok(json!({
                "path": s.path,
                "op": "dominators",
                "node": node_info,
                "dominator_chain": chain,
                "chain_length": chain.len(),
                "offline": true,
            }))
        }
        "paths" => {
            let (paths, limits) = s.retaining_paths(idx, max_depth.max(1), max_paths.max(1));
            Ok(json!({
                "path": s.path,
                "op": "paths",
                "node": node_info,
                "paths": paths,
                "path_count": paths.len(),
                "max_depth": max_depth,
                "limits_reached": limits,
                "offline": true,
            }))
        }
        other => Ok(json!({
            "path": s.path,
            "op": other,
            "node": node_info,
            "offline": true,
        })),
    }
}

/// Close offline analysis handle (summary + explicit closed flag).
pub fn close_snapshot(path: &Path) -> Result<Value, String> {
    let mut summary = summarize(path)?;
    if let Some(obj) = summary.as_object_mut() {
        obj.insert("closed".into(), json!(true));
        obj.insert(
            "note".into(),
            json!("offline analysis complete; no in-process cache retained (one-shot)"),
        );
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Tiny graph:
    /// root(0) -prop-> A(1) -prop-> B(2)
    /// root also retains C(3)
    /// B retained only via A.
    fn write_fixture(path: &Path) {
        // node_fields: type, name, id, self_size, edge_count
        // nodes: root, A, B, C
        // edges for root: 2 (to A, to C); A: 1 (to B); B: 0; C: 0
        // to_node is flat index = node_index * 5
        let body = r#"{
            "snapshot": {
                "meta": {
                    "node_fields": ["type","name","id","self_size","edge_count"],
                    "node_types": [["hidden","object","string","synthetic"]],
                    "edge_fields": ["type","name_or_index","to_node"],
                    "edge_types": [["context","element","property","internal","hidden","shortcut","weak"]]
                },
                "node_count": 4,
                "edge_count": 3
            },
            "nodes": [
                3, 0, 10, 0, 2,
                1, 1, 11, 100, 1,
                1, 2, 12, 50, 0,
                1, 3, 13, 25, 0
            ],
            "edges": [
                2, 4, 5,
                2, 5, 15,
                2, 6, 10
            ],
            "strings": ["(GC roots)", "A", "B", "C", "toA", "toC", "toB"]
        }"#;
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
    }

    #[test]
    fn summarize_minimal_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.heapsnapshot");
        write_fixture(&path);
        let s = summarize(&path).unwrap();
        assert_eq!(s["node_count"], 4);
        assert_eq!(s["offline"], true);
    }

    #[test]
    fn edges_and_retainers_real_graph() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("g.heapsnapshot");
        write_fixture(&path);

        // node id 12 = B
        let edges_b = node_op(&path, 12, "edges").unwrap();
        assert_eq!(edges_b["edge_count"], 0);

        let retainers_b = node_op(&path, 12, "retainers").unwrap();
        assert_eq!(retainers_b["retainer_count"], 1);
        let r0 = &retainers_b["retainers"][0];
        assert_eq!(r0["from_id"], 11); // A

        let edges_a = node_op(&path, 11, "edges").unwrap();
        assert_eq!(edges_a["edge_count"], 1);
        assert_eq!(edges_a["edges"][0]["to_id"], 12);
    }

    #[test]
    fn dominators_chain_includes_root_and_node() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("d.heapsnapshot");
        write_fixture(&path);
        let d = node_op(&path, 12, "dominators").unwrap();
        let chain = d["dominator_chain"].as_array().unwrap();
        assert!(chain.len() >= 2);
        let last = chain.last().unwrap();
        assert_eq!(last["id"], 12);
        let first = &chain[0];
        assert_eq!(first["id"], 10);
    }

    #[test]
    fn retaining_paths_finds_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.heapsnapshot");
        write_fixture(&path);
        let p = node_op(&path, 12, "paths").unwrap();
        let paths = p["paths"].as_array().unwrap();
        assert!(!paths.is_empty());
        assert!(paths[0]["nodes"].as_array().unwrap().len() >= 2);
    }

    #[test]
    fn class_nodes_lists_ids() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.heapsnapshot");
        write_fixture(&path);
        // rank classes; A/B/C each count 1 — any rank 1+ works if class exists
        let cn = class_nodes(&path, 1).unwrap();
        assert!(!cn["nodes"].as_array().unwrap().is_empty());
        assert_eq!(cn["offline"], true);
    }

    #[test]
    fn close_snapshot_flags_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.heapsnapshot");
        write_fixture(&path);
        let c = close_snapshot(&path).unwrap();
        assert_eq!(c["closed"], true);
    }

    #[test]
    fn dup_strings_counts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dups.heapsnapshot");
        let body = r#"{
            "snapshot": { "meta": {
                "node_fields": ["type","name","id","self_size","edge_count"],
                "node_types": [["object"]],
                "edge_fields": ["type","name_or_index","to_node"],
                "edge_types": [["property"]]
            }, "node_count": 0, "edge_count": 0 },
            "nodes": [],
            "edges": [],
            "strings": ["a", "b", "a", "a", "c", "b"]
        }"#;
        std::fs::write(&path, body).unwrap();
        let d = duplicate_strings(&path).unwrap();
        assert_eq!(d["duplicate_groups"], 2);
    }

    #[test]
    fn object_details_includes_distance_and_retained() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("obj.heapsnapshot");
        write_fixture(&path);
        // B id=12: self 50, retained should include only self if nothing dominated
        let o = object_details(&path, 12).unwrap();
        assert_eq!(o["op"], "object-details");
        assert_eq!(o["offline"], true);
        let obj = &o["object"];
        assert_eq!(obj["id"], 12);
        assert_eq!(obj["name"], "B");
        assert_eq!(obj["self_size"], 50);
        assert!(obj["retained_size"].as_u64().unwrap() >= 50);
        assert_eq!(obj["distance"], 2); // root -> A -> B
        assert_eq!(obj["retainer_count"], 1);
        assert_eq!(obj["detachedness"], "unknown");

        // A id=11 retains B (50) + self 100
        let a = object_details(&path, 11).unwrap();
        let ao = &a["object"];
        assert_eq!(ao["distance"], 1);
        assert!(ao["retained_size"].as_u64().unwrap() >= 150);
    }
}
