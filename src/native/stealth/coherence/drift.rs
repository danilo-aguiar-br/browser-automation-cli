// SPDX-License-Identifier: MIT OR Apache-2.0
//! What the process PLANNED against what the page actually emitted.
//!
//! # Why this is not in [`super`]
//!
//! [`super::assess_signals`] scores ONE reading against itself: a Windows
//! User-Agent standing next to a Linux `navigator.platform` inside a single
//! observation. It is structurally incapable of catching a plan the browser
//! declined to honour, because both readings are internally coherent — they
//! only disagree with each other.
//!
//! That makes this a different comparison over the same struct, with its own
//! rule about silence and its own rule about which components of a version
//! string may be compared. Measured under `--no-stealth` before it existed:
//! the plan announced `HeadlessChrome/152.0.0.0` and the page answered
//! `151.0.0.0`, with `ok: true` and an empty mismatch list, inside the one
//! command whose reason to exist is catching exactly that.

use super::{agent_os_from_ua, CoherenceMismatch, FingerprintSignals};

/// Chrome major version a User-Agent claims, or `None` when it names none.
///
/// `Chrome/151.0.7922.137` and `HeadlessChrome/151.0.0.0` both answer `151`:
/// the `Headless` prefix describes the launch mode, not the identity, and the
/// components below the major are what a headed and a headless Chrome disagree
/// on for the SAME binary. Comparing those would manufacture a permanent
/// mismatch out of a difference no bot check reads.
#[must_use]
pub fn ua_chrome_major(ua: &str) -> Option<String> {
    let idx = ua.find("Chrome/")?;
    let tail = &ua[idx + "Chrome/".len()..];
    let major: String = tail.chars().take_while(char::is_ascii_digit).collect();
    if major.is_empty() {
        None
    } else {
        Some(major)
    }
}

/// Contradictions between the identity this process PLANNED and the one the
/// page actually emitted.
///
/// # Why this is not `assess_signals`
///
/// [`super::assess_signals`] scores one set against itself: it catches a
/// Windows UA next to a Linux `navigator.platform` inside a single reading. It
/// cannot catch a plan that the browser silently declined to honour, because
/// both readings are internally coherent — they just disagree with each other.
///
/// # The gap this closes
///
/// This comparison covered `webdriver_value` alone until 0.1.9. Measured under
/// `--no-stealth`: the plan announced `HeadlessChrome/152.0.0.0` while the page
/// answered `151.0.0.0`, and the plan announced `ua_data_platform: "Linux"`
/// while the page answered `null`. Both passed with `ok: true` and an empty
/// mismatch list, inside the one command whose reason to exist is catching
/// exactly that.
///
/// # Silence is not agreement
///
/// A field the plan leaves `None` NEVER produces a mismatch: an absent plan is
/// not an assertion, so contradicting it is impossible. A field the plan fills
/// and the page leaves empty DOES produce one, because the plan promised a
/// signal that never arrived.
#[must_use]
pub fn planned_vs_live(
    planned: &FingerprintSignals,
    live: &FingerprintSignals,
) -> Vec<CoherenceMismatch> {
    let mut out = Vec::new();

    if let (Some(p), Some(l)) = (planned.webdriver_value, live.webdriver_value) {
        if p != l {
            out.push(CoherenceMismatch {
                id: "planned_vs_live_webdriver",
                message: format!("planned webdriver_value {p} != live {l}"),
            });
        }
    }

    // Major and platform token, never the whole string. See `ua_chrome_major`.
    let (p_major, l_major) = (
        ua_chrome_major(&planned.user_agent),
        ua_chrome_major(&live.user_agent),
    );
    let (p_os, l_os) = (
        agent_os_from_ua(&planned.user_agent),
        agent_os_from_ua(&live.user_agent),
    );
    let major_differs = matches!((&p_major, &l_major), (Some(p), Some(l)) if p != l);
    let os_differs = matches!((p_os, l_os), (Some(p), Some(l)) if p != l);
    if major_differs || os_differs {
        out.push(CoherenceMismatch {
            id: "planned_vs_live_user_agent",
            message: format!(
                "planned User-Agent {:?} disagrees with live {:?}",
                planned.user_agent, live.user_agent
            ),
        });
    }

    if !planned
        .navigator_platform
        .eq_ignore_ascii_case(&live.navigator_platform)
    {
        out.push(CoherenceMismatch {
            id: "planned_vs_live_navigator_platform",
            message: format!(
                "planned navigator.platform {:?} != live {:?}",
                planned.navigator_platform, live.navigator_platform
            ),
        });
    }

    if let Some(ref p) = planned.ua_data_platform {
        let expected = p.trim_matches('"');
        match live.ua_data_platform {
            Some(ref l) if !l.trim_matches('"').eq_ignore_ascii_case(expected) => {
                out.push(CoherenceMismatch {
                    id: "planned_vs_live_ua_data_platform",
                    message: format!("planned userAgentData.platform {p:?} != live {l:?}"),
                });
            }
            None => {
                out.push(CoherenceMismatch {
                    id: "planned_vs_live_ua_data_platform",
                    message: format!(
                        "planned userAgentData.platform {p:?} but the page exposes none"
                    ),
                });
            }
            Some(_) => {}
        }
    }

    if planned.screen_width != live.screen_width || planned.screen_height != live.screen_height {
        out.push(CoherenceMismatch {
            id: "planned_vs_live_screen",
            message: format!(
                "planned screen {}x{} != live {}x{}",
                planned.screen_width, planned.screen_height, live.screen_width, live.screen_height
            ),
        });
    }

    if planned.inner_width != live.inner_width || planned.inner_height != live.inner_height {
        out.push(CoherenceMismatch {
            id: "planned_vs_live_viewport",
            message: format!(
                "planned viewport {}x{} != live {}x{}",
                planned.inner_width, planned.inner_height, live.inner_width, live.inner_height
            ),
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::super::planned_stealth_signals;
    use super::*;
    use crate::browser_policy::StealthProfile;

    fn ids(v: Vec<CoherenceMismatch>) -> Vec<&'static str> {
        v.into_iter().map(|m| m.id).collect()
    }

    #[test]
    fn chrome_major_ignores_the_headless_prefix_and_the_patch_components() {
        assert_eq!(
            ua_chrome_major(
                "Mozilla/5.0 (X11; Linux x86_64) HeadlessChrome/151.0.0.0 Safari/537.36"
            )
            .as_deref(),
            Some("151")
        );
        assert_eq!(
            ua_chrome_major("Mozilla/5.0 (X11; Linux x86_64) Chrome/151.0.7922.137 Safari/537.36")
                .as_deref(),
            Some("151")
        );
        assert_eq!(
            ua_chrome_major("Mozilla/5.0 (X11; Linux x86_64) Firefox/128.0"),
            None
        );
        assert_eq!(ua_chrome_major("Chrome/"), None);
    }

    #[test]
    fn planned_equals_live_produces_no_mismatch() {
        let s = planned_stealth_signals(StealthProfile::Auto);
        assert!(planned_vs_live(&s, &s).is_empty());
    }

    /// The headed/headless split of one binary must never read as a conflict.
    #[test]
    fn same_major_with_a_different_patch_version_is_not_a_mismatch() {
        let planned = planned_stealth_signals(StealthProfile::Auto);
        let mut live = planned.clone();
        live.user_agent = planned.user_agent.replace("Chrome/", "HeadlessChrome/");
        assert!(ua_chrome_major(&live.user_agent) == ua_chrome_major(&planned.user_agent));
        assert!(!ids(planned_vs_live(&planned, &live)).contains(&"planned_vs_live_user_agent"));
    }

    /// The measured NC-01 half: plan said 152, page said 151, nothing fired.
    #[test]
    fn a_different_chrome_major_is_a_mismatch() {
        let planned = planned_stealth_signals(StealthProfile::Auto);
        let mut live = planned.clone();
        live.user_agent = planned.user_agent.replace("Chrome/152", "Chrome/151");
        if live.user_agent == planned.user_agent {
            // The crate table moved off 152; force the divergence explicitly.
            live.user_agent =
                "Mozilla/5.0 (X11; Linux x86_64) HeadlessChrome/1.0.0.0 Safari/537.36".to_string();
        }
        assert!(ids(planned_vs_live(&planned, &live)).contains(&"planned_vs_live_user_agent"));
    }

    #[test]
    fn a_foreign_ua_platform_is_a_mismatch() {
        let planned = planned_stealth_signals(StealthProfile::ChromeLinux);
        let live = planned_stealth_signals(StealthProfile::ChromeWindows);
        let got = ids(planned_vs_live(&planned, &live));
        assert!(got.contains(&"planned_vs_live_user_agent"));
        assert!(got.contains(&"planned_vs_live_navigator_platform"));
    }

    /// The other measured NC-01 half: plan promised a signal, page had none.
    #[test]
    fn a_promised_ua_data_platform_that_never_arrives_is_a_mismatch() {
        let planned = planned_stealth_signals(StealthProfile::Auto);
        let mut live = planned.clone();
        live.ua_data_platform = None;
        assert!(ids(planned_vs_live(&planned, &live)).contains(&"planned_vs_live_ua_data_platform"));
    }

    /// An absent plan is not an assertion, so the page cannot contradict it.
    #[test]
    fn an_unplanned_ua_data_platform_is_never_a_mismatch() {
        let mut planned = planned_stealth_signals(StealthProfile::Auto);
        planned.ua_data_platform = None;
        let live = planned_stealth_signals(StealthProfile::Auto);
        assert!(
            !ids(planned_vs_live(&planned, &live)).contains(&"planned_vs_live_ua_data_platform")
        );
    }

    #[test]
    fn divergent_screen_and_viewport_each_fire_their_own_id() {
        let planned = planned_stealth_signals(StealthProfile::Auto);
        let mut live = planned.clone();
        live.screen_width = 800;
        live.screen_height = 600;
        live.inner_width = 1024;
        live.inner_height = 768;
        let got = ids(planned_vs_live(&planned, &live));
        assert!(got.contains(&"planned_vs_live_screen"));
        assert!(got.contains(&"planned_vs_live_viewport"));
    }

    #[test]
    fn a_divergent_webdriver_value_still_fires() {
        let planned = planned_stealth_signals(StealthProfile::Auto);
        let mut live = planned.clone();
        live.webdriver_value = Some(true);
        assert!(ids(planned_vs_live(&planned, &live)).contains(&"planned_vs_live_webdriver"));
    }
}
