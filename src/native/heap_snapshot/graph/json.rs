// SPDX-License-Identifier: MIT OR Apache-2.0

//! JSON views of nodes and edges for agent envelopes.
//!
//! Names and type names are `Arc<str>` and are written as `&*field` on
//! purpose: `Arc<str>` only serialises under serde's `rc` feature, and
//! enabling that crate-wide to print a handful of fields would be a large
//! default for a small need.

use serde_json::{json, Value};

use super::types::{EdgeRec, SnapshotGraph};

impl SnapshotGraph {
    pub(crate) fn node_json(&self, idx: usize) -> Value {
        let n = &self.nodes[idx];
        json!({
            "index": n.index,
            "id": n.id,
            "name": &*n.name,
            "type": &*n.type_name,
            "self_size": n.self_size,
            "edge_count": n.edge_count,
            "retainer_count": self.in_edges[idx].len(),
        })
    }

    pub(crate) fn object_info_json(&self, idx: usize) -> Value {
        let n = &self.nodes[idx];
        let distances = self.distances_from_root();
        let retained = self.retained_sizes();
        let distance = distances[idx];
        json!({
            "index": n.index,
            "id": n.id,
            "name": &*n.name,
            "type": &*n.type_name,
            "self_size": n.self_size,
            "retained_size": retained[idx],
            "distance": distance,
            "edge_count": n.edge_count,
            "retainer_count": self.in_edges[idx].len(),
            "detachedness": Self::detachedness_label(n.detachedness),
        })
    }

    pub(crate) fn edge_json(&self, e: &EdgeRec) -> Value {
        let from = &self.nodes[e.from];
        let to = &self.nodes[e.to];
        json!({
            "type": &*e.type_name,
            "name": &*e.name,
            "from_id": from.id,
            "from_name": &*from.name,
            "to_id": to.id,
            "to_name": &*to.name,
        })
    }
}
