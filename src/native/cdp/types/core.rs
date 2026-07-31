// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chrome DevTools Protocol wire types used by the native CDP stack.
//!
//! Field-level docs are intentionally sparse: shapes mirror the upstream protocol JSON.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Deserialize a value that may be either a string or an integer into a String.
/// Lightpanda sends numeric nodeIds/childIds in AX tree responses, while Chrome
/// sends strings. This accepts both.
pub(crate) fn string_or_int<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Value::deserialize(deserializer)?;
    match v {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        other => Err(serde::de::Error::custom(format!(
            "expected string or integer, got {other}"
        ))),
    }
}

/// Deserialize an optional Vec where each element may be a string or integer.
pub(crate) fn opt_vec_string_or_int<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<Vec<Value>> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(vec) => {
            let mut result = Vec::with_capacity(vec.len());
            for v in vec {
                match v {
                    Value::String(s) => result.push(s),
                    Value::Number(n) => result.push(n.to_string()),
                    other => {
                        return Err(serde::de::Error::custom(format!(
                            "expected string or integer in array, got {other}"
                        )))
                    }
                }
            }
            Ok(Some(result))
        }
    }
}

// ---------------------------------------------------------------------------
// CDP message envelope
// ---------------------------------------------------------------------------

/// One request written to the DevTools websocket.
///
/// This is the JSON-RPC frame the protocol defines, not a domain type: the
/// domain payload travels opaque in `params`, because a single connection
/// carries every domain.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CdpCommand {
    /// Correlates this request with the response that carries the same `id`.
    ///
    /// Responses arrive interleaved with events on one socket, so the id is the
    /// only way to match a reply to its caller.
    pub id: u64,
    /// Fully qualified `Domain.method`, for example `Page.navigate`.
    pub method: String,
    /// Domain-specific arguments, left opaque so one frame serves every domain.
    pub params: Option<Value>,
    /// Target session to route to, absent for browser-level commands.
    ///
    /// Flat session mode multiplexes every attached target over the browser
    /// connection, and this is what distinguishes them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Anything read from the DevTools websocket, before it is classified.
///
/// The protocol sends responses and events over the same connection, so a frame
/// is a response when `id` is set and an event when `method` is. Every field is
/// optional because no single frame carries all of them.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdpMessage {
    /// Present on a response; matches the `id` of the [`CdpCommand`] that asked.
    pub id: Option<u64>,
    /// Success payload of a response. Mutually exclusive with `error`.
    pub result: Option<Value>,
    /// Failure payload of a response. Mutually exclusive with `result`.
    pub error: Option<CdpError>,
    /// Present on an event: the `Domain.event` that fired.
    pub method: Option<String>,
    /// Event payload, meaningful only alongside `method`.
    pub params: Option<Value>,
    /// Session the frame belongs to, absent at browser level.
    pub session_id: Option<String>,
}

/// Error object of a failed CDP response.
#[derive(Debug, Clone, Deserialize)]
pub struct CdpError {
    /// JSON-RPC error code. Chrome does not always send one.
    pub code: Option<i64>,
    /// Human-readable failure reason; this is what `Display` prints.
    pub message: String,
    /// Extra detail Chrome sometimes attaches, such as an exception text.
    pub data: Option<String>,
}

impl std::fmt::Display for CdpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

// ---------------------------------------------------------------------------
// CDP events (broadcast to subscribers)
// ---------------------------------------------------------------------------

/// A [`CdpMessage`] already classified as an event and fanned out to subscribers.
///
/// The optional fields of the wire frame are resolved here: an event always has
/// a method and a payload, so subscribers do not re-check what the classifier
/// already decided.
#[derive(Debug, Clone)]
pub struct CdpEvent {
    /// The `Domain.event` that fired, for example `Page.loadEventFired`.
    pub method: String,
    /// Event payload as sent, left opaque for the domain-specific decoder.
    pub params: Value,
    /// Session that emitted the event, absent at browser level.
    pub session_id: Option<String>,
}

// ---------------------------------------------------------------------------
