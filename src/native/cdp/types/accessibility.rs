// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP Accessibility domain types.
use super::core::{opt_vec_string_or_int, string_or_int};
use serde::Deserialize;
use serde_json::Value;

/// Mirrors CDP `Accessibility.getFullAXTree` response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFullAXTreeResult {
    /// Flat list of every node in the tree; parent/child links live in
    /// [`AXNode::child_ids`], not in the list order.
    pub nodes: Vec<AXNode>,
}

/// Mirrors CDP `Accessibility.AXNode`.
///
/// A node in the accessibility tree.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXNode {
    /// Unique identifier for this node.
    #[serde(deserialize_with = "string_or_int")]
    pub node_id: String,
    /// This `Node`'s role, whether explicit or implicit.
    pub role: Option<AXValue>,
    /// The accessible name for this `Node`.
    pub name: Option<AXValue>,
    /// The value for this `Node`.
    pub value: Option<AXValue>,
    /// The accessible description for this `Node`.
    pub description: Option<AXValue>,
    /// Computed ARIA properties such as `focusable`, `checked` or `disabled`.
    pub properties: Option<Vec<AXProperty>>,
    /// IDs for each of this node's child nodes.
    #[serde(default, deserialize_with = "opt_vec_string_or_int")]
    pub child_ids: Option<Vec<String>>,
    /// The backend ID for the associated DOM node, if any.
    pub backend_d_o_m_node_id: Option<i64>,
    /// Whether the node is excluded from the accessibility tree.
    ///
    /// Ignored nodes are still returned, so the snapshot must filter them out
    /// instead of assuming the tree only contains exposed nodes.
    pub ignored: Option<bool>,
}

/// Mirrors CDP `Accessibility.AXValue`.
///
/// A single computed AX property.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXValue {
    /// Value kind, for example `string`, `boolean`, `idref` or `computedString`.
    ///
    /// Renamed because `type` is a Rust keyword; the wire name stays `type`.
    #[serde(rename = "type")]
    pub value_type: String,
    /// The value itself, left opaque because its shape follows `value_type`.
    pub value: Option<Value>,
}

/// Mirrors CDP `Accessibility.AXProperty`.
///
/// The protocol ships this type without prose; a property is one computed ARIA
/// attribute of an [`AXNode`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXProperty {
    /// The name of this property.
    pub name: String,
    /// The computed value carried by this property.
    pub value: AXValue,
}

// ---------------------------------------------------------------------------
// Network domain (minimal for Phase 1)
// ---------------------------------------------------------------------------
