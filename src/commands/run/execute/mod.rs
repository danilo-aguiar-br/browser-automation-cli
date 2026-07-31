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

use super::{run_supported_suggestion, RunFlags};

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
pub(super) async fn execute_step(
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

/// Error for a step command with no dispatch arm.
pub(super) fn unknown_cmd_error(cmd: &str) -> CliError {
    if cmd.is_empty() {
        return CliError::new(ErrorKind::Usage, "step missing cmd/action field");
    }
    CliError::with_suggestion(
        ErrorKind::Usage,
        format!("unknown script cmd: {cmd}"),
        run_supported_suggestion(),
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
