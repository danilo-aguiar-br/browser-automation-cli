//! robots.txt honor for one-shot invocations (no cross-process cache).
//!
//! Default policy is honor. Override requires BOTH `--ignore-robots` and
//! `--i-accept-robots-risk`. Uses `robotstxt::DefaultMatcher` (Google port).

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
    pub fn from_flags(ignore_robots: bool, accept_risk: bool) -> Result<Self, CliError> {
        match (ignore_robots, accept_risk) {
            (false, false) => Ok(Self::Honor),
            (true, true) => Ok(Self::Ignore),
            (true, false) => Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "--ignore-robots requires --i-accept-robots-risk",
                "Pass both flags together when you intentionally skip robots.txt",
            )),
            (false, true) => Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "--i-accept-robots-risk requires --ignore-robots",
                "Pass both flags together when you intentionally skip robots.txt",
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

/// Schemes that skip robots.txt (local / non-network).
pub fn scheme_skips_robots(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("about:")
        || lower.starts_with("file:")
        || lower.starts_with("data:")
        || lower.starts_with("blob:")
        || lower == "about:blank"
}

/// Check URL against robots.txt body for a user-agent using DefaultMatcher.
pub fn url_allowed_by_robots_body(robots_body: &str, user_agent: &str, url: &str) -> bool {
    let mut matcher = DefaultMatcher::default();
    matcher.one_agent_allowed_by_robots(robots_body, user_agent, url)
}

/// Fetch origin robots.txt and enforce honor policy (async, one-shot).
pub async fn enforce_robots(
    url: &str,
    policy: RobotsPolicy,
    user_agent: &str,
) -> Result<(), CliError> {
    if matches!(policy, RobotsPolicy::Ignore) || scheme_skips_robots(url) {
        return Ok(());
    }

    let parsed = Url::parse(url).map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            format!("invalid URL for robots check: {e}"),
            "Pass an absolute http(s) URL or about:blank/file://",
        )
    })?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Ok(());
    }

    let origin = parsed.origin().ascii_serialization();
    let robots_url = format!("{origin}/robots.txt");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent(user_agent)
        .build()
        .map_err(|e| CliError::new(ErrorKind::Software, format!("http client: {e}")))?;

    let resp = match client.get(&robots_url).send().await {
        Ok(r) => r,
        Err(e) => {
            // PRD: fetch failure → allow with warning on stderr
            eprintln!("robots: fetch failed for {robots_url}: {e}; treating as allow");
            return Ok(());
        }
    };

    if !resp.status().is_success() {
        // Missing robots → allow
        return Ok(());
    }

    let body = resp
        .text()
        .await
        .map_err(|e| CliError::new(ErrorKind::Io, format!("robots body: {e}")))?;

    if url_allowed_by_robots_body(&body, user_agent, url) {
        return Ok(());
    }

    Err(CliError::with_suggestion(
        ErrorKind::Data,
        format!("robots.txt disallows URL: {url}"),
        "Use --ignore-robots --i-accept-robots-risk only when you accept the risk",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn honor_disallow_prefix() {
        assert!(!path_allowed("/private/x", &["/private"]));
        assert!(path_allowed("/public/x", &["/private"]));
        assert!(path_allowed("/any", &[]));
    }

    #[test]
    fn scheme_skips_local() {
        assert!(scheme_skips_robots("about:blank"));
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
}
