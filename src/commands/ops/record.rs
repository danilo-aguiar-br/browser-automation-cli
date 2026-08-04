// SPDX-License-Identifier: MIT OR Apache-2.0
//! `record` handler: drive one session, write replayable NDJSON, report a summary.
//!
//! # Why the file is the product and stdout is only the receipt
//!
//! The recorded steps go to `--path` because that file is what `run --script`
//! consumes; printing them on stdout as well would double every gesture in the
//! envelope for no consumer. The envelope reports how many steps were written
//! and whether the event ceiling truncated the recording, which is what a caller
//! needs to decide if the capture is complete.

use std::path::Path;

use serde_json::{json, Value};

use crate::browser::CaptureOpts;
use crate::commands::common::emit_ok;
use crate::commands::nav::with_session_blank;
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;

/// Record interactions at `url` into an NDJSON script at `path`.
///
/// # Errors
///
/// - [`ErrorKind::Usage`] when either ceiling is zero: a recorder that stops
///   before it starts would write an empty script and report success.
/// - [`ErrorKind::Io`] when `path` is outside the allowed roots or unwritable.
/// - Whatever the session reports for launch, navigation, or CDP failures.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_record(
    life: &Lifecycle,
    url: &str,
    path: &Path,
    seconds: u64,
    max_events: u64,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    let max_events = usize::try_from(require_positive(max_events, "--max-events")?)
        .map_err(|_| CliError::new(ErrorKind::Usage, "--max-events exceeds this platform"))?;
    let seconds = require_positive(seconds, "--seconds")?;

    let url_owned = url.to_string();
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session
            .record_interactions(&url_owned, robots, seconds, max_events)
            .await?;
        Ok((session, v))
    })?;

    let written = write_ndjson(path, data.get("steps"))?;
    let summary = json!({
        "action": "record",
        "path": written,
        "events": data.get("events").cloned().unwrap_or(Value::Null),
        "truncated": data.get("truncated").cloned().unwrap_or(Value::Null),
        "seconds": data.get("seconds").cloned().unwrap_or(Value::Null),
    });
    emit_ok(summary, json, |d| {
        let events = d.get("events").and_then(Value::as_u64).unwrap_or(0);
        let path = d.get("path").and_then(Value::as_str).unwrap_or("");
        crate::output::writeln_stdout(format!("ok record events={events} path={path}"))
    })
}

/// Reject a zero ceiling, naming the flag the caller must fix.
fn require_positive(value: u64, flag: &str) -> Result<u64, CliError> {
    if value == 0 {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("{flag} must be at least 1"),
            crate::i18n::suggestion_key("use_listed_value", None),
        ));
    }
    Ok(value)
}

/// Serialise the recorded steps as NDJSON and write them; returns the real path.
///
/// One step per line, no trailing blank line: that is exactly the shape
/// `run --script` parses, so the file needs no post-processing.
fn write_ndjson(path: &Path, steps: Option<&Value>) -> Result<String, CliError> {
    let mut body = String::new();
    for step in steps.and_then(Value::as_array).into_iter().flatten() {
        body.push_str(&serde_json::to_string(step).map_err(|e| {
            CliError::new(ErrorKind::Software, format!("serialise record step: {e}"))
        })?);
        body.push('\n');
    }
    let target = crate::fs_roots::ensure_write_allowed(path)?;
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Io,
                    format!("record output dir: {e}"),
                    crate::i18n::suggestion_key("file_path_invalid", None),
                )
            })?;
        }
    }
    std::fs::write(&target, body).map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Io,
            format!("record output: {e}"),
            crate::i18n::suggestion_key("file_path_invalid", None),
        )
    })?;
    Ok(target.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_ceiling_is_refused_as_usage() {
        let err = require_positive(0, "--seconds").expect_err("must refuse");
        assert_eq!(err.kind(), ErrorKind::Usage);
        assert!(err.message().contains("--seconds"), "{}", err.message());
        assert!(require_positive(1, "--seconds").is_ok());
    }

    #[test]
    fn ndjson_is_one_step_per_line_with_no_trailing_blank() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("steps.jsonl");
        let steps = json!([{ "cmd": "goto", "url": "https://example.com/" }, { "cmd": "press", "target": "#go" }]);
        write_ndjson(&path, Some(&steps)).expect("write");
        let body = std::fs::read_to_string(&path).expect("read");
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(lines.len(), 2, "one line per step: {body:?}");
        assert!(body.ends_with('\n'), "each line terminated: {body:?}");
        for line in lines {
            serde_json::from_str::<Value>(line).expect("each line is one JSON object");
        }
    }
}
