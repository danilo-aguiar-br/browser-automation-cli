// SPDX-License-Identifier: MIT OR Apache-2.0
//! Spreadsheet text extraction backend for `parse` (xlsx/xlsm/xls/ods).
//!
//! Split out of `parse.rs` so the sequential-by-necessity note below sits next
//! to the loop it constrains instead of three screens away from it.

use std::path::Path;

use crate::error::{CliError, ErrorKind};

/// Flatten every worksheet into tab-separated lines, one sheet header per sheet.
///
/// Returns `(kind, text, engine)`.
///
/// # Errors
///
/// - [`ErrorKind::Data`] when `calamine` cannot open the workbook, which covers
///   an unreadable file, an unsupported variant and a corrupt archive alike
///
/// A worksheet that fails to range is SKIPPED rather than fatal: one damaged
/// sheet should not discard the rest of a workbook the caller can still use.
pub(crate) fn parse_spreadsheet(
    path: &Path,
) -> Result<(&'static str, String, &'static str), CliError> {
    use calamine::{open_workbook_auto, Data, Reader};
    let mut workbook = open_workbook_auto(path)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("spreadsheet open: {e}")))?;
    let mut lines = Vec::new();
    // PAR-59: multi-sheet is sequential — calamine `Reader` is not Sync; opening
    // once and ranging sheets in order is correct and cost ≪ coordination for
    // typical agent workbooks. Do not `par_iter` worksheet_range on &mut self.
    // calamine::Reader::sheet_names returns owned `Vec<String>` — do not re-clone.
    let sheets = workbook.sheet_names();
    for name in sheets {
        if let Ok(range) = workbook.worksheet_range(&name) {
            lines.push(format!("# sheet: {name}"));
            for row in range.rows() {
                let cells: Vec<String> = row
                    .iter()
                    .map(|c| match c {
                        Data::Empty => String::new(),
                        Data::String(s) => s.clone(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        Data::DateTime(dt) => format!("{dt:?}"),
                        Data::DateTimeIso(s) => s.clone(),
                        Data::DurationIso(s) => s.clone(),
                        Data::Error(e) => format!("{e:?}"),
                    })
                    .collect();
                lines.push(cells.join("\t"));
            }
        }
    }
    Ok(("spreadsheet", lines.join("\n"), "calamine"))
}
