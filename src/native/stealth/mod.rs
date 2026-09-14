// SPDX-License-Identifier: MIT OR Apache-2.0
//! Anti-detection patches applied before the first navigation.
//!
//! # The problem this solves
//!
//! A headless Chrome driven over CDP announces itself in about ten places at
//! once. `navigator.webdriver` is defined where a real Chrome leaves it
//! `false`; `navigator.plugins` is empty where a real Chrome lists three;
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

mod chrome_geometry;
mod coherence;
mod identity;
mod screen;
mod script;
mod seed_cache;
mod signal_source;
mod webgl;

pub use coherence::{
    agent_os_from_ua, assess_signals, brands_vs_user_agent, bug01_deleted_webdriver,
    bug02_windows_ua_linux_platform, planned_stealth_signals, planned_vs_live, signals_from_live,
    ua_chrome_major, ua_contradicts_profile, CoherenceMismatch, FingerprintSignals,
};
pub use identity::{chrome_major_from_product, chrome_major_from_version_line, Identity};
pub use screen::{
    current_screen_override, current_screen_source, device_metrics_override, parse_screen_spec,
    resolve_screen, resolved_screen_source, set_screen_override, ScreenSource,
};
pub use signal_source::{signal_sources, SignalSource, SignalSources};

use spider_fingerprint::configs::Tier;
use spider_fingerprint::{EmulationConfiguration, Fingerprint};

use std::sync::OnceLock;

use crate::browser_policy;

/// Cached patch script, built at most once for the life of the process.
static SCRIPT: OnceLock<Option<String>> = OnceLock::new();

/// The pre-navigation script for this process, or `None` when stealth is off.
///
/// # Why this is cached rather than rebuilt
///
/// Not an optimisation — a correctness requirement. Building the script draws a
/// fresh random identity on every call: measured across two consecutive calls
/// it produced a different GPU vendor (`Mesa`/`llvmpipe` against
/// `NVIDIA Corporation`/`GeForce GTX 1050`), a different `hardwareConcurrency`
/// (4 against 8), a different `deviceMemory` (4 against 8), a different
/// `history.length`.
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
            // Built from THIS launch's `Browser.getVersion` reply rather than
            // from the identity the HTTP client may already have pinned: the
            // brands must match the User-Agent the page is about to show. See
            // [`identity_for_process`] for the one run where the two can differ.
            let profile = browser_policy::stealth_profile();
            let (identity, source) = &process_identity(
                browser_policy::stealth_enabled(),
                profile,
                browser_policy::mode().launches_headless(),
                || known_host_major(launched_chrome_major(), cached_host_major),
            )?;
            // After the stealth gate, and a no-op without a seed: the disk
            // contract in `seed_cache` applies to the major as well.
            persist_launched_major();
            let _ = PAGE_MAJOR_SOURCE.set(*source);
            // With a seed, the identity is pinned ACROSS processes too. Without
            // one, N one-shot runs present N machines from one address, which
            // is a pattern no real user produces.
            let Some(seed) = browser_policy::stealth_seed() else {
                return build_script(identity);
            };
            // A script re-centred on the host major carries that major, so it
            // must not be served to a launch that projects the crate table (or a
            // different host binary). The `-native` suffix retires every script
            // cached under the bare profile, which still emulated `userAgentData`.
            let cache_key = match source {
                PageMajorSource::HostBinary => format!(
                    "{}-{}",
                    profile.as_str(),
                    ua_chrome_major(&identity.user_agent).unwrap_or_default()
                ),
                PageMajorSource::HostUnprobed | PageMajorSource::Projected => {
                    format!("{}-native", profile.as_str())
                }
            };
            if let Some(cached) = seed_cache::load(seed, &cache_key) {
                return Some(cached);
            }
            let built = build_script(identity)?;
            seed_cache::store(seed, &cache_key, &built);
            Some(built)
        })
        .as_deref()
}

/// The identity in force for this process, or `None` when stealth is off.
///
/// Cached for the same reason [`script_for_process`] is: a redrawn identity
/// mid-process would contradict the script already injected into the page.
///
/// Derived from [`page_identity`], so the HTTP client announces the same major
/// the page shows. Measured before this did: headed on the host profile, the
/// page said `Chrome/152` while `user-agent` and `sec-ch-ua` on the wire said
/// 153. The launch mode is read once here.
///
/// # No `--version` probe on this path
///
/// The host major comes from this process's launch reply when a browser already
/// started, else from the major the last launch of the same binary stored, else
/// the crate table stands in as [`PageMajorSource::HostUnprobed`]. Spawning
/// `chrome --version` here cost 0.23 s on every HTTP-only command and broke the
/// "never on the hot launch path" contract of `probe_binary_version`.
///
/// # The one run where page and wire can differ
///
/// A process that sends HTTP BEFORE launching Chrome pins this identity from
/// the stored major. The first such run after a Chrome upgrade therefore sends
/// the old major while the page shows the new one; the launch in that run
/// stores the new major, so the next process agrees again.
fn identity_for_process() -> Option<&'static (Identity, PageMajorSource)> {
    static IDENTITY: OnceLock<Option<(Identity, PageMajorSource)>> = OnceLock::new();
    IDENTITY
        .get_or_init(|| {
            process_identity(
                browser_policy::stealth_enabled(),
                browser_policy::stealth_profile(),
                browser_policy::mode().launches_headless(),
                || known_host_major(launched_chrome_major(), cached_host_major),
            )
        })
        .as_ref()
}

/// Major reported by this process's own Chrome launch, once the bridge saw it.
static LAUNCHED_MAJOR: OnceLock<String> = OnceLock::new();

/// Record the major from the `Browser.getVersion` readiness reply.
///
/// Called by the DevTools pipe bridge, which already receives that reply; no
/// process and no extra CDP command is spent to learn it. The first launch in a
/// process wins, which is the binary every later launch in it also uses.
pub fn record_launched_chrome_major(major: String) {
    let _ = LAUNCHED_MAJOR.set(major);
}

fn launched_chrome_major() -> Option<&'static str> {
    LAUNCHED_MAJOR.get().map(String::as_str)
}

/// Whether the host major may be read from or written to disk at all.
///
/// `seed_cache` states the product's disk contract: nothing is written unless
/// `--stealth-seed` (or XDG `stealth_seed`) asks for cross-process stability,
/// and the README promises a run leaves nothing behind. The major is part of
/// that same stability, so it follows the same opt-in — and never under
/// `--no-stealth`, where no identity is projected.
fn host_major_disk_allowed(stealth: bool, seed: Option<&str>) -> bool {
    stealth && seed.is_some()
}

/// Major the last launch of the binary this host would start stored on disk.
fn cached_host_major() -> Option<String> {
    cached_host_major_from(
        host_major_disk_allowed(
            browser_policy::stealth_enabled(),
            browser_policy::stealth_seed(),
        ),
        seed_cache::host_major_dir(),
        crate::native::cdp::chrome::find_chrome,
    )
}

/// Testable half of [`cached_host_major`].
///
/// `find_chrome` runs only when disk is allowed: it walks config and known
/// paths and can print a raw warning, which an HTTP-only command without a
/// seed has no reason to trigger.
fn cached_host_major_from(
    allowed: bool,
    dir: Option<std::path::PathBuf>,
    find_chrome: impl FnOnce() -> Option<std::path::PathBuf>,
) -> Option<String> {
    if !allowed {
        return None;
    }
    seed_cache::load_host_major(&dir?, &find_chrome()?)
}

/// Store this process's launch major for the next HTTP-only process.
fn persist_launched_major() {
    persist_major_to(
        host_major_disk_allowed(
            browser_policy::stealth_enabled(),
            browser_policy::stealth_seed(),
        ),
        seed_cache::host_major_dir(),
        launched_chrome_major(),
        crate::native::cdp::chrome::find_chrome,
    );
}

/// Testable half of [`persist_launched_major`].
fn persist_major_to(
    allowed: bool,
    dir: Option<std::path::PathBuf>,
    launched: Option<&str>,
    find_chrome: impl FnOnce() -> Option<std::path::PathBuf>,
) {
    if !allowed {
        return;
    }
    if let (Some(dir), Some(major), Some(chrome)) = (dir, launched, find_chrome()) {
        seed_cache::store_host_major(&dir, &chrome, major);
    }
}

/// Pure half of [`identity_for_process`]: `None` when stealth is off.
fn process_identity(
    stealth: bool,
    profile: browser_policy::StealthProfile,
    headless: bool,
    host_major: impl FnOnce() -> Option<String>,
) -> Option<(Identity, PageMajorSource)> {
    stealth.then(|| page_identity(profile, headless, host_major))
}

/// Host major without spawning: this launch's reply first, the stored one next.
fn known_host_major(
    launched: Option<&str>,
    load_cached: impl FnOnce() -> Option<String>,
) -> Option<String> {
    launched.map(str::to_string).or_else(load_cached)
}

/// The User-Agent stealth actually puts on the wire, or `None` when it is off.
///
/// Callers that need to reason about what the server saw — `robots.txt`
/// matching above all — must ask this rather than the product's honest
/// identity string, which stealth does not send.
#[must_use]
pub fn wire_user_agent() -> Option<String> {
    identity_for_process().map(|(id, _)| id.user_agent.clone())
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
    identity_for_process().map(|(id, _)| id.chrome_header_order())
}

/// Build the patch script for a given identity.
///
/// Separated from [`script_for_process`] so it is testable without publishing
/// process-global policy first.
#[must_use]
pub(crate) fn build_script(identity: &Identity) -> Option<String> {
    let mut config = EmulationConfiguration::setup_defaults(&identity.user_agent);
    // `NativeGPU` selects which CANVAS payload to install; it does NOT hand the
    // real adapter to WebGL, and the comment that used to sit here said it did.
    //
    // Measured 2026-08-17 against spider_fingerprint 2.39.0: the WebGL lie rides
    // on the TIER, not on this enum. `Tier::BasicWithConsole` (chosen below)
    // expands to `{chrome};{worker};{concurrency};{gpu_adapter};{NAVIGATOR}`,
    // and `worker` is `unified_worker_override(.., webgl_patch = true)`, which
    // rewrites `getParameter(37445/37446)` on the main window. So the reported
    // vendor and renderer come from a table in the crate, drawn at random.
    //
    // The spoof stays, and the reason is the opposite of the old comment's:
    // `GpuProfile` is an ATOMIC bundle. One row carries webgl vendor, webgl
    // renderer, webgpu vendor, webgpu architecture, canvas format AND
    // `hardware_concurrency` together — a GTX 1050 ships 8 cores, an RX 580
    // ships 12. Detectors cross-check exactly that pair ("improbable
    // configuration": strong GPU, few cores). Switching to a NoWebgl tier would
    // report the honest SwiftShader renderer while `spoof_concurrency` kept
    // announcing a core count drawn from the GPU we stopped reporting, which
    // desynchronises the correlation the crate deliberately built.
    //
    // What the product owes the caller is not an honest GPU; it is an honest
    // LABEL. `doctor --fingerprint` reports `gpu_source: "stealth"`, derived
    // through `signal_source::signal_sources`, so nobody has to read this
    // comment to learn the value is synthetic.
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
    // Never emulated: the page's own `navigator.userAgentData` is the one that
    // agrees with the `sec-ch-ua*` headers Chrome actually sends.
    //
    // Without an override that object is the browser's. Measured headed on the
    // host profile, the emulated one said `Chromium/152, Google Chrome/152,
    // Not-A.Brand/8` while the real header said `"Not?A_Brand";v="24",
    // "Chromium";v="152"`. With an override Chrome builds it from the CDP
    // `userAgentMetadata` (see `user_agent_override_params`); measured headless,
    // the crate object on top swapped the header's GREASE for `Not-A.Brand/8`
    // and drew a random build per process.
    config.user_agent_data = Some(false);

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

    // `navigator.webdriver` is defined on Navigator.prototype in a real Chrome
    // and the value is the boolean `false`. Deleting it makes
    // `'webdriver' in navigator` return false — a stronger tell than `false`.
    // The product patch keeps the property and pins the value; the crate
    // payload runs after that, then `platform_patch` re-applies
    // `navigator.platform` so a foreign profile cannot leave the host token.
    let emulated = spider_fingerprint::emulate(&identity.user_agent, &config, &None, &None)?;
    // spider_fingerprint 2.39 Linux canvas noisify calls the *wrapped*
    // `getImageData`, which re-enters noisify until the stack blows. Mac/Win
    // already use the saved original. Fix the payload, then restore natives
    // so a future crate regression cannot take toDataURL down with it.
    let emulated = script::sanitize_crate_canvas(&emulated);
    let patches = script::product_patches(identity.navigator_platform);
    let platform = script::platform_patch(identity.navigator_platform);
    let canvas = script::canvas_restore_native();
    // Runs LAST so it wraps whatever the crate installed: it reads the crate's
    // own spoofed vendor/renderer back, re-spells them the way Chrome does on
    // this platform, and carries the resolved pair into every worker scope the
    // crate's own `Worker` wrapper skips. See `webgl::coherence_patch` for the
    // 10-launch measurement that showed the leak was in workers, not in the
    // window, and never intermittent.
    let webgl = webgl::coherence_patch(identity.navigator_platform);
    // LAST, so it owns `outerWidth`, `screen.*` and `maxTouchPoints` outright:
    // the crate payload defines the same getters, and the loser of that race
    // would be whichever ran first. Seeded by the already-drawn payload, so the
    // geometry is pinned to this identity by the caches that already exist —
    // `OnceLock` within the process, `seed_cache` across processes.
    let geometry = chrome_geometry::geometry_patch();
    Some(format!(
        "{patches};{emulated};{platform};{canvas};{webgl};{geometry}"
    ))
}

/// Whether the browser session should also override the User-Agent over CDP.
///
/// True for a foreign profile or a headless launch. See
/// [`Identity::overrides_browser_user_agent`] and [`page_identity`].
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

/// Where the Chrome major of [`page_identity`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMajorSource {
    /// The User-Agent is overridden, so the crate table IS the identity.
    Projected,
    /// Chrome keeps its own User-Agent and the host binary named its major:
    /// its `--version` (doctor), this launch's `Browser.getVersion` reply, or
    /// the reply a seeded earlier launch stored.
    HostBinary,
    /// Chrome keeps its own User-Agent but no major was known without a probe,
    /// so the crate table stands in as a declared fallback.
    HostUnprobed,
}

impl PageMajorSource {
    /// Stable envelope token for `user_agent_major_source`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Projected => "projected",
            Self::HostBinary => "host_binary",
            Self::HostUnprobed => "host_unprobed",
        }
    }
}

/// Where the major the HTTP client announces came from, `None` without stealth.
#[must_use]
pub fn wire_major_source() -> Option<PageMajorSource> {
    identity_for_process().map(|(_, source)| *source)
}

/// Where the major of the page script came from, `None` before a launch built
/// it or without stealth.
#[must_use]
pub fn page_major_source() -> Option<PageMajorSource> {
    PAGE_MAJOR_SOURCE.get().copied()
}

/// Recorded by [`script_for_process`] for [`page_major_source`].
static PAGE_MAJOR_SOURCE: OnceLock<PageMajorSource> = OnceLock::new();

/// The identity the PAGE presents under stealth for `profile` in this mode.
///
/// When [`Identity::overrides_browser_user_agent`] is false, Chrome keeps its
/// own User-Agent and the page shows the HOST binary's major, not the crate
/// table's. Measured headed on the host profile: `navigator.userAgent`
/// `Chrome/152.0.0.0` while the plan said 153 and the patch script published
/// brands `v="153"`. Both the plan and the script derive from this function so
/// they cannot disagree with each other or with the page.
///
/// `host_major` is only called on the non-override path, so a launch that
/// projects the crate table never pays for a `--version` probe.
pub fn page_identity(
    profile: browser_policy::StealthProfile,
    headless: bool,
    host_major: impl FnOnce() -> Option<String>,
) -> (Identity, PageMajorSource) {
    let identity = Identity::for_profile(profile);
    if Identity::overrides_browser_user_agent(profile, headless) {
        return (identity, PageMajorSource::Projected);
    }
    match host_major() {
        Some(major) => (identity.with_major(&major), PageMajorSource::HostBinary),
        None => (identity, PageMajorSource::HostUnprobed),
    }
}

/// Chrome major version of the binary this host would launch, if it answers.
///
/// Spawns `chrome --version`, so it is for `doctor` only, which describes the
/// truth and may pay for it. Hot paths use `known_host_major` instead — plain
/// text rather than a link, because that helper is private and a rustdoc link
/// from a public item to a private one fails `cargo doc -D warnings`.
#[must_use]
pub fn host_chrome_major() -> Option<String> {
    let path = crate::native::cdp::chrome::find_chrome()?;
    let line = crate::platform::probe_binary_version(&path)?;
    chrome_major_from_version_line(&line)
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
    fn host_major_touches_disk_only_with_a_seed_and_stealth_on() {
        assert!(host_major_disk_allowed(true, Some("s")));
        assert!(!host_major_disk_allowed(true, None), "no seed, no disk");
        assert!(
            !host_major_disk_allowed(false, Some("s")),
            "--no-stealth, no disk"
        );

        let dir = tempfile::tempdir().expect("tempdir");
        let chrome = || Some(dir.path().join("chrome"));
        let empty = |d: &std::path::Path| std::fs::read_dir(d).map_or(0, Iterator::count) == 0;

        // Without a seed (or with stealth off) a real launch writes nothing,
        // and the HTTP path does not even look for Chrome.
        persist_major_to(
            host_major_disk_allowed(true, None),
            Some(dir.path().to_path_buf()),
            Some("152"),
            chrome,
        );
        persist_major_to(
            host_major_disk_allowed(false, Some("s")),
            Some(dir.path().to_path_buf()),
            Some("152"),
            chrome,
        );
        assert!(empty(dir.path()), "a launch without a seed wrote to disk");
        assert_eq!(
            cached_host_major_from(false, Some(dir.path().to_path_buf()), || {
                panic!("find_chrome must not run without a seed")
            }),
            None
        );

        // With a seed the launch major survives to the next process.
        persist_major_to(true, Some(dir.path().to_path_buf()), Some("152"), chrome);
        assert_eq!(
            cached_host_major_from(true, Some(dir.path().to_path_buf()), chrome).as_deref(),
            Some("152")
        );
    }

    #[test]
    fn the_script_never_emulates_user_agent_data() {
        // Without an override the browser's own object matches its headers;
        // with one, the CDP `userAgentMetadata` builds it. Either way the crate
        // object on top contradicted `sec-ch-ua` (measured headed and headless).
        for profile in [
            StealthProfile::ChromeLinux,
            StealthProfile::ChromeWindows,
            StealthProfile::ChromeMac,
        ] {
            let script = build_script(&Identity::for_profile(profile)).expect("emulation script");
            assert!(
                !script.contains("fullVersionList"),
                "{profile:?}: the script still emulates userAgentData"
            );
        }
    }

    #[test]
    fn http_headed_identity_never_spawns_a_version_probe() {
        // No launch and no cache: the crate table stands in, declared as such.
        // Before this, the wire path ran `chrome --version` (0.23 s measured)
        // and answered the host major instead.
        let host = StealthProfile::Auto.resolved();
        assert_eq!(known_host_major(None, || None), None);
        let (id, source) = process_identity(true, host, false, || known_host_major(None, || None))
            .expect("stealth identity");
        assert_eq!(source, PageMajorSource::HostUnprobed);
        assert_eq!(id.user_agent, Identity::for_profile(host).user_agent);
    }

    #[test]
    fn cached_host_major_is_the_one_sent_in_the_headers() {
        let host = StealthProfile::Auto.resolved();
        let (id, source) = process_identity(true, host, false, || {
            known_host_major(None, || Some("151".to_string()))
        })
        .expect("stealth identity");
        assert_eq!(source, PageMajorSource::HostBinary);
        let headers = id.chrome_header_order();
        assert!(
            headers
                .iter()
                .any(|(n, v)| *n == "user-agent" && v.contains("Chrome/151.0.0.0")),
            "{headers:?}"
        );
        assert!(
            headers
                .iter()
                .any(|(n, v)| *n == "sec-ch-ua" && v.contains("v=\"151\"")),
            "{headers:?}"
        );
        // A launch reply in this process outranks the file.
        assert_eq!(
            known_host_major(Some("150"), || panic!("cache must not be read")),
            Some("150".to_string())
        );
    }

    #[test]
    fn wire_identity_carries_the_page_major_in_user_agent_and_sec_ch_ua() {
        let host = StealthProfile::Auto.resolved();
        assert!(process_identity(false, host, false, || panic!("no probe")).is_none());

        // Headed host: the page shows the host binary, so must the HTTP client.
        let (id, _) = process_identity(true, host, false, || Some("151".to_string()))
            .expect("stealth identity");
        let headers = id.chrome_header_order();
        let header = |name: &str| {
            headers
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        assert!(
            header("user-agent").contains("Chrome/151.0.0.0"),
            "{headers:?}"
        );
        assert!(header("sec-ch-ua").contains("v=\"151\""), "{headers:?}");

        // Headless is overridden: the crate table stands and nothing is probed.
        let (id, _) =
            process_identity(true, host, true, || panic!("no probe")).expect("stealth identity");
        assert_eq!(id.user_agent, Identity::for_profile(host).user_agent);
    }

    #[test]
    fn page_identity_follows_the_host_major_only_without_override() {
        let host = StealthProfile::Auto.resolved();
        let table = Identity::for_profile(host);

        // Headless overrides the UA: the crate table stands, nothing is probed.
        let (id, source) = page_identity(host, true, || panic!("probe must not run"));
        assert_eq!(source, PageMajorSource::Projected);
        assert_eq!(id.user_agent, table.user_agent);

        // Headed host keeps Chrome's own UA: plan and brands follow the binary.
        let (id, source) = page_identity(host, false, || Some("151".to_string()));
        assert_eq!(source, PageMajorSource::HostBinary);
        assert!(
            id.user_agent.contains("Chrome/151.0.0.0"),
            "{}",
            id.user_agent
        );
        assert!(id.brands.contains("v=\"151\""), "{}", id.brands);

        // A silent binary falls back to the table and says so.
        let (id, source) = page_identity(host, false, || None);
        assert_eq!(source, PageMajorSource::HostUnprobed);
        assert_eq!(id.user_agent, table.user_agent);
    }

    #[test]
    fn the_script_covers_the_markers_js_owns() {
        let identity = Identity::for_profile(StealthProfile::ChromeLinux);
        let script = build_script(&identity).expect("emulation script");
        // These can only be fixed from inside the page. webdriver stays
        // present (getter returns false); deleting it is the tell.
        for marker in [
            "plugins",
            "deviceMemory",
            "hardwareConcurrency",
            "cdc_",
            "return false",
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
        // deviceMemory and history.length. That is fine for the BUILDER and
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

    #[test]
    fn the_linux_canvas_wrap_does_not_recurse_through_getimagedata() {
        let identity = Identity::for_profile(StealthProfile::ChromeLinux);
        let script = build_script(&identity).expect("emulation script");
        assert!(
            !script.contains("t.getImageData(0,0"),
            "crate Linux canvas wrap still calls the wrapped getImageData"
        );
        assert!(
            script.contains("toDataURL"),
            "canvas restore must reinstall toDataURL"
        );
    }
}
