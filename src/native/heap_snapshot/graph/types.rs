// SPDX-License-Identifier: MIT OR Apache-2.0

//! Graph node/edge records and [`SnapshotGraph`] storage.

use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub(crate) struct NodeRec {
    pub(crate) index: usize,
    pub(crate) type_name: String,
    pub(crate) name: String,
    pub(crate) id: u64,
    pub(crate) self_size: u64,
    pub(crate) edge_count: usize,
    /// V8 detachedness enum when present in `node_fields`; else `None`.
    pub(crate) detachedness: Option<u64>,
}

#[derive(Debug, Clone)]
pub(crate) struct EdgeRec {
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) type_name: String,
    pub(crate) name: String,
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
    pub(crate) class_counts: FxHashMap<String, u64>,
    pub(crate) class_self_sizes: FxHashMap<String, u64>,
    /// class name → node indices
    pub(crate) class_to_nodes: FxHashMap<String, Vec<usize>>,
    pub(crate) node_fields: Vec<String>,
    pub(crate) edge_fields: Vec<String>,
    pub(crate) node_types: Vec<String>,
    pub(crate) edge_types: Vec<String>,
    pub(crate) string_count: u64,
    pub(crate) strings: Vec<String>,
}
