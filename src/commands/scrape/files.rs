// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local document parse, QR, and spreadsheet write handlers (no Chrome).

use std::path::Path;

use crate::commands::common::emit_ok;
use crate::error::CliError;

pub(crate) fn handle_parse(path: &Path, redact_pii: bool, json: bool) -> Result<(), CliError> {
    let data = crate::scrape_local::parse_file_opts(path, redact_pii)?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok parse path={}",
            d.get("path").and_then(|v| v.as_str()).unwrap_or("")
        ))?;
        Ok(())
    })
}

pub(crate) fn handle_qr(action: crate::cli::QrAction, json: bool) -> Result<(), CliError> {
    let data = match action {
        crate::cli::QrAction::Encode { text, format, path } => {
            crate::qr_local::encode(&text, &format, path.as_deref())?
        }
        crate::cli::QrAction::Decode { path } => crate::qr_local::decode(&path)?,
    };
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!("ok qr {d}"))
    })
}

pub(crate) fn handle_sheet_write(
    input: &std::path::Path,
    out: &std::path::Path,
    sheet: &str,
    json: bool,
) -> Result<(), CliError> {
    let data = crate::sheet_local::sheet_write(input, out, sheet)?;
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok sheet-write path={} rows={}",
            d.get("path").and_then(|v| v.as_str()).unwrap_or(""),
            d.get("rows").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}
