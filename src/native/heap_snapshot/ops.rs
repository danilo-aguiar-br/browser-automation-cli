// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public offline heap analysis ops.

use std::path::Path;

use rustc_hash::FxHashMap;
use serde_json::{json, Value};

use super::graph::SnapshotGraph;

mod node_ops;

pub use node_ops::{node_op, node_op_with_limits, object_details};

/// Aggregate a heap snapshot into per-constructor totals.
///
/// # Errors
///
/// Propagates the snapshot load: `"heap file: …"` when `path` cannot be
/// stat'd, `"heap snapshot too large: N bytes > M budget"` above
/// the `heap_snapshot_max_bytes` budget,
/// `"heap allocate failed …"` when the file does not fit in host RAM,
/// `"heap open: …"` / `"heap read: …"` on I/O failure, and
/// `"heap parse JSON: …"` when the file is not a V8 `.heapsnapshot`.
pub fn summarize(path: &Path) -> Result<Value, String> {
    let s = SnapshotGraph::load(path)?;
    let mut top: Vec<(std::sync::Arc<str>, u64)> = s.class_counts.into_iter().collect();
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
            "name": &*name,
            "count": count,
        })).collect::<Vec<_>>(),
        "offline": true,
    }))
}

/// Per-node detail of a heap snapshot, one level below [`summarize`].
///
/// # Errors
///
/// Propagates the snapshot load, exactly as [`summarize`] does: unreadable
/// `path`, a file above the `heap_snapshot_max_bytes` budget, an allocation
/// that does not fit in host RAM, or JSON that is not a V8 `.heapsnapshot`.
pub fn details(path: &Path) -> Result<Value, String> {
    let s = SnapshotGraph::load(path)?;
    let mut classes: Vec<Value> = s
        .class_counts
        .iter()
        .map(|(name, count)| {
            json!({
                "name": &**name,
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
///
/// # Errors
///
/// Propagates the snapshot load of `base` first, then of `current`: an
/// unreadable path, a file above the `heap_snapshot_max_bytes` budget, an
/// allocation that does not fit in host RAM, or JSON that is not a V8
/// `.heapsnapshot`. Both files are held in memory at once, so a pair that
/// each pass the budget alone can still exhaust RAM.
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

/// Shorten a heap string for display, cutting by CHARACTER.
///
/// # Why not `&s[..n]`
///
/// That is what this used to be, and it was the only byte-indexed `&str` slice
/// in the product. The input is a string from the page's own JavaScript heap,
/// so it carries arbitrary UTF-8: emoji, CJK, accented Latin. When the byte
/// boundary lands inside a multi-byte sequence, `&str` indexing PANICS instead
/// of truncating.
///
/// Two things made that worse than a normal panic. It runs inside
/// [`crate::concurrency::map_cpu`], so it fires on a rayon worker; and the
/// release profile sets `panic = "abort"`, so there is no unwinding to catch —
/// the process takes SIGABRT. A user pointing `heap dup-strings` at a snapshot
/// of any page with an emoji in it got a hard abort and no envelope.
fn preview_string(s: &str) -> String {
    let cap = crate::constants::HEAP_DUP_STRING_PREVIEW_CHARS;
    let preview: String = s.chars().take(cap).collect();
    if s.chars().nth(cap).is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

/// Find identical strings retained more than once.
///
/// # Errors
///
/// Propagates the snapshot load, as [`summarize`] does. The frequency pass
/// itself cannot fail: previews are cut by CHARACTER, so a multi-byte string
/// truncates instead of panicking, and the list is capped rather than refused.
pub fn duplicate_strings(path: &Path) -> Result<Value, String> {
    let s = SnapshotGraph::load(path)?;
    let mut freq: FxHashMap<&str, u64> = FxHashMap::default();
    for st in &s.strings {
        if st.is_empty() {
            continue;
        }
        *freq.entry(&**st).or_insert(0) += 1;
    }
    // PAR-65: independent string→json map after sequential freq count.
    let pairs: Vec<(&str, u64)> = freq.into_iter().filter(|(_, c)| *c > 1).collect();
    let mut dups: Vec<Value> = crate::concurrency::map_cpu(&pairs, |(s, c)| {
        json!({
            "string": preview_string(s),
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
    dups.truncate(crate::constants::HEAP_DUP_LIST_CAP);
    Ok(json!({
        "path": s.path,
        "duplicate_groups": total,
        "top_duplicates": dups,
        "offline": true,
    }))
}

/// `id` is 1-based rank into top classes by instance count.
///
/// # Errors
///
/// Propagates the snapshot load, then fails with
/// `"class id N out of range (have M classes; use 1-based rank)"` when `id` is
/// `0` or above the number of constructors in the snapshot. The node list is
/// capped by `heap_max_class_nodes` and reports `truncated: true` rather than
/// failing.
pub fn class_nodes(path: &Path, id: u64) -> Result<Value, String> {
    let s = SnapshotGraph::load(path)?;
    let mut top: Vec<(std::sync::Arc<str>, u64)> = s
        .class_counts
        .iter()
        .map(|(k, v)| (std::sync::Arc::clone(k), *v))
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
    let class_node_cap = super::limits::default_max_class_nodes();
    let truncated = indices.len() > class_node_cap;
    let node_ids: Vec<Value> = indices
        .iter()
        .take(class_node_cap)
        .map(|&i| s.node_json(i))
        .collect();
    Ok(json!({
        "path": s.path,
        "class_id": id,
        "name": &*name,
        "count": count,
        "self_size": s.class_self_sizes.get(&name).copied().unwrap_or(0),
        "nodes": node_ids,
        "truncated": truncated,
        "offline": true,
    }))
}

/// Close offline analysis handle (summary + explicit closed flag).
///
/// # Errors
///
/// Propagates [`summarize`], which reloads the file to build the closing
/// summary: an unreadable `path`, a file above the `heap_snapshot_max_bytes`
/// budget, or JSON that is not a V8 `.heapsnapshot`. There is no in-process
/// handle to release, so nothing else here can fail.
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

    /// The regression that shipped: a byte cut at 120 lands mid code point.
    ///
    /// The repeat unit is chosen so byte 120 lands INSIDE a code point rather
    /// than on a boundary, which is the only case that panics. `日` is three
    /// bytes and `🎯` is four, so the unit is seven: 17 units end at byte 119
    /// and the `日` that follows occupies 119, 120 and 121. A unit whose length
    /// divides 120 — four `🎯`s, three `日`s, two `é`s — would sit on a boundary
    /// and pass even against the broken code, which is why the fixture asserts
    /// its own premise below.
    #[test]
    fn a_multibyte_string_is_previewed_without_panicking() {
        let s = "日🎯".repeat(80);
        assert!(
            s.len() > 120,
            "fixture must exceed the byte cap to be a test"
        );
        assert!(!s.is_char_boundary(120), "fixture must straddle byte 120");

        let out = preview_string(&s);
        assert!(out.ends_with('…'), "a truncated preview must say so: {out}");
        assert_eq!(
            out.chars().count(),
            crate::constants::HEAP_DUP_STRING_PREVIEW_CHARS + 1,
            "preview is capped in CHARACTERS, plus the ellipsis"
        );
    }

    /// A short string is returned whole, with no ellipsis to imply loss.
    #[test]
    fn a_short_string_is_not_marked_as_truncated() {
        assert_eq!(preview_string("curto"), "curto");
    }

    /// Exactly at the cap is not truncation. Off-by-one here would append an
    /// ellipsis to a complete string, which reads as data loss that never
    /// happened.
    #[test]
    fn a_string_exactly_at_the_cap_keeps_its_last_character() {
        let cap = crate::constants::HEAP_DUP_STRING_PREVIEW_CHARS;
        let s = "é".repeat(cap);
        let out = preview_string(&s);
        assert!(!out.ends_with('…'), "no ellipsis at exactly the cap: {out}");
        assert_eq!(out.chars().count(), cap);
    }
}
