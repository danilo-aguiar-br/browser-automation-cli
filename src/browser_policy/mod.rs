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
//! # Layout
//!
//! The module doc named three concerns from the first commit — window, stealth,
//! egress — and they lived in one file until 0.1.9, when the project's own
//! `scripts/filesize-check.sh` refused it at 313 code lines against a 300 line
//! limit. They are now one submodule each, re-exported here so every existing
//! `crate::browser_policy::…` path keeps resolving.
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

mod egress;
mod runtime_events;
mod source;
mod stealth;
mod window;

pub use egress::warmup_url;
pub use egress::{proxy, proxy_bypass, set_proxy, set_warmup, set_warmup_url, warmup_enabled};
pub use runtime_events::{runtime_events_needed, set_runtime_events_needed};
pub use source::PolicySource;
pub use stealth::{
    set_stealth, set_stealth_installed, set_stealth_profile, set_stealth_profile_source,
    set_stealth_seed, stealth_cache_token, stealth_enabled, stealth_installed, stealth_profile,
    stealth_profile_source, stealth_profile_token, stealth_seed, ProfileSource, StealthProfile,
};
pub use window::{mode, mode_source, no_xvfb, set_auto_headed, set_mode, set_no_xvfb, BrowserMode};

/// The CLI-side inputs this policy resolves. Plain data, so the resolver stays
/// testable without building a clap parse.
#[derive(Debug, Default, Clone)]
pub struct PolicyFlags {
    /// The mode named on argv by `--browser-mode`, `--headed` or `--headless`.
    ///
    /// `None` means argv said nothing about the window, which is exactly what
    /// lets the XDG step below run. This is deliberately NOT three booleans:
    /// the parser already refuses more than one of them, so three fields would
    /// let an impossible state be represented here and force every reader to
    /// re-decide the precedence for itself.
    pub browser_mode: Option<BrowserMode>,
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
    /// `--capture-console` was passed.
    ///
    /// The only argv input that gives this process a consumer for CDP
    /// `Runtime` events on the one-shot launch path. It lives here rather than
    /// being read at the launch site because the launch site is six call sites
    /// deep and knows nothing about capture flags.
    pub capture_console: bool,
}

/// Resolve the window mode and record which precedence step decided it.
///
/// # Why this is a free function and not inline in [`publish`]
///
/// [`publish`] reads the XDG file and writes five process globals, so nothing
/// about it can be asserted without a filesystem and a fresh process. That is
/// why its precedence had ZERO test coverage while being the most consequential
/// decision in the module: the shape made testing expensive enough to skip.
///
/// The rule itself is pure — a flag, an optional config token, a compiled
/// default — so it lives here where a test can state it directly.
///
/// # Precedence
///
/// Flag, then XDG, then the compiled default. The flag WINS unconditionally,
/// which is the property that makes `--headless` a usable guarantee: without
/// it, a `config set browser_mode headed` performed for an unrelated task would
/// still decide the run.
///
/// An unparsable XDG token falls through to the default rather than aborting,
/// matching [`BrowserMode::parse`] refusing to guess. The source reported in
/// that case is `Default`, because default is genuinely what was used.
fn resolve_mode(flag: Option<BrowserMode>, xdg: Option<&str>) -> (BrowserMode, PolicySource) {
    if let Some(m) = flag {
        return (m, PolicySource::Flag);
    }
    if let Some(m) = xdg.and_then(BrowserMode::parse) {
        return (m, PolicySource::Xdg);
    }
    (
        BrowserMode::parse(crate::constants::DEFAULT_BROWSER_MODE).unwrap_or(BrowserMode::Auto),
        PolicySource::Default,
    )
}

/// Publish the whole browser policy once, during CLI dispatch.
///
/// Precedence is flag, then XDG, then the compiled default, applied per field
/// rather than per struct: an operator who sets only `stealth_profile` must not
/// lose the `browser_mode` they set in the same file. The window mode's share of
/// that rule lives in `resolve_mode` below, where a test can state it
/// directly. The link is deliberately plain text: `resolve_mode` is private,
/// and a rustdoc link from a public item to a private one fails `cargo doc -D
/// warnings`, which is the third time that shape has broken this tree.
///
/// The XDG file is read ONCE here. Resolving each field independently would
/// re-read and re-parse the same TOML five times on a path that runs before any
/// work starts.
pub fn publish(flags: &PolicyFlags) {
    let cfg = crate::xdg::load_config().ok();

    let (mode, mode_source) = resolve_mode(
        flags.browser_mode,
        cfg.as_ref().and_then(|c| c.browser_mode.as_deref()),
    );
    set_mode(mode, mode_source);

    // `--no-stealth` is a one-way switch: the flag can only turn stealth off,
    // never on. There is no `--stealth`, because the default is already on and a
    // second spelling for "leave it alone" earns nothing.
    let stealth = !flags.no_stealth && cfg.as_ref().and_then(|c| c.stealth).unwrap_or(true);
    set_stealth(stealth);

    let (profile, source) = if let Some(p) = flags
        .stealth_profile
        .as_deref()
        .and_then(StealthProfile::parse)
    {
        (p, ProfileSource::Flag)
    } else if let Some(p) = cfg
        .as_ref()
        .and_then(|c| c.stealth_profile.as_deref())
        .and_then(StealthProfile::parse)
    {
        (p, ProfileSource::Xdg)
    } else {
        (StealthProfile::Auto, ProfileSource::Default)
    };
    set_stealth_profile(profile);
    set_stealth_profile_source(source);

    set_no_xvfb(flags.no_xvfb);
    // ORDER MATTERS: `set_auto_headed` decides what `auto` means on this host,
    // and every later reader of `launches_headless` — stealth setup, the doctor
    // probes, the envelope witness, the Xvfb gate in the launch path — depends
    // on it already being stored. Resolving it after the mode and the opt-out,
    // and before any of those run, is what makes the value a constant for the
    // rest of the process instead of a race.
    set_auto_headed(flags.no_xvfb);
    // `Runtime.enable` is a fingerprint, so it is issued only where something
    // reads the events it turns on. On the one-shot native path that is
    // `--capture-console` and nothing else; `record` enables the domain itself
    // because `Runtime.bindingCalled` is its own dependency, not this one.
    set_runtime_events_needed(flags.capture_console);
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

/// The resolved browser-window policy, as JSON, for an envelope to publish.
///
/// # Why this exists at all
///
/// `robots_policy` has been published by hand in ten sites since it was added,
/// and the browser mode in zero — even though both are decisions the binary
/// makes on its own that change what the run does in the world. One was
/// promoted to a contract field and the other stayed an implementation detail,
/// and the asymmetry was never technical.
///
/// The cost of that gap is specific: a caller whose requirement is "never paint
/// a window on the user's screen" has no post-hoc proof that it held. If a
/// regression, an inherited config or a new path launches headed, the envelope
/// comes out IDENTICAL and nobody notices until someone sees the window.
///
/// `browser_mode_source` is the most valuable of the three, because it
/// separates "headless because I asked" from "headless by luck of the default",
/// and only the first can be relied on.
///
/// # What this deliberately does NOT report
///
/// There is no `xvfb.display` here. The display number lives on `XvfbGuard`,
/// which is owned by the launch and dropped with it, so reporting it would mean
/// introducing global state for a string. `display_backend` answers the
/// question that actually gets asked — did this run have a real window — and
/// the number can be added later if a caller ever needs to attach to it.
#[must_use]
pub fn witness() -> serde_json::Value {
    let effective = mode();
    serde_json::json!({
        // What argv/XDG asked for, before resolution.
        "browser_mode_requested": effective.as_str(),
        // What the launch will actually do. These differ under `auto`, which is
        // exactly the case the caller cannot otherwise see.
        "browser_mode_effective": if effective.launches_headless() { "headless" } else { "headed" },
        // "default" | "xdg" | "flag" — the precedence step that won.
        "browser_mode_source": mode_source().as_str(),
        "display_backend": display_backend(),
        // Whether this launch issued `Runtime.enable`. Published because the
        // claim "the default path no longer enables Runtime" is otherwise only
        // prose, and prose has no gate. A caller comparing two runs can see the
        // domain appear the moment `--capture-console` is passed.
        "runtime_enable_used": runtime_events_needed(),
    })
}

/// Merge [`witness`] into an existing `data` object, in place.
///
/// A helper instead of a free-standing block because the alternative — pasting
/// four keys at each call site — is exactly how `robots_policy` ended up
/// published by hand in ten places, where a fifth key would now have to be
/// added ten times and could be forgotten in one.
///
/// Silently does nothing when `data` is not an object, because a command whose
/// payload is an array or a scalar has no room for a witness and refusing here
/// would fail a run over telemetry.
pub fn attach_witness(data: &mut serde_json::Value) {
    let Some(obj) = data.as_object_mut() else {
        return;
    };
    let serde_json::Value::Object(fields) = witness() else {
        return;
    };
    for (k, v) in fields {
        // A call site that already published its own value wins: this is a
        // default, not an override.
        obj.entry(k).or_insert(v);
    }
}

/// Remove the witness keys from a nested payload that should not carry them.
///
/// # Why a multi-step run needs this
///
/// [`attach_witness`] is called from `with_capture_fields`, which every session
/// result passes through. For a single command that result IS the envelope's
/// `data`, and one copy is right. For `run` it is EACH STEP's data, so the same
/// four process-global facts — facts that by construction cannot vary inside
/// one process — were repeated once per step.
///
/// Measured 2026-09-04 on the ten-step reference fixture: six copies, 786 of
/// 2631 bytes, roughly thirty percent of the envelope, while the top-level
/// `data` — where a consumer would look first — carried none of them. The byte
/// budget in `tests/envelope_shape_gate.rs` exists to catch exactly this shape
/// of duplication, and it caught it.
///
/// So `run` strips the copies and publishes one at the top. A caller that wants
/// to know what the browser did reads it once, in the place the question is
/// asked about the whole run.
pub fn strip_witness(data: &mut serde_json::Value) {
    let Some(obj) = data.as_object_mut() else {
        return;
    };
    for k in WITNESS_KEYS {
        obj.remove(*k);
    }
}

/// The keys [`witness`] publishes, named once so `strip_witness` cannot drift
/// from it. A fifth key added above and forgotten here would be stripped from
/// nothing and duplicated forever.
const WITNESS_KEYS: &[&str] = &[
    "browser_mode_requested",
    "browser_mode_effective",
    "browser_mode_source",
    "display_backend",
    "runtime_enable_used",
];

/// Which surface the browser draws onto, if any.
///
/// Not derived from `browser_mode` alone: `--headed` with a private virtual
/// display is NOT the user's screen, and conflating the two is what let the
/// original report accuse this product of painting windows it never painted.
fn display_backend() -> &'static str {
    if mode().launches_headless() {
        return "headless";
    }
    if no_xvfb() {
        // Headed with the virtual display refused: this is the one path that
        // can reach the operator's compositor.
        return "host";
    }
    "xvfb"
}

#[cfg(test)]
mod tests {
    use super::{resolve_mode, BrowserMode, PolicySource};

    /// The property `--headless` exists to provide.
    ///
    /// Before 0.1.9 there was no flag that could produce `Headless`, so an XDG
    /// `browser_mode = headed` set for an unrelated debugging task decided every
    /// automated run on the machine. A consumer with headless as a security
    /// requirement had no way to override it, and no way to detect that it had
    /// been overridden.
    #[test]
    fn a_flag_beats_a_conflicting_xdg_value() {
        let (mode, source) = resolve_mode(Some(BrowserMode::Headless), Some("headed"));
        assert_eq!(mode, BrowserMode::Headless);
        assert_eq!(source, PolicySource::Flag);
    }

    /// The reverse direction, so the test above cannot pass by always returning
    /// the flag's value regardless of which flag was passed.
    #[test]
    fn the_flag_that_wins_is_the_one_that_was_passed() {
        let (mode, source) = resolve_mode(Some(BrowserMode::Headed), Some("headless"));
        assert_eq!(mode, BrowserMode::Headed);
        assert_eq!(source, PolicySource::Flag);
    }

    #[test]
    fn xdg_decides_when_argv_says_nothing() {
        let (mode, source) = resolve_mode(None, Some("headed"));
        assert_eq!(mode, BrowserMode::Headed);
        assert_eq!(source, PolicySource::Xdg);
    }

    #[test]
    fn the_compiled_default_decides_when_nothing_else_does() {
        let (_, source) = resolve_mode(None, None);
        assert_eq!(source, PolicySource::Default);
    }

    /// An unreadable token must not be reported as an XDG decision.
    ///
    /// Saying `xdg` here would tell an agent the config file chose the mode when
    /// the config file was in fact ignored — a provenance that is worse than
    /// none, because it is confidently wrong.
    #[test]
    fn an_unparsable_xdg_token_reports_default_not_xdg() {
        let (mode, source) = resolve_mode(None, Some("headfull"));
        assert_eq!(source, PolicySource::Default);
        assert_eq!(
            mode,
            BrowserMode::parse(crate::constants::DEFAULT_BROWSER_MODE).expect("valid default")
        );
    }

    /// Every mode must be reachable from argv, which is the whole defect.
    #[test]
    fn all_three_modes_are_reachable_from_a_flag() {
        for m in [
            BrowserMode::Auto,
            BrowserMode::Headed,
            BrowserMode::Headless,
        ] {
            let (got, source) = resolve_mode(Some(m), Some("headed"));
            assert_eq!(got, m, "mode {m:?} is not reachable from argv");
            assert_eq!(source, PolicySource::Flag);
        }
    }
}
