// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared emit helpers and meta handlers for command dispatch.

use crate::envelope::{print_error_json, print_success_json};
use crate::error::{CliError, ErrorKind};

pub(crate) fn handle_version(json: bool) -> Result<(), CliError> {
    let data = crate::build_identity();
    emit_ok(data, json, |d| {
        let ver = d.get("version").and_then(|v| v.as_str()).unwrap_or("");
        let sha = d
            .get("git_sha")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        if sha != "unknown" {
            crate::output::writeln_stdout(format!("{ver} ({sha})"))
        } else {
            crate::output::writeln_stdout(ver)
        }
    })
}

pub(crate) fn handle_locale(json: bool) -> Result<(), CliError> {
    let data = crate::i18n::locale_diagnostics();
    emit_ok(data, json, |d| {
        let resolved = d.get("resolved").and_then(|v| v.as_str()).unwrap_or("en");
        let source = d
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let system = d
            .get("system_locale")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        // Human labels follow resolved locale; values stay BCP47/English keys.
        let label_r =
            crate::i18n::UiMessage::LocaleResolved.text(crate::i18n::effective_ui_locale());
        let label_s = crate::i18n::UiMessage::LocaleSource.text(crate::i18n::effective_ui_locale());
        crate::output::writeln_stdout(format!(
            "{label_r}: {resolved}\n{label_s}: {source}\nsystem: {system}"
        ))
    })
}

/// Render one envelope as a single `key=value` line for the text mode.
///
/// # The shape this replaces
///
/// Thirteen commands used to answer `ok <verb> {json}` when `--json` was NOT
/// passed: the serialised payload with a human prefix glued to its front. That
/// is neither of the two things a caller can use. A human cannot read it, and a
/// machine cannot parse it without first stripping a prefix that no format
/// describes. Measured 2026-08-30: `config list-keys` emitted 23_248 bytes on a
/// SINGLE line that way, the whole key catalogue with descriptions included.
///
/// # The rules, and why each one
///
/// - Scalars render as `key=value`, the shape `page` already used, which a
///   shell can split without a JSON parser.
/// - Arrays and objects render as their CARDINALITY, never their contents. In
///   text mode the question being asked is "how many"; whoever wants the items
///   passes `--json`. This one rule is what turns 23 KB back into one line.
/// - `null` renders as `key=null` instead of being dropped. A field that
///   vanishes when empty is indistinguishable from a field that never existed,
///   and that ambiguity is the exact defect this release spent its time closing.
/// - A value carrying whitespace, a quote or an equals sign is quoted, so
///   `title=Hello World` cannot be misread as two fields.
///
/// Key order follows the envelope's own order, which `serde_json` keeps stable,
/// so the same command answers with the same line every time.
fn summary_line(verb: &str, data: &serde_json::Value) -> String {
    let mut out = format!("ok {verb}");
    match data {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                out.push(' ');
                out.push_str(k);
                out.push('=');
                out.push_str(&render_value(v));
            }
        }
        // A non-object envelope has no keys to name, so the value stands alone
        // rather than being wrapped in an invented field.
        other => {
            out.push(' ');
            out.push_str(&render_value(other));
        }
    }
    out
}

/// Render one JSON value as the right-hand side of a `key=value` pair.
fn render_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => quote_if_needed(s),
        serde_json::Value::Array(a) => format!("<{} items>", a.len()),
        serde_json::Value::Object(o) => format!("<{} fields>", o.len()),
    }
}

/// Quote a string only when leaving it bare would be ambiguous.
///
/// Bare values keep the common case readable, and a value holding a separator
/// would otherwise split into what looks like two fields. The quoting itself is
/// `str`'s own `Debug`, so the escape table is the language's rather than one
/// maintained here, which is one fewer place for this file to be wrong.
fn quote_if_needed(s: &str) -> String {
    let needs = s.is_empty()
        || s.chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '=' || c == char::from(92));
    if needs {
        format!("{s:?}")
    } else {
        s.to_string()
    }
}

/// Emit `data` as JSON, or as the one-line summary described by [`summary_line`].
///
/// # Errors
///
/// Propagates serialisation and stdout failures.
pub(crate) fn emit_ok_summary(
    data: serde_json::Value,
    json: bool,
    verb: &str,
) -> Result<(), CliError> {
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(summary_line(verb, d))
    })
}

pub(crate) fn emit_ok<F>(data: serde_json::Value, json: bool, text: F) -> Result<(), CliError>
where
    F: FnOnce(&serde_json::Value) -> Result<(), CliError>,
{
    if json {
        print_success_json(data)?;
    } else {
        text(&data)?;
    }
    crate::output::flush_stdout()?;
    Ok(())
}

pub(crate) fn emit_err(err: &CliError, json: bool) -> i32 {
    let localized = crate::i18n::localize_error_suggestion(err);
    if json {
        match print_error_json(&localized) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::BrokenPipe => return 141,
            Err(_) => {}
        }
    } else {
        let _ = crate::output::writeln_stderr(format!("error: {localized}"));
        if let Some(s) = localized.suggestion() {
            let _ = crate::output::writeln_stderr(format!("suggestion: {s}"));
        }
    }
    let _ = crate::output::flush_stdout();
    localized.exit_code() as i32
}

/// Peel known global flags mistakenly captured by `exec` trailing_var_arg.
pub(crate) fn peel_trailing_globals(args: &[String]) -> (Vec<String>, bool) {
    let mut json = false;
    let mut out = Vec::with_capacity(args.len());
    for a in args {
        match a.as_str() {
            "--json" => json = true,
            other => out.push(other.to_string()),
        }
    }
    (out, json)
}

#[cfg(test)]
mod summary_tests {
    use super::summary_line;
    use serde_json::json;

    #[test]
    fn scalars_render_as_key_value_pairs() {
        let v = json!({"count": 3, "ok": true, "path": "/tmp/x.json"});
        assert_eq!(
            summary_line("console", &v),
            "ok console count=3 ok=true path=/tmp/x.json"
        );
    }

    /// The rule that turns a 23 KB line back into a readable one.
    ///
    /// `config list-keys` used to serialise its whole catalogue here. If this
    /// test ever fails because someone rendered contents instead of a count,
    /// the regression is that exact 23 KB line coming back.
    #[test]
    fn collections_render_as_cardinality_never_contents() {
        let v = json!({"keys": [1, 2, 3], "meta": {"a": 1, "b": 2}});
        assert_eq!(
            summary_line("config", &v),
            "ok config keys=<3 items> meta=<2 fields>"
        );
    }

    /// Absence must stay visible, which is the whole subject of this release.
    #[test]
    fn null_is_named_rather_than_dropped() {
        let v = json!({"status": null});
        assert_eq!(summary_line("net", &v), "ok net status=null");
    }

    #[test]
    fn a_value_with_a_space_cannot_split_into_two_fields() {
        let v = json!({"title": "Hello World"});
        let line = summary_line("page", &v);
        assert_eq!(line, "ok page title=\"Hello World\"");
        // One separator means one field, which is what the quoting buys.
        assert_eq!(line.split(' ').count(), 4);
    }

    #[test]
    fn an_embedded_quote_or_equals_is_escaped() {
        let v = json!({"q": "a=b", "r": "say \"hi\""});
        assert_eq!(
            summary_line("exec", &v),
            "ok exec q=\"a=b\" r=\"say \\\"hi\\\"\""
        );
    }

    #[test]
    fn an_empty_string_is_quoted_so_the_field_stays_visible() {
        let v = json!({"note": ""});
        assert_eq!(summary_line("wait", &v), "ok wait note=\"\"");
    }

    #[test]
    fn an_empty_envelope_is_just_the_verb() {
        assert_eq!(summary_line("qr", &json!({})), "ok qr");
    }

    /// A non-object envelope has no keys, so the value stands on its own.
    #[test]
    fn a_non_object_envelope_still_renders() {
        assert_eq!(summary_line("perf", &json!([1, 2])), "ok perf <2 items>");
        assert_eq!(summary_line("perf", &json!("done")), "ok perf done");
    }

    /// No output of this helper may ever contain a raw JSON payload.
    #[test]
    fn a_deep_payload_never_reaches_the_line() {
        let big: Vec<_> = (0..500)
            .map(|i| json!({"key": format!("k{i}"), "description": "x"}))
            .collect();
        let line = summary_line("config", &json!({"keys": big, "path": "/tmp/c"}));
        assert_eq!(line, "ok config keys=<500 items> path=/tmp/c");
        assert!(line.len() < 80, "line must stay readable: {line}");
        assert!(
            !line.contains("description"),
            "payload leaked into text mode: {line}"
        );
    }
}
