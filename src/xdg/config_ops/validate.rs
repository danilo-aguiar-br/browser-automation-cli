// SPDX-License-Identifier: MIT OR Apache-2.0
//! Value parsers, range validators, and secret redaction for config keys.

use crate::error::{CliError, ErrorKind};

/// Characters the `config.toml` writer and reader cannot represent in a value.
///
/// A double quote closes the string the writer opened; a newline or carriage
/// return starts a line the reader parses as a fresh key. Either one lets a
/// value smuggle in a key the operator never set.
///
/// A backslash is deliberately NOT here. The reader decodes no escape
/// sequences, so a backslash is written literally and read back literally: it
/// round-trips correctly and injects nothing. Rejecting it would block a
/// legitimate password character for no gain, and a validator that refuses
/// more than it must trains operators to work around it.
const UNREPRESENTABLE_IN_VALUE: &[char] = &['"', '\n', '\r'];

/// Reject a value the config file format cannot round-trip.
///
/// # Why rejection and not escaping
///
/// Escaping is the usual answer and here it would be wrong. The file is read
/// back by `parse_simple_toml`, a hand-written line loop that splits on the
/// first `=` and then trims quote characters off the ends. It decodes no TOML
/// escape sequences, so an escaped quote would be read back with its backslash
/// intact and the value would not survive the round trip. The format genuinely
/// cannot express these characters, so the honest answer to the operator is a
/// refusal, not a value quietly altered on the way to disk.
///
/// # Why this lives at the single funnel and not in each setter
///
/// Every `config set` reaches disk through one function. Validating there
/// covers the keys that exist today AND the ones added tomorrow. Per-setter
/// checks would cover only what someone remembered to check, which is the
/// shape of defect this module has already paid for: the doc comment in
/// `config_write_optional` records two separate sweeps that each missed keys.
///
/// # What it prevents, concretely
///
/// The writer builds each line by interpolating the value between quotes. A
/// value carrying a quote and a newline closes its own string and opens a line
/// of its choosing. Pointing that line at a key that names an executable turns
/// a configuration write into code execution on the next run. The most exposed
/// keys are the free-text ones, and those are exactly the credential keys:
/// enum-valued keys are shielded only as a side effect of their vocabulary
/// check, which is protection by accident rather than by design.
pub fn reject_unrepresentable_value(key: &str, value: &str) -> Result<(), CliError> {
    let Some(bad) = value.chars().find(|c| UNREPRESENTABLE_IN_VALUE.contains(c)) else {
        return Ok(());
    };
    let named = match bad {
        '"' => "a double quote",
        '\n' => "a newline",
        _ => "a carriage return",
    };
    Err(CliError::with_suggestion(
        ErrorKind::Usage,
        format!("`{key}` value contains {named}, which config.toml cannot store"),
        "Remove the character. The config file format has no escape sequence for it, so a value carrying one cannot be read back as it was written.".to_string(),
    ))
}

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
///
/// # Errors
///
/// [`ErrorKind::Usage`] when the trimmed, lowercased token is not one of
/// [`BOOL_TOKENS`]; the message echoes the accepted vocabulary and the offending
/// value, and carries the `config_bool_value` suggestion.
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

/// Parse an unsigned 64-bit config value.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when `str::parse::<u64>` rejects the token — empty
/// string, non-digit characters, a leading sign, or a value above `u64::MAX`.
pub(super) fn parse_u64(value: &str, name: &str) -> Result<u64, CliError> {
    value
        .parse()
        .map_err(|_| CliError::new(ErrorKind::Usage, format!("{name} must be an integer")))
}

/// Parse an unsigned 64-bit config value that must be non-zero.
///
/// # Errors
///
/// [`ErrorKind::Usage`] propagated from [`parse_u64`] for a token that is not an
/// integer, or raised here when the parsed value is exactly `0`.
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

/// Parse an unsigned 32-bit config value.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when `str::parse::<u32>` rejects the token — empty
/// string, non-digit characters, a leading sign, or a value above `u32::MAX`.
pub(super) fn parse_u32(value: &str, name: &str) -> Result<u32, CliError> {
    value
        .parse()
        .map_err(|_| CliError::new(ErrorKind::Usage, format!("{name} must be an integer")))
}

/// Parse an unsigned 32-bit config value that must be non-zero.
///
/// # Errors
///
/// [`ErrorKind::Usage`] propagated from [`parse_u32`] for a token that is not an
/// integer, or raised here when the parsed value is exactly `0`.
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
///
/// # Errors
///
/// [`ErrorKind::Usage`] when the token does not parse as `u8`, and again when it
/// parses but falls outside `lo..=hi`. The two cases carry different messages so
/// the operator can tell a typo from an out-of-range knob.
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

/// Parse a lossy-quality knob constrained to `1..=100`.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when the token does not parse as `u8`, and again when it
/// parses to `0` or to a value above `100`.
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

#[cfg(test)]
mod value_representability_tests {
    use super::reject_unrepresentable_value;

    /// The three characters are built from `char` literals on purpose.
    ///
    /// Spelling the payload with inline escape sequences makes the test hard to
    /// read at the exact place where precision matters most, and invites a
    /// future edit to change the payload while believing it changed nothing.
    const QUOTE: char = '"';
    const NEWLINE: char = '\n';
    const CARRIAGE_RETURN: char = '\r';

    /// The payload that turns a config write into code execution.
    ///
    /// The writer emits `key = "VALUE"` by interpolation. A value that closes
    /// the quote and starts a new line declares a key of its own choosing, and
    /// the reader — a line loop that splits on the first `=` — accepts it. When
    /// the smuggled key names an executable, the next run executes it.
    #[test]
    fn quote_and_newline_payload_is_refused() {
        let payload = format!("x{QUOTE}{NEWLINE}ffmpeg_path = {QUOTE}/tmp/evil");
        let err = reject_unrepresentable_value("proxy_password", &payload)
            .expect_err("a value that can forge a second key must be refused");
        assert!(
            err.message().contains("proxy_password"),
            "the refusal must name the key the operator typed, got: {}",
            err.message()
        );
    }

    /// Each unrepresentable character is refused on its own, not only together.
    ///
    /// Testing only the combined payload would pass with a check that caught
    /// just one of them, so the gate would hold while half the hole stayed open.
    #[test]
    fn each_unrepresentable_character_is_refused_alone() {
        for (label, bad) in [
            ("quote", QUOTE),
            ("newline", NEWLINE),
            ("carriage return", CARRIAGE_RETURN),
        ] {
            let value = format!("pass{bad}word");
            assert!(
                reject_unrepresentable_value("proxy_password", &value).is_err(),
                "a value containing a {label} must be refused on its own"
            );
        }
    }

    /// A backslash round-trips and must stay legal.
    ///
    /// The reader decodes no escape sequences, so a backslash is written and
    /// read back unchanged. Refusing it would block a legitimate password
    /// character for no security gain, and a validator that refuses more than
    /// it must is one operators learn to route around.
    #[test]
    fn backslash_and_ordinary_secrets_stay_legal() {
        let backslash = char::from(92u8);
        let with_backslash = format!("p4ss{backslash}word");
        assert!(
            reject_unrepresentable_value("proxy_password", &with_backslash).is_ok(),
            "a backslash round-trips through the reader and must be accepted"
        );
        for value in ["hunter2", "a b c", "", "  spaced  ", "=equals="] {
            assert!(
                reject_unrepresentable_value("proxy_password", value).is_ok(),
                "a value the format can store must be accepted: {value:?}"
            );
        }
    }
}
/// Refuse a value that could not survive the config round-trip intact.
///
/// # Why refusal and not escaping
///
/// The writer builds `config.toml` by string interpolation, so a value carrying
/// a quote and a newline closes its string and opens a LINE of TOML the operator
/// never asked for. Setting `proxy_password` to a value that ends the string and
/// declares `ffmpeg_path` plants a binary path the next media command executes:
/// a config write escalating to code execution.
///
/// The obvious fix is to serialise with a real TOML writer. It is the wrong one
/// HERE, and the reader is why. `config_io` parses a line with `split_once('=')`
/// followed by `trim_matches`, which is not a TOML parser and decodes no escapes
/// at all. A correctly escaped newline would be read back as the two literal
/// characters that spell it. Escaping on write alone converts a security defect
/// into a silent write/read divergence, which is worse: the defect below fails
/// loudly the first time someone tries it.
///
/// This defect class has a public precedent under the name "TOML injection":
/// heimdallm issue #6 (<https://github.com/theburrowhub/heimdallm/issues/6>)
/// is the same shape — string interpolation into a TOML body letting a value
/// with a quote and a newline open sections nobody asked for. Worth knowing
/// that the fix chosen THERE was to escape, which is the usual answer and the
/// one that does not work here, for the reader reason above.
///
/// The general principle is OWASP's Input Validation guidance: validate at the
/// first trust boundary, and treat validation and output encoding as separate
/// controls applied at their own points. Refusing at the setter is the
/// validation half; the encoding half is unavailable while the reader decodes
/// nothing.
///
/// So the boundary is stated where it is enforceable. The refusal names the
/// offending character class rather than leaving the caller to guess, and it
/// covers every control character instead of the two that motivated it — a tab
/// or a form feed breaks the same line-oriented reader.
pub(super) fn reject_untransportable_value(key: &str, value: &str) -> Result<(), CliError> {
    const BACKSLASH: char = '\u{5c}';
    let named = value.chars().find_map(|c| match c {
        '"' => Some("a double quote"),
        BACKSLASH => Some("a backslash"),
        c if c.is_control() => Some("a control character"),
        _ => None,
    });
    let Some(named) = named else {
        return Ok(());
    };
    Err(CliError::with_suggestion(
        ErrorKind::Usage,
        format!(
            "config key `{key}` cannot hold {named}: the value would not survive \
             a write/read round-trip of the config file"
        ),
        crate::i18n::suggestion_key("use_listed_value", None),
    ))
}

#[cfg(test)]
mod round_trip_tests {
    use super::reject_untransportable_value;

    #[test]
    fn a_value_that_could_close_its_toml_string_is_refused() {
        // The escalation this closes, measured against the real writer: the
        // config file is built by interpolating the value between quotes, so a
        // value carrying a quote and a newline ends its own string and opens a
        // LINE of TOML. Declaring `ffmpeg_path` there plants a binary the next
        // media command executes — a config write reaching code execution.
        let payload = "hunter2\"\nffmpeg_path = \"/tmp/evil";
        let err = reject_untransportable_value("proxy_password", payload)
            .expect_err("a value that can open a new TOML key must be refused");
        assert!(
            err.message().contains("proxy_password"),
            "the refusal names the key: {}",
            err.message()
        );
    }

    #[test]
    fn each_offending_class_is_named_rather_than_lumped_together() {
        for (value, expected) in [
            ("a\"b", "a double quote"),
            ("a\\b", "a backslash"),
            ("a\nb", "a control character"),
            ("a\tb", "a control character"),
        ] {
            let err =
                reject_untransportable_value("screen", value).expect_err("value must be refused");
            assert!(
                err.message().contains(expected),
                "expected {expected:?} in: {}",
                err.message()
            );
        }
    }

    #[test]
    fn ordinary_values_still_pass() {
        // The refusal has to stay narrow enough that the keys it guards remain
        // usable: a proxy URL carries `:`, `/` and `=`, and none of those break
        // the line-oriented reader.
        for value in [
            "http://user:pass@proxy.example:8080/path?a=b",
            "1920x1080",
            "Mozilla/5.0 (X11; Linux x86_64)",
            "",
        ] {
            assert!(
                reject_untransportable_value("proxy_url", value).is_ok(),
                "must accept {value:?}"
            );
        }
    }
}
