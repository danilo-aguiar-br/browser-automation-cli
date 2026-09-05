// SPDX-License-Identifier: MIT OR Apache-2.0
//! How the browser window is materialized: mode plus the Xvfb opt-out.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use super::source::PolicySource;

/// How the browser window is materialized for this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserMode {
    /// Decide per platform: headed inside a private Xvfb on Linux, headed
    /// directly on macOS and Windows, headless only as a last resort.
    Auto,
    /// Headed with a window on the caller's own display. Debugging.
    Headed,
    /// Headless. Cheapest and most detectable; an explicit opt-in to that cost.
    Headless,
}

impl BrowserMode {
    /// Parse a `browser_mode` config token, rejecting anything unknown.
    ///
    /// Returns `None` rather than silently falling back, so a typo in the XDG
    /// file surfaces as a validation error instead of a mode nobody chose.
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        match token.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "headed" => Some(Self::Headed),
            "headless" => Some(Self::Headless),
            _ => None,
        }
    }

    /// The canonical token, for envelopes and diagnostics.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Headed => "headed",
            Self::Headless => "headless",
        }
    }

    /// Whether Chrome launches without a window under this mode.
    ///
    /// # Why `Auto` is not a constant
    ///
    /// `Auto` is defined as "headed inside a private virtual display on Linux".
    /// For one version this method answered `true` for `Auto` unconditionally,
    /// with a comment saying the display half had not landed yet. It has: the
    /// whole of `native::cdp::xvfb` is built and reached in production from
    /// `native::cdp::chrome::spawn`. What kept it disconnected was this very
    /// method, because `xvfb::should_use_private_display` asks it first and an
    /// `Auto` that calls itself headless can never be granted a display.
    ///
    /// # The trap that a global flip walks into
    ///
    /// Answering `false` for `Auto` everywhere is the wrong repair, and it was
    /// measured to be wrong. It regressed `tests/grab_envelope_gate.rs`: a
    /// `--full-page` capture came back 4500px tall against an expected 3000,
    /// exactly the host's 1.5 scaling factor. That host was macOS, where there
    /// is no Xvfb and no repair — a headed `Auto` there is a real window on the
    /// operator's own display, inheriting its device pixel ratio.
    ///
    /// So the answer for `Auto` is per host, resolved once by
    /// [`set_auto_headed`] during dispatch and read here as an atomic. Off
    /// Linux the resolver's `cfg!` short-circuits to `false` and `Auto` stays
    /// headless, which is why the 4500px regression cannot come back on this
    /// class of host.
    ///
    /// Reading a resolved atomic rather than probing is also what keeps the
    /// cycle open: `should_use_private_display` calls this, and this calls
    /// nothing.
    #[must_use]
    pub fn launches_headless(self) -> bool {
        match self {
            Self::Headed => false,
            Self::Headless => true,
            Self::Auto => !auto_headed(),
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Auto => 0,
            Self::Headed => 1,
            Self::Headless => 2,
        }
    }

    fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Headed,
            2 => Self::Headless,
            _ => Self::Auto,
        }
    }
}

static MODE: AtomicU8 = AtomicU8::new(0);
static MODE_SOURCE: AtomicU8 = AtomicU8::new(0);
static NO_XVFB: AtomicBool = AtomicBool::new(false);
static AUTO_HEADED: AtomicBool = AtomicBool::new(false);

/// Publish the resolved window mode and where it came from.
///
/// Called once from CLI dispatch. The SOURCE is not decoration: it is the only
/// thing that separates "headless because the caller asked for it" from
/// "headless because nobody said otherwise". A consumer with headless as a
/// security requirement can act on the first and must not trust the second,
/// and without this value the two are indistinguishable after the fact.
pub fn set_mode(mode: BrowserMode, source: PolicySource) {
    MODE.store(mode.code(), Ordering::Relaxed);
    MODE_SOURCE.store(source.code(), Ordering::Relaxed);
}

/// The window mode in force for this process.
#[must_use]
pub fn mode() -> BrowserMode {
    BrowserMode::from_code(MODE.load(Ordering::Relaxed))
}

/// Which precedence step produced [`mode`].
#[must_use]
pub fn mode_source() -> PolicySource {
    PolicySource::from_code(MODE_SOURCE.load(Ordering::Relaxed))
}

/// Publish the `--no-xvfb` opt-out. Called once from CLI dispatch.
pub fn set_no_xvfb(disabled: bool) {
    NO_XVFB.store(disabled, Ordering::Relaxed);
}

/// Whether the caller refused the private Xvfb display on Linux.
#[must_use]
pub fn no_xvfb() -> bool {
    NO_XVFB.load(Ordering::Relaxed)
}

/// Resolve, once per process, whether `auto` may launch headed on this host.
///
/// # Why the probe happens here and not in `launches_headless`
///
/// `xvfb_available` walks `PATH`. `launches_headless` is asked on hot paths —
/// stealth setup, the doctor probes, every envelope witness — so a probe inside
/// it would turn a comparison into a filesystem scan repeated per call, and the
/// answer cannot change mid-process anyway.
///
/// # Why the `cfg!` comes first
///
/// Short-circuiting on the target keeps macOS and Windows from paying for a
/// `PATH` walk looking for a binary that does not exist on either. It is also
/// the guard that pins `auto` to headless off Linux, which is the property that
/// keeps the `grab_envelope_gate` screenshot regression from returning.
///
/// # Why `no_xvfb` is an input rather than a later check
///
/// An operator who passed `--no-xvfb` asked to SEE the window. Honouring that
/// by leaving `auto` headed would launch onto their compositor — the exact
/// outcome `auto` exists to avoid. `--no-xvfb` remains a live opt-out for
/// explicit `--headed`, where seeing the window is the stated intent.
pub fn set_auto_headed(no_xvfb_requested: bool) {
    let headed = cfg!(target_os = "linux")
        && !no_xvfb_requested
        && crate::native::cdp::xvfb::xvfb_available();
    AUTO_HEADED.store(headed, Ordering::Relaxed);
}

/// Whether `auto` resolved to a headed launch on this host.
///
/// Deliberately NOT `pub`: the only consumer is `launches_headless` in this
/// file, and every outside caller wants that question, not this one. It was
/// exported once and `tests/phantom_flag_gate.rs` refused it — a published
/// value with no production reader is a flag that changes nothing, and the
/// envelope already answers "what did `auto` become" through
/// `browser_mode_effective`.
fn auto_headed() -> bool {
    AUTO_HEADED.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_tokens_round_trip() {
        for token in ["auto", "headed", "headless"] {
            let parsed = BrowserMode::parse(token).expect("known token");
            assert_eq!(parsed.as_str(), token);
            assert_eq!(BrowserMode::from_code(parsed.code()), parsed);
        }
    }

    #[test]
    fn unknown_mode_token_is_rejected_not_defaulted() {
        // A silent fallback to Auto would hide a typo in the XDG file and run
        // the browser in a mode the operator never chose.
        assert!(BrowserMode::parse("headfull").is_none());
        assert!(BrowserMode::parse("").is_none());
    }

    #[test]
    fn explicit_modes_never_consult_the_auto_resolver() {
        // The regression this pins: a future edit that routes all three arms
        // through `auto_headed`. `--headless` would then become headed on a
        // Linux box with Xvfb installed, which is the opposite of what the flag
        // exists to guarantee, and no other test in this tree would notice.
        //
        // Both stores run so the assertion holds under either resolution,
        // rather than passing by accident on whichever host runs it.
        for resolved in [false, true] {
            AUTO_HEADED.store(resolved, Ordering::Relaxed);
            assert!(!BrowserMode::Headed.launches_headless());
            assert!(BrowserMode::Headless.launches_headless());
        }
    }

    #[test]
    fn auto_is_the_inverse_of_what_the_host_resolved() {
        // `Auto` is the only mode with an answer this process computed, so it
        // is the only one whose answer can drift from the resolver in silence.
        for resolved in [false, true] {
            AUTO_HEADED.store(resolved, Ordering::Relaxed);
            assert_eq!(BrowserMode::Auto.launches_headless(), !resolved);
        }
    }

    #[test]
    fn a_non_linux_host_can_never_resolve_auto_to_headed() {
        // The guard that keeps the measured screenshot regression buried: on
        // macOS a headed `Auto` is a real window on the operator's display, and
        // `tests/grab_envelope_gate.rs` caught it as a 4500px capture against
        // an expected 3000. `set_auto_headed` must refuse regardless of what
        // the other two inputs say.
        if cfg!(target_os = "linux") {
            return;
        }
        set_auto_headed(false);
        assert!(!auto_headed());
        assert!(BrowserMode::Auto.launches_headless());
    }

    #[test]
    fn refusing_xvfb_keeps_auto_headless_even_on_linux() {
        // `--no-xvfb` means "let me see the window". Granting that under `auto`
        // would put Chrome on the operator's compositor, which is the single
        // outcome `auto` exists to prevent. Seeing the window stays available
        // through explicit `--headed`, where it is the stated intent.
        set_auto_headed(true);
        assert!(!auto_headed());
        assert!(BrowserMode::Auto.launches_headless());
    }
}
