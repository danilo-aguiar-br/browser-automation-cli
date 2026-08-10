// SPDX-License-Identifier: MIT OR Apache-2.0
//! Process-level browser policy: window mode, stealth, and egress.
//!
//! **Why this module exists.** Three global flags were declared on
//! [`crate::cli::GlobalOpts`] and read by nobody. `--headed` was one of them:
//! the one-shot session hard-coded `headless: true`, so
//! passing the flag changed the help text and nothing else. `no_xvfb` was
//! declared on [`crate::native::cdp::chrome::LaunchOptions`] and had zero read
//! sites anywhere in `src/` or `tests/`. A flag that parses and is then dropped
//! is worse than a missing flag, because the caller has no way to notice.
//!
//! The fix is the idiom the input layer already uses (see
//! [`crate::native::interaction::set_input_seed`]): publish once during CLI
//! dispatch into a process-global, read at the point of use. That works here
//! precisely because the product is one-shot — the process owns exactly one
//! browser lifetime, so there is no session to disambiguate.
//!
//! # Precedence
//!
//! Flag, then XDG config, then the compiled default. There are no product
//! environment variables: `CHROME_HEADLESS` and friends are deliberately not
//! read, because configuration belongs to `config set` and the XDG file.
//!
//! # Concurrency
//!
//! Every value is `Relaxed`. Nothing else is published through these atomics,
//! and they are written once before any browser launch runs.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::OnceLock;

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
    /// # Why `Auto` is still headless
    ///
    /// `Auto` is defined as "headed inside a private virtual display on Linux",
    /// and that definition needs the Xvfb machinery to be true. Until it lands,
    /// `Auto` resolving to a plain headed launch would put Chrome on the
    /// operator's OWN display and inherit its device pixel ratio.
    ///
    /// That is not speculative. Making the switch early regressed
    /// `tests/grab_envelope_gate.rs`: a `--full-page` capture came back 4500px
    /// tall against an expected 3000, exactly the host's 1.5 scaling factor.
    /// Every screenshot in the product would have silently changed size.
    ///
    /// So the window mode stays put until the display it needs exists. Stealth
    /// itself does NOT wait on this — the patches, the launch switches and the
    /// identity all apply to a headless launch too.
    #[must_use]
    pub fn launches_headless(self) -> bool {
        match self {
            Self::Headed => false,
            Self::Headless | Self::Auto => true,
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

static MODE: AtomicU8 = AtomicU8::new(0);
static STEALTH: AtomicBool = AtomicBool::new(true);
static STEALTH_PROFILE: AtomicU8 = AtomicU8::new(0);
static NO_XVFB: AtomicBool = AtomicBool::new(false);
static WARMUP: AtomicBool = AtomicBool::new(false);
static PROXY: OnceLock<Option<String>> = OnceLock::new();
static PROXY_BYPASS: OnceLock<Option<String>> = OnceLock::new();
static STEALTH_SEED: OnceLock<Option<String>> = OnceLock::new();
static WARMUP_URL: OnceLock<Option<String>> = OnceLock::new();

/// Publish the resolved window mode. Called once from CLI dispatch.
pub fn set_mode(mode: BrowserMode) {
    MODE.store(mode.code(), Ordering::Relaxed);
}

/// The window mode in force for this process.
#[must_use]
pub fn mode() -> BrowserMode {
    BrowserMode::from_code(MODE.load(Ordering::Relaxed))
}

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

/// Publish the resolved stealth identity. Called once from CLI dispatch.
pub fn set_stealth_profile(profile: StealthProfile) {
    STEALTH_PROFILE.store(profile.code(), Ordering::Relaxed);
}

/// The stealth identity in force, already resolved against the host.
#[must_use]
pub fn stealth_profile() -> StealthProfile {
    StealthProfile::from_code(STEALTH_PROFILE.load(Ordering::Relaxed)).resolved()
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

/// Publish the `--warmup` request. Called once from CLI dispatch.
pub fn set_warmup(enabled: bool) {
    WARMUP.store(enabled, Ordering::Relaxed);
}

/// Whether to visit the origin root before the target URL.
#[must_use]
pub fn warmup_enabled() -> bool {
    WARMUP.load(Ordering::Relaxed)
}

/// Publish the `--warmup-url` override. Called once from CLI dispatch.
pub fn set_warmup_url(url: Option<String>) {
    let _ = WARMUP_URL.set(url);
}

/// The URL to warm instead of the target's origin root.
///
/// The default warm-up visits the root because that is where a browser lands.
/// Some edges hand out the session somewhere else — a login page, a locale
/// splash, a region redirector — and warming the root there buys a cookie the
/// target does not accept. This lets the caller name the real entry point
/// instead of giving up on the warm-up entirely.
#[must_use]
pub fn warmup_url() -> Option<&'static str> {
    WARMUP_URL.get().and_then(|v| v.as_deref())
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

/// Publish the resolved egress proxy. Called once from CLI dispatch.
///
/// Repeat calls are ignored, which keeps the value immutable for the rest of
/// the process rather than letting a later caller redirect egress silently.
pub fn set_proxy(url: Option<String>, bypass: Option<String>) {
    let _ = PROXY.set(url);
    let _ = PROXY_BYPASS.set(bypass);
}

/// The egress proxy URL for both Chrome and the HTTP engine.
#[must_use]
pub fn proxy() -> Option<&'static str> {
    PROXY.get().and_then(|v| v.as_deref())
}

/// Hosts that bypass the proxy, in Chrome's bypass-list syntax.
#[must_use]
pub fn proxy_bypass() -> Option<&'static str> {
    PROXY_BYPASS.get().and_then(|v| v.as_deref())
}

/// The CLI-side inputs this policy resolves. Plain data, so the resolver stays
/// testable without building a clap parse.
#[derive(Debug, Default, Clone)]
pub struct PolicyFlags {
    /// `--headed` was passed.
    pub headed: bool,
    /// `--no-xvfb` was passed.
    pub no_xvfb: bool,
    /// `--no-stealth` was passed.
    pub no_stealth: bool,
    /// `--stealth-profile <token>`, already restricted by `value_parser`.
    pub stealth_profile: Option<String>,
    /// `--warmup` was passed.
    pub warmup: bool,
    /// `--warmup-url` value, which implies `--warmup`.
    pub warmup_url: Option<String>,
    /// `--proxy <url>`.
    pub proxy: Option<String>,
    /// `--proxy-bypass <hosts>`.
    pub proxy_bypass: Option<String>,
    /// `--stealth-seed <seed>`.
    pub stealth_seed: Option<String>,
}

/// Publish the whole browser policy once, during CLI dispatch.
///
/// Precedence is flag, then XDG, then the compiled default, applied per field
/// rather than per struct: an operator who sets only `stealth_profile` must not
/// lose the `browser_mode` they set in the same file.
///
/// The XDG file is read ONCE here. Resolving each field independently would
/// re-read and re-parse the same TOML five times on a path that runs before any
/// work starts.
pub fn publish(flags: &PolicyFlags) {
    let cfg = crate::xdg::load_config().ok();

    let mode = if flags.headed {
        BrowserMode::Headed
    } else {
        cfg.as_ref()
            .and_then(|c| c.browser_mode.as_deref())
            .and_then(BrowserMode::parse)
            .unwrap_or_else(|| {
                BrowserMode::parse(crate::constants::DEFAULT_BROWSER_MODE)
                    .unwrap_or(BrowserMode::Auto)
            })
    };
    set_mode(mode);

    // `--no-stealth` is a one-way switch: the flag can only turn stealth off,
    // never on. There is no `--stealth`, because the default is already on and a
    // second spelling for "leave it alone" earns nothing.
    let stealth = !flags.no_stealth && cfg.as_ref().and_then(|c| c.stealth).unwrap_or(true);
    set_stealth(stealth);

    let profile = flags
        .stealth_profile
        .as_deref()
        .and_then(StealthProfile::parse)
        .or_else(|| {
            cfg.as_ref()
                .and_then(|c| c.stealth_profile.as_deref())
                .and_then(StealthProfile::parse)
        })
        .unwrap_or(StealthProfile::Auto);
    set_stealth_profile(profile);

    set_no_xvfb(flags.no_xvfb);
    let warmup_url = flags
        .warmup_url
        .clone()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    // Naming a warm-up URL IS asking for a warm-up. Requiring both flags would
    // let `--warmup-url` parse and then do nothing, which is the class of dead
    // flag this module was written to remove.
    set_warmup(flags.warmup || warmup_url.is_some());
    set_warmup_url(warmup_url);

    set_stealth_seed(
        flags
            .stealth_seed
            .clone()
            .or_else(|| cfg.as_ref().and_then(|c| c.stealth_seed.clone()))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    );

    set_proxy(
        flags
            .proxy
            .clone()
            .or_else(|| cfg.as_ref().and_then(|c| c.proxy_url.clone())),
        flags
            .proxy_bypass
            .clone()
            .or_else(|| cfg.as_ref().and_then(|c| c.proxy_bypass.clone())),
    );
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
}
