// SPDX-License-Identifier: MIT OR Apache-2.0
//! Anti-detection defaults: window mode, impersonated identity, virtual display.
//!
//! Every value here is a named default that an operator overrides through
//! `config set`. None of them is read from the environment, and none is spelled
//! inline at a call site.

/// Default window mode when neither a flag nor XDG decides.
///
/// `auto` resolves per platform at launch: headed inside a private Xvfb on
/// Linux, headed directly on macOS and Windows, headless only when a virtual
/// display cannot be obtained. Headless is not the default because the
/// product's own A/B measurement recorded a challenge on every headless run.
pub const DEFAULT_BROWSER_MODE: &str = "auto";

/// Default impersonated identity.
///
/// `auto` follows the host operating system. Claiming a foreign platform is a
/// net loss while TLS and HTTP/2 impersonation are out of scope: the User-Agent
/// would say Windows while the Canvas hash, the WebGL renderer and the TLS
/// fingerprint all still say Linux, and that contradiction is a stronger signal
/// than the honest platform ever was.
pub const DEFAULT_STEALTH_PROFILE: &str = "auto";

/// Width of the private virtual display, in pixels.
///
/// Chosen to match a common real monitor. A virtual display at, say, 800x600
/// is itself a fingerprint, because almost no human desktop reports it.
pub const DEFAULT_XVFB_WIDTH: u32 = 1920;

/// Height of the private virtual display, in pixels.
pub const DEFAULT_XVFB_HEIGHT: u32 = 1080;

/// Colour depth of the private virtual display, in bits.
///
/// 24 is what a real desktop reports. A lower depth changes how the page
/// rasterises and therefore changes the Canvas hash.
pub const DEFAULT_XVFB_DEPTH: u32 = 24;

/// Seconds to wait for a spawned Xvfb to accept connections before giving up.
pub const DEFAULT_XVFB_STARTUP_TIMEOUT_SECS: u64 = 10;

/// First display number the private-display search considers.
///
/// Starts high so the search can never reach a human's session. `:0` is the
/// operator's own desktop and `:1`..`:9` are routinely taken by display
/// managers and remote sessions; a bug in the search must not be able to point
/// an automated Chrome at any of them.
pub const XVFB_DISPLAY_SEARCH_START: u32 = 99;

/// How many consecutive display numbers the search will try.
///
/// Bounded rather than open-ended: a host where every number in the range is
/// taken has a problem the CLI should report, not one it should keep probing.
pub const XVFB_DISPLAY_SEARCH_SPAN: u32 = 32;

/// Poll interval while waiting for a spawned Xvfb to create its socket.
pub const XVFB_READY_POLL_MS: u64 = 50;

// Compile-time invariants, following the pattern in `constants/mod.rs`.
//
// These belong here rather than in a `#[test]`: the comparisons are known at
// compile time, so `assert!` inside a test is optimized away and the test can
// never fail. `const _` evaluates during compilation and breaks the BUILD, which
// is the only enforcement a constant can actually receive.
//
// A virtual display smaller than a real monitor is itself a fingerprint: almost
// no human desktop reports 800x600, and a depth below 24 changes how the page
// rasterises and therefore changes the Canvas hash.
const _: () = assert!(DEFAULT_XVFB_WIDTH >= 1280);
const _: () = assert!(DEFAULT_XVFB_HEIGHT >= 720);
const _: () = assert!(DEFAULT_XVFB_DEPTH == 24);
const _: () = assert!(DEFAULT_XVFB_STARTUP_TIMEOUT_SECS > 0);
// `:0` is the operator's own desktop and the low numbers belong to display
// managers. A search that could reach them would put an automated browser on a
// human's screen, so the floor is enforced at compile time rather than trusted.
const _: () = assert!(XVFB_DISPLAY_SEARCH_START >= 10);
const _: () = assert!(XVFB_DISPLAY_SEARCH_SPAN >= 8);
const _: () = assert!(XVFB_READY_POLL_MS > 0);
// The poll must fit inside the deadline it is polling against, or the first
// sleep would already exceed the whole budget.
const _: () = assert!(XVFB_READY_POLL_MS < DEFAULT_XVFB_STARTUP_TIMEOUT_SECS * 1000);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_parse_as_their_own_enums() {
        // A default its own parser rejects would make every unconfigured
        // process fall back to something nobody wrote down. Not a const
        // assertion because the parsers are runtime functions.
        assert!(crate::browser_policy::BrowserMode::parse(DEFAULT_BROWSER_MODE).is_some());
        assert!(crate::browser_policy::StealthProfile::parse(DEFAULT_STEALTH_PROFILE).is_some());
    }
}
