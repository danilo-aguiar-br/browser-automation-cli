// SPDX-License-Identifier: MIT OR Apache-2.0
//! `heap` command handler: live capture plus offline snapshot analysis.

mod paginate;

use paginate::{paginate_filter_json, validate_heap_filter_name};

use crate::browser::{CaptureOpts, OneShotSession};
use crate::cli::HeapAction;
use crate::commands::common::emit_ok;
use crate::commands::nav::with_session_at;
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;
use crate::robots::RobotsPolicy;

/// Run a `heap` subcommand and write the agent envelope.
///
/// Only `take` touches Chrome; every other action reads a `.heapsnapshot` file
/// offline, which is why `robots` reaches exactly one arm.
///
/// # Errors
///
/// Propagates browser, IO, and parse failures as [`CliError`].
pub(crate) fn handle_heap(
    life: &Lifecycle,
    action: HeapAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    robots: RobotsPolicy,
    json: bool,
) -> Result<(), CliError> {
    match action {
        HeapAction::Take { path, url } => {
            let data = with_session_at(
                life,
                capture,
                timeout_secs,
                url,
                robots,
                move |mut session| async move {
                    let v = session.heap_take(&path).await?;
                    Ok((session, v))
                },
            )?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!(
                    "ok heap take path={}",
                    d.get("path").and_then(|v| v.as_str()).unwrap_or("")
                ))?;
                Ok(())
            })
        }
        HeapAction::Close { path } => {
            let data = OneShotSession::heap_close(&path)?;
            emit_ok(data, json, |_| {
                crate::output::writeln_stdout(format!("ok heap close path={}", path.display()))
            })
        }
        HeapAction::Compare {
            base,
            current,
            class_index,
        } => {
            let mut data = OneShotSession::heap_compare(&base, &current)?;
            if let Some(ci) = class_index {
                if let Some(obj) = data.as_object_mut() {
                    obj.insert("class_index".into(), serde_json::json!(ci));
                }
            }
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok heap compare {d}"))
            })
        }
        HeapAction::Summary { path } => {
            let data = OneShotSession::heap_file_summary(&path)?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok heap summary {d}"))
            })
        }
        HeapAction::Details {
            path,
            filter_name,
            page_idx,
            page_size,
        } => {
            validate_heap_filter_name(filter_name.as_deref())?;
            let mut data = OneShotSession::heap_details(&path)?;
            paginate_filter_json(
                &mut data,
                "classes",
                filter_name.as_deref(),
                page_idx,
                page_size,
            );
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok heap details {d}"))
            })
        }
        HeapAction::DupStrings {
            path,
            page_idx,
            page_size,
        } => {
            let mut data = OneShotSession::heap_dup_strings(&path)?;
            paginate_filter_json(&mut data, "strings", None, page_idx, page_size);
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok heap dup-strings {d}"))
            })
        }
        HeapAction::ClassNodes {
            path,
            id,
            filter_name,
            page_idx,
            page_size,
        } => {
            validate_heap_filter_name(filter_name.as_deref())?;
            let mut data = OneShotSession::heap_class_nodes(&path, id)?;
            paginate_filter_json(
                &mut data,
                "nodes",
                filter_name.as_deref(),
                page_idx,
                page_size,
            );
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok heap class-nodes {d}"))
            })
        }
        HeapAction::Dominators { path, node } => {
            let data = OneShotSession::heap_node_op(&path, node, "dominators")?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok heap dominators {d}"))
            })
        }
        HeapAction::Edges {
            path,
            node,
            page_idx,
            page_size,
        } => {
            let mut data = OneShotSession::heap_node_op(&path, node, "edges")?;
            paginate_filter_json(&mut data, "edges", None, page_idx, page_size);
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok heap edges {d}"))
            })
        }
        HeapAction::Retainers {
            path,
            node,
            page_idx,
            page_size,
        } => {
            let mut data = OneShotSession::heap_node_op(&path, node, "retainers")?;
            paginate_filter_json(&mut data, "retainers", None, page_idx, page_size);
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok heap retainers {d}"))
            })
        }
        HeapAction::Paths {
            path,
            node,
            max_depth,
            max_nodes,
            max_siblings,
        } => {
            let data = crate::native::heap_snapshot::node_op_with_limits(
                &path,
                node,
                "paths",
                max_depth as usize,
                // A flag that is absent must fall back to the SAME ceiling the
                // default path uses, and that ceiling is now the XDG knob. The
                // literals that used to sit here (32 / 200 / 200) silently
                // shadowed `heap_max_paths`, `heap_max_retainers` and
                // `heap_max_edges`, so `heap paths` ignored the operator while
                // every sibling operation honoured them.
                max_siblings.map_or_else(
                    crate::native::heap_snapshot::limits::default_max_paths,
                    |n| n as usize,
                ),
                max_nodes.map_or_else(
                    crate::native::heap_snapshot::limits::default_max_retainers,
                    |n| n as usize,
                ),
                crate::native::heap_snapshot::limits::default_max_edges(),
            )
            .map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Io,
                    e,
                    crate::i18n::suggestion_key("heap_snapshot_input", None),
                )
            })?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok heap paths {d}"))
            })
        }
        HeapAction::ObjectDetails { path, node } => {
            let data = OneShotSession::heap_object_details(&path, node)?;
            emit_ok(data, json, |d| {
                crate::output::writeln_stdout(format!("ok heap object-details {d}"))
            })
        }
    }
}
