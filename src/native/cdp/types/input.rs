// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP Input domain types.
use serde::Serialize;

/// Mirrors CDP `Input.dispatchMouseEvent` request.
///
/// Dispatches a mouse event to the page.
///
/// # Why there is no `coalescedEvents` field
///
/// A real `PointerEvent` in Chrome answers `getCoalescedEvents()` with the
/// sub-frame samples the OS delivered between two frames; a CDP-synthesised
/// one answers with an empty list, and that difference is readable from page
/// JavaScript. `gaps.md` filed it as a defect of this struct.
///
/// It is not one. Measured 2026-09-01 against the tip-of-tree protocol
/// reference at `chromedevtools.github.io/devtools-protocol/tot/Input/`, the
/// whole `Input` domain contains ZERO occurrences of "coalesc": no command in
/// it accepts coalesced samples, so no field here could carry them. This is a
/// ceiling of the CDP surface, not an omission in this file, and adding a
/// field would put a key on the wire that Chrome discards.
///
/// Re-open this only with a protocol reference that lists such a parameter.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchMouseEventParams {
    /// Event kind: `mousePressed`, `mouseReleased`, `mouseMoved` or `mouseWheel`.
    ///
    /// Renamed because `type` is a Rust keyword; the wire name stays `type`.
    #[serde(rename = "type")]
    pub event_type: String,
    /// X coordinate of the event relative to the main frame's viewport in CSS pixels.
    pub x: f64,
    /// Y coordinate of the event relative to the main frame's viewport in CSS pixels.
    pub y: f64,
    /// Mouse button (default: "none").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<String>,
    /// A number indicating which buttons are pressed on the mouse when a mouse event is
    /// triggered. Left=1, Right=2, Middle=4, Back=8, Forward=16, None=0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<i32>,
    /// Number of times the mouse button was clicked (default: 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click_count: Option<i32>,
    /// X delta in CSS pixels for mouse wheel event (default: 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_x: Option<f64>,
    /// Y delta in CSS pixels for mouse wheel event (default: 0).
    ///
    /// # On `deltaMode`, which cannot be set here
    ///
    /// A `WheelEvent` in the page also carries `deltaMode`, and the same
    /// `deltaY` means different distances across its three values — pixel,
    /// line, page. CDP has no parameter for it: `Input.dispatchMouseEvent`
    /// defines these deltas as CSS pixels unconditionally, so Chrome
    /// synthesises the DOM event with `deltaMode: 0` and there is nothing to
    /// declare. Measured 2026-08-31 on a real page: `deltaMode` came back `0`
    /// and `deltaY` a canonical `100`.
    ///
    /// Recorded because a search for `deltaMode` in this tree returns nothing,
    /// and absence reads as an oversight rather than as a protocol boundary.
    ///
    /// Confirmed against the primary source on 2026-08-31: the CDP reference at
    /// <https://chromedevtools.github.io/devtools-protocol/tot/Input/> lists
    /// `deltaX` and `deltaY` for `dispatchMouseEvent`, both worded "in CSS
    /// pixels for mouse wheel event", and lists no `deltaMode` parameter at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_y: Option<f64>,
    /// Bit field representing pressed modifier keys. Alt=1, Ctrl=2, Meta/Command=4, Shift=8
    /// (default: 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<i32>,
    /// X coordinate of the event relative to the SCREEN, in CSS pixels.
    ///
    /// # Why this is here and why it stays `None`
    ///
    /// `event.pageX == event.screenX` was a public automation check: crbug
    /// 1477537 had Chromium copy `pageX` into `screenX` for a synthesized
    /// event, and no real pointer satisfies that equality, because a window
    /// sits somewhere on a desktop.
    ///
    /// MEASURED 2026-08-31 on the FINAL event, not on the version string.
    /// Chromium 151.0.7922.173, `press` under `--input-profile human`, these
    /// two fields OMITTED from the wire: `pageX` 198 against `screenX` 208,
    /// `pageY` 202 against `screenY` 299, with `window.screenX` 10 and
    /// `window.screenY` 10. The browser derives both coordinates itself and
    /// the equality is FALSE. The vector is closed upstream (fixed in
    /// Chrome-Stable 142) and there is nothing here to defeat.
    ///
    /// The fields exist so the type mirrors the CDP command in full and so a
    /// caller pinned to a pre-142 Chrome can fill them. They are left `None`
    /// DELIBERATELY, and filling them by default would make things worse: the
    /// Y offset is the browser chrome height, which this process does not
    /// know, so a client-computed `screenY` would CONTRADICT the value the
    /// renderer already produces. A declared value the execution cannot
    /// sustain is a new signal, not a removed one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_x: Option<f64>,
    /// Y coordinate of the event relative to the SCREEN, in CSS pixels.
    ///
    /// See [`Self::screen_x`] for the measurement and for why this stays `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_y: Option<f64>,
}

/// Mirrors CDP `Input.dispatchKeyEvent` request.
///
/// Dispatches a key event to the page.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchKeyEventParams {
    /// Event kind: `keyDown`, `keyUp`, `rawKeyDown` or `char`.
    ///
    /// Renamed because `type` is a Rust keyword; the wire name stays `type`.
    #[serde(rename = "type")]
    pub event_type: String,
    /// Unique DOM defined string value describing the meaning of the key in the context of
    /// active modifiers, keyboard layout, etc (e.g., 'AltGr') (default: "").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Unique DOM defined string value for each physical key (e.g., 'KeyA') (default: "").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Text as generated by processing a virtual key code with a keyboard layout. Not needed
    /// for for `keyUp` and `rawKeyDown` events (default: "")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Text that would have been generated by the keyboard if no modifiers were pressed (except
    /// for shift). Useful for shortcut (accelerator) key handling (default: "").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unmodified_text: Option<String>,
    /// Windows virtual key code (default: 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub windows_virtual_key_code: Option<i32>,
    /// Native virtual key code (default: 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_virtual_key_code: Option<i32>,
    /// Bit field representing pressed modifier keys. Alt=1, Ctrl=2, Meta/Command=4, Shift=8
    /// (default: 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<i32>,
}

/// Mirrors CDP `Input.insertText` request.
///
/// This method emulates inserting text that doesn't come from a key press, for example an emoji
/// keyboard or an IME.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertTextParams {
    /// Text to insert verbatim, without synthesising key events for it.
    pub text: String,
}

// ---------------------------------------------------------------------------
// Page.captureScreenshot
// ---------------------------------------------------------------------------
