// SPDX-License-Identifier: MIT OR Apache-2.0
//! Parse NDJSON / JSON-array run scripts (rules_rust_json_e_ndjson).

use serde_json::Value;

use crate::error::{CliError, ErrorKind};

/// Parse a `run` script body as NDJSON objects and/or a top-level JSON array (GAP-A003).
///
/// Rules (`rules_rust_json_e_ndjson`):
/// - BOM-aware; LF-delimited NDJSON; one complete JSON value per line
/// - Per-line size ceiling via XDG `max_ndjson_line_bytes` (default
///   [`crate::json_util::MAX_NDJSON_LINE_BYTES`])
/// - No pretty-print mixing; root of each step must be an object after normalize
pub fn parse_run_script(text: &str) -> Result<Vec<Value>, CliError> {
    let text = crate::json_util::strip_utf8_bom(text);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let max_line = crate::xdg::resolve_max_ndjson_line_bytes();

    // Whole-file JSON array (common agent shape).
    if trimmed.starts_with('[') {
        match crate::json_util::value_from_str(trimmed) {
            Ok(Value::Array(items)) => {
                return normalize_step_values(items, "script array");
            }
            Ok(_) => {
                return Err(CliError::with_suggestion(
                    ErrorKind::Data,
                    "script starts with '[' but is not a JSON array of step objects",
                    crate::i18n::suggestion_key("run_script_array_shape", None),
                ));
            }
            Err(e) => {
                // Fall through to line mode if multi-line NDJSON accidentally starts with [
                // only when parse fails for a pure array file.
                if !trimmed.contains('\n') {
                    return Err(CliError::with_suggestion(
                        ErrorKind::Data,
                        format!("invalid JSON array script: {e}"),
                        crate::i18n::suggestion_key("run_array_element_object", None),
                    ));
                }
            }
        }
    }

    let mut steps: Vec<Value> = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        crate::json_util::check_ndjson_line_len_max(line, lineno + 1, max_line)?;
        let v: Value = crate::json_util::value_from_str(line).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Data,
                format!("script line {}: invalid JSON: {e}", lineno + 1),
                crate::i18n::suggestion_key("run_ndjson_line_object", None),
            )
        })?;
        match v {
            Value::Array(items) if steps.is_empty() && lineno == 0 => {
                // Single-line array as the only content.
                return normalize_step_values(items, &format!("script line {}", lineno + 1));
            }
            Value::Array(_) => {
                return Err(CliError::with_suggestion(
                    ErrorKind::Data,
                    format!(
                        "script line {}: nested JSON array not allowed in NDJSON mode",
                        lineno + 1
                    ),
                    crate::i18n::suggestion_key("run_script_array_or_ndjson", None),
                ));
            }
            other => steps.push(other),
        }
    }
    normalize_step_values(steps, "script")
}

fn normalize_step_values(items: Vec<Value>, ctx: &str) -> Result<Vec<Value>, CliError> {
    let mut out = Vec::with_capacity(items.len());
    for (i, v) in items.into_iter().enumerate() {
        if !v.is_object() {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("{ctx} step {i}: expected object with \"cmd\""),
                crate::i18n::suggestion_key("run_step_object_example", None),
            ));
        }
        out.push(v);
    }
    Ok(out)
}

/// True when the step requests auto-handling of beforeunload dialogs (GAP-A009).
/// Accepts bool `true` or string `"accept"` / `"dismiss"` (dismiss still arms the handler).
/// GAP-003: tool-ref handleBeforeUnload is accept|dismiss (off when absent/false).
#[cfg(test)]
mod parse_script_tests {
    use super::*;

    #[test]
    fn parses_ndjson_objects() {
        let text = r#"{"cmd":"goto","url":"https://example.com"}
{"cmd":"eval","expression":"1+1"}
"#;
        let steps = parse_run_script(text).unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0]["cmd"], "goto");
    }

    #[test]
    fn parses_json_array() {
        let text = r#"[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"reload","ignore_cache":true}
]"#;
        let steps = parse_run_script(text).unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[1]["cmd"], "reload");
    }

    #[test]
    fn parses_single_line_array() {
        let text = r#"[{"cmd":"goto","url":"about:blank"}]"#;
        let steps = parse_run_script(text).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn parses_ndjson_with_utf8_bom() {
        let text = "\u{FEFF}{\"cmd\":\"goto\",\"url\":\"https://example.com\"}\n";
        let steps = parse_run_script(text).unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0]["cmd"], "goto");
    }

    #[test]
    fn rejects_oversized_ndjson_line() {
        let huge = format!(
            "{{\"cmd\":\"eval\",\"expression\":\"{}\"}}",
            "x".repeat(crate::json_util::MAX_NDJSON_LINE_BYTES)
        );
        assert!(parse_run_script(&huge).is_err());
    }

    #[test]
    fn quoted_one_line_eval_keeps_js_regex_and_quotes() {
        let text = "{\"cmd\":\"eval\",\"expression\":\"(/hello/i).test(\\\"hello\\\")\"}\n";
        let steps = parse_run_script(text).unwrap();
        assert_eq!(steps[0]["expression"], "(/hello/i).test(\"hello\")");
    }

    #[test]
    fn a_split_json_string_is_invalid_ndjson() {
        let text = "{\"cmd\":\"eval\",\"expression\":\"(/hello/i).test(\n\\\"hello\\\")\"}\n";
        let err = parse_run_script(text).unwrap_err();
        assert!(
            err.message().contains("invalid JSON") || err.message().contains("EOF"),
            "{}",
            err.message()
        );
    }
}
