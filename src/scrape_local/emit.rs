// SPDX-License-Identifier: MIT OR Apache-2.0
//! DRY scrape stdout emit (json envelope path is caller's `emit_ok`; ndjson/csv here).

use serde_json::Value;

use crate::error::{CliError, ErrorKind};

/// Emit pages/results as NDJSON lines (one object per line) to stdout.
///
/// Borrows the array instead of cloning it: these rows carry whole pages of
/// markdown and HTML, so a `.cloned()` here doubled peak memory for a crawl
/// just to iterate. `_json_mode` is accepted for call-site symmetry and does
/// not change the output — one JSON object per line is the pipe-safe shape in
/// both modes.
pub fn emit_ndjson_array(data: &Value, arr_key: &str, _json_mode: bool) -> Result<(), CliError> {
    let Some(arr) = data.get(arr_key).and_then(|v| v.as_array()) else {
        return Ok(());
    };
    let lines = arr
        .iter()
        .map(|item| {
            serde_json::to_string(item)
                .map_err(|e| CliError::new(ErrorKind::Software, format!("json: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    crate::output::writeln_stdout_batch(lines)
}

/// Emit pages/results as CSV with header (agent tabular CLEAN STDOUT).
///
/// Columns: union of object keys in first row order, then extras sorted.
pub fn emit_csv_array(data: &Value, arr_key: &str) -> Result<(), CliError> {
    // Borrowed, not cloned: same reason as `emit_ndjson_array` — the rows hold
    // full page payloads and nothing here mutates them.
    let Some(arr) = data.get(arr_key).and_then(|v| v.as_array()) else {
        return Ok(());
    };
    if arr.is_empty() {
        return Ok(());
    }
    let mut headers: Vec<String> = Vec::new();
    if let Some(Value::Object(first)) = arr.first() {
        for k in first.keys() {
            headers.push(k.clone());
        }
    }
    for item in arr {
        if let Some(obj) = item.as_object() {
            for k in obj.keys() {
                if !headers.iter().any(|h| h == k) {
                    headers.push(k.clone());
                }
            }
        }
    }
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(&headers)
        .map_err(|e| CliError::new(ErrorKind::Software, format!("csv header: {e}")))?;
    for item in arr {
        let mut row = Vec::with_capacity(headers.len());
        for h in &headers {
            let cell = item.get(h).map(csv_cell).unwrap_or_default();
            row.push(cell);
        }
        wtr.write_record(&row)
            .map_err(|e| CliError::new(ErrorKind::Software, format!("csv row: {e}")))?;
    }
    let bytes = wtr
        .into_inner()
        .map_err(|e| CliError::new(ErrorKind::Software, format!("csv flush: {e}")))?;
    let s = String::from_utf8(bytes)
        .map_err(|e| CliError::new(ErrorKind::Software, format!("csv utf8: {e}")))?;
    // Write without extra trailing logic; csv crate ends with newline.
    crate::output::writeln_stdout_batch(s.lines())
}

fn csv_cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Dispatch output mode for batch/crawl envelopes.
///
/// `seed` is only consulted by the `llms-txt` mode, which needs the crawl
/// origin to pick an H1 when no page offers a title.
pub fn emit_scrape_collection(
    data: &Value,
    arr_key: &str,
    output_mode: &str,
    json: bool,
    seed: &str,
) -> Result<bool, CliError> {
    let mode = output_mode.trim().to_ascii_lowercase();
    match mode.as_str() {
        "ndjson" => {
            emit_ndjson_array(data, arr_key, json)?;
            Ok(true)
        }
        "csv" => {
            emit_csv_array(data, arr_key)?;
            Ok(true)
        }
        "llms-txt" | "llmstxt" => {
            super::llms_txt::emit_llms_txt(data, seed)?;
            Ok(true)
        }
        _ => Ok(false), // caller uses emit_ok envelope
    }
}
