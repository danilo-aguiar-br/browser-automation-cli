// SPDX-License-Identifier: MIT OR Apache-2.0
//! The anti-detection half of the browser policy: on/off, identity, seed, and
//! whether the patch actually reached the page.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::OnceLock;

/// Which browser identity the stealth layer impersonates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StealthProfile {
    /// Match the host operating system. The only value that cannot contradict
    /// the Canvas and WebGL hashes the real GPU produces.
    Auto,
    /// Chrome on Linux.
    ChromeLinux,
    /// Chrome on Windows.
    ChromeWindows,
    /// Chrome on macOS.
    ChromeMac,
}

impl StealthProfile {
    /// Parse a `stealth_profile` config token, rejecting anything unknown.
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        match token.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "chrome-linux" => Some(Self::ChromeLinux),
            "chrome-win" | "chrome-windows" => Some(Self::ChromeWindows),
            "chrome-mac" | "chrome-macos" => Some(Self::ChromeMac),
            _ => None,
        }
    }

    /// The canonical token, for envelopes and diagnostics.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::ChromeLinux => "chrome-linux",
            Self::ChromeWindows => "chrome-win",
            Self::ChromeMac => "chrome-mac",
        }
    }

    /// Resolve `Auto` against the host so callers never branch on it.
    ///
    /// Impersonating a foreign platform is a net loss without TLS and HTTP/2
    /// impersonation to match: the User-Agent would claim Windows while the
    /// Canvas hash, the WebGL renderer and the TLS fingerprint all still say
    /// Linux, and that contradiction is a stronger signal than the honest
    /// platform ever was.
    #[must_use]
    pub fn resolved(self) -> Self {
        match self {
            Self::Auto => {
                if cfg!(target_os = "windows") {
                    Self::ChromeWindows
                } else if cfg!(target_os = "macos") {
                    Self::ChromeMac
                } else {
                    Self::ChromeLinux
                }
            }
            other => other,
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Auto => 0,
            Self::ChromeLinux => 1,
            Self::ChromeWindows => 2,
            Self::ChromeMac => 3,
        }
    }

    fn from_code(code: u8) -> Self {
        match code {
            1 => Self::ChromeLinux,
            2 => Self::ChromeWindows,
            3 => Self::ChromeMac,
            _ => Self::Auto,
        }
    }
}

/// Where the active `--stealth-profile` token came from.
///
/// This is an alias for [`PolicySource`](super::PolicySource). The three steps
/// it names — flag, XDG, compiled default — are the SAME three every policy in
/// this module resolves through, so the type moved out of this file and the old
/// name stayed. The name is already published in envelopes and in `doctor`, and
/// renaming a token an agent branches on would buy nothing.
pub use super::source::PolicySource as ProfileSource;

static STEALTH: AtomicBool = AtomicBool::new(true);
/// Whether the patch script reached the page, as opposed to merely being wanted.
static STEALTH_INSTALLED: AtomicU8 = AtomicU8::new(STEALTH_INSTALL_UNATTEMPTED);
static STEALTH_PROFILE: AtomicU8 = AtomicU8::new(0);
static STEALTH_PROFILE_SOURCE: AtomicU8 = AtomicU8::new(0);
static STEALTH_SEED: OnceLock<Option<String>> = OnceLock::new();

/// No launch has tried to install the patch yet (no browser in this process).
const STEALTH_INSTALL_UNATTEMPTED: u8 = 0;
/// `Page.addScriptToEvaluateOnNewDocument` accepted the patch.
const STEALTH_INSTALL_OK: u8 = 1;
/// The launch tried and the CDP call failed; markers stay visible.
const STEALTH_INSTALL_FAILED: u8 = 2;

/// Publish whether stealth is active. Called once from CLI dispatch.
pub fn set_stealth(enabled: bool) {
    STEALTH.store(enabled, Ordering::Relaxed);
}

/// Whether anti-detection patches apply to this process.
///
/// Default is on: the product's own A/B measurement showed every non-stealth
/// configuration receiving a challenge, so shipping it off by default would
/// have made the common path the broken one.
#[must_use]
pub fn stealth_enabled() -> bool {
    STEALTH.load(Ordering::Relaxed)
}

/// Record whether the anti-detection patch actually reached the page.
///
/// Installation is best effort by design — a page that cannot take the script
/// is still a page the caller asked for. But "best effort" only stays honest
/// if the effort's OUTCOME is observable, and until 0.1.9 it was not: the
/// failure produced a `tracing::warn!` that `--json -q` swallows, while
/// `doctor --fingerprint` went on reporting `stealth: true`. The comment at the
/// call site promised "the absence is observable through `doctor`"; this is the
/// state that makes the promise true.
pub fn set_stealth_installed(ok: bool) {
    STEALTH_INSTALLED.store(
        if ok {
            STEALTH_INSTALL_OK
        } else {
            STEALTH_INSTALL_FAILED
        },
        Ordering::Relaxed,
    );
}

/// Outcome of the patch installation: `None` when no launch attempted it.
#[must_use]
pub fn stealth_installed() -> Option<bool> {
    match STEALTH_INSTALLED.load(Ordering::Relaxed) {
        STEALTH_INSTALL_OK => Some(true),
        STEALTH_INSTALL_FAILED => Some(false),
        _ => None,
    }
}

/// Publish the stealth identity token. Called once from CLI dispatch.
pub fn set_stealth_profile(profile: StealthProfile) {
    STEALTH_PROFILE.store(profile.code(), Ordering::Relaxed);
}

/// Publish whether the token came from the flag, XDG, or the default.
pub fn set_stealth_profile_source(source: ProfileSource) {
    STEALTH_PROFILE_SOURCE.store(source.code(), Ordering::Relaxed);
}

/// The stealth identity token as the operator wrote it (`auto` stays `auto`).
#[must_use]
pub fn stealth_profile_token() -> StealthProfile {
    StealthProfile::from_code(STEALTH_PROFILE.load(Ordering::Relaxed))
}

/// The stealth identity in force, already resolved against the host.
#[must_use]
pub fn stealth_profile() -> StealthProfile {
    stealth_profile_token().resolved()
}

/// The identity token a cache key must carry, or `off` when stealth is off.
///
/// # The collision this closes
///
/// [`stealth_profile`] resolves `auto` against the HOST, and it keeps doing so
/// when stealth is disabled — there is no such thing as a resolved profile for
/// a process that impersonates nothing. So a run with stealth ON and a run with
/// stealth OFF produced the SAME token and shared one cache entry.
///
/// Measured 2026-09-04 against a loopback server that echoes its request
/// headers: a `--no-stealth` scrape returned a body carrying
/// `Mozilla/5.0 ... Chrome/152.0.0.0` and the three `sec-ch-ua` hints, with
/// `cache_hit: true` and `stealth: false` in the same envelope. The request the
/// operator actually made sends
/// [`crate::constants::HTTP_USER_AGENT`] and no Client Hints at all; they were
/// reading a stored answer to a different question.
///
/// This is the same defect `crate::cache::CacheContext` documents for
/// `--proxy`, in a second field: the key described the route only when the
/// route was taken.
#[must_use]
pub fn stealth_cache_token() -> &'static str {
    if stealth_enabled() {
        stealth_profile().as_str()
    } else {
        "off"
    }
}

/// Whether the token came from argv, XDG, or the compiled default.
#[must_use]
pub fn stealth_profile_source() -> ProfileSource {
    ProfileSource::from_code(STEALTH_PROFILE_SOURCE.load(Ordering::Relaxed))
}

/// Publish the stealth identity seed. Called once from CLI dispatch.
pub fn set_stealth_seed(seed: Option<String>) {
    let _ = STEALTH_SEED.set(seed);
}

/// The seed pinning this process's stealth identity, when the caller set one.
///
/// `None` restores the historical behaviour: a fresh identity per process.
/// That is deliberately the default, because caching an identity writes it to
/// disk and a caller must opt into that.
#[must_use]
pub fn stealth_seed() -> Option<&'static str> {
    STEALTH_SEED.get().and_then(|v| v.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stealth_profile_accepts_both_windows_spellings() {
        assert_eq!(
            StealthProfile::parse("chrome-windows"),
            Some(StealthProfile::ChromeWindows)
        );
        assert_eq!(
            StealthProfile::parse("chrome-win"),
            Some(StealthProfile::ChromeWindows)
        );
    }

    #[test]
    fn auto_profile_resolves_to_the_host_never_stays_auto() {
        assert_ne!(StealthProfile::Auto.resolved(), StealthProfile::Auto);
    }

    #[test]
    fn explicit_profile_survives_resolution() {
        assert_eq!(
            StealthProfile::ChromeMac.resolved(),
            StealthProfile::ChromeMac
        );
    }

    #[test]
    fn stealth_is_on_before_anyone_publishes_a_value() {
        // The default has to be the safe one: a process that never calls
        // set_stealth still gets the patches.
        assert!(STEALTH.load(Ordering::Relaxed));
    }

    /// `None` is not `Some(false)`. A `--quick` run never launches a browser,
    /// and reporting "the patch failed" there would invent a failure.
    #[test]
    fn install_outcome_is_absent_until_a_launch_attempts_it() {
        assert_eq!(
            STEALTH_INSTALLED.load(Ordering::Relaxed),
            STEALTH_INSTALL_UNATTEMPTED
        );
    }
}
