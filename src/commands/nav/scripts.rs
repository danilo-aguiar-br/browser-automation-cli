// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::{block_on_browser_timeout, CaptureOpts};
use crate::commands::common::{emit_ok, emit_ok_summary};
use crate::commands::nav::goto::handle_goto;
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;
use std::path::Path;

pub(crate) fn handle_run(
    life: &Lifecycle,
    script: &Path,
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
    flags: crate::commands::run::RunFlags,
) -> Result<(), CliError> {
    let script = script.to_path_buf();
    // GAP-034 pillar 4: `--script -` streams NDJSON from stdin with the browser
    // alive between lines. Still one BORN and one DIE; EOF is FINALIZE.
    let data = if crate::commands::run::is_stdin_marker(&script) {
        block_on_browser_timeout(
            crate::commands::run::run_stream_with_flags(life, robots, capture, flags),
            timeout_secs,
        )?
    } else {
        block_on_browser_timeout(
            crate::commands::run::run_script_with_flags(life, &script, robots, capture, flags),
            timeout_secs,
        )?
    };
    // Fail-fast payload: ok:false with partial steps (still non-zero exit).
    if data.get("ok") == Some(&serde_json::json!(false)) {
        let kind = data
            .pointer("/error/kind")
            .and_then(|v| v.as_str())
            .unwrap_or("data");
        let message = data
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("run fail-fast")
            .to_string();
        let suggestion = data
            .pointer("/error/suggestion")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| crate::i18n::suggestion_key("run_fail_fast", None))
            .to_string();
        // Round-trip the failing step's kind instead of re-listing it here. The
        // previous hand-written match fell back to Software for anything absent,
        // so `precondition` (75) and `capability-disabled` (64) — and already
        // `no-input`, `config`, `io`, `protocol` — all surfaced as 70.
        let err_kind = ErrorKind::from_str(kind).unwrap_or(ErrorKind::Software);
        let mut partial = serde_json::json!({
            "total": data.get("total"),
            "failed_index": data.get("failed_index"),
            "failed_line": data.get("failed_line"),
            "failed_cmd": data.get("failed_cmd"),
            "mode": data.get("mode"),
            "steps": data.get("steps"),
        });
        // GAP-039: carry the evidence artifact path onto the error envelope.
        if let Some(dump) = data.get("failure_dump_path") {
            partial
                .as_object_mut()
                .expect("partial object")
                .insert("failure_dump_path".into(), dump.clone());
        }
        return Err(CliError::with_suggestion(err_kind, message, suggestion).with_data(partial));
    }
    emit_ok(data, json, |d| {
        let total = d.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
        crate::output::writeln_stdout(format!("ok run steps={total}"))?;
        Ok(())
    })
}

pub(crate) fn handle_exec(
    life: &Lifecycle,
    args: &[String],
    robots: RobotsPolicy,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
    flags: crate::commands::run::RunFlags,
) -> Result<(), CliError> {
    if args.is_empty() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "exec requires a subcommand (e.g. goto)",
            crate::i18n::suggestion_key("exec_goto_example", None),
        ));
    }
    // Single-step path for simple argv forms; multi-step uses run --script.
    match args[0].as_str() {
        "goto" => {
            let url = args.get(1).ok_or_else(|| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    "exec goto requires a URL",
                    crate::i18n::suggestion_key("exec_goto_example", None),
                )
            })?;
            handle_goto(
                life,
                url,
                robots,
                capture,
                timeout_secs,
                json,
                None,
                None,
                None,
            )
        }
        // GAP-056: ask the dispatcher what it accepts instead of keeping a
        // second list here. The hand-written copy this replaces had drifted —
        // `select-option` and `pick` were dispatchable and absent from it, so
        // `exec select-option` failed for a command `commands` advertised and
        // the published formulas taught. A copy of a list is a divergence
        // waiting to happen, and nothing about it breaks compilation.
        cmd if crate::commands::run::is_dispatchable_cmd(cmd) => {
            let step = crate::commands::run::argv_to_step(args)?;
            let data = block_on_browser_timeout(
                crate::commands::run::run_one_step(life, step, robots, capture, flags),
                timeout_secs,
            )?;
            emit_ok_summary(data, json, "exec")
        }
        other => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unknown exec subcommand: {other}"),
            crate::i18n::suggestion_key("run_script_multi_step", None),
        )),
    }
}
