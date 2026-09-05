// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared step-field helpers for run/exec dispatch.

use serde_json::Value;

use super::super::inventory::{
    canonical_step_cmd, step_allowed_fields, step_item_allowed_fields, step_key_reads,
    STEP_ITEM_ARRAYS, STEP_OBJECT_NODES,
};
use crate::browser::OneShotSession;
use crate::capability::Capability;
use crate::error::{CliError, ErrorKind};

use super::RunFlags;

/// The `action` sub-selector of a step, if present (`heap` → `summary`).
///
/// Capability lookup is specific-first, so this is what lets one `heap` row gate
/// eleven actions while `take` stays free.
pub(super) fn step_action(step: &Value) -> Option<&str> {
    step.get("action")
        .or_else(|| step.get("kind"))
        .and_then(|v| v.as_str())
}

/// Parse `format` / `formats` from a run step (GAP-057; mirrors top-level scrape).
///
/// Accepts a string (optionally CSV), an array of strings, or absence (empty →
/// session default `text`).
pub(super) fn scrape_formats_from_step(step: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let push_token = |raw: &str, out: &mut Vec<String>| {
        for part in raw.split(',') {
            let t = part.trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
        }
    };
    if let Some(v) = step.get("formats").or_else(|| step.get("format")) {
        match v {
            Value::String(s) => push_token(s, &mut out),
            Value::Array(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        push_token(s, &mut out);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// True when `cap` is enabled for this `run` invocation.
///
/// Policy gates come from `RunFlags`; capture buffers come from how the session
/// was launched, because `--capture-*` shapes the launch rather than the dispatch.
pub(super) fn step_capability_enabled(
    cap: Capability,
    flags: RunFlags,
    session: &OneShotSession,
) -> bool {
    match cap {
        Capability::Memory => flags.category_memory,
        Capability::Extensions => flags.category_extensions,
        Capability::ThirdParty => flags.category_third_party,
        Capability::Webmcp => flags.category_webmcp,
        Capability::Vision => flags.experimental_vision,
        Capability::Screencast => flags.experimental_screencast,
        Capability::CaptureConsole => session.capture().console,
        Capability::CaptureNetwork => session.capture().network,
    }
}

pub(super) fn step_beforeunload_action(step: &Value) -> Option<&'static str> {
    let v = step
        .get("handle_before_unload")
        .or_else(|| step.get("handleBeforeUnload"));
    match v {
        Some(Value::Bool(true)) => Some("accept"),
        Some(Value::Bool(false)) => None,
        Some(Value::String(s)) => {
            let s = s.trim().to_ascii_lowercase();
            match s.as_str() {
                "accept" | "true" | "1" | "yes" => Some("accept"),
                "dismiss" | "cancel" => Some("dismiss"),
                "off" | "false" | "0" | "no" | "none" => None,
                _ => Some("accept"),
            }
        }
        _ => None,
    }
}

/// Backward-compatible bool form (true when any auto-handle is requested).
#[allow(dead_code)]
pub(super) fn step_wants_beforeunload_handle(step: &Value) -> bool {
    step_beforeunload_action(step).is_some()
}

/// Reject a step field no handler of `cmd` reads.
///
/// # Why this is a rejection and not a warning
///
/// Until 2026-08-31 this function named four commands and returned `Ok(())`
/// for the other twenty-four. A step like `{"cmd":"net","filter":"…"}` then
/// returned every record unfiltered under `ok: true`, and the caller had no
/// way to tell that from a filter that matched everything. A silent discard
/// inside a validator that exists to fail closed is worse than a hard error,
/// because the agent goes on to reason on top of the result.
///
/// The allowlist is derived from [`step_allowed_fields`], the same table
/// `step_key_reads` feeds to the handlers, so a spelling cannot be accepted
/// here and dropped there, nor read there and refused here.
pub(crate) fn reject_unknown_step_fields(cmd: &str, step: &Value) -> Result<(), CliError> {
    let Some(obj) = step.as_object() else {
        return Ok(());
    };
    // No row means no dispatch arm either. Falling through hands the step to
    // the dispatcher, whose `unknown script cmd` names the supported commands —
    // strictly more useful than a field list for a command that does not exist.
    let Some(allowed) = step_allowed_fields(cmd) else {
        return Ok(());
    };
    let unknown: Vec<&str> = obj
        .keys()
        .map(String::as_str)
        .filter(|k| !allowed.contains(k))
        .collect();
    // The `key`-on-`press` diagnosis is picked out of the whole set instead of
    // reported in iteration order: key order decides which unknown field is
    // seen first, so a step carrying BOTH `key` and a typo would otherwise
    // report whichever sorted lower. This one changes what the caller does
    // next, so it wins unconditionally.
    if let Some(err) = unknown.iter().find_map(|k| press_key_confusion(cmd, k)) {
        return Err(err);
    }
    if unknown.contains(&"engine") && canonical_step_cmd(cmd) == "scrape" {
        return Err(scrape_engine_refusal());
    }
    if let Some(key) = unknown.first() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unknown field `{key}` on step cmd={cmd}"),
            format!("Allowed fields for {cmd}: {}", allowed.join(", ")),
        ));
    }
    reject_unknown_item_fields(cmd, step)
}

/// Reject a field no handler reads inside a step's nested objects.
///
/// # Why the top-level check was not enough
///
/// [`reject_unknown_step_fields`] inspects `obj.keys()`, which are the step's
/// OWN keys, and never descends. Measured 2026-08-31, with the top level
/// already closed, THREE steps carried nested objects and all three discarded
/// invented keys in silence: `fill-form.fields`, `cookie set`'s array — where
/// the cookie was actually stored and the key vanished — and
/// `drag.synthetic_payload`, where a typo in `dragOperationsMask` silently fell
/// back to the default mask and changed the drop.
///
/// # Why the depth is a table and not a recursion
///
/// The surface is not uniformly one level: `fill-form` and `cookie` put an
/// array directly under a step key, while `drag` puts it inside an object, two
/// levels down. That is why the paths below are DOTTED — the depth travels as
/// data, so a new nesting is a new row rather than a new walker.
///
/// A general recursion is still not warranted: it would have to invent a rule
/// for objects with no declared row, and every such rule either rejects a
/// payload the browser accepts or accepts anything, which is the defect being
/// removed. The day a step nests an array of objects inside an array of
/// objects, a dotted path cannot address it and the walk MUST become recursive.
fn reject_unknown_item_fields(cmd: &str, step: &Value) -> Result<(), CliError> {
    for (owner, path, row) in STEP_OBJECT_NODES {
        if canonical_step_cmd(cmd) != *owner {
            continue;
        }
        let Some((written, value)) = resolve_step_path(step, owner, path) else {
            continue;
        };
        let Some(obj) = value.as_object() else {
            continue;
        };
        let Some(allowed) = step_item_allowed_fields(row) else {
            continue;
        };
        for key in obj.keys() {
            if !allowed.iter().any(|a| a == key) {
                return Err(nested_field_error(cmd, &written, None, key, &allowed));
            }
        }
    }

    for (owner, path, row) in STEP_ITEM_ARRAYS {
        if canonical_step_cmd(cmd) != *owner {
            continue;
        }
        let Some((written, raw)) = resolve_step_path(step, owner, path) else {
            continue;
        };
        // The array may arrive as a JSON STRING: `fields_json` is the CLI long
        // name and `cookie set` takes one too, and both are parsed downstream.
        // Validating here covers that path; a string that does NOT parse is
        // left alone so the dispatcher still raises its own error, unchanged.
        let parsed;
        let value = match raw.as_str() {
            Some(s) => match crate::json_util::value_from_str(s) {
                Ok(v) => {
                    parsed = v;
                    &parsed
                }
                Err(_) => continue,
            },
            None => raw,
        };
        let Some(items) = value.as_array() else {
            continue;
        };
        let Some(allowed) = step_item_allowed_fields(row) else {
            continue;
        };
        for (idx, item) in items.iter().enumerate() {
            let Some(obj) = item.as_object() else {
                continue;
            };
            for key in obj.keys() {
                if !allowed.iter().any(|a| a == key) {
                    return Err(nested_field_error(cmd, &written, Some(idx), key, &allowed));
                }
            }
        }
    }
    Ok(())
}

/// The error for an unknown key inside a nested object or array item.
///
/// `where_` is the path AS THE CALLER WROTE IT and `idx` the array offset when
/// there is one. Both exist for the same reason: `on step cmd=cookie` and
/// `in cookies[2] of cmd=cookie` send the reader to different lines, and a
/// message naming the canonical `json` for a step that says `cookies` would
/// point at a key absent from the script — the misdirection this validator
/// exists to remove, reintroduced in its own error text.
fn nested_field_error(
    cmd: &str,
    where_: &str,
    idx: Option<usize>,
    key: &str,
    allowed: &[&str],
) -> CliError {
    let at = match idx {
        Some(i) => format!("{where_}[{i}]"),
        None => where_.to_string(),
    };
    CliError::with_suggestion(
        ErrorKind::Usage,
        format!("unknown field `{key}` in {at} of cmd={cmd}"),
        format!("Allowed fields for {at}: {}", allowed.join(", ")),
    )
}

/// Walk a dotted path, returning the spelling used and the value found.
///
/// Only the FIRST segment goes through the synonym table: it is the step key,
/// and the step key is the only one a caller may spell more than one way. The
/// rest are names inside a payload the browser defines, where a second spelling
/// would be an invention rather than a tolerance.
fn resolve_step_path<'a>(step: &'a Value, cmd: &str, path: &str) -> Option<(String, &'a Value)> {
    let mut segments = path.split('.');
    let (spelling, mut cur) = first_table_value(step, cmd, segments.next()?)?;
    let mut written = spelling.to_string();
    for seg in segments {
        cur = cur.get(seg)?;
        written.push('.');
        written.push_str(seg);
    }
    Some((written, cur))
}

/// The step value for `key`, with the spelling it was actually written under.
fn first_table_value<'a>(
    step: &'a Value,
    cmd: &str,
    key: &str,
) -> Option<(&'static str, &'a Value)> {
    step_key_reads(cmd, key)
        .into_iter()
        .find_map(|spelling| step.get(spelling).map(|v| (spelling, v)))
}

/// The one wrong field that costs hours instead of minutes.
///
/// `press` reads as "press a key" and dispatches a MOUSE CLICK: it resolves
/// `target` and calls `interaction::click`. Measured 2026-08-31 against a local
/// form, a `press` step carrying `key` answered `ok: true` with the target
/// echoed back and produced ZERO keyboard events — `key` was dropped in
/// silence, the form never submitted, and every signal the caller could read
/// said the step had worked.
///
/// The generic "unknown field" error would be true and useless here, because
/// what is wrong is the caller's model of the command and not their spelling.
/// So this arm names the substitute: keystrokes are `keys`, clicks are `press`.
/// The single refusal for `engine` on a `scrape` step, shared by both layers.
///
/// # Why it is refused rather than honoured or ignored
///
/// Inside `run` the browser session is already live, so the engine was settled
/// at launch; honouring the field would mean tearing the session down and
/// relaunching mid-script, which no step has the authority to do.
///
/// Ignoring it is what the step used to do, and MEASURED 2026-08-31 that
/// produced the one shape a caller cannot detect by reading `ok`: a step asking
/// for `"engine":"http"` returned `ok: true` carrying `engine: "browser"`. The
/// answer contradicted the request and still reported success.
///
/// # Why one function and not two copies
///
/// `perf_steps.rs` keeps its own check as a second line of defence, for a
/// dispatch path added later that does not reach the preflight. Two layers are
/// deliberate; two WORDINGS would not be, because the caller would get a
/// different explanation depending on which one fired first.
pub(crate) fn scrape_engine_refusal() -> CliError {
    CliError::with_suggestion(
        ErrorKind::Usage,
        "scrape step does not accept `engine`: inside `run` the browser session \
         is already live, so the engine is fixed at launch and cannot change per \
         step. Use the top-level `scrape <url> --engine http` for a one-shot fetch \
         without a session, or drop the field to use the running browser.",
        crate::i18n::suggestion_key("scrape_engine_choice", None),
    )
}

fn press_key_confusion(cmd: &str, key: &str) -> Option<CliError> {
    if canonical_step_cmd(cmd) != "press" || key != "key" {
        return None;
    }
    Some(CliError::with_suggestion(
        ErrorKind::Usage,
        format!("`{cmd}` takes no `key` field: it dispatches a mouse click, not a keystroke"),
        r#"Use cmd=keys with "key" to send a keystroke, optionally with "target" to focus an element first; keep cmd=press (or cmd=click) with "target" for a mouse click"#
            .to_string(),
    ))
}
