// SPDX-License-Identifier: MIT OR Apache-2.0
//! Turning raw `scrape` argv into the shapes the handler needs.
//!
//! # Why these live next to the dispatcher and not inside the handler
//!
//! Both convert operator input, and both can only fail because the operator
//! typed something wrong. Running them here means a mismatch costs one error
//! message; running them inside the handler would mean discovering the mistake
//! after Chrome has launched and a page has loaded.
//!
//! They are their own file because the dispatcher's job is routing. Argv
//! conversion is a second responsibility, and keeping it here is what holds
//! `scrape.rs` under the file-size gate.

use crate::error::{CliError, ErrorKind};

/// Parse each `--action` value as one `run --script` step.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when a value is not JSON, or is JSON but
/// not an object. The index is named, because with several actions "invalid
/// JSON" does not say which one.
pub(super) fn parse_actions(raw: &[String]) -> Result<Vec<serde_json::Value>, CliError> {
    raw.iter()
        .enumerate()
        .map(|(i, text)| {
            let value: serde_json::Value = serde_json::from_str(text).map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("--action[{i}] is not valid JSON: {e}"),
                    crate::i18n::suggestion_key("use_listed_value", None),
                )
            })?;
            if !value.is_object() {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("--action[{i}] must be a JSON object with a `cmd` field"),
                    crate::i18n::suggestion_key("use_listed_value", None),
                ));
            }
            Ok(value)
        })
        .collect()
}

/// Zip `--attribute-selector` with `--attribute-name`, or explain the mismatch.
///
/// # Why unequal counts are rejected rather than truncated
///
/// Truncating to the shorter list drops a question the caller believes it
/// asked, and the envelope would come back with fewer rows than selectors and
/// no indication why. Failing at argv time costs one error message and leaves
/// no wrong answer behind.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when the two lists differ in length.
pub(super) fn pair_attribute_targets(
    selectors: &[String],
    names: &[String],
) -> Result<Vec<(String, String)>, CliError> {
    if selectors.len() != names.len() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!(
                "--attribute-selector was given {} time(s) and --attribute-name {}; they pair one to one",
                selectors.len(),
                names.len()
            ),
            crate::i18n::suggestion_key("use_listed_value", None),
        ));
    }
    Ok(selectors
        .iter()
        .cloned()
        .zip(names.iter().cloned())
        .collect())
}
#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn equal_counts_pair_in_order() {
        let pairs = pair_attribute_targets(&v(&["a", "img"]), &v(&["href", "src"])).expect("pairs");
        assert_eq!(pairs[0], ("a".to_string(), "href".to_string()));
        assert_eq!(pairs[1], ("img".to_string(), "src".to_string()));
    }

    #[test]
    fn a_mismatch_fails_instead_of_dropping_a_question() {
        let err = pair_attribute_targets(&v(&["a", "img"]), &v(&["href"]))
            .expect_err("unequal counts must fail");
        assert_eq!(err.kind(), ErrorKind::Usage);
    }

    #[test]
    fn no_attribute_flags_is_not_an_error() {
        assert!(pair_attribute_targets(&[], &[]).expect("empty").is_empty());
    }

    #[test]
    fn an_action_parses_as_one_run_step() {
        let steps = parse_actions(&v(&[r##"{"cmd":"press","target":"#go"}"##])).expect("parse");
        assert_eq!(steps[0]["cmd"], serde_json::json!("press"));
    }

    #[test]
    fn a_malformed_action_names_its_index() {
        // With five actions, "invalid JSON" alone sends the caller hunting.
        let err = parse_actions(&v(&["{}", "not json"])).expect_err("must fail");
        assert!(err.message().contains("[1]"), "{}", err.message());
    }

    #[test]
    fn an_action_that_is_not_an_object_is_rejected() {
        assert!(parse_actions(&v(&["[1,2]"])).is_err());
    }
}
