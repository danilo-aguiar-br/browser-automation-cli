// SPDX-License-Identifier: MIT OR Apache-2.0
//! Snapshot queries that start from ONE node the caller names.
//!
//! The seam is the question, not the line count. [`super`] answers about the
//! snapshot AS A WHOLE — totals per constructor, a diff between two files, the
//! string frequency table — and needs no node id to do it, so its only failure
//! is loading the file. Everything here resolves a node first, so it owns the
//! `"node id/index N not found"` refusal, and it is the only side that has to
//! carry traversal caps: a retainer walk is unbounded by the shape of the
//! graph, while an aggregate is bounded by the snapshot itself.

use std::path::Path;

use serde_json::{json, Value};

use super::super::graph::SnapshotGraph;
use super::super::limits;

/// Detailed information about one heap object by node id (offline).
///
/// Returns id, name, type, self_size, retained_size, distance, edge_count,
/// retainer_count, and detachedness — matching the official object-details surface.
///
/// # Errors
///
/// Propagates the snapshot load, then fails with
/// `"node id/index N not found (node_count=M)"` when `node` matches neither a
/// V8 node id nor a valid node index.
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

/// Run a node-scoped query (`retainers`, `paths`, …) with default limits.
///
/// # Errors
///
/// Propagates [`object_details`] for the `object-details` op, and
/// [`node_op_with_limits`] otherwise: a snapshot that cannot be loaded, or a
/// `node` that resolves to neither an id nor an index. An unrecognized `op` is
/// not an error.
pub fn node_op(path: &Path, node: u64, op: &str) -> Result<Value, String> {
    if op == "object-details" || op == "object_details" {
        return object_details(path, node);
    }
    node_op_with_limits(
        path,
        node,
        op,
        limits::default_max_path_depth(),
        limits::default_max_paths(),
        limits::default_max_retainers(),
        limits::default_max_edges(),
    )
}

/// [`node_op`] with explicit traversal caps.
///
/// Retainer graphs can be enormous, so an uncapped walk on a real snapshot is
/// how this turns into an out-of-memory instead of an answer.
///
/// # Errors
///
/// Propagates the snapshot load, then fails with
/// `"node id/index N not found (node_count=M)"` when `node` matches neither a
/// V8 node id nor a valid node index.
///
/// An `op` outside `edges`, `retainers`, `dominators` and `paths` is **not**
/// an error: it echoes the op name with the node info and no payload, so a
/// typo reads as an empty answer rather than as a rejection. Hitting
/// `max_depth` or `max_paths` is likewise reported in `limits_reached` /
/// `truncated`, never as a failure.
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
