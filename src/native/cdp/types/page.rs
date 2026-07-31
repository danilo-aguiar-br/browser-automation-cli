// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP Page domain types.
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------

/// Mirrors CDP `Page.navigate` request.
///
/// Navigates current page to the given URL.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageNavigateParams {
    /// URL to navigate the page to.
    pub url: String,
    /// Referrer URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
}

/// Mirrors CDP `Page.navigate` response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageNavigateResult {
    /// Frame id that has navigated (or failed to navigate)
    pub frame_id: String,
    /// Loader identifier. This is omitted in case of same-document navigation, as the
    /// previously committed loaderId would not change.
    pub loader_id: Option<String>,
    /// User-friendly error message, present only when the navigation failed.
    ///
    /// The command still returns success in that case, so this field is the only
    /// signal that the page did not go where it was told.
    pub error_text: Option<String>,
}

/// Mirrors CDP `Page.frameNavigated` event.
///
/// Fired once navigation of the frame has completed. Frame is now associated with the new
/// loader.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameNavigatedEvent {
    /// The frame in its post-navigation state.
    pub frame: FrameInfo,
}

/// Mirrors CDP `Page.Frame`.
///
/// Only the subset this CLI reads is modelled; the protocol type carries more
/// fields that nothing here consumes.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameInfo {
    /// Frame identifier, used to tell the main frame from its iframes.
    pub id: String,
    /// Current document URL of the frame.
    pub url: String,
    /// Parent frame id. Absent on the main frame, which is how it is recognised.
    pub parent_id: Option<String>,
    /// `name` attribute of the owning `iframe` element, when it has one.
    pub name: Option<String>,
}

// Page.javascriptDialogOpening
/// Mirrors CDP `Page.javascriptDialogOpening` event.
///
/// Fired when a JavaScript initiated dialog (alert, confirm, prompt, or onbeforeunload) is
/// about to open.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavascriptDialogOpeningEvent {
    /// Frame url.
    pub url: String,
    /// Message the dialog displays.
    pub message: String,
    /// Dialog kind: `alert`, `confirm`, `prompt` or `beforeunload`.
    ///
    /// Renamed because `type` is a Rust keyword; the wire name stays `type`.
    #[serde(rename = "type")]
    pub dialog_type: String,
    /// Default text of a `prompt` dialog, absent for the other kinds.
    pub default_prompt: Option<String>,
}

/// Mirrors CDP `Page.handleJavaScriptDialog` request.
///
/// Accepts or dismisses a JavaScript initiated dialog (alert, confirm, prompt, or
/// onbeforeunload).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandleJavaScriptDialogParams {
    /// `true` accepts the dialog, `false` dismisses it.
    pub accept: bool,
    /// The text to enter into the dialog prompt before accepting. Used only if this is a prompt
    /// dialog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_text: Option<String>,
}

// ---------------------------------------------------------------------------
// Runtime domain
// ---------------------------------------------------------------------------
