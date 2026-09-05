// SPDX-License-Identifier: MIT OR Apache-2.0
//! Step executor for `run` / `exec` (SRP: dispatch cmd -> OneShotSession).
//!
//! # Module map (Pass 31 SRP-03)
//!
//! | Module | Commands |
//! |--------|----------|
//! | helpers | beforeunload, unknown-field reject |
//! | nav_steps | goto..eval |
//! | capture_steps | grab, extract, text, scroll |
//! | page_steps | cookie, attr, console, net, page, dialog |
//! | assert_steps | assert |
//! | emulate_steps | emulate, resize |
//! | perf_steps | perf, screencast, heap, scrape, print-pdf, lighthouse |
//! | ext_steps | extension, devtools3p, webmcp |
//!
//! Step loop remains sequential (N-134); this module only splits dispatch arms.

use serde_json::Value;

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::{run_unknown_cmd_suggestion, RunFlags};

mod assert_steps;
mod capture_steps;
mod emulate_steps;
mod ext_steps;
mod helpers;
mod nav_steps;
mod page_steps;
mod perf_steps;

pub(super) use helpers::reject_unknown_step_fields;
use helpers::{step_action, step_capability_enabled};

/// Dispatch one script step.
///
/// This is the **single** point where per-step policy is applied, so the rules
/// live in one place instead of being re-encoded in each handler:
///
/// 1. unknown-field rejection
/// 2. capability gates (GAP-010 / GAP-011 / GAP-029)
/// 3. dialog precondition (GAP-041)
/// 4. `@eN` invalidation marker on the result (GAP-042)
pub(crate) async fn execute_step(
    session: &mut OneShotSession,
    cmd: &str,
    step: &Value,
    robots: RobotsPolicy,
    flags: RunFlags,
    prev: Option<&Value>,
) -> Result<Value, CliError> {
    reject_unknown_step_fields(cmd, step)?;
    enforce_step_capabilities(cmd, step, flags, session)?;
    enforce_step_preconditions(cmd, step, session)?;

    let result = dispatch_step(session, cmd, step, robots, flags, prev).await;
    result.map(|data| mark_ref_invalidation(cmd, data))
}

/// Enforce the capability gates the table declares for this step.
fn enforce_step_capabilities(
    cmd: &str,
    step: &Value,
    flags: RunFlags,
    session: &OneShotSession,
) -> Result<(), CliError> {
    let action = step_action(step);
    let label = match action {
        Some(a) => format!("{cmd} {a}"),
        None => cmd.to_string(),
    };
    for cap in crate::capability::required_capabilities(cmd, action) {
        if !step_capability_enabled(*cap, flags, session) {
            return Err(cap.disabled_error(&label));
        }
    }
    Ok(())
}

/// Enforce the page-state preconditions the table declares for this step.
fn enforce_step_preconditions(
    cmd: &str,
    step: &Value,
    session: &mut OneShotSession,
) -> Result<(), CliError> {
    use crate::capability::Precondition;

    for pre in crate::capability::required_preconditions(cmd, step_action(step)) {
        match pre {
            Precondition::NoDialogOpen => {
                if session.dialog_open_on_active_page() {
                    return Err(pre.unmet_error(cmd));
                }
            }
        }
    }
    Ok(())
}

/// Stamp `refs_invalidated` on results of DOM-mutating steps (GAP-042).
///
/// Only ever adds the field; a handler that already computed it wins.
fn mark_ref_invalidation(cmd: &str, mut data: Value) -> Value {
    if !crate::capability::invalidates_refs(cmd) {
        return data;
    }
    if let Some(obj) = data.as_object_mut() {
        obj.entry("refs_invalidated")
            .or_insert_with(|| Value::Bool(true));
    }
    data
}

/// Steps handled by `nav_steps`.
const NAV_CMDS: &[&str] = &[
    "goto",
    "wait",
    "hover",
    "drag",
    "fill-form",
    "fill_form",
    "select-option",
    "select_option",
    "pick",
    "upload",
    "submit",
    "back",
    "forward",
    "reload",
    "view",
    "press",
    "click",
    "write",
    "fill",
    "keys",
    "type",
    "click-at",
    "click_at",
    "eval",
];
/// Steps handled by `capture_steps`.
const CAPTURE_CMDS: &[&str] = &["grab", "screenshot", "extract", "text", "scroll"];
/// Steps handled by `page_steps`.
const PAGE_CMDS: &[&str] = &["cookie", "attr", "console", "net", "page", "dialog"];
/// Steps handled by `emulate_steps`.
const EMULATE_CMDS: &[&str] = &["emulate", "resize"];
/// Steps handled by `perf_steps`.
const PERF_CMDS: &[&str] = &[
    "perf",
    "screencast",
    "heap",
    "scrape",
    "print-pdf",
    "print_pdf",
    "lighthouse",
];
/// Steps handled by `ext_steps`.
const EXT_CMDS: &[&str] = &[
    "extension",
    "devtools3p-list",
    "devtools3p-exec",
    "devtools3p",
    "webmcp-list",
    "webmcp-exec",
    "webmcp",
];

/// True when `cmd` has a dispatch arm below.
///
/// Preflight (GAP-012) needs the accepted set **before** BORN, and the only
/// honest source is the same slices the dispatcher matches on. Deriving it from
/// a parallel inventory would drift and reject a step that would have run.
///
/// `exec` asks the same question (GAP-056). It used to answer it from a
/// hand-written list of its own, which drifted: `select-option` and `pick` were
/// in the slices below and missing from the copy, so `exec select-option`
/// reported `unknown exec subcommand` for a command `commands` advertised and
/// the published formulas taught. One source, and the consumers ask it.
pub(crate) fn is_dispatchable_cmd(cmd: &str) -> bool {
    cmd == "assert"
        || NAV_CMDS.contains(&cmd)
        || CAPTURE_CMDS.contains(&cmd)
        || PAGE_CMDS.contains(&cmd)
        || EMULATE_CMDS.contains(&cmd)
        || PERF_CMDS.contains(&cmd)
        || EXT_CMDS.contains(&cmd)
}

// Preflight validates the `cookie set` payload with the SAME reader the
// dispatcher uses, so the accepted key names keep one definition.
pub(crate) use page_steps::{cookie_set_payload, cookie_set_payload_error};

// Same contract for the two commands that take NO `action` and still require a
// payload. `cookie set` was reachable from preflight because it hangs off an
// action; `eval` and `goto` were not, so a step missing `expression` or `url`
// paid for a full browser launch before the dispatcher raised the usage error
// it could have raised from argv. Measured 2026-08-28: `eval` with no
// expression cost 1485 ms and launched Chrome, against 145 ms for the cases
// preflight already covered.
pub(crate) use nav_steps::{eval_expression, eval_expression_error, goto_url, goto_url_error};

/// Actions `cmd` accepts, or `None` when `cmd` takes no `action` field.
///
/// Eight step commands branch on `action`, and until now every one of them
/// discovered a typo only INSIDE the dispatcher — that is, after BORN, with a
/// browser already launched. `tests/cookie_jar_gate.rs` shows what that costs:
/// it asserts that a malformed step is a usage error, a question that needs no
/// browser at all, yet the script opens with a `goto` and paid a full Chrome to
/// ask it. A launch that lost a contended host therefore failed a test about
/// argv validation, which is an accidental coupling and not a real dependency.
///
/// Each slice is defined against the `match` it mirrors rather than here, for
/// the same reason [`is_dispatchable_cmd`] derives from the dispatcher slices:
/// a parallel inventory drifts, and a drifted inventory rejects a step that
/// would have run.
pub(crate) fn known_actions(cmd: &str) -> Option<&'static [&'static str]> {
    match cmd {
        "cookie" => Some(page_steps::COOKIE_ACTIONS),
        "page" => Some(page_steps::PAGE_ACTIONS),
        "dialog" => Some(page_steps::DIALOG_ACTIONS),
        "console" => Some(page_steps::CONSOLE_ACTIONS),
        "net" => Some(page_steps::NET_ACTIONS),
        "perf" => Some(perf_steps::PERF_ACTIONS),
        "screencast" => Some(perf_steps::SCREENCAST_ACTIONS),
        "heap" => Some(perf_steps::HEAP_ACTIONS),
        _ => None,
    }
}

/// Error for a step whose `action` has no arm, worded like the dispatcher's.
pub(crate) fn unknown_action_error(cmd: &str, action: &str) -> CliError {
    CliError::new(ErrorKind::Usage, format!("unknown {cmd} action: {action}"))
}

/// Error for a step command with no dispatch arm.
pub(super) fn unknown_cmd_error(cmd: &str) -> CliError {
    if cmd.is_empty() {
        return CliError::new(ErrorKind::Usage, "step missing cmd/action field");
    }
    CliError::with_suggestion(
        ErrorKind::Usage,
        format!("unknown script cmd: {cmd}"),
        run_unknown_cmd_suggestion(cmd),
    )
}

async fn dispatch_step(
    session: &mut OneShotSession,
    cmd: &str,
    step: &Value,
    robots: RobotsPolicy,
    flags: RunFlags,
    prev: Option<&Value>,
) -> Result<Value, CliError> {
    match cmd {
        "assert" => assert_steps::execute_assert(session, step, prev).await,
        c if NAV_CMDS.contains(&c) => nav_steps::handle(session, cmd, step, robots, flags).await,
        c if CAPTURE_CMDS.contains(&c) => {
            capture_steps::handle(session, cmd, step, robots, flags).await
        }
        c if PAGE_CMDS.contains(&c) => page_steps::handle(session, cmd, step, robots, flags).await,
        c if EMULATE_CMDS.contains(&c) => {
            emulate_steps::handle(session, cmd, step, robots, flags).await
        }
        c if PERF_CMDS.contains(&c) => perf_steps::handle(session, cmd, step, robots, flags).await,
        c if EXT_CMDS.contains(&c) => ext_steps::handle(session, cmd, step, robots, flags).await,
        other => Err(unknown_cmd_error(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::run::RUN_DISPATCHED_CMDS;

    /// Every spelling the dispatcher matches, in one set.
    fn dispatcher_cmds() -> std::collections::BTreeSet<&'static str> {
        std::iter::once("assert")
            .chain(NAV_CMDS.iter().copied())
            .chain(CAPTURE_CMDS.iter().copied())
            .chain(PAGE_CMDS.iter().copied())
            .chain(EMULATE_CMDS.iter().copied())
            .chain(PERF_CMDS.iter().copied())
            .chain(EXT_CMDS.iter().copied())
            .collect()
    }

    /// The published inventory and the dispatcher must name the same commands.
    ///
    /// They did not, in both directions, and neither direction was visible from
    /// either side. `RUN_DISPATCHED_CMDS` advertised four underscore spellings
    /// of the devtools/webmcp pair that no arm matched — a step naming one got
    /// `unknown script cmd` quoting a `Supported:` line that had just listed
    /// it — while `submit`, `click`, `fill`, `screenshot`, `devtools3p` and
    /// `webmcp` ran and were never advertised.
    ///
    /// The set is compared in BOTH directions because each direction is a
    /// different lie: an extra name promises a command that does not exist, a
    /// missing name hides one that does.
    #[test]
    fn dispatched_cmds_match_inventory() {
        let dispatcher = dispatcher_cmds();
        let inventory: std::collections::BTreeSet<&str> =
            RUN_DISPATCHED_CMDS.iter().copied().collect();

        let phantom: Vec<&str> = inventory.difference(&dispatcher).copied().collect();
        assert!(
            phantom.is_empty(),
            "RUN_DISPATCHED_CMDS advertises commands the dispatcher does not match: {phantom:?}"
        );

        // `assert` is dispatched by its own arm and belongs to no family slice;
        // it is in the inventory, so the difference below must still be empty.
        let unlisted: Vec<&str> = dispatcher.difference(&inventory).copied().collect();
        assert!(
            unlisted.is_empty(),
            "dispatcher runs commands RUN_DISPATCHED_CMDS never names: {unlisted:?}"
        );
    }

    /// `is_dispatchable_cmd` is what preflight asks before BORN, so it has to
    /// agree with the inventory the suggestion text quotes.
    #[test]
    fn is_dispatchable_agrees_with_the_inventory() {
        for cmd in RUN_DISPATCHED_CMDS {
            assert!(is_dispatchable_cmd(cmd), "{cmd} is advertised and refused");
        }
    }
}
