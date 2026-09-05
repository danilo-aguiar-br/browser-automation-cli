// SPDX-License-Identifier: MIT OR Apache-2.0
//! Perf/screencast/heap/scrape/pdf/lighthouse steps for run/exec.

use std::path::Path;

use serde_json::{json, Value};

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::super::RunFlags;
use super::helpers::{scrape_formats_from_step, step_beforeunload_action};

/// Actions the `perf` arm below accepts.
///
/// See [`COOKIE_ACTIONS`](super::page_steps::COOKIE_ACTIONS) for why the
/// slice sits beside the `match` it mirrors. The path goes through the
/// `page_steps` re-export rather than `page_steps::state`, which is private to
/// that module and unreachable from here.
pub(crate) const PERF_ACTIONS: &[&str] = &["start", "stop", "insight"];

/// Actions the `screencast` arm below accepts.
pub(crate) const SCREENCAST_ACTIONS: &[&str] = &["start", "stop"];

/// Actions the `heap` arm below accepts, aliases included.
pub(crate) const HEAP_ACTIONS: &[&str] = &[
    "take",
    "summary",
    "close",
    "details",
    "dup-strings",
    "dup_strings",
    "compare",
    "class-nodes",
    "dominators",
    "edges",
    "retainers",
    "paths",
    "object-details",
    "object_details",
];

pub(super) async fn handle(
    session: &mut OneShotSession,
    cmd: &str,
    step: &Value,
    robots: RobotsPolicy,
    // Gates are enforced centrally in execute_step from the capability table.
    _flags: RunFlags,
) -> Result<Value, CliError> {
    match cmd {
        "perf" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("insight");
            match action {
                "start" => {
                    let path = step.get("path").and_then(|v| v.as_str()).map(Path::new);
                    let auto_stop = step
                        .get("auto_stop")
                        .or_else(|| step.get("autoStop"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let reload = step
                        .get("reload")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    session.perf_start(path, reload, auto_stop).await
                }
                "stop" => {
                    let path = step.get("path").and_then(|v| v.as_str()).map(Path::new);
                    session.perf_stop(path).await
                }
                "insight" => {
                    let name = step
                        .get("name")
                        .or_else(|| step.get("insight_name"))
                        .or_else(|| step.get("insightName"))
                        .and_then(|v| v.as_str());
                    let set_id = step
                        .get("insight_set_id")
                        .or_else(|| step.get("insightSetId"))
                        .and_then(|v| v.as_str());
                    // `step_fields` has always listed `path` among the keys a
                    // `perf` step accepts, and nothing here read it: the key was
                    // accepted and silently ignored, so a script asking to
                    // analyse a trace offline got a live capture instead, with
                    // no error to say the request had been dropped.
                    //
                    // `perf_insight_file` carries its own root check, so the
                    // path is bounded before the file is opened.
                    match step.get("path").and_then(|v| v.as_str()) {
                        Some(trace) => {
                            // Same refusal the CLI arm makes: an offline trace
                            // has no insight sets, so honouring `path` while
                            // discarding `insight_set_id` would answer a
                            // request nobody made and report success over it.
                            if set_id.is_some() {
                                return Err(CliError::new(
                                    ErrorKind::Usage,
                                    "perf insight with `path` reads a trace file offline and has \
                                     no insight sets; drop `insight_set_id`, or drop `path`",
                                ));
                            }
                            OneShotSession::perf_insight_file(Path::new(trace), name)
                        }
                        None => session.perf_insight(name, set_id).await,
                    }
                }
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown perf action: {other}"),
                )),
            }
        }
        "screencast" => {
            // Gate enforced centrally from the capability table (GAP-010/011).
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("start");
            match action {
                "start" => {
                    let path = step
                        .get("path")
                        .or_else(|| step.get("dir"))
                        .and_then(|v| v.as_str())
                        .map(Path::new);
                    session.screencast_start(path).await
                }
                "stop" => {
                    // `dir` is read here for the same reason `start` reads it:
                    // `STEP_KEY_SYNONYMS` declares it as a spelling of `path`
                    // for the COMMAND, so the validator accepts it on either
                    // action. Reading it in only one arm left
                    // `{"cmd":"screencast","action":"stop","dir":"..."}`
                    // accepted and discarded, which breaks the invariant
                    // `step_key_reads` states in so many words: an allowed
                    // synonym cannot go unread.
                    let path = step
                        .get("path")
                        .or_else(|| step.get("dir"))
                        .and_then(|v| v.as_str())
                        .map(Path::new);
                    session.screencast_stop(path).await
                }
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown screencast action: {other}"),
                )),
            }
        }
        "heap" => {
            let action = step
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("take");
            // Gate enforced centrally from the capability table (GAP-010/011):
            // only `take` is free, the other eleven require --category-memory.
            match action {
                "take" => {
                    let path = step.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "heap take requires path")
                    })?;
                    session.heap_take(Path::new(path)).await
                }
                "summary" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap path required"))?;
                    OneShotSession::heap_file_summary(Path::new(path))
                }
                "close" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap path required"))?;
                    OneShotSession::heap_close(Path::new(path))
                }
                "details" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap path required"))?;
                    OneShotSession::heap_details(Path::new(path))
                }
                "dup-strings" | "dup_strings" => {
                    let path = step
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| CliError::new(ErrorKind::Usage, "heap path required"))?;
                    OneShotSession::heap_dup_strings(Path::new(path))
                }
                "compare" => {
                    let base = step.get("base").and_then(|v| v.as_str()).ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "heap compare needs base")
                    })?;
                    let current =
                        step.get("current")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                CliError::new(ErrorKind::Usage, "heap compare needs current")
                            })?;
                    OneShotSession::heap_compare(Path::new(base), Path::new(current))
                }
                "class-nodes" => {
                    let path = step.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "heap class-nodes needs path")
                    })?;
                    let id = step.get("id").and_then(|v| v.as_u64()).ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "heap class-nodes needs id")
                    })?;
                    OneShotSession::heap_class_nodes(Path::new(path), id)
                }
                "dominators" | "edges" | "retainers" | "paths" => {
                    let path = step.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "heap node op needs path")
                    })?;
                    let node = step.get("node").and_then(|v| v.as_u64()).ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "heap node op needs node")
                    })?;
                    OneShotSession::heap_node_op(Path::new(path), node, action)
                }
                "object-details" | "object_details" => {
                    let path = step.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "heap object-details needs path")
                    })?;
                    let node = step.get("node").and_then(|v| v.as_u64()).ok_or_else(|| {
                        CliError::new(ErrorKind::Usage, "heap object-details needs node")
                    })?;
                    OneShotSession::heap_object_details(Path::new(path), node)
                }
                other => Err(CliError::new(
                    ErrorKind::Usage,
                    format!("unknown heap action: {other}"),
                )),
            }
        }
        "scrape" => {
            let url = step
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "scrape requires url"))?;
            // Refuse `engine` rather than discard it.
            //
            // Inside `run` the browser session is already live, so the engine was
            // settled at launch and no step can move it: honouring the field would
            // mean tearing the session down and relaunching mid-script.
            //
            // Discarding it was worse than it sounds. Measured 2026-08-31, a step
            // asking for `"engine":"http"` returned `ok: true` with `engine:
            // "browser"` in the same envelope — the answer contradicted the request
            // and still reported success, which is the one shape a caller cannot
            // detect by reading `ok`.
            // SECOND line of defence. `reject_unknown_step_fields` already
            // refuses this before the browser launches, because `engine` is
            // absent from the `scrape` row in `STEP_FIELDS`. This arm survives
            // for a dispatch path added later that does not reach the preflight,
            // and it calls the SAME constructor so the two layers cannot drift
            // into two different explanations of one refusal.
            if step.get("engine").is_some() {
                return Err(super::helpers::scrape_engine_refusal());
            }
            // GAP-057: honour format/formats like the top-level scrape subcommand.
            let formats = scrape_formats_from_step(step);
            let fmt_refs: Vec<&str> = formats.iter().map(String::as_str).collect();
            session.scrape(url, robots, &fmt_refs).await
        }
        "print-pdf" | "print_pdf" => {
            // GAP-001: Page.printToPDF inside multi-step run (same process as goto/view).
            if let Some(url) = step.get("url").and_then(|v| v.as_str()) {
                let init = step
                    .get("init_script")
                    .or_else(|| step.get("initScript"))
                    .and_then(|v| v.as_str());
                let beforeunload = step_beforeunload_action(step);
                let nav_timeout_ms = step
                    .get("navigation_timeout_ms")
                    .or_else(|| step.get("timeout_ms"))
                    .and_then(|v| v.as_u64());
                let _ = session
                    .goto_with_options(url, robots, init, beforeunload, nav_timeout_ms)
                    .await?;
            } else {
                // GAP-013: refuse blank about:blank PDF unless allow_empty.
                let allow_empty = step
                    .get("allow_empty")
                    .or_else(|| step.get("allowEmpty"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let info = session.page_info().await.unwrap_or_else(|_| json!({}));
                let url_now = info
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or(crate::constants::ABOUT_BLANK);
                if !allow_empty
                    && (url_now.is_empty()
                        || url_now == crate::constants::ABOUT_BLANK
                        || url_now.starts_with("chrome://"))
                {
                    return Err(CliError::with_suggestion(
                        ErrorKind::Usage,
                        "print-pdf requires a navigated page or step url (blank page refused)",
                        crate::i18n::suggestion_key("print_pdf_needs_navigation", None),
                    ));
                }
            }
            let path = step.get("path").and_then(|v| v.as_str()).map(Path::new);
            // `landscape` now travels INTO the call. It used to be written onto
            // the returned object after the fact, with a comment claiming it was
            // "passed through if session supports" -- it was passed nowhere, and
            // `print_pdf` took no such argument. The step answered `ok: true`
            // with `landscape: true` and produced a portrait PDF, so the
            // envelope confirmed a request that was never made.
            let landscape = step
                .get("landscape")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(session.print_pdf(path, landscape).await?)
        }
        "lighthouse" => {
            let url = step
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| CliError::new(ErrorKind::Usage, "lighthouse requires url"))?;
            let out_dir = step
                .get("out_dir")
                .or_else(|| step.get("outDir"))
                .and_then(|v| v.as_str())
                .map(Path::new);
            let device = step
                .get("device")
                .and_then(|v| v.as_str())
                .unwrap_or("desktop");
            let mode = step
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("navigation");
            let lighthouse_path = step
                .get("lighthouse_path")
                .or_else(|| step.get("lighthousePath"))
                .and_then(|v| v.as_str())
                .map(Path::new);
            // External binary; run off the browser session but same process.
            crate::commands::ops::lighthouse_to_value(url, out_dir, device, mode, lighthouse_path)
        }
        other => Err(CliError::new(
            ErrorKind::Usage,
            format!("internal: unexpected cmd in this family: {other}"),
        )),
    }
}
