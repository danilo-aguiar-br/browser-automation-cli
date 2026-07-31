// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP Network domain types.
use serde::Deserialize;

/// Mirrors CDP `Network.requestWillBeSent` event.
///
/// Fired when page is about to send HTTP request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestWillBeSentEvent {
    /// Request identifier.
    pub request_id: String,
    /// The request that is about to go out.
    pub request: NetworkRequest,
}

/// Mirrors CDP `Network.Request`.
///
/// HTTP request data.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkRequest {
    /// Request URL (without fragment).
    pub url: String,
    /// HTTP method, uppercase as sent on the wire.
    pub method: String,
}

/// Mirrors CDP `Network.loadingFinished` event.
///
/// Fired when HTTP request has finished loading.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadingFinishedEvent {
    /// Identifier of the request that completed; matches
    /// [`RequestWillBeSentEvent::request_id`].
    pub request_id: String,
}

/// Mirrors CDP `Network.loadingFailed` event.
///
/// Fired when HTTP request has failed to load.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadingFailedEvent {
    /// Identifier of the request that failed; matches
    /// [`RequestWillBeSentEvent::request_id`].
    pub request_id: String,
}

// ---------------------------------------------------------------------------
// DOM domain
// ---------------------------------------------------------------------------
