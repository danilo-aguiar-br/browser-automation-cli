// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP Target domain types.
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------

/// Mirrors CDP `Target.TargetInfo`.
///
/// The protocol ships this type without prose, so the field docs below describe
/// what the CLI relies on rather than restating the upstream schema.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    /// Stable id of the target for the lifetime of the browser process.
    pub target_id: String,
    /// Kind of target: `page`, `iframe`, `worker`, `service_worker`, `browser`, …
    ///
    /// Renamed because `type` is a Rust keyword; the wire name stays `type`.
    #[serde(rename = "type")]
    pub target_type: String,
    /// Document title at the time the info was produced.
    pub title: String,
    /// Current document URL of the target.
    pub url: String,
    /// Whether the target has an attached client.
    pub attached: Option<bool>,
    /// Browser context that owns the target, which is what isolates cookies
    /// and storage between `page new --isolated-context` tabs.
    pub browser_context_id: Option<String>,
}

/// Mirrors CDP `Target.getTargets` response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTargetsResult {
    /// Every target the browser currently knows about.
    pub target_infos: Vec<TargetInfo>,
}

/// Mirrors CDP `Target.attachToTarget` request.
///
/// Attaches to the target with given id.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetParams {
    /// Target to attach to.
    pub target_id: String,
    /// Requests flat session mode, which this CLI always uses.
    ///
    /// Flat mode multiplexes the attached session over the existing browser
    /// connection instead of opening a second websocket per target.
    pub flatten: bool,
}

/// Mirrors CDP `Target.attachToTarget` response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetResult {
    /// Session id to put on later commands so they reach this target.
    pub session_id: String,
}

/// Mirrors CDP `Target.setDiscoverTargets` request.
///
/// Controls whether to discover available targets and notify via
/// `targetCreated/targetInfoChanged/targetDestroyed` events.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDiscoverTargetsParams {
    /// Turns target discovery events on or off.
    pub discover: bool,
}

/// Mirrors CDP `Target.createTarget` request.
///
/// Creates a new page.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetParams {
    /// URL the new page opens at.
    pub url: String,
    /// Optional BrowserContext id from `Browser.createBrowserContext` (cookie isolation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<String>,
}

/// Mirrors CDP `Target.createTarget` response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetResult {
    /// Id of the page that was created.
    pub target_id: String,
}

/// Mirrors CDP `Target.closeTarget` request.
///
/// Closes the target. If the target is a page that gets closed too.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseTargetParams {
    /// Target to close.
    pub target_id: String,
}

// Target events
/// Mirrors CDP `Target.targetCreated` event.
///
/// Issued when a possible inspection target is created.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCreatedEvent {
    /// Description of the target that appeared.
    pub target_info: TargetInfo,
}

/// Mirrors CDP `Target.targetDestroyed` event.
///
/// Issued when a target is destroyed.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetDestroyedEvent {
    /// Id of the target that went away.
    pub target_id: String,
}

/// Mirrors CDP `Target.targetInfoChanged` event.
///
/// Issued when some information about a target has changed. This only happens between
/// `targetCreated` and `targetDestroyed`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfoChangedEvent {
    /// Updated description of the target, for example after a navigation.
    pub target_info: TargetInfo,
}

// ---------------------------------------------------------------------------
// Page domain
