// SPDX-License-Identifier: MIT OR Apache-2.0
//! The page-side capture script injected by `record`.
//!
//! # Why the script lives in the page and not in Rust
//!
//! A recorder that polled the DOM would see state, not gestures: it cannot tell
//! a click from a programmatic focus, and it cannot observe the element the user
//! actually aimed at. Listening in the page is the only place where the gesture
//! and its target exist at the same time.
//!
//! # Why the listeners are capturing and on `document`
//!
//! A page that calls `stopPropagation()` in its own handler would hide the event
//! from a bubbling listener. Capturing on `document` runs before any page
//! handler, so a recorded flow reflects what the operator did rather than what
//! the page chose to let through.
//!
//! # Known limitations
//!
//! - Only the top frame is recorded; a gesture inside an iframe is not captured,
//!   because the CSS selector it would produce is not resolvable from the top
//!   document that `run --script` replays against.
//! - `value` is read from `el.value`, so a checkbox records its `value`
//!   attribute rather than its checked state.

use crate::constants::{RECORD_BINDING_NAME, RECORD_MAX_SELECTOR_DEPTH};

/// Template for the injected script; placeholders are substituted by [`capture_script`].
///
/// Kept as a template rather than a `format!` literal so the JavaScript stays
/// readable: `format!` would require every brace in the source to be doubled.
const CAPTURE_SCRIPT_TEMPLATE: &str = r#"
(() => {
  if (window.top !== window) { return; }
  const send = window.__BINDING__;
  if (typeof send !== 'function') { return; }
  if (window.__BINDING___armed) { return; }
  window.__BINDING___armed = true;

  const MAX_DEPTH = __DEPTH__;

  const escapeId = (id) =>
    (window.CSS && typeof CSS.escape === 'function') ? CSS.escape(id) : id;

  // Stable-first selector: an id is unique by contract, a name is unique inside
  // a form in practice, and the structural path is the last resort.
  const selectorFor = (el) => {
    if (!el || el.nodeType !== 1) { return null; }
    if (el.id) { return '#' + escapeId(el.id); }
    const named = el.getAttribute ? el.getAttribute('name') : null;
    if (named) {
      return el.tagName.toLowerCase() + '[name="' + named.replace(/"/g, '\\"') + '"]';
    }
    const parts = [];
    let node = el;
    while (node && node.nodeType === 1 && parts.length < MAX_DEPTH) {
      if (node.id) { parts.unshift('#' + escapeId(node.id)); break; }
      const tag = node.tagName.toLowerCase();
      const parent = node.parentElement;
      if (!parent) { parts.unshift(tag); break; }
      const twins = Array.prototype.filter.call(
        parent.children, (c) => c.tagName === node.tagName);
      parts.unshift(twins.length > 1
        ? tag + ':nth-of-type(' + (twins.indexOf(node) + 1) + ')'
        : tag);
      node = parent;
    }
    return parts.join(' > ');
  };

  const emit = (payload) => {
    try { send(JSON.stringify(payload)); } catch (_) { /* recorder must never break the page */ }
  };

  const emitTarget = (type, el, value) => {
    const selector = selectorFor(el);
    if (!selector) { return; }
    emit(value === undefined
      ? { type: type, selector: selector }
      : { type: type, selector: selector, value: String(value) });
  };

  emit({ type: 'navigate', url: String(location.href) });

  document.addEventListener('click', (e) => emitTarget('click', e.target), true);
  document.addEventListener('input', (e) => emitTarget('input', e.target, e.target.value), true);
  document.addEventListener('change', (e) => emitTarget('change', e.target, e.target.value), true);
  document.addEventListener('submit', (e) => emitTarget('submit', e.target), true);
})();
"#;

/// The capture script with the binding name and selector depth substituted in.
pub(super) fn capture_script() -> String {
    CAPTURE_SCRIPT_TEMPLATE
        .replace("__BINDING__", RECORD_BINDING_NAME)
        .replace("__DEPTH__", &RECORD_MAX_SELECTOR_DEPTH.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_substituted() {
        let src = capture_script();
        assert!(
            !src.contains("__BINDING__"),
            "binding placeholder left in: {src}"
        );
        assert!(
            !src.contains("__DEPTH__"),
            "depth placeholder left in: {src}"
        );
        assert!(src.contains(RECORD_BINDING_NAME));
    }
}
