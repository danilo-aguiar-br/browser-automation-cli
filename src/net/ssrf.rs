// SPDX-License-Identifier: MIT OR Apache-2.0
//! SSRF policy for operator-supplied HTTP(S) URLs (scrape, webhook, LLM).

use std::net::IpAddr;

use url::Url;

use crate::error::{CliError, ErrorKind};

/// HTTP SSRF policy mode (XDG `http_ssrf_mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsrfMode {
    /// Block private, loopback, link-local, multicast, unspecified, metadata hosts.
    Strict,
    /// Like strict, but allow loopback (127.0.0.0/8, ::1).
    AllowLoopback,
    /// No IP/host policy (operator conscious override).
    Off,
}

impl SsrfMode {
    /// Parse mode string (`strict` | `allow_loopback` | `off`).
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "allow_loopback" | "allow-loopback" | "loopback" => Self::AllowLoopback,
            "off" | "disabled" | "none" => Self::Off,
            _ => Self::Strict,
        }
    }

    /// Resolve from XDG config.
    pub fn from_xdg() -> Self {
        Self::parse(&crate::xdg::resolve_http_ssrf_mode())
    }
}

/// Cloud / link-local metadata hostnames (case-insensitive).
const METADATA_HOSTS: &[&str] = &[
    "metadata.google.internal",
    "metadata.goog",
    "metadata",
    "instance-data",
];

/// Assert URL is http(s) and passes the current XDG SSRF mode.
pub fn assert_safe_http_url(url: &str) -> Result<(), CliError> {
    assert_safe_http_url_mode(url, SsrfMode::from_xdg())
}

/// Assert URL is http(s) and passes `mode`.
pub fn assert_safe_http_url_mode(url: &str, mode: SsrfMode) -> Result<(), CliError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "empty URL",
            crate::i18n::suggestion_key("url_absolute_http", None),
        ));
    }
    let parsed = Url::parse(trimmed).map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            format!("invalid URL: {e}"),
            crate::i18n::suggestion_key("url_absolute_http", None),
        )
    })?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("unsupported scheme `{other}` for HTTP client"),
                crate::i18n::suggestion_key("url_absolute_http", None),
            ));
        }
    }
    if matches!(mode, SsrfMode::Off) {
        return Ok(());
    }
    let host = parsed.host_str().ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            "URL has no host",
            crate::i18n::suggestion_key("url_absolute_http", None),
        )
    })?;
    let host_l = host.to_ascii_lowercase();
    if METADATA_HOSTS.iter().any(|h| host_l == *h) {
        return ssrf_err(host);
    }
    // Bracket-stripped IPv6 or plain IPv4 literal.
    let ip_str = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = ip_str.parse::<IpAddr>() {
        return check_ip(ip, mode, host);
    }
    // Non-literal hostname: block obvious local names in strict mode.
    if matches!(mode, SsrfMode::Strict)
        && (host_l == "localhost" || host_l == "localhost." || host_l.ends_with(".localhost"))
    {
        return ssrf_err(host);
    }
    // Hostname without literal IP: allow (public DNS); operator may still hit
    // private after DNS — final_url recheck covers redirect to literal private.
    Ok(())
}

fn check_ip(ip: IpAddr, mode: SsrfMode, host_label: &str) -> Result<(), CliError> {
    if ip.is_unspecified() || ip.is_multicast() {
        return ssrf_err(host_label);
    }
    if ip.is_loopback() {
        return match mode {
            SsrfMode::AllowLoopback | SsrfMode::Off => Ok(()),
            SsrfMode::Strict => ssrf_err(host_label),
        };
    }
    match ip {
        IpAddr::V4(v4) => {
            if v4.is_private() || v4.is_link_local() || v4.is_broadcast() {
                return ssrf_err(host_label);
            }
            // 0.0.0.0/8 already unspecified; 169.254 is link_local; CGNAT 100.64/10:
            let o = v4.octets();
            if o[0] == 100 && (o[1] & 0xc0) == 64 {
                return ssrf_err(host_label);
            }
        }
        IpAddr::V6(v6) => {
            if v6.is_unicast_link_local() {
                return ssrf_err(host_label);
            }
            // Unique local fc00::/7
            let s = v6.segments();
            if (s[0] & 0xfe00) == 0xfc00 {
                return ssrf_err(host_label);
            }
        }
    }
    Ok(())
}

fn ssrf_err(host: &str) -> Result<(), CliError> {
    Err(CliError::with_suggestion(
        ErrorKind::Usage,
        format!("URL host '{host}' blocked by SSRF policy"),
        crate::i18n::suggestion_key("ssrf_blocked", None),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_https_ok() {
        assert!(assert_safe_http_url_mode("https://example.com/a", SsrfMode::Strict).is_ok());
    }

    #[test]
    fn loopback_strict_blocked() {
        assert!(assert_safe_http_url_mode("http://127.0.0.1/", SsrfMode::Strict).is_err());
        assert!(assert_safe_http_url_mode("http://localhost/", SsrfMode::Strict).is_err());
        assert!(assert_safe_http_url_mode("http://[::1]/", SsrfMode::Strict).is_err());
    }

    #[test]
    fn loopback_allow_mode_ok() {
        assert!(assert_safe_http_url_mode("http://127.0.0.1/", SsrfMode::AllowLoopback).is_ok());
    }

    #[test]
    fn private_and_metadata_blocked() {
        assert!(assert_safe_http_url_mode("http://10.0.0.1/", SsrfMode::Strict).is_err());
        assert!(assert_safe_http_url_mode("http://192.168.1.1/", SsrfMode::Strict).is_err());
        assert!(assert_safe_http_url_mode("http://169.254.169.254/", SsrfMode::Strict).is_err());
        assert!(
            assert_safe_http_url_mode("http://metadata.google.internal/", SsrfMode::Strict)
                .is_err()
        );
    }

    #[test]
    fn off_mode_allows_private() {
        assert!(assert_safe_http_url_mode("http://10.0.0.1/", SsrfMode::Off).is_ok());
    }

    #[test]
    fn non_http_scheme_blocked() {
        assert!(assert_safe_http_url_mode("file:///etc/passwd", SsrfMode::Strict).is_err());
    }
}
