// SPDX-License-Identifier: MIT OR Apache-2.0

//! Load a V8 heapsnapshot into [`SnapshotGraph`].

use std::path::Path;

use rustc_hash::FxHashMap;
use serde_json::Value;

use super::super::parse::{field_index, i64_list, nested_string_list, string_array, string_list};
use super::types::{EdgeRec, NodeRec, SnapshotGraph};

impl SnapshotGraph {
    pub(crate) fn load(path: &Path) -> Result<Self, String> {
        // GAP-026, read axis. Every `heap` verb that names a file lands here:
        // `ops.rs` calls this from eight sites, and `heap compare` calls it
        // twice in one command for `--base` and `--current`. All eight paths
        // are operator argv and none is an artifact this product generated, so
        // the funnel is the right place and the twelve arms of the command
        // module are not — a check per arm covers only the arms someone
        // remembered.
        //
        // Placed before `metadata` so a refused path is never even stat'd,
        // matching `scrape_local::urls::read_urls_file`.
        //
        // The error is flattened to `String` because that is this function's
        // channel; the caller re-wraps it. The refusal keeps its own wording,
        // so the operator still reads why the path was rejected.
        crate::fs_roots::ensure_read_allowed(path).map_err(|e| e.message().to_string())?;
        let meta = std::fs::metadata(path).map_err(|e| format!("heap file: {e}"))?;
        let max_bytes = super::super::limits::max_heap_snapshot_bytes();
        if meta.len() > max_bytes {
            return Err(format!(
                "heap snapshot too large: {} bytes > {max_bytes} budget (use a smaller capture)",
                meta.len(),
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
        let v: Value =
            crate::json_util::from_str(&raw).map_err(|e| format!("heap parse JSON: {e}"))?;
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
        // Intern the string table once. Node and edge names index into it, so
        // sharing each buffer replaces one allocation per node (and per named
        // edge) with a refcount bump.
        let strings: Vec<std::sync::Arc<str>> = string_array(&v, "strings")
            .into_iter()
            .map(std::sync::Arc::from)
            .collect();

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
        // Interned per distinct node type, not per node: a snapshot has a
        // dozen types and hundreds of thousands of nodes. `materialize` runs on
        // Rayon workers, so the table is built up front rather than behind a
        // shared interner.
        let node_types_interned: Vec<std::sync::Arc<str>> = node_types
            .iter()
            .map(|t| std::sync::Arc::from(t.as_str()))
            .collect();
        let materialize = |index: usize| -> Option<NodeRec> {
            let base = index * node_stride;
            if base + node_stride > nodes_flat.len() {
                return None;
            }
            let chunk = &nodes_flat[base..base + node_stride];
            let type_id = chunk[type_idx].max(0) as usize;
            let type_name = node_types_interned.get(type_id).map_or_else(
                || std::sync::Arc::from(format!("type_{type_id}")),
                std::sync::Arc::clone,
            );
            let name = name_idx
                .and_then(|ni| {
                    let sid = chunk[ni].max(0) as usize;
                    strings
                        .get(sid)
                        .filter(|s| !s.is_empty())
                        .map(std::sync::Arc::clone)
                })
                .unwrap_or_else(|| std::sync::Arc::clone(&type_name));
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
        // FxHashMap: in-process heap graph keys (u64 ids / class names from CDP).
        // with_capacity_and_hasher avoids SipHash + rehash churn on multi-MB snaps.
        let mut class_counts: FxHashMap<std::sync::Arc<str>, u64> =
            FxHashMap::with_capacity_and_hasher(64, Default::default());
        let mut class_self_sizes: FxHashMap<std::sync::Arc<str>, u64> =
            FxHashMap::with_capacity_and_hasher(64, Default::default());
        let mut class_to_nodes: FxHashMap<std::sync::Arc<str>, Vec<usize>> =
            FxHashMap::with_capacity_and_hasher(64, Default::default());
        let mut id_to_index: FxHashMap<u64, usize> =
            FxHashMap::with_capacity_and_hasher(nodes.len(), Default::default());
        // Clone the class name once per DISTINCT class, not once per node.
        //
        // `entry()` takes an owned key, so the previous `entry(node.name.clone())`
        // ×3 allocated three Strings for every node whether the class was new or
        // not. A 500k-node snapshot has on the order of a hundred distinct class
        // names, so that was ~1.5 million allocations to build a map with ~100
        // keys. Looking up by `&str` first keeps the insert on the cold path,
        // and with `Arc<str>` keys even that insert is a refcount bump.
        for node in &nodes {
            if let Some(count) = class_counts.get_mut(&*node.name) {
                *count += 1;
                if let Some(size) = class_self_sizes.get_mut(&*node.name) {
                    *size += node.self_size;
                }
                if let Some(list) = class_to_nodes.get_mut(&*node.name) {
                    list.push(node.index);
                }
            } else {
                class_counts.insert(std::sync::Arc::clone(&node.name), 1);
                class_self_sizes.insert(std::sync::Arc::clone(&node.name), node.self_size);
                class_to_nodes.insert(std::sync::Arc::clone(&node.name), vec![node.index]);
            }
            id_to_index.insert(node.id, node.index);
        }

        let n = nodes.len();
        let mut out_edges: Vec<Vec<EdgeRec>> = vec![Vec::new(); n];
        let mut in_edges: Vec<Vec<EdgeRec>> = vec![Vec::new(); n];

        let mut edge_cursor = 0usize;
        let mut edge_type_interner: FxHashMap<usize, std::sync::Arc<str>> = FxHashMap::default();
        for (from, node) in nodes.iter().enumerate() {
            for _ in 0..node.edge_count {
                let base = edge_cursor * edge_stride;
                if base + edge_stride > edges_flat.len() {
                    break;
                }
                let etype_id = edges_flat[base + edge_type_idx].max(0) as usize;
                // Interned per distinct edge type, not per edge: a snapshot has
                // a dozen edge types and millions of edges, so this turns one
                // allocation per edge into one per type for the whole file.
                let type_name = match edge_type_interner.get(&etype_id) {
                    Some(shared) => std::sync::Arc::clone(shared),
                    None => {
                        let owned: std::sync::Arc<str> = edge_types
                            .get(etype_id)
                            .map_or_else(|| format!("edge_type_{etype_id}"), Clone::clone)
                            .into();
                        edge_type_interner.insert(etype_id, std::sync::Arc::clone(&owned));
                        owned
                    }
                };
                let ename: std::sync::Arc<str> = edge_name_idx
                    .map(|ni| {
                        let raw = edges_flat[base + ni];
                        // element/property edges store string index; others may store numeric index
                        let interned = if raw >= 0 {
                            strings
                                .get(raw as usize)
                                .filter(|s| !s.is_empty())
                                .map(std::sync::Arc::clone)
                        } else {
                            None
                        };
                        interned.unwrap_or_else(|| std::sync::Arc::from(raw.to_string()))
                    })
                    .unwrap_or_else(|| std::sync::Arc::from(""));
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
}
