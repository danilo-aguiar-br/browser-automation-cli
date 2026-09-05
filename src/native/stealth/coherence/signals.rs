// SPDX-License-Identifier: MIT OR Apache-2.0
//! Where a [`FingerprintSignals`] comes FROM, as opposed to what it is compared against.
//!
//! # The seam
//!
//! [`super`] and [`super::drift`] both consume a reading and answer with
//! mismatches; neither of them may care how it was obtained. This file is the
//! other side: three producers, each with a different source of truth — the
//! page's own eval, the identity table this process planned, and two historical
//! measurements kept as named fixtures.
//!
//! Keeping them apart is what lets the comparisons stay pure. Every producer
//! here reaches for something outside the struct — `serde_json`, the crate
//! identity table, the Xvfb constants — and a comparison that grew such a
//! dependency would stop being unit-testable without a browser, which is the
//! one property that makes `doctor --fingerprint` and the `emulate` gate able
//! to share a definition of "coherent" at all.

use crate::browser_policy::StealthProfile;

use super::super::Identity;
use super::FingerprintSignals;

/// Parse the page-side object `doctor --fingerprint` evals.
#[must_use]
pub fn signals_from_live(v: &serde_json::Value) -> Option<FingerprintSignals> {
    Some(FingerprintSignals {
        webdriver_in_navigator: v.get("webdriver_in_navigator")?.as_bool()?,
        webdriver_in_prototype: v.get("webdriver_in_prototype")?.as_bool()?,
        webdriver_value: match v.get("webdriver_value") {
            Some(serde_json::Value::Null) | None => None,
            Some(x) => Some(x.as_bool()?),
        },
        user_agent: v.get("user_agent")?.as_str()?.to_string(),
        navigator_platform: v.get("navigator_platform")?.as_str()?.to_string(),
        ua_data_platform: v
            .get("ua_data_platform")
            .and_then(|x| x.as_str())
            .map(str::to_string),
        screen_width: i32::try_from(v.get("screen_width")?.as_i64()?).ok()?,
        screen_height: i32::try_from(v.get("screen_height")?.as_i64()?).ok()?,
        inner_width: i32::try_from(v.get("inner_width")?.as_i64()?).ok()?,
        inner_height: i32::try_from(v.get("inner_height")?.as_i64()?).ok()?,
    })
}

/// Planned stealth-on signals for `profile` (what the page should see).
///
/// # The `screen_*` fields are the VIEWPORT and must be reconciled
///
/// They are filled from the Xvfb constant, which is the window size, so this
/// function alone claims a screen exactly as large as the viewport — a browser
/// with no chrome and a desktop with no panel. Measured 2026-09-01, before the
/// caller reconciled them: `planned screen 1920x1080 != live 1920x1233`, and
/// `doctor --fingerprint` exited 1.
///
/// Every caller that PUBLISHES this pair must pass it through
/// [`super::super::resolve_screen`] first, which is the single place the screen
/// is computed and which floors it with `chrome_geometry::screen_for_viewport`.
/// `src/doctor/fingerprint.rs` is the one caller that publishes today and does
/// exactly that.
///
/// The reconciliation is NOT done here on purpose: `resolve_screen` reads
/// process state, and the unit tests use this function as a pure fixture.
/// Moving the call inside would make a fixture depend on a `Mutex`, which is
/// how a test suite starts measuring the order it happened to run in.
#[must_use]
pub fn planned_stealth_signals(profile: StealthProfile) -> FingerprintSignals {
    let id = Identity::for_profile(profile);
    FingerprintSignals {
        webdriver_in_navigator: true,
        webdriver_in_prototype: true,
        webdriver_value: Some(false),
        user_agent: id.user_agent.clone(),
        navigator_platform: id.navigator_platform.to_string(),
        ua_data_platform: Some(id.platform.trim_matches('"').to_string()),
        screen_width: crate::constants::DEFAULT_XVFB_WIDTH as i32,
        screen_height: crate::constants::DEFAULT_XVFB_HEIGHT as i32,
        inner_width: crate::constants::DEFAULT_XVFB_WIDTH as i32,
        inner_height: crate::constants::DEFAULT_XVFB_HEIGHT as i32,
    }
}

/// The historical BUG-01 measurement: property deleted, value undefined.
#[must_use]
pub fn bug01_deleted_webdriver() -> FingerprintSignals {
    let mut s = planned_stealth_signals(StealthProfile::Auto);
    s.webdriver_in_navigator = false;
    s.webdriver_in_prototype = false;
    s.webdriver_value = None;
    s
}

/// The historical BUG-02 measurement: Windows UA, Linux `navigator.platform`.
#[must_use]
pub fn bug02_windows_ua_linux_platform() -> FingerprintSignals {
    let mut s = planned_stealth_signals(StealthProfile::ChromeWindows);
    s.navigator_platform = "Linux x86_64".to_string();
    s
}
