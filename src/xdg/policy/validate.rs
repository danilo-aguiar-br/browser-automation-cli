// SPDX-License-Identifier: MIT OR Apache-2.0
//! Value parsing and range checks shared by every promoted policy knob.

use crate::error::{CliError, ErrorKind};

/// Parse and range-check one policy value (`> 0`, integer seconds/bytes/count).
pub(super) fn parse_policy_value(key: &str, raw: &str) -> Result<u64, CliError> {
    let n: u64 = raw.trim().parse().map_err(|_| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            format!("{key} must be a positive integer"),
            crate::i18n::suggestion_key("config_list_keys", None),
        )
    })?;
    if n == 0 {
        return Err(CliError::new(
            ErrorKind::Usage,
            format!("{key} must be > 0"),
        ));
    }
    Ok(n)
}
