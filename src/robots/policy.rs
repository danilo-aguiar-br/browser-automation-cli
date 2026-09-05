// SPDX-License-Identifier: MIT OR Apache-2.0
//! Robots policy, exemptions, and path matching (no network).

use robotstxt::DefaultMatcher;
use url::Url;

use crate::error::{CliError, ErrorKind};

/// Effective robots policy for one invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotsPolicy {
    /// Honor robots.txt rules (default).
    Honor,
    /// Skip robots.txt only when dual risk flags are set.
    Ignore,
}

impl RobotsPolicy {
    /// Stable string for JSON and logs (`honor` | `ignore`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Honor => "honor",
            Self::Ignore => "ignore",
        }
    }

    /// Build policy from CLI flags. Ignore only when both flags are set.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Usage`] when exactly one of the two flags is present:
    /// `--ignore-robots` without `--i-accept-robots-risk`, or the reverse. Both
    /// carry the `robots_dual` suggestion. Bypassing robots takes two explicit
    /// flags, so a single one is never enough and never silently honoured.
    pub fn from_flags(ignore_robots: bool, accept_risk: bool) -> Result<Self, CliError> {
        match (ignore_robots, accept_risk) {
            (false, false) => Ok(Self::Honor),
            (true, true) => Ok(Self::Ignore),
            (true, false) => Err(CliError::with_suggestion(
                ErrorKind::Usage,
                // message stays EN (stable); suggestion localized at emit via catalog map
                "--ignore-robots requires --i-accept-robots-risk",
                crate::i18n::suggestion_key("robots_dual", None),
            )),
            (false, true) => Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "--i-accept-robots-risk requires --ignore-robots",
                crate::i18n::suggestion_key("robots_dual", None),
            )),
        }
    }
}

/// Returns true when path is allowed by a simple robots Disallow set.
/// Empty disallow list means allow all. Exact prefix match on path.
pub fn path_allowed(path: &str, disallows: &[&str]) -> bool {
    if disallows.is_empty() {
        return true;
    }
    let path = if path.is_empty() { "/" } else { path };
    !disallows.iter().any(|d| {
        if d.is_empty() {
            return false;
        }
        path.starts_with(d)
    })
}

/// Reason a URL was exempted from the robots.txt lookup.
///
/// Recorded in the envelope so a blocked navigation is never confused with a
/// network failure (GAP-033).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotsExemption {
    /// Non-network scheme (`about:`, `file:`, `data:`, `blob:`).
    LocalScheme,
    /// Host resolves to a loopback address (local test loop).
    LoopbackHost,
    /// Operator passed both risk-acceptance flags.
    OperatorOverride,
}

impl RobotsExemption {
    /// Stable machine token for the JSON envelope.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalScheme => "local-scheme",
            Self::LoopbackHost => "loopback-host",
            Self::OperatorOverride => "operator-override",
        }
    }
}

/// True when the URL host is a loopback address or `localhost` (GAP-033).
///
/// A loopback target is the local test loop, never a third-party site whose
/// crawl policy could be violated, so robots.txt is not consulted. The
/// exemption is reported in the envelope rather than applied silently.
pub fn host_is_loopback(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url.trim()) else {
        return false;
    };
    match parsed.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        Some(url::Host::Domain(d)) => {
            let d = d.trim_end_matches('.').to_ascii_lowercase();
            d == "localhost" || d.ends_with(".localhost")
        }
        None => false,
    }
}

/// Exemption that applies to `url` under `policy`, or `None` when robots is enforced.
pub fn robots_exemption(url: &str, policy: RobotsPolicy) -> Option<RobotsExemption> {
    // GAP-033: loopback is exempt by DEFAULT, but the policy is configurable.
    // Without the XDG switch the block path is unreachable by any hermetic
    // test, because a local fixture server is necessarily loopback — the
    // exemption would silently swallow every enforcement assertion.
    robots_exemption_with(url, policy, crate::xdg::resolve_robots_loopback_exempt())
}

/// [`robots_exemption`] with the loopback policy supplied explicitly.
///
/// Pure decision function: takes no configuration and reads no files, so the
/// exemption rules can be asserted deterministically regardless of the XDG
/// config present on the machine running the tests.
pub fn robots_exemption_with(
    url: &str,
    policy: RobotsPolicy,
    loopback_exempt: bool,
) -> Option<RobotsExemption> {
    if matches!(policy, RobotsPolicy::Ignore) {
        return Some(RobotsExemption::OperatorOverride);
    }
    if scheme_skips_robots(url) {
        return Some(RobotsExemption::LocalScheme);
    }
    if loopback_exempt && host_is_loopback(url) {
        return Some(RobotsExemption::LoopbackHost);
    }
    None
}

/// Schemes that skip robots.txt (local / non-network).
pub fn scheme_skips_robots(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("about:")
        || lower.starts_with("file:")
        || lower.starts_with("data:")
        || lower.starts_with("blob:")
        || lower == crate::constants::ABOUT_BLANK
}

/// Check URL against robots.txt body for a user-agent using DefaultMatcher.
pub fn url_allowed_by_robots_body(robots_body: &str, user_agent: &str, url: &str) -> bool {
    let mut matcher = DefaultMatcher::default();
    matcher.one_agent_allowed_by_robots(robots_body, user_agent, url)
}
