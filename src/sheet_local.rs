// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot XLSX write via `rust_xlsxwriter` (§5Z / GAP-A011).
//!
//! Read path remains `calamine` in `scrape_local`/`parse`. This module is write-only.
//!
//! # Workload
//!
//! **CPU-light + disk I/O** for a single workbook. Sequential by design:
//! `rust_xlsxwriter::Workbook` is not shared across threads, and one-shot
//! sheet sizes are small enough that Rayon overhead would dominate. Fan-out
//! belongs to callers that produce many independent workbooks (not this path).

use std::fs;
use std::path::Path;

use rust_xlsxwriter::Workbook;
use serde_json::Value;

use crate::error::{CliError, ErrorKind};

/// Write an XLSX workbook from CSV or JSON array-of-objects input.
pub fn sheet_write(input: &Path, out: &Path, sheet_name: &str) -> Result<Value, CliError> {
    // GAP-026, read axis. Both operator-named paths are bounded here rather
    // than inside `read_csv_rows` and its JSON sibling, because the check
    // belongs where the OPERATOR's path enters — the readers underneath are
    // shared with `xdg::config_io`, which legitimately reads the product's own
    // `config.toml` from a directory that is not an allowed root.
    crate::fs_roots::ensure_read_allowed(input)?;
    crate::fs_roots::ensure_write_allowed(out)?;
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
                crate::i18n::suggestion_key("sheet_input_format", None),
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
        "{}{}.tmp",
        crate::constants::XLSX_TMP_NAME_PREFIX,
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

/// Read CSV rows with the same byte ceiling the JSON arm already enforces.
///
/// # Why a ceiling was missing here
///
/// [`read_json_rows`] below gates its input through
/// [`crate::json_util::read_json_value_file`] with
/// `max_json_file_bytes`, but this arm called `fs::read` with no
/// `metadata` check at all — and both are the SAME surface: `sheet write`
/// accepts either format for the same operation. Two formats of one input with
/// different ceilings is the incoherence; sharing the ceiling is the fix.
///
/// The cost is not just the file: the `Vec<Vec<String>>` this returns is
/// LARGER than the bytes on disk, because every field carries a `String`
/// header plus its own allocation. A CSV of short fields inflates several-fold
/// on the way in, so the unbounded read was the cheaper half of the problem.
///
/// # Why the LOSSY reader, and not the strict one next to it
///
/// Sharing the ceiling with the JSON arm is right; sharing the DECODER is not.
/// The obvious-looking `read_text_file_limited` finishes with `read_to_string`,
/// which is strict UTF-8, and CSV carries no encoding guarantee at all: the
/// default Excel export in a pt-BR locale is windows-1252, so `José` arrives as
/// the byte `0xE9`. Under the strict reader that ordinary spreadsheet failed
/// the whole command; under the lossy one the byte becomes `U+FFFD` and the
/// remaining 99% of the sheet still converts. JSON keeps the strict reader
/// because RFC 8259 mandates UTF-8 there — the format decides, not the ceiling.
fn read_csv_rows(path: &Path, delim: u8) -> Result<Vec<Vec<String>>, CliError> {
    let raw = crate::json_util::read_text_file_limited_lossy(
        path,
        crate::xdg::resolve_max_json_file_bytes(),
    )?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delim)
        .flexible(true)
        .has_headers(false)
        .from_reader(raw.as_bytes());
    let mut rows = Vec::new();
    for rec in rdr.records() {
        let rec = rec.map_err(|e| CliError::new(ErrorKind::Data, format!("csv: {e}")))?;
        rows.push(rec.iter().map(|s| s.to_string()).collect());
    }
    Ok(rows)
}

fn read_json_rows(path: &Path) -> Result<Vec<Vec<String>>, CliError> {
    let v: Value =
        crate::json_util::read_json_value_file(path, crate::xdg::resolve_max_json_file_bytes())?;
    let arr = v.as_array().ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Data,
            crate::i18n::suggestion_key("json_array_objects", None),
            crate::i18n::suggestion_key("sheet_json_rows_example", None),
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

    /// A windows-1252 CSV converts instead of failing the whole command.
    ///
    /// This is the default export of Excel in a pt-BR locale, so it is the
    /// ordinary input rather than a corner case. Asserting the replacement
    /// character — and not merely `is_ok()` — pins WHICH decoder runs: the
    /// strict one cannot produce `U+FFFD`, it can only error.
    #[test]
    fn csv_latin1_converts_lossily_instead_of_failing() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("l.csv");
        // `José,São Paulo`: `0xE9` and `0xE3` are invalid UTF-8.
        fs::write(&p, b"nome,cidade\nJos\xe9,S\xe3o Paulo\n").unwrap();
        let rows = read_csv_rows(&p, b',').expect("latin-1 CSV must not fail the command");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1][0], "Jos\u{FFFD}");
        assert_eq!(rows[1][1], "S\u{FFFD}o Paulo");
    }

    /// Control group: the same document in UTF-8 keeps its accents intact.
    ///
    /// Without this pair the lossy assertion above would also hold for a
    /// decoder that mangled everything, so the control is what makes the
    /// latin-1 result attributable to the ENCODING and not to the reader.
    #[test]
    fn csv_utf8_accents_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("u.csv");
        fs::write(&p, "nome,cidade\nJosé,São Paulo\n".as_bytes()).unwrap();
        let rows = read_csv_rows(&p, b',').expect("utf-8 CSV");
        assert_eq!(rows[1][0], "José");
        assert_eq!(rows[1][1], "São Paulo");
    }

    /// End to end: a latin-1 CSV reaches a written workbook, not an error exit.
    #[test]
    fn sheet_write_accepts_latin1_csv_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("l.csv");
        fs::write(&csv_path, b"nome\nJos\xe9\n").unwrap();
        let out = dir.path().join("l.xlsx");
        let v = sheet_write(&csv_path, &out, "Sheet1").expect("latin-1 CSV end to end");
        assert_eq!(v.get("rows").and_then(|r| r.as_u64()), Some(2));
        assert!(out.exists());
    }

    /// A TSV keeps the same decoder, because the delimiter is not the encoding.
    #[test]
    fn tsv_latin1_converts_lossily_too() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("l.tsv");
        fs::write(&p, b"nome\tcidade\nJos\xe9\tS\xe3o Paulo\n").unwrap();
        let rows = read_csv_rows(&p, b'\t').expect("latin-1 TSV");
        assert_eq!(rows[1][0], "Jos\u{FFFD}");
    }
}
