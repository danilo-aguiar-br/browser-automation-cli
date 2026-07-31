// SPDX-License-Identifier: MIT OR Apache-2.0
//! Assert step executor for run/exec.

use serde_json::{json, Value};

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};

/// Assert over the previous step's payload (GAP-038).
///
/// Without this, `exit 0` means only "the automation ran", so a domain failure
/// reported *inside* an `eval` result still exits zero and every script has to
/// re-extract the verdict itself. `kind: "step"` moves that verdict into the
/// process exit code, failing the run like any other step.
///
/// Forms:
/// - `{"cmd":"assert","kind":"step","path":"result","equals":"OK"}`
/// - `{"cmd":"assert","kind":"step","path":"result.status","contains":"pass"}`
/// - `{"cmd":"assert","kind":"step","path":"console_count","exists":true}`
fn assert_previous_step(step: &Value, prev: Option<&Value>) -> Result<Value, CliError> {
    let path = step
        .get("path")
        .or_else(|| step.get("json_path"))
        .or_else(|| step.get("jsonPath"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            CliError::with_suggestion(
                ErrorKind::Usage,
                "assert step requires path",
                crate::i18n::suggestion_key("assert_step_path", None),
            )
        })?;

    let Some(prev) = prev else {
        return Err(CliError::with_suggestion(
            ErrorKind::Precondition,
            "assert step has no previous step to read",
            crate::i18n::suggestion_key("assert_step_order", None),
        ));
    };

    let found = lookup_path(prev, path);

    if let Some(expect_exists) = step.get("exists").and_then(|v| v.as_bool()) {
        let present = found.is_some();
        if present != expect_exists {
            return Err(assertion_failed(format!(
                "assert step failed: path={path:?} exists={present} expected_exists={expect_exists}"
            )));
        }
        return Ok(json!({ "assert": "step", "ok": true, "path": path, "exists": present }));
    }

    let actual = found.ok_or_else(|| {
        assertion_failed(format!(
            "assert step failed: path={path:?} not present in previous step payload"
        ))
    })?;

    if let Some(expected) = step.get("equals") {
        if actual != expected {
            return Err(assertion_failed(format!(
                "assert step failed: path={path:?} got={actual} expected={expected}"
            )));
        }
        return Ok(json!({
            "assert": "step", "ok": true, "path": path, "value": actual, "equals": expected
        }));
    }

    if let Some(needle) = step.get("contains").and_then(|v| v.as_str()) {
        let hay = value_to_haystack(actual);
        if !hay.contains(needle) {
            return Err(assertion_failed(format!(
                "assert step failed: path={path:?} got={hay:?} expected to contain {needle:?}"
            )));
        }
        return Ok(json!({
            "assert": "step", "ok": true, "path": path, "value": actual, "contains": needle
        }));
    }

    Err(CliError::with_suggestion(
        ErrorKind::Usage,
        "assert step requires one of equals, contains or exists",
        crate::i18n::suggestion_key("assert_step_operator", None),
    ))
}

/// Domain-assertion failure: same kind the url/text asserts already use.
fn assertion_failed(message: String) -> CliError {
    CliError::with_suggestion(
        ErrorKind::Data,
        message,
        crate::i18n::suggestion_key("assert_step_inspect", None),
    )
}

/// Resolve a dotted path, descending objects by key and arrays by index.
///
/// A leading `data.` is optional: run rows nest the payload under `data`, and an
/// agent writing `result` should not have to know that.
fn lookup_path<'v>(root: &'v Value, path: &str) -> Option<&'v Value> {
    descend(root, path).or_else(|| root.get("data").and_then(|d| descend(d, path)))
}

fn descend<'v>(root: &'v Value, path: &str) -> Option<&'v Value> {
    let mut cur = root;
    for seg in path.split('.').filter(|s| !s.is_empty()) {
        cur = match cur {
            Value::Array(items) => seg.parse::<usize>().ok().and_then(|i| items.get(i))?,
            _ => cur.get(seg)?,
        };
    }
    Some(cur)
}

/// Render a value for substring matching without quoting plain strings.
fn value_to_haystack(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

pub(super) async fn execute_assert(
    session: &mut OneShotSession,
    step: &Value,
    prev: Option<&Value>,
) -> Result<Value, CliError> {
    // Forms:
    // {"cmd":"assert","kind":"url","value":"...","contains":true}
    // {"cmd":"assert","kind":"url","url_contains":"..."}
    // {"cmd":"assert","kind":"text","value":"...","ref":"@e1"}
    // {"cmd":"assert","kind":"console","level":"error","max":0}
    // {"cmd":"assert","url":"..."} / {"cmd":"assert","text":"..."}
    // {"cmd":"assert","url_contains":"..."} / {"cmd":"assert","text_contains":"..."}
    if let Some(kind) = step.get("kind").and_then(|v| v.as_str()) {
        match kind {
            "url" => {
                let value = step
                    .get("value")
                    .or_else(|| step.get("url_contains"))
                    .or_else(|| step.get("url"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        CliError::with_suggestion(
                            ErrorKind::Usage,
                            "assert url requires value",
                            "Use {\"cmd\":\"assert\",\"kind\":\"url\",\"value\":\"example.com\"} or url_contains",
                        )
                    })?;
                let contains = step
                    .get("contains")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                return session.assert_url(value, contains).await;
            }
            "text" => {
                let value = step
                    .get("value")
                    .or_else(|| step.get("text_contains"))
                    .or_else(|| step.get("text"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        CliError::with_suggestion(
                            ErrorKind::Usage,
                            "assert text requires value",
                            "Use {\"cmd\":\"assert\",\"kind\":\"text\",\"value\":\"Hello\"}",
                        )
                    })?;
                let target = step
                    .get("ref")
                    .or_else(|| step.get("target"))
                    .and_then(|v| v.as_str());
                return session.assert_text(value, target).await;
            }
            "console" => {
                let level = step
                    .get("level")
                    .and_then(|v| v.as_str())
                    .unwrap_or("error");
                let max = step.get("max").and_then(|v| v.as_u64()).unwrap_or(0);
                return session.assert_console(level, max).await;
            }
            // GAP-025
            "console_empty" | "console-empty" => {
                return session.assert_console_empty().await;
            }
            // GAP-038: domain verdict from the previous step's payload.
            "step" | "result" | "previous" => {
                return assert_previous_step(step, prev);
            }
            "console_no_match" | "console-no-match" => {
                let pattern = step
                    .get("pattern")
                    .or_else(|| step.get("text"))
                    .or_else(|| step.get("value"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        CliError::with_suggestion(
                            ErrorKind::Usage,
                            "assert console_no_match requires pattern",
                            "Use {\"cmd\":\"assert\",\"kind\":\"console_no_match\",\"pattern\":\"TypeError\"}",
                        )
                    })?;
                return session.assert_console_no_match(pattern).await;
            }
            other => {
                return Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown assert kind: {other}"),
                ));
            }
        }
    }
    if let Some(url) = step
        .get("url_contains")
        .or_else(|| step.get("url"))
        .and_then(|v| v.as_str())
    {
        let contains = step
            .get("contains")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        return session.assert_url(url, contains).await;
    }
    if let Some(text) = step
        .get("text_contains")
        .or_else(|| step.get("text"))
        .and_then(|v| v.as_str())
    {
        let target = step
            .get("ref")
            .or_else(|| step.get("target"))
            .and_then(|v| v.as_str());
        return session.assert_text(text, target).await;
    }
    Err(CliError::with_suggestion(
        ErrorKind::Usage,
        "assert requires kind=url|text|console|console_empty|console_no_match|step or url/text/url_contains fields",
        "Example: {\"cmd\":\"assert\",\"kind\":\"step\",\"path\":\"result\",\"equals\":\"OK\"}",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::needless_pass_by_value)] // moved into json! below
    fn prev(payload: serde_json::Value) -> serde_json::Value {
        json!({ "data": payload })
    }

    /// GAP-038: a domain failure inside the payload fails the step.
    #[test]
    fn equals_mismatch_fails_the_step() {
        let step = json!({"cmd":"assert","kind":"step","path":"result","equals":"OK"});
        let err = assert_previous_step(&step, Some(&prev(json!({"result":"FAILED"}))))
            .expect_err("mismatch must fail");
        assert_eq!(err.kind(), ErrorKind::Data);
        assert_eq!(err.exit_code(), 65);
    }

    #[test]
    fn equals_match_passes() {
        let step = json!({"cmd":"assert","kind":"step","path":"result","equals":"OK"});
        let out = assert_previous_step(&step, Some(&prev(json!({"result":"OK"})))).expect("match");
        assert_eq!(out["ok"], json!(true));
    }

    /// The `data.` wrapper is optional for the caller.
    #[test]
    fn path_resolves_with_or_without_data_prefix() {
        let p = prev(json!({"result": {"status": "pass"}}));
        for path in ["result.status", "data.result.status"] {
            let step = json!({"cmd":"assert","kind":"step","path":path,"equals":"pass"});
            assert!(assert_previous_step(&step, Some(&p)).is_ok(), "path {path}");
        }
    }

    #[test]
    fn contains_matches_substring_of_a_string() {
        let step = json!({"cmd":"assert","kind":"step","path":"result","contains":"pass"});
        assert!(assert_previous_step(&step, Some(&prev(json!({"result":"all passed"})))).is_ok());
        assert!(assert_previous_step(&step, Some(&prev(json!({"result":"all failed"})))).is_err());
    }

    #[test]
    fn array_indices_are_addressable() {
        let p = prev(json!({"rows": [{"v": 1}, {"v": 2}]}));
        let step = json!({"cmd":"assert","kind":"step","path":"rows.1.v","equals":2});
        assert!(assert_previous_step(&step, Some(&p)).is_ok());
    }

    #[test]
    fn exists_checks_presence_without_a_value() {
        let p = prev(json!({"console_count": 0}));
        let yes = json!({"cmd":"assert","kind":"step","path":"console_count","exists":true});
        assert!(assert_previous_step(&yes, Some(&p)).is_ok());
        let no = json!({"cmd":"assert","kind":"step","path":"nope","exists":false});
        assert!(assert_previous_step(&no, Some(&p)).is_ok());
        let wrong = json!({"cmd":"assert","kind":"step","path":"nope","exists":true});
        assert!(assert_previous_step(&wrong, Some(&p)).is_err());
    }

    /// A missing path is a domain failure, never a silent pass.
    #[test]
    fn missing_path_fails_rather_than_passing() {
        let step = json!({"cmd":"assert","kind":"step","path":"absent","equals":1});
        let err = assert_previous_step(&step, Some(&prev(json!({"result":1}))))
            .expect_err("missing path must fail");
        assert_eq!(err.kind(), ErrorKind::Data);
    }

    /// Asserting before any step ran is a precondition error, not a pass.
    #[test]
    fn no_previous_step_is_a_precondition_error() {
        let step = json!({"cmd":"assert","kind":"step","path":"result","equals":1});
        let err = assert_previous_step(&step, None).expect_err("no previous step must fail");
        assert_eq!(err.kind(), ErrorKind::Precondition);
        assert_eq!(err.exit_code(), 75);
    }

    #[test]
    fn missing_operator_is_usage() {
        let step = json!({"cmd":"assert","kind":"step","path":"result"});
        let err = assert_previous_step(&step, Some(&prev(json!({"result":1}))))
            .expect_err("no operator must fail");
        assert_eq!(err.kind(), ErrorKind::Usage);
    }

    #[test]
    fn missing_path_field_is_usage() {
        let step = json!({"cmd":"assert","kind":"step","equals":1});
        let err = assert_previous_step(&step, Some(&prev(json!({"result":1}))))
            .expect_err("no path must fail");
        assert_eq!(err.kind(), ErrorKind::Usage);
    }
}
