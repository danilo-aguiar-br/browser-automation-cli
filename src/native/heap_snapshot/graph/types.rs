// SPDX-License-Identifier: MIT OR Apache-2.0

//! Graph node/edge records and [`SnapshotGraph`] storage.

use rustc_hash::FxHashMap;

/// One graph node.
///
/// `type_name` and `name` are `Arc<str>` rather than `String` because a real
/// snapshot has hundreds of thousands of nodes drawn from a few dozen types
/// and a string table where class names repeat massively — thousands of nodes
/// share the constructor name `Object`, `Array` or `system / Context`. As
/// `String` every node allocated and memcpy'd its own copy of both fields, and
/// the class maps below cloned the name a third time. Sharing the interned
/// buffer turns each of those copies into a refcount bump.
#[derive(Debug, Clone)]
pub(crate) struct NodeRec {
    pub(crate) index: usize,
    pub(crate) type_name: std::sync::Arc<str>,
    pub(crate) name: std::sync::Arc<str>,
    pub(crate) id: u64,
    pub(crate) self_size: u64,
    pub(crate) edge_count: usize,
    /// V8 detachedness enum when present in `node_fields`; else `None`.
    pub(crate) detachedness: Option<u64>,
}

/// One graph edge.
///
/// `type_name` and `name` are `Arc<str>` rather than `String` because every
/// edge is stored TWICE — once in `out_edges[from]` and once in
/// `in_edges[to]` — and a real snapshot has two to five times more edges than
/// nodes. As `String` that second copy allocated and memcpy'd both fields, so
/// a 500k-node snapshot paid roughly two million allocations for data it
/// already had. The clone is now a refcount bump; the strings themselves are
/// short and highly repeated (`property`, `internal`, `element`).
#[derive(Debug, Clone)]
pub(crate) struct EdgeRec {
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) type_name: std::sync::Arc<str>,
    pub(crate) name: std::sync::Arc<str>,
}

#[derive(Debug)]
pub(crate) struct SnapshotGraph {
    pub(crate) path: String,
    pub(crate) bytes: u64,
    pub(crate) nodes: Vec<NodeRec>,
    /// Outgoing edges by node index.
    pub(crate) out_edges: Vec<Vec<EdgeRec>>,
    /// Incoming edges by node index (retainers).
    pub(crate) in_edges: Vec<Vec<EdgeRec>>,
    /// node id field → node index (trusted snapshot ids → FxHash, not SipHash)
    pub(crate) id_to_index: FxHashMap<u64, usize>,
    pub(crate) class_counts: FxHashMap<std::sync::Arc<str>, u64>,
    pub(crate) class_self_sizes: FxHashMap<std::sync::Arc<str>, u64>,
    /// class name → node indices
    pub(crate) class_to_nodes: FxHashMap<std::sync::Arc<str>, Vec<usize>>,
    pub(crate) node_fields: Vec<String>,
    pub(crate) edge_fields: Vec<String>,
    pub(crate) node_types: Vec<String>,
    pub(crate) edge_types: Vec<String>,
    pub(crate) string_count: u64,
    /// Snapshot string table, interned once at load so that every node and
    /// edge naming an entry shares its buffer instead of copying it.
    pub(crate) strings: Vec<std::sync::Arc<str>>,
}
