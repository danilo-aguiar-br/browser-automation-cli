// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests for robots policy and exemptions.

use super::*;

#[test]
fn honor_disallow_prefix() {
    assert!(!path_allowed("/private/x", &["/private"]));
    assert!(path_allowed("/public/x", &["/private"]));
    assert!(path_allowed("/any", &[]));
}

#[test]
fn loopback_hosts_are_exempt_without_any_flag() {
    // GAP-033: the local test loop must not need the dual risk flags.
    for url in [
        "http://127.0.0.1:8080/login",
        "http://localhost:3000/",
        "https://127.0.0.1/health",
        "http://[::1]:9000/api",
        "http://app.localhost/",
    ] {
        assert!(host_is_loopback(url), "{url} must be loopback");
        // Explicit policy: asserting the default rule must not depend on the
        // XDG config of the machine running the suite.
        assert_eq!(
            robots_exemption_with(url, RobotsPolicy::Honor, true),
            Some(RobotsExemption::LoopbackHost),
            "{url}"
        );
    }
}

#[test]
fn loopback_is_enforced_when_exemption_is_disabled() {
    // The knob is what makes the block path reachable for a hermetic
    // fixture; without this the enforcement assertion is unreachable.
    for url in ["http://127.0.0.1:8080/login", "http://localhost:3000/"] {
        assert_eq!(
            robots_exemption_with(url, RobotsPolicy::Honor, false),
            None,
            "{url} must be enforced when loopback exemption is off"
        );
    }
}

#[test]
fn public_hosts_are_not_exempt_under_honor() {
    for url in ["https://example.com/", "http://93.184.216.34/"] {
        assert!(!host_is_loopback(url), "{url}");
        assert_eq!(robots_exemption(url, RobotsPolicy::Honor), None, "{url}");
    }
}

#[test]
fn exemption_reason_distinguishes_scheme_from_override() {
    assert_eq!(
        robots_exemption("file:///tmp/x.html", RobotsPolicy::Honor),
        Some(RobotsExemption::LocalScheme)
    );
    assert_eq!(
        robots_exemption("https://example.com/", RobotsPolicy::Ignore),
        Some(RobotsExemption::OperatorOverride)
    );
}

#[test]
fn lookalike_hosts_are_not_treated_as_loopback() {
    // Defence against a host that merely embeds the token.
    for url in [
        "https://localhost.evil.com/",
        "https://127.0.0.1.evil.com/",
        "https://notlocalhost/",
    ] {
        assert!(!host_is_loopback(url), "{url} must not be loopback");
    }
}

#[test]
fn scheme_skips_local() {
    assert!(scheme_skips_robots(crate::constants::ABOUT_BLANK));
    assert!(scheme_skips_robots("file:///tmp/x.html"));
    assert!(!scheme_skips_robots("https://example.com/"));
}

#[test]
fn policy_requires_both_flags() {
    assert!(matches!(
        RobotsPolicy::from_flags(false, false).unwrap(),
        RobotsPolicy::Honor
    ));
    assert!(matches!(
        RobotsPolicy::from_flags(true, true).unwrap(),
        RobotsPolicy::Ignore
    ));
    assert!(RobotsPolicy::from_flags(true, false).is_err());
    assert!(RobotsPolicy::from_flags(false, true).is_err());
}

#[test]
fn default_matcher_blocks_disallow_all() {
    let body = "user-agent: *\ndisallow: /\n";
    assert!(!url_allowed_by_robots_body(
        body,
        "browser-automation-cli",
        "https://example.com/secret"
    ));
}

#[test]
fn default_matcher_allows_empty() {
    assert!(url_allowed_by_robots_body(
        "",
        "browser-automation-cli",
        "https://example.com/"
    ));
}
