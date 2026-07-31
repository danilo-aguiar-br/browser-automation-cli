// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared native `<select>` event dispatch (GAP-055).
//!
//! Reactive forms listen for `input`; plain handlers listen for `change`.
//! Both `pick` (session) and `select_option` (fill-form path) must fire the
//! same pair so agents never see a silent half-update.

/// JS statements that dispatch `input` then `change` on `this` (a `<select>`).
///
/// Embed inside a `function(...) { ... }` body that already set the value.
pub const DISPATCH_INPUT_AND_CHANGE: &str = r#"
            this.dispatchEvent(new Event('input', { bubbles: true }));
            this.dispatchEvent(new Event('change', { bubbles: true }));
"#;
