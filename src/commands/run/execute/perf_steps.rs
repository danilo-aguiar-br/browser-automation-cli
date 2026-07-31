// SPDX-License-Identifier: MIT OR Apache-2.0
//! Perf/screencast/heap/scrape/pdf/lighthouse steps for run/exec.

use std::path::Path;

use serde_json::{json, Value};

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::super::RunFlags;
use super::helpers::{scrape_formats_from_step, step_beforeunload_action};

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
                    session.perf_insight(name, set_id).await
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
                    let path = step.get("path").and_then(|v| v.as_str()).map(Path::new);
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
            // GAP-057: honour format/formats like the top-level scrape subcommand.
            let formats = scrape_formats_from_step(step);
            let fmt_refs: Vec<&str> = formats.iter().map(String::as_str).collect();
            // Prefer browser engine inside `run` (session already live).
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
                    .unwrap_or("about:blank");
                if !allow_empty
                    && (url_now.is_empty()
                        || url_now == "about:blank"
                        || url_now.starts_with("chrome://"))
                {
                    return Err(CliError::with_suggestion(
                        ErrorKind::Usage,
                        "print-pdf requires a navigated page or step url (blank page refused)",
                        "Add {\"cmd\":\"goto\",\"url\":\"…\"} before print-pdf, or pass \"url\" on the step, or allow_empty:true",
                    ));
                }
            }
            let path = step.get("path").and_then(|v| v.as_str()).map(Path::new);
            let mut pdf = session.print_pdf(path).await?;
            // GAP-020: optional landscape/scale when provided (passed through if session supports).
            if let Some(land) = step.get("landscape").and_then(|v| v.as_bool()) {
                pdf["landscape"] = json!(land);
            }
            Ok(pdf)
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
