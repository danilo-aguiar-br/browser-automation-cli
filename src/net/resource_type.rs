// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP `Network.ResourceType` vocabulary parsing for capture filters.
//!
//! # Why one reader serves two surfaces
//!
//! `--resource-types` reaches the product twice: through clap on the argv
//! surface, and as a `run --script` step key that never touches clap. Two
//! readers drift, and a drifted reader rejects a step that would have run, so
//! the CSV is parsed and validated here once and both surfaces call in.
//!
//! # Why an unknown token is refused instead of filtered away
//!
//! Until this change the filter compared against a key the capture log never
//! wrote, so every value answered zero. A typo and a genuinely absent resource
//! type were indistinguishable, and both looked like success. Refusing the
//! unknown token restores the difference: an empty result now means the page
//! had no such resource, and nothing else.

use crate::constants::CDP_RESOURCE_TYPES;

/// Parse a comma-separated resource-type filter into lowercase tokens.
///
/// Comparison is case-insensitive, so `document` and `Document` both select
/// `Document`. Matching is exact rather than substring: `s` used to reach
/// `Script`, `Stylesheet` and `SignedExchange` at once. No caller can have
/// depended on that, because the filter never matched anything at all.
///
/// Empty segments are skipped, so a trailing comma is not an error, and
/// repeated tokens collapse.
///
/// # Errors
///
/// Returns the offending token and the accepted vocabulary when a token is not
/// a CDP resource type.
pub fn parse_resource_types(raw: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for token in raw.split(',') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !CDP_RESOURCE_TYPES
            .iter()
            .any(|known| known.eq_ignore_ascii_case(trimmed))
        {
            return Err(format!(
                "unknown resource type '{trimmed}' (accepted: {})",
                CDP_RESOURCE_TYPES.join(", ")
            ));
        }
        let lowered = trimmed.to_ascii_lowercase();
        if !out.contains(&lowered) {
            out.push(lowered);
        }
    }
    Ok(out)
}

/// clap `value_parser` adapter: validate, then hand back the original string.
///
/// Returning `String` rather than the parsed vector is deliberate. The schemas
/// under `docs/schemas/` are projected from the parser's value-parser type id,
/// so a richer return type would silently retype the published
/// `--resource-types` property. Validation still happens here, which is before
/// any browser launch: a rejected token costs a parse instead of a Chrome.
///
/// # Errors
///
/// Propagates [`parse_resource_types`].
pub fn validate_resource_types_arg(raw: &str) -> Result<String, String> {
    parse_resource_types(raw)?;
    Ok(raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_protocol_casing_and_lowercases() {
        assert_eq!(
            parse_resource_types("Document,XHR").unwrap(),
            vec!["document".to_string(), "xhr".to_string()]
        );
    }

    #[test]
    fn accepts_lowercase_input() {
        assert_eq!(
            parse_resource_types("document").unwrap(),
            vec!["document".to_string()]
        );
    }

    #[test]
    fn skips_empty_segments_and_collapses_repeats() {
        assert_eq!(
            parse_resource_types("Font, ,Font,").unwrap(),
            vec!["font".to_string()]
        );
    }

    #[test]
    fn other_is_selectable_because_it_is_a_real_variant() {
        assert_eq!(
            parse_resource_types("Other").unwrap(),
            vec!["other".to_string()]
        );
    }

    #[test]
    fn refuses_typo_and_names_it() {
        let err = parse_resource_types("Documnet").unwrap_err();
        assert!(
            err.contains("Documnet"),
            "message must name the token: {err}"
        );
        assert!(err.contains("Document"), "message must show the vocabulary");
    }

    #[test]
    fn refuses_substring_that_the_old_contains_match_would_have_accepted() {
        // `s` reached Script, Stylesheet and SignedExchange under `contains`.
        assert!(parse_resource_types("s").is_err());
    }

    #[test]
    fn empty_filter_selects_everything_by_returning_no_tokens() {
        assert!(parse_resource_types("").unwrap().is_empty());
    }

    #[test]
    fn arg_adapter_returns_the_input_unchanged() {
        assert_eq!(
            validate_resource_types_arg("Document,XHR").unwrap(),
            "Document,XHR".to_string()
        );
    }
}
