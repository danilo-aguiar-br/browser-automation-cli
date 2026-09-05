// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cross-read identity checks: UA, platform, webdriver, screen vs viewport.
//!
//! A bot check does not read these fields one at a time. It CROSS-READS them.
//! This module is the same comparison, so `doctor --fingerprint` and the
//! `emulate --user-agent` gate share one definition of "coherent".

use spider_fingerprint::configs::AgentOs;

use crate::browser_policy::StealthProfile;

use super::Identity;

mod drift;
mod signals;

pub use drift::{planned_vs_live, ua_chrome_major};
pub use signals::{
    bug01_deleted_webdriver, bug02_windows_ua_linux_platform, planned_stealth_signals,
    signals_from_live,
};

/// Operating system a User-Agent string claims, when the token is unambiguous.
#[must_use]
pub fn agent_os_from_ua(ua: &str) -> Option<AgentOs> {
    let lower = ua.to_ascii_lowercase();
    if lower.contains("windows") || lower.contains("win64") || lower.contains("wow64") {
        Some(AgentOs::Windows)
    } else if lower.contains("macintosh") || lower.contains("mac os") || lower.contains("iphone") {
        Some(AgentOs::Mac)
    } else if lower.contains("linux") || lower.contains("x11") || lower.contains("cros") {
        Some(AgentOs::Linux)
    } else {
        None
    }
}

/// Whether `ua` names a different platform than `profile`.
///
/// An unrecognised UA is not a contradiction: the operator may be sending a
/// custom string this product cannot classify, and refusing it would invent a
/// conflict. Only a *named* foreign platform fails closed.
#[must_use]
pub fn ua_contradicts_profile(ua: &str, profile: StealthProfile) -> bool {
    match agent_os_from_ua(ua) {
        None => false,
        Some(claimed) => claimed != Identity::for_profile(profile).agent_os,
    }
}

/// Page-visible identity signals one comparison can score.
#[derive(Debug, Clone, PartialEq)]
pub struct FingerprintSignals {
    /// `'webdriver' in navigator`
    pub webdriver_in_navigator: bool,
    /// `'webdriver' in Navigator.prototype`
    pub webdriver_in_prototype: bool,
    /// `navigator.webdriver` as a JS boolean. `None` means undefined/absent.
    pub webdriver_value: Option<bool>,
    /// `navigator.userAgent`
    pub user_agent: String,
    /// `navigator.platform`
    pub navigator_platform: String,
    /// `navigator.userAgentData.platform` when present.
    pub ua_data_platform: Option<String>,
    /// `screen.width`
    pub screen_width: i32,
    /// `screen.height`
    pub screen_height: i32,
    /// `window.innerWidth`
    pub inner_width: i32,
    /// `window.innerHeight`
    pub inner_height: i32,
}

/// One named contradiction between two signals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoherenceMismatch {
    /// Stable machine id (`webdriver_absent`, `platform_vs_ua`, …).
    pub id: &'static str,
    /// What the comparison found.
    pub message: String,
}

/// Compare the four signal families the 0.1.9 audit named.
///
/// `stealth` selects the webdriver expectation: with patches on, the value
/// must be boolean `false`. With `--no-stealth`, CDP automation may report
/// `true` and that is honest — only an *absent* property is still a tell.
#[must_use]
pub fn assess_signals(s: &FingerprintSignals, stealth: bool) -> Vec<CoherenceMismatch> {
    let mut out = Vec::new();
    if !s.webdriver_in_navigator || !s.webdriver_in_prototype {
        out.push(CoherenceMismatch {
            id: "webdriver_absent",
            message: "navigator.webdriver is missing; a real Chrome keeps the property".to_string(),
        });
    }
    if stealth && s.webdriver_value != Some(false) {
        out.push(CoherenceMismatch {
            id: "webdriver_not_false",
            message: format!(
                "navigator.webdriver must be boolean false, got {:?}",
                s.webdriver_value
            ),
        });
    }
    if let Some(claimed) = agent_os_from_ua(&s.user_agent) {
        if !platform_matches_os(&s.navigator_platform, claimed) {
            out.push(CoherenceMismatch {
                id: "platform_vs_ua",
                message: format!(
                    "navigator.platform {:?} disagrees with User-Agent platform {:?}",
                    s.navigator_platform, claimed
                ),
            });
        }
        if let Some(ref uad) = s.ua_data_platform {
            if !ua_data_matches_os(uad, claimed) {
                out.push(CoherenceMismatch {
                    id: "ua_data_vs_ua",
                    message: format!(
                        "userAgentData.platform {uad:?} disagrees with User-Agent platform {claimed:?}"
                    ),
                });
            }
        }
    }
    if s.inner_width >= 1280
        && s.inner_height >= 720
        && s.screen_width == 800
        && s.screen_height == 600
    {
        out.push(CoherenceMismatch {
            id: "screen_headless_default",
            message: format!(
                "screen {}x{} is the headless default under a {}x{} viewport",
                s.screen_width, s.screen_height, s.inner_width, s.inner_height
            ),
        });
    }
    out
}

fn platform_matches_os(platform: &str, os: AgentOs) -> bool {
    match os {
        AgentOs::Windows => platform.eq_ignore_ascii_case("Win32") || platform.contains("Win"),
        AgentOs::Mac => platform.contains("Mac"),
        _ => platform.contains("Linux") || platform.contains("X11"),
    }
}

fn ua_data_matches_os(token: &str, os: AgentOs) -> bool {
    let t = token.trim_matches('"');
    match os {
        AgentOs::Windows => t.eq_ignore_ascii_case("Windows"),
        AgentOs::Mac => t.eq_ignore_ascii_case("macOS") || t.eq_ignore_ascii_case("Mac OS"),
        _ => t.eq_ignore_ascii_case("Linux"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_ua_is_classified() {
        assert_eq!(
            agent_os_from_ua(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0.0.0"
            ),
            Some(AgentOs::Windows)
        );
    }

    #[test]
    fn linux_ua_does_not_contradict_auto_on_this_host() {
        let ua = Identity::for_profile(StealthProfile::Auto).user_agent;
        assert!(!ua_contradicts_profile(&ua, StealthProfile::Auto));
    }

    #[test]
    fn windows_ua_contradicts_linux_profile() {
        let ua = Identity::for_profile(StealthProfile::ChromeWindows).user_agent;
        // On a Linux/macOS CI host Auto resolves away from Windows.
        if StealthProfile::Auto.resolved() != StealthProfile::ChromeWindows {
            assert!(ua_contradicts_profile(&ua, StealthProfile::Auto));
        }
    }

    #[test]
    fn assess_flags_the_old_webdriver_deletion() {
        let ids: Vec<_> = assess_signals(&bug01_deleted_webdriver(), true)
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert!(ids.contains(&"webdriver_absent"));
        assert!(ids.contains(&"webdriver_not_false"));
    }

    #[test]
    fn assess_flags_windows_ua_with_linux_platform() {
        let ids: Vec<_> = assess_signals(&bug02_windows_ua_linux_platform(), true)
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert!(ids.contains(&"platform_vs_ua"));
    }

    #[test]
    fn planned_stealth_identity_is_coherent() {
        for profile in [
            StealthProfile::Auto,
            StealthProfile::ChromeLinux,
            StealthProfile::ChromeWindows,
            StealthProfile::ChromeMac,
        ] {
            let mismatches = assess_signals(&planned_stealth_signals(profile), true);
            assert!(
                mismatches.is_empty(),
                "{profile:?} planned identity is incoherent: {mismatches:?}"
            );
        }
    }

    #[test]
    fn assess_flags_800x600_under_a_desktop_viewport() {
        let mut s = planned_stealth_signals(StealthProfile::Auto);
        s.screen_width = 800;
        s.screen_height = 600;
        s.inner_width = 1920;
        s.inner_height = 1080;
        let ids: Vec<_> = assess_signals(&s, true).into_iter().map(|m| m.id).collect();
        assert!(ids.contains(&"screen_headless_default"));
    }

    #[test]
    fn no_stealth_webdriver_true_is_not_a_mismatch() {
        let mut s = planned_stealth_signals(StealthProfile::Auto);
        s.webdriver_value = Some(true);
        s.user_agent =
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 HeadlessChrome/151.0.0.0"
                .to_string();
        let ids: Vec<_> = assess_signals(&s, false)
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert!(!ids.contains(&"webdriver_not_false"));
    }
}
