// SPDX-License-Identifier: MIT OR Apache-2.0

/// Outcome of a click. `dialog_opened` is true if a JavaScript dialog opened
/// mid-sequence (the page is then blocked until `dialog accept`/`dismiss`).
/// `pending_release` is set only when the dialog opened after mousePressed but
/// before mouseReleased: the button is logically held until the caller
/// dispatches the release (done once the dialog is resolved), otherwise the
/// next click would register as a drag or double-click.
#[derive(Default)]
pub struct ClickResult {
    /// A JavaScript dialog opened during the click.
    ///
    /// The page is blocked until it is answered, so the caller must handle the
    /// dialog before issuing anything else.
    pub dialog_opened: bool,
    /// A mouse button left pressed on purpose, to be released later.
    pub pending_release: Option<PendingRelease>,
}

/// A press that still owes a release.
///
/// Everything needed to release the button at the SAME point it went down:
/// releasing elsewhere turns the gesture into a drag.
pub struct PendingRelease {
    /// Session the press was dispatched on.
    pub session_id: String,
    /// X of the press, in viewport coordinates.
    pub x: f64,
    /// Y of the press, in viewport coordinates.
    pub y: f64,
    /// Which button is held: `left`, `right` or `middle`.
    pub button: String,
}
