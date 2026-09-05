// SPDX-License-Identifier: MIT OR Apache-2.0
//! Console and network capture-buffer steps.
// The blanket `allow(missing_docs, unused_imports)` this module carried hid
// four imports nothing referenced. An allow that silences a whole file silences
// the next dead import too, which is why it is gone rather than narrowed.

use std::path::Path;

use serde_json::Value;

use crate::browser::OneShotSession;
use crate::commands::run::step_key_aliases;
use crate::error::{CliError, ErrorKind};

// # The camelCase step keys this module accepts, and where they are declared
//
// Every step key here is read twice: once in the snake_case spelling the schema
// publishes, and once in a camelCase alias. There are EIGHT of them, and until
// 2026-08-31 `docs/schemas/*.json` declared NONE — not by oversight in the
// schema files, but by construction: they are projected from the clap parser,
// and an alias reached with `step.get("pageIdx")` is invisible to clap.
//
// An audit named `resourceTypes` alone as a hidden surface, which read as one
// field having slipped through. Measured 2026-08-30: it is not one field, it is
// a systematic tolerance covering every optional key this module takes.
//
// A COMMENT USED TO STAND IN FOR THE DECLARATION, AND THAT WAS THE REMAINING
// DEFECT. It told whoever opened THIS file; an agent writing a `run --script`
// step reads the SCHEMA, so for the only reader who needed it nothing had
// changed. The aliases now live in `STEP_KEY_ALIASES` beside
// `RUN_DISPATCHED_CMDS`, are resolved here by `step_value`, and are published by
// `schema --cmd` as `step_key_aliases` on the property — so accepting a spelling
// and documenting it became the same edit instead of two.
//
// They stay accepted, rather than being removed, because a `run --script` file
// is written by hand and by agents that emit JSON in the casing of their own
// language, and a step silently ignored is worse than one accepted twice.

/// Read a step key by its schema spelling, then by any alias declared for it.
///
/// Resolution goes through [`step_key_aliases`] rather than a chain of
/// `or_else` calls at each site, so the spellings this module accepts and the
/// ones `schema --cmd` publishes cannot drift apart: both read one table.
fn step_value<'a>(step: &'a Value, cmd: &str, key: &str) -> Option<&'a Value> {
    step.get(key).or_else(|| {
        step_key_aliases(cmd, key)
            .into_iter()
            .find_map(|alias| step.get(alias))
    })
}

/// Refuse `include_preserved` on the console actions that cannot honour it.
///
/// # The asymmetry this closes
///
/// `docs/schemas/console.schema.json` publishes the key for `list` and `get`
/// only: `clear` carries no properties at all and `dump` carries `path`. The
/// step validator is indexed by COMMAND ALONE, so `STEP_FIELDS["console"]` is
/// the UNION of every action's keys and let this one through on the two that
/// have no use for it. Measured 2026-09-01 across the 71 schemas: 21 commands
/// carry an `actions` block, 94 action pairs in total, and 627 (action, key)
/// pairs the validator accepts that the published contract does not admit.
///
/// # Why refusing does not narrow the contract
///
/// The schema is ALREADY the stricter document. Refusing here does not tighten
/// what the product promises; it makes the binary honour what it publishes.
///
/// # Why `dump` is the worse of the two
///
/// `console_clear` empties `console_log` entirely and no separate preserved
/// buffer exists, so the key is merely meaningless there. `console_dump` calls
/// `console_list(None, None, None, true, None)` with the flag HARDCODED to
/// true, so `include_preserved: false` was validated, accepted, discarded, and
/// then CONTRADICTED: the dump included preserved messages anyway and the
/// envelope reported success. A key that is ignored yields the default; a key
/// that is contradicted yields the opposite of the request.
fn reject_preserved_scope(action: &str, step: &Value) -> Result<(), CliError> {
    if step_value(step, "console", "include_preserved").is_some() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!(
                "console {action} takes no `include_preserved`; the key applies to `list` and \
                 `get`, and the schema publishes it for those two only"
            ),
            crate::i18n::suggestion_key("use_listed_value", None),
        ));
    }
    Ok(())
}

/// Actions the `console` arm below accepts.
///
/// See [`COOKIE_ACTIONS`](super::state::COOKIE_ACTIONS) for why the slice sits
/// beside the `match` it mirrors.
pub(crate) const CONSOLE_ACTIONS: &[&str] = &["list", "get", "clear", "dump"];

/// Actions the `net` arm below accepts.
pub(crate) const NET_ACTIONS: &[&str] = &["list", "get"];

/// Dispatch one `console` or `net` step against the in-process capture buffers.
///
/// # Errors
///
/// Fails with [`ErrorKind::Usage`] for an unknown action, a missing required
/// step key, or a capture flag the invocation did not pass, and propagates the
/// session error otherwise.
pub(super) async fn handle(
    session: &mut OneShotSession,
    cmd: &str,
    step: &Value,
) -> Result<Value, CliError> {
    match cmd {
        "console" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("list");
            // Read once for the whole command: `list` and `get` MUST agree on
            // which buffer the ids address, and a per-branch read is how they
            // came to disagree in the first place.
            let include_preserved = step_value(step, "console", "include_preserved")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            match action {
                "list" => {
                    let page_idx = step_value(step, "console", "page_idx")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize);
                    let page_size = step_value(step, "console", "page_size")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize);
                    let types = step.get("types").and_then(|v| v.as_str());
                    let sw =
                        step_value(step, "console", "service_worker_id").and_then(|v| v.as_str());
                    session.console_list(page_idx, page_size, types, include_preserved, sw)
                }
                "get" => {
                    let id = step
                        .get("id")
                        .or_else(|| step.get("msgid"))
                        .or_else(|| step.get("index"))
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| {
                            CliError::with_suggestion(
                                ErrorKind::Usage,
                                "console get requires id|msgid|index (0-based)",
                                crate::i18n::suggestion_key("step_missing_argument", None),
                            )
                        })? as usize;
                    session.console_get(id, include_preserved)
                }
                "clear" => {
                    reject_preserved_scope("clear", step)?;
                    session.console_clear()
                }
                "dump" => {
                    reject_preserved_scope("dump", step)?;
                    let path = step.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                        CliError::with_suggestion(
                            ErrorKind::Usage,
                            "console dump requires path",
                            crate::i18n::suggestion_key("step_missing_argument", None),
                        )
                    })?;
                    session.console_dump(Path::new(path)).await
                }
                other => Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("unknown console action: {other}"),
                    crate::i18n::suggestion_key("unknown_step_action", None),
                )),
            }
        }
        "net" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("list");
            // Read once for the whole command, for the same reason as console.
            let include_preserved = step_value(step, "net", "include_preserved")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            match action {
                "list" => {
                    let page_idx = step_value(step, "net", "page_idx")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize);
                    let page_size = step_value(step, "net", "page_size")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize);
                    let resource_types =
                        step_value(step, "net", "resource_types").and_then(|v| v.as_str());
                    session.net_list(page_idx, page_size, resource_types, include_preserved)
                }
                "get" => {
                    let id = step
                        .get("id")
                        .map(|v| {
                            if let Some(s) = v.as_str() {
                                s.to_string()
                            } else if let Some(n) = v.as_u64() {
                                n.to_string()
                            } else {
                                String::new()
                            }
                        })
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| {
                            CliError::with_suggestion(
                                ErrorKind::Usage,
                                "net get requires id (index or requestId)",
                                crate::i18n::suggestion_key("step_missing_argument", None),
                            )
                        })?;
                    let request_path = step
                        .get("request_path")
                        .and_then(|v| v.as_str())
                        .map(Path::new);
                    let response_path = step
                        .get("response_path")
                        .and_then(|v| v.as_str())
                        .map(Path::new);
                    session
                        .net_get(&id, request_path, response_path, include_preserved)
                        .await
                }
                other => Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("unknown net action: {other}"),
                    crate::i18n::suggestion_key("unknown_step_action", None),
                )),
            }
        }
        other => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("internal: unexpected cmd in this family: {other}"),
            crate::i18n::suggestion_key("internal_defect", None),
        )),
    }
}
