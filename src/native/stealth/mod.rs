// SPDX-License-Identifier: MIT OR Apache-2.0
//! Anti-detection patches applied before the first navigation.
//!
//! # The problem this solves
//!
//! A headless Chrome driven over CDP announces itself in about ten places at
//! once. `navigator.webdriver` is defined where a real Chrome leaves it
//! `undefined`; `navigator.plugins` is empty where a real Chrome lists three;
//! `chrome.runtime` is missing; `window.outerHeight` reads `0`; WebGL renders
//! through a software rasteriser instead of the host GPU.
//!
//! None of those blocks a request on its own. A bot check sums them into a
//! score, and the score is what crosses a threshold. That is why this module
//! patches every marker it can rather than the loudest one: fixing `webdriver`
//! alone leaves the other nine paying into the same total.
//!
//! # Timing is the whole contract
//!
//! The patches run through `Page.addScriptToEvaluateOnNewDocument`, which
//! executes BEFORE any page script on every document. Applying them after a
//! navigation is worthless: the challenge script has already read the values it
//! came for, and the fix arrives to an audience that left.
//!
//! # What this does not do
//!
//! It does not touch TLS or HTTP/2. Impersonating a JA4 fingerprint needs
//! BoringSSL, which is C, and this product is pure Rust by policy. The envelope
//! says so rather than implying the transport is covered — see
//! `tls_impersonation` in the scrape envelope.

mod identity;
mod script;
mod seed_cache;

pub use identity::Identity;

use spider_fingerprint::configs::{AgentOs, Tier};
use spider_fingerprint::{EmulationConfiguration, Fingerprint};

use std::sync::OnceLock;

use crate::browser_policy;

/// Cached patch script, built at most once for the life of the process.
static SCRIPT: OnceLock<Option<String>> = OnceLock::new();

/// The pre-navigation script for this process, or `None` when stealth is off.
///
/// # Why this is cached rather than rebuilt
///
/// Not an optimisation — a correctness requirement. [`build_script`] draws a
/// fresh random identity on every call: measured across two consecutive calls
/// it produced a different GPU vendor (`Mesa`/`llvmpipe` against
/// `NVIDIA Corporation`/`GeForce GTX 1050`), a different `hardwareConcurrency`
/// (4 against 8), a different `deviceMemory` (4 against 8), a different
/// `history.length` and even a different Chrome build number.
///
/// Handing a different identity to each navigation is WORSE than an honest
/// one. A page that reads the GPU on load and again after a click sees the
/// hardware change underneath it, and no real machine does that. Caching pins
/// one identity for the whole one-shot, which is exactly the lifetime a real
/// browser session has.
///
/// # Cost
///
/// A few kilobytes, built once and handed to CDP once per page.
#[must_use]
pub fn script_for_process() -> Option<&'static str> {
    SCRIPT
        .get_or_init(|| {
            if !browser_policy::stealth_enabled() {
                return None;
            }
            let profile = browser_policy::stealth_profile();
            // With a seed, the identity is pinned ACROSS processes too. Without
            // one, N one-shot runs present N machines from one address, which
            // is a pattern no real user produces.
            let Some(seed) = browser_policy::stealth_seed() else {
                return build_script(&Identity::for_profile(profile));
            };
            if let Some(cached) = seed_cache::load(seed, profile.as_str()) {
                return Some(cached);
            }
            let built = build_script(&Identity::for_profile(profile))?;
            seed_cache::store(seed, profile.as_str(), &built);
            Some(built)
        })
        .as_deref()
}

/// The identity in force for this process, or `None` when stealth is off.
///
/// Cached for the same reason [`script_for_process`] is: a redrawn identity
/// mid-process would contradict the script already injected into the page.
fn identity_for_process() -> Option<&'static Identity> {
    static IDENTITY: OnceLock<Option<Identity>> = OnceLock::new();
    IDENTITY
        .get_or_init(|| {
            browser_policy::stealth_enabled()
                .then(|| Identity::for_profile(browser_policy::stealth_profile()))
        })
        .as_ref()
}

/// The User-Agent stealth actually puts on the wire, or `None` when it is off.
///
/// Callers that need to reason about what the server saw — `robots.txt`
/// matching above all — must ask this rather than the product's honest
/// identity string, which stealth does not send.
#[must_use]
pub fn wire_user_agent() -> Option<String> {
    identity_for_process().map(|id| id.user_agent.clone())
}

/// Chrome's request headers, in Chrome's order, or `None` when stealth is off.
///
/// # What this delivers and what it cannot
///
/// The VALUES are delivered. The ORDER is not, and the limit is structural
/// rather than an omission: `reqwest` writes headers by iterating a
/// `HeaderMap`, whose iteration order is a function of its hash state, not of
/// insertion. Preserving Chrome's order would mean replacing the HTTP client
/// with one built on BoringSSL, which is C, and this product is pure Rust by
/// policy.
///
/// Shipping the values while the order stays uncontrolled is still worth
/// doing — a request missing `sec-ch-ua` and the four `sec-fetch-*` entirely is
/// a far louder signal than one that has them in the wrong sequence. The
/// envelope reports `header_order_controlled: false` so the caller can tell
/// which half it got instead of inferring a guarantee that does not exist.
#[must_use]
pub fn wire_headers() -> Option<Vec<(&'static str, String)>> {
    identity_for_process().map(Identity::chrome_header_order)
}

/// Build the patch script for a given identity.
///
/// Separated from [`script_for_process`] so it is testable without publishing
/// process-global policy first.
#[must_use]
pub fn build_script(identity: &Identity) -> Option<String> {
    let mut config = EmulationConfiguration::setup_defaults(&identity.user_agent);
    // `NativeGPU` reports the real adapter instead of inventing one. A claimed
    // vendor the driver cannot back is worse than an honest one, and a headed
    // launch inside a virtual display already has a real GPU behind it.
    config.fingerprint = Fingerprint::NativeGPU;
    config.agent_os = identity.agent_os;

    // `BasicWithConsole` reads like "adds console spoofing" and means the
    // opposite: it is the tier that does NOT install `HIDE_CONSOLE`. Every
    // other Basic tier rewrites `console.log`, `warn`, `error`, `info`, `debug`,
    // `table` and `dir` into no-ops.
    //
    // That would have silently broken three shipped features at once:
    // `--capture-console` would record nothing, `console list` would return an
    // empty ring, and `assert console-empty` would pass on any page no matter
    // how loud. All three would look like the page was quiet.
    config.tier = Tier::BasicWithConsole;

    // The product answers dialogs through its own `dialog` command and reports
    // `data.dialog_settled`. Letting the patch script swallow them would make
    // that command observe a page nobody can drive, and the failure would be
    // silent: the dialog simply never appears.
    config.dismiss_dialogs = false;

    config.hardware_concurrency = true;
    config.user_agent_data = Some(true);

    // Both default to OFF in the crate and are opt-in. `deviceMemory` is one of
    // the enumerated headless markers, and the CDP marker cleanup removes the
    // `cdc_`/`$cdc_` globals that a CDP session leaves on `window` — which is a
    // DIRECT automation tell, not a probabilistic one.
    config.enable_device_memory = true;
    config.enable_cdp_marker_cleanup = true;

    // `setup_defaults` turns the plugin spoof OFF. An empty `navigator.plugins`
    // is one of the enumerated headless markers — a real Chrome reports three
    // entries — so it goes back on here.
    config.disable_plugins = false;

    // NOT patched here on purpose: `navigator.webdriver`. The crate documents
    // that the launch switch is the right lever ("You should not need this if
    // you use the cli args ... disabled-features=AutomationEnabled"), and
    // `build_chrome_args` passes `--disable-blink-features=AutomationControlled`
    // plus the matching `--disable-features` entry. A JS getter override is the
    // weaker fix: it leaves a patched `Function.prototype.toString` behind,
    // which is itself detectable. Killing the property at the source leaves
    // nothing to find.
    // The crate payload runs AFTER the product patches, so it observes a
    // browser that already looks unautomated. See `script::PRODUCT_PATCHES` for
    // the measurement that made this necessary: the launch switch alone left
    // `navigator.webdriver` defined and `false`.
    let emulated = spider_fingerprint::emulate(&identity.user_agent, &config, &None, &None)?;
    Some(format!("{};{emulated}", script::PRODUCT_PATCHES))
}

/// Whether the browser session should also override the User-Agent over CDP.
///
/// Only true for an explicitly foreign profile. See
/// [`Identity::overrides_browser_user_agent`].
#[must_use]
pub fn user_agent_override() -> Option<Identity> {
    if !browser_policy::stealth_enabled() {
        return None;
    }
    let profile = browser_policy::stealth_profile();
    // The launch mode is read here, once, and handed down. Deciding it inside
    // the identity would let two call sites disagree about whether this
    // process is headless.
    let headless = browser_policy::mode().launches_headless();
    if Identity::overrides_browser_user_agent(profile, headless) {
        Some(Identity::for_profile(profile))
    } else {
        None
    }
}

/// The `AgentOs` this process claims, for diagnostics.
#[must_use]
pub fn active_agent_os() -> AgentOs {
    Identity::for_profile(browser_policy::stealth_profile()).agent_os
}

/// Whether the stealth profile contradicts the host platform.
///
/// Reported rather than blocked: the caller may want a foreign identity for a
/// reason this product cannot see. But it is a mismatch the transport will
/// betray, so it is stated instead of hidden.
///
/// Deliberately does NOT consider the launch mode. Overriding the UA because
/// Chrome is headless keeps the host platform, so it is not a contradiction —
/// folding it in here would report a mismatch that the wire does not show, and
/// a false alarm trains the caller to ignore the field.
#[must_use]
pub fn profile_contradicts_host() -> bool {
    let profile = browser_policy::stealth_profile();
    let host = browser_policy::StealthProfile::Auto.resolved();
    browser_policy::stealth_enabled() && profile.resolved() != host
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser_policy::StealthProfile;

    #[test]
    fn the_script_covers_the_markers_js_owns() {
        let identity = Identity::for_profile(StealthProfile::ChromeLinux);
        let script = build_script(&identity).expect("emulation script");
        // These four can only be fixed from inside the page. `navigator.webdriver`
        // is deliberately NOT in this list: the launch switch removes it at the
        // source, and asserting the JS form here would pass while the flag
        // regressed.
        for marker in [
            "plugins",
            "deviceMemory",
            "hardwareConcurrency",
            "cdc_",
            "delete proto.webdriver",
        ] {
            assert!(
                script.contains(marker),
                "patch script never mentions {marker}"
            );
        }
    }

    #[test]
    fn the_script_is_not_empty_or_trivially_short() {
        let identity = Identity::for_profile(StealthProfile::ChromeLinux);
        let script = build_script(&identity).expect("emulation script");
        assert!(
            script.len() > 512,
            "suspiciously short patch script: {} bytes",
            script.len()
        );
    }

    #[test]
    fn build_script_is_deliberately_not_deterministic() {
        // Measured: two calls disagree on GPU vendor, hardwareConcurrency,
        // deviceMemory and the Chrome build. That is fine for the BUILDER and
        // fatal for the CALLER, which is precisely why `script_for_process`
        // caches. This test pins the hazard so the cache is never removed as
        // "an optimisation nobody needs".
        let identity = Identity::for_profile(StealthProfile::ChromeLinux);
        let runs: std::collections::HashSet<_> =
            (0..8).filter_map(|_| build_script(&identity)).collect();
        assert!(
            runs.len() > 1,
            "builder became deterministic; the caching rationale needs revisiting"
        );
    }

    #[test]
    fn the_process_script_is_stable_across_reads() {
        // One identity for the whole one-shot. A page that reads the GPU twice
        // must see the same answer both times.
        let first = script_for_process().map(str::to_string);
        let second = script_for_process().map(str::to_string);
        assert_eq!(first, second);
    }

    #[test]
    fn the_script_never_silences_the_page_console() {
        // `Tier::Basic` rewrites console.log into a no-op, which would make
        // --capture-console, `console list` and `assert console-empty` all
        // report a quiet page regardless of what the page actually logged.
        let identity = Identity::for_profile(StealthProfile::ChromeLinux);
        let script = build_script(&identity).expect("emulation script");
        assert!(
            !script.contains("console[method]=()=>{}"),
            "patch script silences console; --capture-console would go blind"
        );
    }
}
