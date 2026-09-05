// SPDX-License-Identifier: MIT OR Apache-2.0
//! Streaming `run --script -`: NDJSON from stdin, one live browser (GAP-034 pillar 4).
//!
//! # Why this is still one-shot
//!
//! The one-shot law bounds the **number** of processes, not their **duration**.
//! This is one BORN and one DIE with the browser alive in between; there is no
//! socket, no daemon, no state written for a later invocation to find. The
//! caller holds the session open exactly as long as it keeps stdin open, and
//! EOF is the FINALIZE trigger.
//!
//! # Difference from file mode
//!
//! File mode pre-flights the whole script before BORN, so a typo never launches
//! Chrome. A stream has no end to validate up front, so each line is validated
//! as it arrives. The trade is stated in the envelope as `validation: "per-line"`
//! rather than left for the caller to discover.
//!
//! # Failure policy
//!
//! Fail-fast matches file mode: the first failing line stops the loop and the
//! envelope carries the steps already executed.

use serde_json::{json, Value};

use crate::browser::{CaptureOpts, OneShotSession};
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;

use super::execute::execute_step;
use super::flags::RunFlags;

/// Marker that selects stream mode on `--script`.
pub const STDIN_MARKER: &str = "-";

/// True when `--script` asked for stdin rather than a file.
pub fn is_stdin_marker(raw: &std::path::Path) -> bool {
    raw.as_os_str() == STDIN_MARKER
}

/// Read one NDJSON step per line from stdin and run each against a live session.
pub async fn run_stream_with_flags(
    life: &Lifecycle,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    flags: RunFlags,
) -> Result<Value, CliError> {
    let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
    life.record_chrome(session.chrome_pid());

    let mut results: Vec<Value> = Vec::new();
    let mut last_ok_data: Option<Value> = None;
    let mut lineno = 0usize;

    loop {
        if life.is_cancelled() {
            let _ = session.shutdown().await;
            life.clear_chrome();
            return Err(CliError::with_suggestion(
                ErrorKind::Cancelled,
                "cancelled by signal (SIGINT/SIGTERM) between stream steps",
                crate::i18n::suggestion_key("retry_after_cancel", None),
            ));
        }

        let Some(line) = next_line().await? else {
            break; // EOF is the FINALIZE trigger.
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        lineno += 1;

        let step = match parse_line(trimmed, lineno) {
            Ok(step) => step,
            Err(e) => {
                let dump = crate::browser::dump_failure_evidence(&mut session, &e);
                let _ = session.shutdown().await;
                life.clear_chrome();
                return Ok(stream_envelope(results, lineno, "", &e, dump));
            }
        };
        let cmd = step
            .get("cmd")
            .or_else(|| step.get("action"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Timed exactly as the file path times it, in `engine.rs`.
        //
        // The two paths already diverge on row keys — this one writes `line`,
        // that one writes `step` — and a timing field present on only one of
        // them would make an agent`s measurement depend on whether its script
        // arrived from a file or a pipe. Measured 2026-09-04: that asymmetry is
        // how a whole investigation ended up reading the wrong code path, and it
        // only surfaced because a NEW field came back null.
        let started = std::time::Instant::now();
        match execute_step(
            &mut session,
            &cmd,
            &step,
            robots,
            flags,
            last_ok_data.as_ref(),
        )
        .await
        {
            Ok(data) => {
                let row = json!({
                    "index": lineno - 1,
                    "line": lineno,
                    "cmd": cmd,
                    "ok": true,
                    "duration_ms": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                    "data": data,
                });
                flags.emit_step_row(&row)?;
                last_ok_data = Some(data);
                results.push(row);
            }
            Err(e) => {
                let dump = crate::browser::dump_failure_evidence(&mut session, &e);
                let row = json!({
                    "index": lineno - 1,
                    "line": lineno,
                    "cmd": cmd,
                    "ok": false,
                    "duration_ms": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                    "error": {
                        "kind": e.kind().as_str(),
                        "message": e.message(),
                        "suggestion": e.suggestion(),
                    }
                });
                flags.emit_step_row(&row)?;
                results.push(row);
                let _ = session.shutdown().await;
                life.clear_chrome();
                return Ok(stream_envelope(results, lineno, &cmd, &e, dump));
            }
        }
    }

    let close = session.shutdown().await;
    life.clear_chrome();
    close?;

    Ok(super::engine::finish_run_envelope(json!({
        "ok": true,
        "mode": "stream",
        "validation": "per-line",
        "total": results.len(),
        "steps": results,
    })))
}

/// Read one line from stdin without pinning the async worker.
async fn next_line() -> Result<Option<String>, CliError> {
    crate::concurrency::read_stdin_line_blocking()
        .await
        .map_err(|e| CliError::new(ErrorKind::Io, format!("stream read stdin: {e}")))
}

/// Parse one NDJSON line into a step object.
fn parse_line(line: &str, lineno: usize) -> Result<Value, CliError> {
    let max = crate::xdg::resolve_max_ndjson_line_bytes();
    if line.len() > max {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "stream line {lineno}: line too large ({} bytes > {max})",
                line.len()
            ),
            crate::i18n::suggestion_key("raise_size_limit", None),
        ));
    }
    let value: Value = crate::json_util::value_from_str(line).map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Data,
            format!("stream line {lineno}: invalid JSON: {e}"),
            crate::i18n::suggestion_key("json_object_payload", None),
        )
    })?;
    if !value.is_object() {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!("stream line {lineno}: each line must be one JSON object"),
            crate::i18n::suggestion_key("json_object_payload", None),
        ));
    }
    if value.get("cmd").is_none() && value.get("action").is_none() {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!("stream line {lineno}: step needs a cmd field"),
            crate::i18n::suggestion_key("json_object_payload", None),
        ));
    }
    Ok(value)
}

/// Fail-fast envelope carrying the steps already executed.
// `needless_pass_by_value` false positive: the value IS consumed, moved into the
// `json!` object below. Macro expansion hides the move from the lint.
#[allow(clippy::needless_pass_by_value)]
fn stream_envelope(
    results: Vec<Value>,
    lineno: usize,
    cmd: &str,
    err: &CliError,
    dump: Option<std::path::PathBuf>,
) -> Value {
    let mut envelope = json!({
        "ok": false,
        "mode": "stream",
        "validation": "per-line",
        "total": results.len(),
        "failed_line": lineno,
        "failed_cmd": cmd,
        "steps": results,
        "error": {
            "kind": err.kind().as_str(),
            "message": format!("run stream fail-fast at line {lineno} cmd={cmd}: {err}"),
            "suggestion": err
                .suggestion()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| crate::i18n::suggestion_key("run_fail_fast", None)),
            "exit_code": err.exit_code(),
        }
    });
    if let Some(path) = dump {
        envelope.as_object_mut().expect("envelope object").insert(
            "failure_dump_path".into(),
            json!(path.display().to_string()),
        );
    }
    super::engine::finish_run_envelope(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn stdin_marker_is_a_single_dash() {
        assert!(is_stdin_marker(Path::new("-")));
        assert!(!is_stdin_marker(Path::new("steps.jsonl")));
        assert!(!is_stdin_marker(Path::new("./-")));
    }

    #[test]
    fn parse_line_accepts_a_step_object() {
        let v = parse_line(r#"{"cmd":"goto","url":"about:blank"}"#, 1).expect("step");
        assert_eq!(v.get("cmd").and_then(|c| c.as_str()), Some("goto"));
    }

    #[test]
    fn parse_line_rejects_array_and_scalar() {
        assert!(parse_line(r#"[{"cmd":"goto"}]"#, 1).is_err());
        assert!(parse_line("42", 1).is_err());
    }

    #[test]
    fn parse_line_requires_cmd_or_action() {
        let err = parse_line(r#"{"url":"about:blank"}"#, 3).expect_err("must fail");
        assert!(err.message().contains("line 3"), "{}", err.message());
        assert!(err.message().contains("cmd"), "{}", err.message());
    }

    #[test]
    fn parse_line_accepts_action_alias() {
        assert!(parse_line(r#"{"action":"view"}"#, 1).is_ok());
    }

    #[test]
    fn stream_envelope_carries_partial_steps_and_dump() {
        let err = CliError::new(ErrorKind::Browser, "boom");
        let rows = vec![json!({"index": 0, "ok": true})];
        let env = stream_envelope(rows, 2, "press", &err, Some("/tmp/d.json".into()));
        assert_eq!(env["ok"], json!(false));
        assert_eq!(env["failed_line"], json!(2));
        assert_eq!(env["mode"], json!("stream"));
        assert_eq!(env["steps"].as_array().map(|a| a.len()), Some(1));
        assert_eq!(env["failure_dump_path"], json!("/tmp/d.json"));
    }
}
