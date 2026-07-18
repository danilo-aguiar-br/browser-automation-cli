//! One-shot XLSX write via `rust_xlsxwriter` (§5Z / GAP-A011).
//!
//! Read path remains `calamine` in `scrape_local`/`parse`. This module is write-only.

use std::fs;
use std::path::Path;

use rust_xlsxwriter::Workbook;
use serde_json::Value;

use crate::error::{CliError, ErrorKind};

/// Write an XLSX workbook from CSV or JSON array-of-objects input.
pub fn sheet_write(input: &Path, out: &Path, sheet_name: &str) -> Result<Value, CliError> {
    if sheet_name.trim().is_empty() {
        return Err(CliError::new(
            ErrorKind::Usage,
            "sheet name must not be empty",
        ));
    }
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let rows = match ext.as_str() {
        "csv" | "tsv" => read_csv_rows(input, if ext == "tsv" { b'\t' } else { b',' })?,
        "json" => read_json_rows(input)?,
        other => {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("unsupported sheet-write input extension: {other}"),
                "Pass a .csv, .tsv, or .json (array of objects) file",
            ));
        }
    };
    if rows.is_empty() {
        return Err(CliError::new(ErrorKind::Data, "no rows to write"));
    }

    let mut workbook = Workbook::new();
    let worksheet = workbook
        .add_worksheet()
        .set_name(sheet_name)
        .map_err(|e| CliError::new(ErrorKind::Software, format!("worksheet name: {e}")))?;

    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            worksheet
                .write_string(r as u32, c as u16, cell)
                .map_err(|e| CliError::new(ErrorKind::Software, format!("write cell: {e}")))?;
        }
    }

    // Write to temp then rename for residual-friendly finalize.
    let parent = out.parent().unwrap_or_else(|| Path::new("."));
    let tmp = parent.join(format!(
        ".browser-automation-cli-xlsx-{}.tmp",
        std::process::id()
    ));
    workbook
        .save(&tmp)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("xlsx save: {e}")))?;
    fs::rename(&tmp, out).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        CliError::new(
            ErrorKind::Io,
            format!("rename xlsx {} → {}: {e}", tmp.display(), out.display()),
        )
    })?;

    Ok(serde_json::json!({
        "ok": true,
        "path": out.display().to_string(),
        "rows": rows.len(),
        "cols": rows.first().map(|r| r.len()).unwrap_or(0),
        "sheet": sheet_name,
        "chrome": false,
    }))
}

fn read_csv_rows(path: &Path, delim: u8) -> Result<Vec<Vec<String>>, CliError> {
    let raw = fs::read(path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("read {}: {e}", path.display())))?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delim)
        .flexible(true)
        .has_headers(false)
        .from_reader(raw.as_slice());
    let mut rows = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| CliError::new(ErrorKind::Data, format!("csv: {e}")))?;
        rows.push(rec.iter().map(|s| s.to_string()).collect());
    }
    Ok(rows)
}

fn read_json_rows(path: &Path) -> Result<Vec<Vec<String>>, CliError> {
    let raw = fs::read_to_string(path)
        .map_err(|e| CliError::new(ErrorKind::Io, format!("read {}: {e}", path.display())))?;
    let v: Value = serde_json::from_str(&raw)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("json: {e}")))?;
    let arr = v.as_array().ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Data,
            "JSON input must be an array of objects or array of arrays",
            "Example: [{\"a\":1,\"b\":2},{\"a\":3,\"b\":4}]",
        )
    })?;
    if arr.is_empty() {
        return Ok(Vec::new());
    }
    // Array of arrays.
    if arr[0].is_array() {
        let mut rows = Vec::new();
        for row in arr {
            let cells = row
                .as_array()
                .ok_or_else(|| CliError::new(ErrorKind::Data, "expected array row"))?
                .iter()
                .map(json_cell)
                .collect();
            rows.push(cells);
        }
        return Ok(rows);
    }
    // Array of objects: header = sorted keys of first object for stability.
    let first = arr[0]
        .as_object()
        .ok_or_else(|| CliError::new(ErrorKind::Data, "expected object row"))?;
    let mut keys: Vec<String> = first.keys().cloned().collect();
    keys.sort();
    let mut rows = vec![keys.clone()];
    for obj in arr {
        let map = obj
            .as_object()
            .ok_or_else(|| CliError::new(ErrorKind::Data, "expected object row"))?;
        let row = keys
            .iter()
            .map(|k| map.get(k).map(json_cell).unwrap_or_default())
            .collect();
        rows.push(row);
    }
    Ok(rows)
}

fn json_cell(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_csv_to_xlsx() {
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("t.csv");
        fs::write(&csv_path, "a,b\n1,2\n").unwrap();
        let out = dir.path().join("t.xlsx");
        let v = sheet_write(&csv_path, &out, "Sheet1").unwrap();
        assert!(out.exists());
        assert_eq!(v.get("rows").and_then(|r| r.as_u64()), Some(2));
        // Magic: ZIP/XLSX starts with PK
        let bytes = fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"PK"));
    }
}
