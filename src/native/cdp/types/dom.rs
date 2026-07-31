// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP DOM domain types.
use super::runtime::RemoteObject;
use serde::{Deserialize, Serialize};

/// Mirrors CDP `DOM.resolveNode` request.
///
/// Resolves the JavaScript node object for a given NodeId or BackendNodeId.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomResolveNodeParams {
    /// Backend identifier of the node to resolve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<i64>,
    /// Id of the node to resolve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<i64>,
    /// Symbolic group name that can be used to release multiple objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
}

/// Mirrors CDP `DOM.resolveNode` response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomResolveNodeResult {
    /// JavaScript handle for the node, usable with `Runtime.callFunctionOn`.
    pub object: RemoteObject,
}

/// Mirrors CDP `DOM.getBoxModel` request.
///
/// Returns boxes for the given node.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomGetBoxModelParams {
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<i64>,
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<i64>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
}

/// Mirrors CDP `DOM.getBoxModel` response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomGetBoxModelResult {
    /// Geometry of the node, in CSS pixels relative to the main frame viewport.
    pub model: BoxModel,
}

/// Mirrors CDP `DOM.BoxModel`.
///
/// Box model.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxModel {
    /// Content box
    pub content: Vec<f64>,
    /// Padding box
    pub padding: Vec<f64>,
    /// Border box
    pub border: Vec<f64>,
    /// Margin box
    pub margin: Vec<f64>,
    /// Node width
    pub width: i64,
    /// Node height
    pub height: i64,
}

/// Mirrors CDP `DOM.querySelector` request.
///
/// Executes `querySelector` on a given node.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomQuerySelectorParams {
    /// Id of the node to query upon.
    pub node_id: i64,
    /// CSS selector evaluated against the subtree of `node_id`.
    pub selector: String,
}

/// Mirrors CDP `DOM.querySelector` response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomQuerySelectorResult {
    /// Id of the first matching node, or `0` when nothing matched.
    ///
    /// The protocol reports "no match" as a zero id rather than an error, so a
    /// caller that only checks for an error treats a miss as a hit.
    pub node_id: i64,
}

/// Mirrors CDP `DOM.getDocument` request.
///
/// Returns the root DOM node (and optionally the subtree) to the caller. Implicitly enables the
/// DOM domain events for the current target.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomGetDocumentParams {
    /// The maximum depth at which children should be retrieved, defaults to 1. Use -1 for the
    /// entire subtree or provide an integer larger than 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i32>,
}

/// Mirrors CDP `DOM.getDocument` response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomGetDocumentResult {
    /// Root node of the document, with children up to the requested depth.
    pub root: DomNode,
}

/// Mirrors CDP `DOM.Node`.
///
/// DOM interaction is implemented in terms of mirror objects that represent the actual DOM
/// nodes. DOMNode is a base node mirror type.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomNode {
    /// Node identifier that is passed into the rest of the DOM messages as the `nodeId`.
    /// Backend will only push node with given `id` once. It is aware of all requested nodes and
    /// will only fire DOM events for nodes known to the client.
    pub node_id: i64,
    /// The BackendNodeId for this node.
    pub backend_node_id: Option<i64>,
    /// `Node`'s nodeType.
    pub node_type: Option<i64>,
    /// `Node`'s nodeName.
    pub node_name: Option<String>,
    /// Child nodes, present only as deep as the `depth` asked for.
    ///
    /// Absent does not mean childless: it means the traversal stopped here.
    pub children: Option<Vec<DomNode>>,
}

// ---------------------------------------------------------------------------
// Input domain
// ---------------------------------------------------------------------------
