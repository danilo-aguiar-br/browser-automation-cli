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

/// Profile tokens `--stealth-profile` accepts.
///
/// `list` is a clap discovery value (early-exit, no browser). It is not a
/// profile and does not belong in this array.
pub const STEALTH_PROFILE_TOKENS: &[&str] = &["auto", "chrome-linux", "chrome-win", "chrome-mac"];

/// Signal name: WebGL vendor and renderer.
pub const SIGNAL_GPU: &str = "gpu";
/// Signal name: `AudioContext.sampleRate`.
pub const SIGNAL_AUDIO_SAMPLE_RATE: &str = "audio.sampleRate";
/// Signal name: `document.fonts.size`.
pub const SIGNAL_FONT_COUNT: &str = "fonts.count";

/// Signals the installed anti-detection patch actually rewrites.
///
/// This answers "does the patch touch it", which is NOT the question
/// [`STEALTH_SEED_FIELDS`] answers. Keeping them apart matters: `userAgent`
/// appears in [`STEALTH_SEED_DOES_NOT_VARY`] and is spoofed regardless, so
/// deriving provenance from the seed list would mislabel it as host-backed.
///
/// Measured 2026-08-17 against `spider_fingerprint` 2.39.0 under
/// `Tier::BasicWithConsole`: `unified_worker_override` rewrites WebGL
/// `getParameter(37445/37446)`, and `spoof_gpu_adapter` rewrites the WebGPU
/// adapter. Audio and fonts are absent from the generated script — the audio
/// block lives inside `SPOOF_FINGERPRINT`, which `FP_JS_GPU_LINUX` replaces
/// with an empty string, and no font surface is touched anywhere in the crate.
pub const STEALTH_CONTROLLED_SIGNALS: &[&str] = &[
    SIGNAL_GPU,
    "userAgent",
    "navigator.platform",
    "navigator.webdriver",
];

/// True when the anti-detection patch rewrites `signal`.
#[must_use]
pub fn stealth_controls(signal: &str) -> bool {
    STEALTH_CONTROLLED_SIGNALS.contains(&signal)
}

/// Fields `--stealth-seed` actually pins. Measured: the crate redraws these
/// on every `emulate()` call, and the seed cache freezes the generated script.
///
/// "Pins" is memoisation, not determinism. The GPU profile is drawn by
/// `rand::rng()` on every call; the seed freezes the GENERATED SCRIPT on disk
/// under XDG state, so the same seed replays the same draw on the same machine
/// and may draw differently on another one.
pub const STEALTH_SEED_FIELDS: &[&str] = &[
    "hardwareConcurrency",
    "deviceMemory",
    "gpu.vendor",
    "gpu.renderer",
    "history.length",
    "chrome.build",
];

/// Fields `--stealth-seed` does **not** vary. They come from the profile or
/// the host, not from the seed.
pub const STEALTH_SEED_DOES_NOT_VARY: &[&str] = &[
    "userAgent",
    "navigator.platform",
    "navigator.languages",
    "timezone",
    "screen.width",
    "screen.height",
    "plugins.length",
];

/// Live fingerprint evidence recorte for `doctor --fingerprint`.
///
/// Canvas / WebGL / audio probes have been scored on Linux headless + Xvfb.
/// macOS and Windows compile the same types but have no live corpus here.
pub const FINGERPRINT_MEASUREMENT_SCOPE: &str = "linux-headless-xvfb";

/// Operating systems with no live fingerprint corpus in this product.
pub const FINGERPRINT_UNMEASURED_OS: &[&str] = &["macos", "windows"];

/// One-line envelope note. Types compile on every Tier-1 target; GPU live
/// scoring is Linux-only until another host records a corpus.
pub const FINGERPRINT_MEASUREMENT_NOTE: &str =
    "types compile on macos/windows; live Canvas/WebGL/audio scored only on linux-headless-xvfb";

/// Default shape of the temporal dispersion applied to every input delay.
///
/// # Why log-normal and not uniform
///
/// A detector measures the SECOND moment, because the first is trivial to
/// imitate. Scaling a base delay by a uniform factor produces the right mean and
/// a symmetric spread; measured 2026-08-31 on the final browser event, 20
/// characters under `human`, the skewness came out at 0.036. Human inter-key
/// intervals are asymmetric with a long right tail, skewness typically between 1
/// and 3. Zero skewness is not "an unusual human"; it is no human at all.
///
/// `normal` and `uniform` stay reachable as escape hatches for a caller who is
/// reproducing an older trace, not because either is a better model.
pub const DEFAULT_INPUT_TIMING_DISTRIBUTION: &str = "lognormal";

/// Distribution tokens `input_timing_distribution` accepts.
pub const INPUT_TIMING_DISTRIBUTION_TOKENS: &[&str] = &["lognormal", "normal", "uniform"];

/// Smallest dispersion any input delay may carry, as a fraction of its mean.
///
/// A DECLARED floor against variance zero. Zero dispersion is the single
/// strongest automation signal there is: a wrong mean reads as an unusual
/// human, and no variance reads as no human at all. Small enough that an
/// operator who deliberately wants a tight rhythm still gets one, and non-zero
/// so that "tight" can never become "constant".
pub const TIMING_MIN_DISPERSION_RATIO: f64 = 0.05;

/// Floor of a sampled delay, as a fraction of its mean.
///
/// A log-normal has unbounded support, so an unclamped draw can land arbitrarily
/// close to zero. A delay of one millisecond between two keys is not a fast
/// typist, it is a paste, and it reads as one.
pub const TIMING_SAMPLE_FLOOR_RATIO: f64 = 0.25;

/// Ceiling of a sampled delay, as a fraction of its mean.
///
/// The tail is the point of the model, so this is deliberately far out: four
/// means keeps the long pauses that make the shape human while refusing the
/// draw that would stall a one-shot process past its own timeout.
pub const TIMING_SAMPLE_CEILING_RATIO: f64 = 4.0;

/// Characters after which a typist may stop to think.
///
/// Sentence punctuation and the space that ends a word. The pause is what gives
/// the interval distribution its right tail; widening the fast rhythm alone
/// reproduces the width of human timing without its shape.
pub const WORD_BOUNDARY_PAUSE_CHARS: &[char] = &['.', ',', ';', ':', '!', '?', ' '];

/// Chance in a thousand that a word boundary earns a long pause.
///
/// # Why per mille and not a fraction
///
/// This is the ONE decision here that a later reader will mistake for a whim,
/// so the reason is recorded rather than left to be re-derived.
///
/// A probability is naturally a float in `[0, 1]`. It is an integer here
/// because the `policy_knobs!` macro in `src/xdg/policy/knobs/table.rs` is
/// `u64`-only, and one line in that macro generates all six coupled surfaces:
/// serde field, key constant, default, `config get`, `config set` and
/// `config list-keys`. A fractional key does not fit it and would instead be
/// hand-edited into SEVEN files -- `config_model.rs`, `config_io.rs`,
/// `config_ops/set.rs`, `config_ops/get_table.rs`, `config_ops/keys.rs`,
/// `config_ops/key_entries.rs` and `config_write.rs`.
///
/// One line against seven files, for the same amount of configuration. Per
/// mille buys the whole knob for a line and costs the operator one unit
/// conversion.
///
/// Do NOT "fix" this to a float for elegance: that reopens the seven files the
/// unit exists to avoid, and a probability that cannot be configured the same
/// way as its own magnitude ([`INPUT_WORD_PAUSE_MS`]) is a knob that only half
/// exists.
///
/// [`INPUT_WORD_PAUSE_MS`]: crate::constants::INPUT_WORD_PAUSE_MS
pub const INPUT_WORD_PAUSE_PERMILLE: u64 = 120;

/// Chance in a thousand that a character is typed wrong and then corrected.
///
/// Zero by DEFAULT, and the only humanisation knob in this file that is. Every
/// other one disperses TIMING, which no page can observe as a different value;
/// this one changes the CHARACTER STREAM, so a field with an `input` listener
/// sees the wrong prefix, sends it, and may autocomplete or navigate on it.
/// That is a side effect on the target site, and the caller has to ask for it.
///
/// The final text is always correct: the wrong key is followed by `Backspace`
/// and then by the intended character.
pub const INPUT_TYPO_PERMILLE: u64 = 0;

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
