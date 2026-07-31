// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public offline heap analysis ops.

use std::path::Path;

use rustc_hash::FxHashMap;
use serde_json::{json, Value};

use super::graph::SnapshotGraph;

/// Aggregate a heap snapshot into per-constructor totals.
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

/// Per-node detail of a heap snapshot, one level below [`summarize`].
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

/// Diff two snapshots, reporting what grew between them.
///
/// Growth between two points is the signal for a leak; a single snapshot only
/// shows what is allocated, which is not the same question.
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

/// Find identical strings retained more than once.
pub fn duplicate_strings(path: &Path) -> Result<Value, String> {
    let s = SnapshotGraph::load(path)?;
    let mut freq: FxHashMap<&str, u64> = FxHashMap::default();
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
    let truncated = indices.len() > super::limits::DEFAULT_MAX_CLASS_NODES;
    let node_ids: Vec<Value> = indices
        .iter()
        .take(super::limits::DEFAULT_MAX_CLASS_NODES)
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

/// Run a node-scoped query (`retainers`, `paths`, …) with default limits.
pub fn node_op(path: &Path, node: u64, op: &str) -> Result<Value, String> {
    if op == "object-details" || op == "object_details" {
        return object_details(path, node);
    }
    node_op_with_limits(
        path,
        node,
        op,
        super::limits::DEFAULT_MAX_PATH_DEPTH,
        super::limits::DEFAULT_MAX_PATHS,
        super::limits::DEFAULT_MAX_RETAINERS,
        super::limits::DEFAULT_MAX_EDGES,
    )
}

/// [`node_op`] with explicit traversal caps.
///
/// Retainer graphs can be enormous, so an uncapped walk on a real snapshot is
/// how this turns into an out-of-memory instead of an answer.
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
