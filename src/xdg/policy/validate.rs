// SPDX-License-Identifier: MIT OR Apache-2.0
//! Value parsing and range checks shared by every promoted policy knob.

use crate::error::{CliError, ErrorKind};

/// Parse and range-check one policy value (integer seconds/bytes/count/chance).
///
/// # Why `0` is refused for a budget and accepted for a chance
///
/// Every knob in this table used to be a BUDGET — a timeout, a size, a
/// capacity — and zero disables the thing the budget protects, so refusing it
/// is right. A knob whose unit is a CHANCE is the opposite: zero is the only
/// way to say "never", it is a legitimate value, and refusing it leaves the
/// operator with no way to turn the behaviour off at all.
///
/// The `_permille` suffix is the discriminator because it is already the
/// naming convention for that unit in this table, so the rule needs no second
/// list to fall out of sync with the first.
///
/// Measured 2026-09-04, this was already a live defect and not only a
/// constraint on new keys: `docs/CONFIGURATION.md` documents
/// `input_word_pause_permille` with "`0` removes the tail and leaves only the
/// fast rhythm", and `config set input_word_pause_permille 0` answered
/// `input_word_pause_permille must be > 0`. The documentation described the
/// intended contract and the validator refused it.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when the trimmed token does not parse as `u64` (carrying
/// the `config_list_keys` suggestion), and again when a BUDGET key parses to
/// `0`.
pub(super) fn parse_policy_value(key: &str, raw: &str) -> Result<u64, CliError> {
    let n: u64 = raw.trim().parse().map_err(|_| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            format!("{key} must be a positive integer"),
            crate::i18n::suggestion_key("config_list_keys", None),
        )
    })?;
    if n == 0 && !is_chance(key) {
        return Err(CliError::new(
            ErrorKind::Usage,
            format!("{key} must be > 0"),
        ));
    }
    Ok(n)
}

/// Whether `key` carries a probability rather than a budget.
#[must_use]
pub(super) fn is_chance(key: &str) -> bool {
    key.ends_with("_permille")
}

/// Whether a stored `n` survives the READ path for `key`.
///
/// # Why this is a function and not a `.filter(|&n| n > 0)`
///
/// The setter and the reader have to agree about what `0` means, and until
/// 2026-09-04 they did not. `parse_policy_value` learned that a CHANCE may be
/// `0` while BOTH read paths still carried a bare `.filter(|&n| n > 0)`: the
/// loose TOML loader dropped the stored zero on the way in, and `policy_u64`
/// dropped it again on the way out, falling back to the named default.
///
/// Measured on the shipped 0.1.9 binary: `config set input_word_pause_permille
/// 0` answered `ok`, wrote `input_word_pause_permille = 0` into the file,
/// `config get` answered `value: null`, and the effective value stayed at the
/// default of 120. The refusal message had been fixed and the cause left in
/// place, so the documented way to remove the long-pause tail never took
/// effect.
///
/// One function now decides for every path, so the halves cannot drift again.
#[must_use]
pub(super) fn keeps_stored(key: &str, n: u64) -> bool {
    n > 0 || is_chance(key)
}
