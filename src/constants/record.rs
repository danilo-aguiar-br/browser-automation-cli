// SPDX-License-Identifier: MIT OR Apache-2.0
//! Interaction-recorder (`record`) compile-time defaults.
//!
//! Operator overrides belong in XDG (`config set …`) or on argv; these are the
//! named fallbacks only (never product environment variables).

/// Default wall-clock recording budget, in seconds, when `--seconds` is omitted.
///
/// Thirty seconds is long enough for a human to complete a login or a checkout
/// step by hand, and short enough that an agent that forgot to pass a ceiling
/// still gets its NDJSON back inside a normal command timeout.
pub const RECORD_DEFAULT_SECONDS: u64 = 30;

/// Default recorded-event ceiling when `--max-events` is omitted.
///
/// A hand-driven flow worth replaying is tens of gestures, not thousands; two
/// hundred leaves generous headroom while bounding both the NDJSON file and the
/// in-process buffer for a page that fires `input` on every keystroke.
pub const RECORD_DEFAULT_MAX_EVENTS: usize = 200;

/// Name of the `Runtime.addBinding` function the injected capture script calls.
///
/// Deliberately verbose and product-prefixed: the binding lands on the page's
/// own `window`, so a short name would risk colliding with page globals.
pub const RECORD_BINDING_NAME: &str = "__browserAutomationCliRecordEvent";

/// Longest CSS path, in ancestor steps, the capture script will build.
///
/// Beyond this depth an `nth-of-type` chain is no more selective and only makes
/// the recorded step harder to read and more brittle to re-render.
pub const RECORD_MAX_SELECTOR_DEPTH: usize = 8;

const _: () = assert!(RECORD_DEFAULT_SECONDS > 0);
const _: () = assert!(RECORD_DEFAULT_MAX_EVENTS > 0);
const _: () = assert!(RECORD_MAX_SELECTOR_DEPTH > 0);
const _: () = assert!(!RECORD_BINDING_NAME.is_empty());
