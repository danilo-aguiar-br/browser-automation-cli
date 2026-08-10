// SPDX-License-Identifier: MIT OR Apache-2.0
//! `config set` mutation path for the egress and HTTP/2 knob family.
//!
//! Split out of `set.rs` (mirroring `set_media` and `set_scrape`) so the proxy
//! credentials, the stealth seed and the HTTP/2 `SETTINGS` values validate in
//! one place and the parent file stays under the project file-size gate.
//!
//! # Why these knobs share a module
//!
//! They share a failure mode rather than a subject. Each one is read once when
//! the process-wide HTTP client is built, and a bad value there is not a
//! rejected request — it is a connection that never completes, or worse, one
//! that completes by a route the caller did not ask for. Validating them at
//! `config set` keeps that failure at the moment the operator can still read
//! the message.

use super::super::config_model::ProductConfig;
use super::validate::parse_boolish;
use crate::error::{CliError, ErrorKind};

/// Lower bound of `SETTINGS_MAX_FRAME_SIZE` (RFC 9113 §6.5.2).
const HTTP2_FRAME_SIZE_MIN: u32 = 16_384;

/// Upper bound of `SETTINGS_MAX_FRAME_SIZE` (RFC 9113 §6.5.2).
const HTTP2_FRAME_SIZE_MAX: u32 = 16_777_215;

/// Apply an egress/HTTP2-family `config set` mutation.
///
/// Returns `Ok(false)` when `key` belongs to another family, so the caller
/// falls through to its own match without treating it as an unknown key.
pub(super) fn apply_network_set(
    cfg: &mut ProductConfig,
    key: &str,
    value: &str,
) -> Result<bool, CliError> {
    match key {
        "proxy_url" => {
            // Rejected here rather than at launch: an unusable proxy discovered
            // mid-run has already leaked the request through the direct route.
            let trimmed = value.trim();
            if !(trimmed.starts_with("http://")
                || trimmed.starts_with("https://")
                || trimmed.starts_with("socks5://")
                || trimmed.starts_with("socks5h://"))
            {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "proxy_url must start with http://, https://, socks5:// or socks5h://",
                ));
            }
            cfg.proxy_url = Some(trimmed.to_string());
        }
        "proxy_bypass" => cfg.proxy_bypass = Some(value.trim().to_string()),
        "cdp_proxy_bypass_loopback" => {
            cfg.cdp_proxy_bypass_loopback = Some(parse_boolish(value, key)?);
        }
        // Credentials are not trimmed: leading or trailing whitespace can be
        // part of a generated secret, and silently eating it would produce an
        // authentication failure with no visible cause.
        "proxy_username" => cfg.proxy_username = Some(value.to_string()),
        "proxy_password" => cfg.proxy_password = Some(value.to_string()),
        "stealth_seed" => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "stealth_seed must not be empty (unset the key to redraw per process)",
                ));
            }
            cfg.stealth_seed = Some(trimmed.to_string());
        }
        "http2_enabled" => cfg.http2_enabled = Some(parse_boolish(value, key)?),
        "http2_adaptive_window" => cfg.http2_adaptive_window = Some(parse_boolish(value, key)?),
        "http2_initial_stream_window_size" => {
            cfg.http2_initial_stream_window_size =
                Some(parse_window(value, "http2_initial_stream_window_size")?);
        }
        "http2_initial_connection_window_size" => {
            cfg.http2_initial_connection_window_size =
                Some(parse_window(value, "http2_initial_connection_window_size")?);
        }
        "http2_max_header_list_size" => {
            let n = parse_u32(value, "http2_max_header_list_size")?;
            if n == 0 {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "http2_max_header_list_size must be greater than zero",
                ));
            }
            cfg.http2_max_header_list_size = Some(n);
        }
        "http2_max_frame_size" => {
            let n = parse_u32(value, "http2_max_frame_size")?;
            if !(HTTP2_FRAME_SIZE_MIN..=HTTP2_FRAME_SIZE_MAX).contains(&n) {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    "http2_max_frame_size must be in 16384..=16777215 (RFC 9113 §6.5.2)",
                ));
            }
            cfg.http2_max_frame_size = Some(n);
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Parse a `u32` knob, naming the key in the failure so the operator can act.
fn parse_u32(value: &str, key: &str) -> Result<u32, CliError> {
    value.trim().parse::<u32>().map_err(|_| {
        CliError::new(
            ErrorKind::Usage,
            format!("{key} must be a non-negative integer"),
        )
    })
}

/// Parse an HTTP/2 flow-control window.
///
/// The ceiling is `i32::MAX` rather than `u32::MAX` because RFC 9113 §6.9.1
/// makes the window a signed 31-bit quantity; a larger value is a
/// `FLOW_CONTROL_ERROR` on the wire, which closes the connection instead of
/// failing the request.
fn parse_window(value: &str, key: &str) -> Result<u32, CliError> {
    let n = parse_u32(value, key)?;
    if n > i32::MAX as u32 {
        return Err(CliError::new(
            ErrorKind::Usage,
            format!("{key} must not exceed 2147483647 (RFC 9113 §6.9.1)"),
        ));
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ProductConfig {
        ProductConfig::default()
    }

    #[test]
    fn unknown_key_falls_through() {
        let mut c = cfg();
        assert!(!apply_network_set(&mut c, "lang", "en").expect("fall through"));
    }

    #[test]
    fn proxy_url_requires_known_scheme() {
        let mut c = cfg();
        assert!(apply_network_set(&mut c, "proxy_url", "ftp://h:1").is_err());
        assert!(apply_network_set(&mut c, "proxy_url", "socks5://h:1").is_ok());
        assert_eq!(c.proxy_url.as_deref(), Some("socks5://h:1"));
    }

    #[test]
    fn credentials_keep_surrounding_whitespace() {
        let mut c = cfg();
        apply_network_set(&mut c, "proxy_password", " s3cret ").expect("set");
        assert_eq!(c.proxy_password.as_deref(), Some(" s3cret "));
    }

    #[test]
    fn empty_stealth_seed_is_rejected() {
        let mut c = cfg();
        assert!(apply_network_set(&mut c, "stealth_seed", "  ").is_err());
    }

    #[test]
    fn frame_size_honours_rfc_bounds() {
        let mut c = cfg();
        assert!(apply_network_set(&mut c, "http2_max_frame_size", "1024").is_err());
        assert!(apply_network_set(&mut c, "http2_max_frame_size", "16777216").is_err());
        assert!(apply_network_set(&mut c, "http2_max_frame_size", "16384").is_ok());
    }

    #[test]
    fn window_rejects_values_above_signed_31_bits() {
        let mut c = cfg();
        assert!(
            apply_network_set(&mut c, "http2_initial_stream_window_size", "2147483648").is_err()
        );
        assert!(
            apply_network_set(&mut c, "http2_initial_stream_window_size", "2147483647").is_ok()
        );
    }

    #[test]
    fn header_list_size_rejects_zero() {
        let mut c = cfg();
        assert!(apply_network_set(&mut c, "http2_max_header_list_size", "0").is_err());
        assert!(apply_network_set(&mut c, "http2_max_header_list_size", "262144").is_ok());
    }
}
