// SPDX-License-Identifier: MIT OR Apache-2.0
//! Value parsers, range validators, and secret redaction for config keys.

use crate::error::{CliError, ErrorKind};

pub(super) fn parse_boolish(value: &str) -> bool {
    matches!(value, "true" | "1" | "yes")
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
