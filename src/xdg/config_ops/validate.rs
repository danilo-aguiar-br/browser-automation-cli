// SPDX-License-Identifier: MIT OR Apache-2.0
//! Value parsers, range validators, and secret redaction for config keys.

use crate::error::{CliError, ErrorKind};

/// The one boolean vocabulary this product accepts in configuration.
///
/// Listed here rather than inline at each call site so the error message, the
/// documentation and the gate all read the same array. A token added here is
/// accepted everywhere at once; a token added to a `matches!` arm is accepted
/// in one place and rejected in the next, which is the defect this constant
/// exists to prevent.
pub const BOOL_TOKENS: &[&str] = &["true", "false", "1", "0", "yes", "no", "on", "off"];

/// Parse a boolean config value, rejecting anything outside [`BOOL_TOKENS`].
///
/// # Why this returns `Result` when it used to return `bool`
///
/// Every other parser in this module reports a bad value to the operator. This
/// one could not: with a `bool` return there is no channel to say "no", so an
/// unrecognised token became `false`. That is silent for any key, and dangerous
/// for the seven whose default is `true` — `stealth` among them. Writing
/// `config set stealth True` turned OFF the anti-detection patches while the
/// command answered `ok`.
///
/// # Why the comparison is case-insensitive and trimmed
///
/// Three different grammars used to read these keys: `config set` accepted
/// `true|1|yes`, one reader arm accepted `true|1`, another accepted
/// `1|true|yes|on` lowercased. The same TOML file therefore resolved `"on"` to
/// `false` for `ignore_robots` and to `true` for `scrape_honor_nofollow`.
/// Normalising once, here, is what makes those three agree.
pub(crate) fn parse_boolish(value: &str, name: &str) -> Result<bool, CliError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!(
                "{name} must be one of {} (got {value:?})",
                BOOL_TOKENS.join("|")
            ),
            crate::i18n::suggestion_key("config_bool_value", None),
        )),
    }
}

pub(super) fn parse_u64(value: &str, name: &str) -> Result<u64, CliError> {
    value
        .parse()
        .map_err(|_| CliError::new(ErrorKind::Usage, format!("{name} must be an integer")))
}

pub(super) fn parse_positive_u64(value: &str, name: &str) -> Result<u64, CliError> {
    let n = parse_u64(value, name)?;
    if n == 0 {
        return Err(CliError::new(
            ErrorKind::Usage,
            format!("{name} must be > 0"),
        ));
    }
    Ok(n)
}

pub(super) fn parse_u32(value: &str, name: &str) -> Result<u32, CliError> {
    value
        .parse()
        .map_err(|_| CliError::new(ErrorKind::Usage, format!("{name} must be an integer")))
}

pub(super) fn parse_positive_u32(value: &str, name: &str) -> Result<u32, CliError> {
    let n = parse_u32(value, name)?;
    if n == 0 {
        return Err(CliError::new(
            ErrorKind::Usage,
            format!("{name} must be > 0"),
        ));
    }
    Ok(n)
}

/// Parse an inclusive-range `u8` knob (used by AVIF speed 1..=10).
pub(super) fn parse_range_u8(value: &str, name: &str, lo: u8, hi: u8) -> Result<u8, CliError> {
    let n: u8 = value.parse().map_err(|_| {
        CliError::new(
            ErrorKind::Usage,
            format!("{name} must be an integer {lo}..={hi}"),
        )
    })?;
    if n < lo || n > hi {
        return Err(CliError::new(
            ErrorKind::Usage,
            format!("{name} must be {lo}..={hi}"),
        ));
    }
    Ok(n)
}

pub(super) fn parse_quality_u8(value: &str, name: &str) -> Result<u8, CliError> {
    let n: u8 = value.parse().map_err(|_| {
        CliError::new(
            ErrorKind::Usage,
            format!("{name} must be an integer 1..=100"),
        )
    })?;
    if n == 0 || n > 100 {
        return Err(CliError::new(
            ErrorKind::Usage,
            format!("{name} must be 1..=100"),
        ));
    }
    Ok(n)
}

pub(super) fn redacted_secret(v: &Option<String>) -> &'static str {
    if v.as_ref().map(|s| !s.is_empty()).unwrap_or(false) {
        "[set]"
    } else {
        ""
    }
}
