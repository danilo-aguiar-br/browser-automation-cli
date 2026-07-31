// SPDX-License-Identifier: MIT OR Apache-2.0
//! Product network policy helpers (SSRF, body caps, loopback addressing).
//!
//! # Product law
//!
//! One-shot agent CLI: HTTP **client** surfaces + loopback MITM. No daemon
//! accept loop. Timeouts and safety modes come from **CLI + XDG**, never product
//! env. Reqwest system proxy env (`HTTP_PROXY` / …) is disabled via `no_proxy()`.

mod body;
mod ssrf;

pub use body::read_body_limited;
pub use ssrf::{assert_safe_http_url, assert_safe_http_url_mode, SsrfMode};

use std::net::{IpAddr, SocketAddr};

use crate::constants::{LOOPBACK_HOST, MITM_BIND_HOST};
use crate::error::{CliError, ErrorKind};

/// Parse product loopback host (`LOOPBACK_HOST` / `MITM_BIND_HOST`) to [`IpAddr`].
pub fn loopback_ip() -> IpAddr {
    LOOPBACK_HOST
        .parse()
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST))
}

/// Loopback [`SocketAddr`] for the given port (MITM / local probes).
pub fn loopback_socket_addr(port: u16) -> SocketAddr {
    SocketAddr::new(loopback_ip(), port)
}

/// Bind string for ephemeral loopback port probe (`host:0`).
pub fn loopback_bind_ephemeral() -> String {
    format!("{MITM_BIND_HOST}:0")
}

/// Ensure a Redis host is allowed (loopback by default; remote only when XDG says so).
pub fn assert_redis_host_allowed(host: &str) -> Result<(), CliError> {
    let host = host.trim();
    if host.is_empty() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "empty redis host",
            crate::i18n::suggestion_key("redis_config_required", None),
        ));
    }
    if crate::xdg::resolve_redis_allow_remote() {
        return Ok(());
    }
    let lower = host.to_ascii_lowercase();
    if lower == "localhost" || lower == "localhost." {
        return Ok(());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if ip.is_loopback() {
            return Ok(());
        }
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("redis host '{host}' is not loopback (product default)"),
            crate::i18n::suggestion_key("redis_host_blocked", None),
        ));
    }
    // Non-literal hostname under default policy: reject (no silent remote DNS).
    Err(CliError::with_suggestion(
        ErrorKind::Usage,
        format!("redis host '{host}' is not a loopback address"),
        crate::i18n::suggestion_key("redis_host_blocked", None),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_addr_is_127() {
        let a = loopback_socket_addr(0);
        assert!(a.ip().is_loopback());
        assert_eq!(a.port(), 0);
    }

    #[test]
    fn redis_host_loopback_ok() {
        assert!(assert_redis_host_allowed("127.0.0.1").is_ok());
        assert!(assert_redis_host_allowed("localhost").is_ok());
    }

    #[test]
    fn redis_host_remote_rejected_by_default() {
        // Default XDG has redis_allow_remote=false (or unset).
        let err = assert_redis_host_allowed("example.com").expect_err("remote");
        assert_eq!(err.kind(), ErrorKind::Usage);
        let err = assert_redis_host_allowed("8.8.8.8").expect_err("public ip");
        assert_eq!(err.kind(), ErrorKind::Usage);
    }
}
