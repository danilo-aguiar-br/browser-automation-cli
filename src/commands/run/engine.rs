// SPDX-License-Identifier: MIT OR Apache-2.0
//! Multi-step run engine (sequential fail-fast, one browser process).

use std::path::Path;

use serde_json::{json, Value};

use crate::browser::{CaptureOpts, OneShotSession};
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;

use super::execute::execute_step;
use super::flags::RunFlags;

/// Execute NDJSON script with feature gates (vision/screencast/memory).
///
/// # Panics
///
/// Never in practice. The one `expect` inside is `as_object_mut()` on a `Value`
/// this function built two lines earlier with `json!({ .. })`, so the `Some` is
/// guaranteed by construction rather than by input. It would only fire if that
/// literal stopped being an object, which the compiler would not hide.
pub async fn run_script_with_flags(
    life: &Lifecycle,
    script_path: &Path,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    flags: RunFlags,
) -> Result<Value, CliError> {
    // GAP-012 / GAP-029: load, expand includes and validate the WHOLE script
    // before BORN. Nothing below this line runs for a script with a typo, an
    // unknown field, or a missing capability flag — so the browser is never
    // launched and the target is never half-mutated to report a load-time error.
    let steps = super::preflight::preflight_script(script_path, flags, capture)?;

    let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
    life.record_chrome(session.chrome_pid());

    let mut results: Vec<Value> = Vec::new();
    // GAP-038: payload of the last successful step, for `assert kind=step`.
    let mut last_ok_data: Option<Value> = None;
    for (idx, step) in steps.iter().enumerate() {
        // Cooperative cancel between steps (SIGINT/SIGTERM → exit 130).
        if life.is_cancelled() {
            let _ = session.shutdown().await;
            life.clear_chrome();
            return Err(CliError::with_suggestion(
                ErrorKind::Cancelled,
                "cancelled by signal (SIGINT/SIGTERM) between run steps",
                crate::i18n::suggestion_key("retry_after_cancel", None),
            ));
        }

        let cmd = step
            .get("cmd")
            .or_else(|| step.get("action"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let step_fut = execute_step(
            &mut session,
            cmd,
            step,
            robots,
            flags,
            last_ok_data.as_ref(),
        );
        // Timed around the step and NOTHING else.
        //
        // The envelope used to report no duration at all, which left every
        // deadline assertion to infer one from a clock it did not control.
        // `tests/wait_conditions_gate.rs` documents what that cost: a run whose
        // deadline the product HONOURED took 10 799 ms of process time inside
        // the full suite and turned a case red, because the process clock
        // carries a browser launch that varied between 2.5 s and 10 s.
        //
        // An agent reading this envelope had the same problem and no file to
        // read about it: `ok: false, kind: timeout` says a deadline passed and
        // never says which one, so a caller could not tell a 2 s budget that was
        // honoured from a 10 s default that silently replaced it.
        let started = std::time::Instant::now();
        let step_res = if flags.step_timeout_secs > 0 {
            match tokio::time::timeout(
                std::time::Duration::from_secs(flags.step_timeout_secs),
                step_fut,
            )
            .await
            {
                Ok(inner) => inner,
                Err(_) => Err(CliError::with_suggestion(
                    ErrorKind::Timeout,
                    format!(
                        "run step {idx} cmd={cmd} exceeded --step-timeout {}s",
                        flags.step_timeout_secs
                    ),
                    crate::i18n::suggestion_key("raise_step_timeout", None),
                )),
            }
        } else {
            step_fut.await
        };
        match step_res {
            Ok(data) => {
                // GAP-019: one payload key per row. `result` was a byte-for-byte
                // duplicate of `data`, doubling every run envelope for nothing.
                let mut row = json!({
                    "index": idx,
                    "step": idx,
                    "cmd": cmd,
                    "ok": true,
                    "duration_ms": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                    "data": data,
                });
                if let Some(cid) = crate::agent_context::correlation_id() {
                    row.as_object_mut()
                        .expect("row object")
                        .insert("correlation_id".into(), json!(cid));
                }
                // GAP-020: stream NDJSON per step when --json-steps is set.
                // Compact encode only; propagate encode errors (never swallow).
                flags.emit_step_row(&row)?;
                last_ok_data = Some(data);
                results.push(row);
            }
            Err(e) => {
                // GAP-039: read the capture rings while the session is still alive.
                let dump_path = crate::browser::dump_failure_evidence(&mut session, &e);
                let _ = session.shutdown().await;
                life.clear_chrome();
                // Fail-fast keeps partial steps so agents retain context (GAP-006/016).
                let mut row = json!({
                    "index": idx,
                    "step": idx,
                    "cmd": cmd,
                    "ok": false,
                    "duration_ms": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                    "error": {
                        "kind": e.kind().as_str(),
                        "message": e.message(),
                        "suggestion": e.suggestion(),
                    }
                });
                if let Some(cid) = crate::agent_context::correlation_id() {
                    row.as_object_mut()
                        .expect("row object")
                        .insert("correlation_id".into(), json!(cid));
                }
                flags.emit_step_row(&row)?;
                results.push(row);
                let mut envelope = json!({
                    "total": steps.len(),
                    "failed_index": idx,
                    "failed_cmd": cmd,
                    "steps": results,
                    "ok": false,
                    "error": {
                        "kind": e.kind().as_str(),
                        "message": format!("run fail-fast at step {idx} cmd={cmd}: {e}"),
                        "suggestion": e
                            .suggestion()
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| crate::i18n::suggestion_key("run_fail_fast", None)),
                        "exit_code": e.exit_code(),
                    }
                });
                if let Some(path) = dump_path {
                    envelope.as_object_mut().expect("envelope object").insert(
                        "failure_dump_path".into(),
                        json!(path.display().to_string()),
                    );
                }
                return Ok(finish_run_envelope(envelope));
            }
        }
    }

    let close = session.shutdown().await;
    life.clear_chrome();
    close?;

    // GAP-020: final envelope always includes per-step results for --json agents.
    Ok(finish_run_envelope(json!({
        "ok": true,
        "total": results.len(),
        "steps": results,
    })))
}

/// Move the policy witness from every step to the envelope, once.
///
/// The four witness keys describe the PROCESS, so inside one run they are the
/// same in every step by construction. `with_capture_fields` attaches them to
/// each step result because for a single command that result is the envelope
/// itself; here it is not, and the copies are pure duplication — measured at
/// roughly thirty percent of the ten-step reference envelope on 2026-09-04.
///
/// Applied at EVERY return of a run envelope, including the fail-fast ones,
/// because a caller debugging a failed run is exactly who needs to know whether
/// the browser had a window.
pub(crate) fn finish_run_envelope(mut envelope: Value) -> Value {
    if let Some(steps) = envelope.get_mut("steps").and_then(|s| s.as_array_mut()) {
        for step in steps.iter_mut() {
            if let Some(data) = step.get_mut("data") {
                crate::browser_policy::strip_witness(data);
            }
        }
    }
    crate::browser_policy::attach_witness(&mut envelope);
    envelope
}

/// Run a single step object in one browser process (exec parity with run).
pub async fn run_one_step(
    life: &Lifecycle,
    step: Value,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    flags: RunFlags,
) -> Result<Value, CliError> {
    let cmd = step
        .get("cmd")
        .or_else(|| step.get("action"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    // Argv validation BEFORE the browser, exactly as `run --script` does it.
    // This used to sit after the launch — the session was opened as the first
    // instruction here — so a typo in `--action` cost a whole Chrome to reach a
    // usage error. It is the same accidental coupling the script preflight
    // removed, on the single-step surface.
    super::preflight::validate_action(&cmd, &step)?;

    let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
    life.record_chrome(session.chrome_pid());
    let step_res = execute_step(&mut session, &cmd, &step, robots, flags, None).await;
    let close = session.shutdown().await;
    life.clear_chrome();
    close?;
    step_res.map(|data| {
        json!({
            "cmd": cmd,
            "ok": true,
            "data": data,
        })
    })
}
