// SPDX-License-Identifier: MIT OR Apache-2.0
//! Step and capability validation performed before BORN.

use super::super::execute::{
    cookie_set_payload, cookie_set_payload_error, eval_expression, eval_expression_error, goto_url,
    goto_url_error, is_dispatchable_cmd, known_actions, reject_unknown_step_fields,
    unknown_action_error, unknown_cmd_error,
};
use super::super::flags::RunFlags;
use super::include::step_cmd;
use super::INCLUDE_CMD;
use crate::browser::CaptureOpts;
use crate::capability::Capability;
use crate::error::{CliError, ErrorKind};
use serde_json::Value;
use std::collections::BTreeSet;

/// Reject unknown commands, unknown fields, bad actions and missing payloads.
///
/// Every rejection here was already a usage error; what changes is WHEN it is
/// detected. Before, an unknown `action` survived preflight because `action` is
/// a KNOWN field — only its value was wrong — and the dispatcher raised it
/// after BORN, with a browser already paid for. Answering a question about argv
/// therefore depended on a host being able to launch Chrome, which is an
/// accidental coupling: a lost launch failed a test that never needed a
/// browser. The `kind` and the exit code are unchanged.
pub(super) fn validate_steps(steps: &[Value]) -> Result<(), CliError> {
    for (idx, step) in steps.iter().enumerate() {
        // Expansion removed every include, so any left here is a bug, not input.
        let cmd = step_cmd(step);
        debug_assert_ne!(cmd, INCLUDE_CMD, "include survived expansion");
        if !is_dispatchable_cmd(cmd) {
            return Err(prefix_step(idx, &unknown_cmd_error(cmd)));
        }
        reject_unknown_step_fields(cmd, step).map_err(|e| prefix_step(idx, &e))?;
        validate_action(cmd, step).map_err(|e| prefix_step(idx, &e))?;
        validate_required_payload(cmd, step).map_err(|e| prefix_step(idx, &e))?;
    }
    Ok(())
}

/// Reject a step whose command needs a payload the step does not carry.
///
/// # Why this is separate from [`validate_action`]
///
/// [`validate_action`] only reaches commands that BRANCH on `action`, and it
/// checks the payload of the arm it selected — which is why `cookie set` was
/// already covered. A command with no `action` never entered it, so `eval`
/// without `expression` and `goto` without `url` travelled all the way to the
/// dispatcher. The dispatcher raised the very same usage error; the difference
/// was that by then a browser had been launched and paid for. The question was
/// always about argv, so the only thing that changed is WHEN it is answered.
///
/// # Why each arm calls the dispatcher's reader
///
/// Nothing here restates which key a command accepts. `eval` takes any of
/// `expression`, `function` or `js`, and writing that list a second time is how
/// the list drifts — after which preflight rejects a step the dispatcher would
/// have run, which is strictly worse than the launch this saves.
fn validate_required_payload(cmd: &str, step: &Value) -> Result<(), CliError> {
    match cmd {
        "eval" if eval_expression(step).is_none() => Err(eval_expression_error()),
        "goto" if goto_url(step).is_none() => Err(goto_url_error()),
        _ => Ok(()),
    }
}

/// Reject an `action` with no dispatch arm, and a payload the arm requires.
///
/// An ABSENT `action` is never rejected: every arm that reads one applies a
/// default, so absence is a valid script and not an omission.
///
/// `exec` calls this too. It runs ONE step and used to launch the browser as
/// its first instruction, before it had even read the `cmd`, so `exec cookie
/// --action nonsense` paid a full Chrome to reach the same rejection. Same
/// defect, second surface: the check belongs to the argv, not to the session.
pub(crate) fn validate_action(cmd: &str, step: &Value) -> Result<(), CliError> {
    let Some(known) = known_actions(cmd) else {
        return Ok(());
    };
    let Some(action) = step.get("action").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    if !known.contains(&action) {
        return Err(unknown_action_error(cmd, action));
    }
    if cmd == "cookie" && action == "set" && cookie_set_payload(step).is_none() {
        return Err(cookie_set_payload_error());
    }
    Ok(())
}

/// Refuse before BORN when the script needs a flag this invocation lacks.
pub(super) fn validate_capabilities(
    steps: &[Value],
    flags: RunFlags,
    capture: CaptureOpts,
) -> Result<(), CliError> {
    let mut missing: BTreeSet<Capability> = BTreeSet::new();
    for step in steps {
        let cmd = step_cmd(step);
        let action = step
            .get("action")
            .or_else(|| step.get("kind"))
            .and_then(|v| v.as_str());
        for cap in crate::capability::required_capabilities(cmd, action) {
            if !capability_enabled(*cap, flags, capture) {
                missing.insert(*cap);
            }
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    let flags_list = missing
        .iter()
        .map(|c| c.flag())
        .collect::<Vec<_>>()
        .join(" ");
    Err(CliError::with_suggestion(
        ErrorKind::CapabilityDisabled,
        format!("script requires flags not enabled for this invocation: {flags_list}"),
        format!("Re-run with: {flags_list}"),
    ))
}

/// True when `cap` is enabled by this invocation's flags / capture options.
fn capability_enabled(cap: Capability, flags: RunFlags, capture: CaptureOpts) -> bool {
    match cap {
        Capability::Memory => flags.category_memory,
        Capability::Extensions => flags.category_extensions,
        Capability::ThirdParty => flags.category_third_party,
        Capability::Webmcp => flags.category_webmcp,
        Capability::Vision => flags.experimental_vision,
        Capability::Screencast => flags.experimental_screencast,
        Capability::CaptureConsole => capture.console,
        Capability::CaptureNetwork => capture.network,
    }
}

/// Prefix an error message with the step index that produced it.
fn prefix_step(idx: usize, e: &CliError) -> CliError {
    let mut out = CliError::new(e.kind(), format!("script step {idx}: {}", e.message()));
    if let Some(s) = e.suggestion() {
        out = CliError::with_suggestion(e.kind(), format!("script step {idx}: {}", e.message()), s);
    }
    out
}
