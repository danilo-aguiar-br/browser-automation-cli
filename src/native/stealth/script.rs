// SPDX-License-Identifier: MIT OR Apache-2.0
//! Product-owned patches that the emulation crate does not cover.
//!
//! # Why this file exists at all
//!
//! It was written against a measurement, not against a document. The first
//! version of this layer trusted the crate's own guidance — "You should not
//! need this if you use the cli args ... `disabled-features=AutomationEnabled`"
//! — and passed `--disable-blink-features=AutomationControlled` instead of
//! patching from inside the page.
//!
//! Measured on Chrome against a local probe page, with stealth on and off:
//!
//! ```text
//! stealth off -> {"webdriver": false, "webdriver_type": "boolean", ...}
//! stealth on  -> {"webdriver": false, "webdriver_type": "boolean", ...}
//! ```
//!
//! The switch changed nothing, and `false` is the worst of the three possible
//! answers. A real Chrome reports `undefined`, because the property does not
//! exist. `true` says "automated". `false` says "something removed the flag",
//! which only automation ever has reason to do — so the defensive value is
//! itself the tell.
//!
//! The fix is deletion, not a getter. Overriding the getter to return `false`
//! leaves the property present and leaves a patched
//! `Function.prototype.toString` behind. Deleting it from the prototype makes
//! `navigator.webdriver` `undefined` AND `'webdriver' in navigator` false,
//! which is what an unautomated browser reports.

/// Patches applied before the crate's emulation payload.
///
/// Ordering matters: this runs first so the crate's script observes a browser
/// that already looks unautomated, rather than racing it for the same
/// properties.
pub const PRODUCT_PATCHES: &str = r#"(()=>{try{
const proto = Object.getPrototypeOf(navigator);
if (proto && 'webdriver' in proto) { delete proto.webdriver; }
if ('webdriver' in navigator) { try { delete navigator.webdriver; } catch (_) {} }
}catch(_){}
try{
const c = window.chrome || {};
if (!c.runtime) {
  Object.defineProperty(c, 'runtime', {
    value: { id: undefined, connect: function connect(){}, sendMessage: function sendMessage(){} },
    enumerable: true, configurable: true
  });
}
Object.defineProperty(window, 'chrome', { value: c, writable: true, enumerable: true, configurable: true });
}catch(_){}
try{
if (window.outerHeight === 0) { Object.defineProperty(window, 'outerHeight', { get: () => window.innerHeight, configurable: true }); }
if (window.outerWidth === 0) { Object.defineProperty(window, 'outerWidth', { get: () => window.innerWidth, configurable: true }); }
}catch(_){}
})();"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webdriver_is_deleted_not_set_to_false() {
        // Setting it to `false` is the documented anti-pattern: a real browser
        // has no such property, so a defined-and-false value announces that
        // something removed it.
        assert!(PRODUCT_PATCHES.contains("delete proto.webdriver"));
        assert!(
            !PRODUCT_PATCHES.contains("webdriver:false")
                && !PRODUCT_PATCHES.contains("webdriver', {get: () => false"),
            "webdriver must be deleted, never assigned false"
        );
    }

    #[test]
    fn every_patch_is_individually_guarded() {
        // One `try` around the whole script would let the first failure skip
        // every later patch. Each concern gets its own.
        assert!(PRODUCT_PATCHES.matches("try{").count() >= 3);
    }

    #[test]
    fn outer_dimensions_are_only_patched_when_they_are_zero() {
        // A headed browser reports real values; overwriting them would replace
        // a true answer with a guess.
        assert!(PRODUCT_PATCHES.contains("window.outerHeight === 0"));
    }
}
