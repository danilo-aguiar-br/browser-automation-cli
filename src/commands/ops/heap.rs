// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::{CaptureOpts, OneShotSession};
use crate::cli::HeapAction;
use crate::commands::common::emit_ok;
use crate::commands::nav::with_session_blank;
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;

pub(crate) fn handle_heap(
    life: &Lifecycle,
    action: HeapAction,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    match action {
        HeapAction::Take { path } => {
            let data =
                with_session_blank(life, capture, timeout_secs, move |mut session| async move {
                    let v = session.heap_take(&path).await?;
                    Ok((session, v))
                })?;
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
                max_siblings.unwrap_or(32) as usize,
                max_nodes.unwrap_or(200) as usize,
                200,
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

/// Tool-ref heap filterName enum (closed set).
const HEAP_FILTER_NAME_ENUM: &[&str] = &[
    "objectsRetainedByDetachedDomNodes",
    "objectsRetainedByConsole",
    "objectsRetainedByEventHandlers",
    "objectsRetainedByContexts",
];

pub(crate) fn validate_heap_filter_name(filter_name: Option<&str>) -> Result<(), CliError> {
    let Some(f) = filter_name else {
        return Ok(());
    };
    // Free-text substring filters stay allowed; enum-like names must match tool-ref.
    if f.starts_with("objectsRetained")
        && !HEAP_FILTER_NAME_ENUM
            .iter()
            .any(|e| e.eq_ignore_ascii_case(f))
    {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("invalid heap --filter-name enum: {f}"),
            crate::i18n::suggestion_key("use_listed_value", None),
        ));
    }
    Ok(())
}

/// Paginate/filter a JSON array field for heap list ops (tool-ref pageIdx/pageSize/filterName).
pub(crate) fn paginate_filter_json(
    data: &mut serde_json::Value,
    array_key: &str,
    filter_name: Option<&str>,
    page_idx: Option<usize>,
    page_size: Option<usize>,
) {
    let key = {
        if data.get(array_key).and_then(|v| v.as_array()).is_some() {
            array_key.to_string()
        } else {
            let mut found = None;
            for alt in ["items", "results", "list"] {
                if data.get(alt).and_then(|v| v.as_array()).is_some() {
                    found = Some(alt.to_string());
                    break;
                }
            }
            match found {
                Some(k) => k,
                None => return,
            }
        }
    };

    let is_enum_filter = filter_name
        .map(|f| {
            HEAP_FILTER_NAME_ENUM
                .iter()
                .any(|e| e.eq_ignore_ascii_case(f))
        })
        .unwrap_or(false);

    if is_enum_filter {
        // Offline heapsnapshot parser does not recompute retainer-kind filters;
        // record the requested enum for agents and keep full list (honest Partial).
        // `is_enum_filter` is only true when `filter_name` was Some (see above).
        if let (Some(name), Some(obj)) = (filter_name, data.as_object_mut()) {
            obj.insert("filter_name".into(), serde_json::json!(name));
            obj.insert(
                "filter_applied".into(),
                serde_json::json!("enum_recorded_offline_not_recomputed"),
            );
        }
    }

    let Some(arr) = data.get_mut(&key).and_then(|v| v.as_array_mut()) else {
        return;
    };
    if let Some(f) = filter_name {
        if !is_enum_filter {
            let f_low = f.to_ascii_lowercase();
            arr.retain(|item| {
                item.get("name")
                    .or_else(|| item.get("class_name"))
                    .or_else(|| item.get("string"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_ascii_lowercase().contains(&f_low))
                    .unwrap_or(true)
            });
        }
    }
    let total = arr.len();
    let page = page_idx.unwrap_or(0);
    let size = page_size.unwrap_or(total.max(1));
    let start = page.saturating_mul(size).min(total);
    let end = (start + size).min(total);
    let page_items: Vec<serde_json::Value> = arr[start..end].to_vec();
    *arr = page_items;
    if let Some(obj) = data.as_object_mut() {
        obj.insert("total".into(), serde_json::json!(total));
        obj.insert("page_idx".into(), serde_json::json!(page));
        obj.insert("page_size".into(), serde_json::json!(size));
    }
}
