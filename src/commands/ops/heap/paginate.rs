// SPDX-License-Identifier: MIT OR Apache-2.0
//! Offline pagination and filtering for heap list operations.
//!
//! Split out of the handler so both stay under the file-size gate: the
//! handler decides WHICH snapshot operation runs, this module decides which
//! slice of its result is emitted.

use crate::error::{CliError, ErrorKind};

/// Tool-ref heap filterName enum (closed set).
const HEAP_FILTER_NAME_ENUM: &[&str] = &[
    "objectsRetainedByDetachedDomNodes",
    "objectsRetainedByConsole",
    "objectsRetainedByEventHandlers",
    "objectsRetainedByContexts",
];

pub(super) fn validate_heap_filter_name(filter_name: Option<&str>) -> Result<(), CliError> {
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
pub(super) fn paginate_filter_json(
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
