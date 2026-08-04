// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local document parse, QR, image, and spreadsheet write handlers (no Chrome).

use std::path::Path;

use crate::commands::common::emit_ok;
use crate::error::{CliError, ErrorKind};
use crate::image_local::{self, ImageSource};

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

fn image_source(path: Option<std::path::PathBuf>, stdin: bool) -> Result<ImageSource, CliError> {
    if stdin {
        return Ok(ImageSource::Stdin);
    }
    let p = path.ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            "image requires --path or --stdin",
            crate::i18n::suggestion_key("file_path_invalid", None),
        )
    })?;
    Ok(ImageSource::Path(p))
}

pub(crate) fn handle_image(action: crate::cli::ImageAction, json: bool) -> Result<(), CliError> {
    let data = match action {
        crate::cli::ImageAction::Info {
            path,
            stdin,
            include_gps,
            select,
        } => {
            let src = image_source(path, stdin)?;
            image_local::info(&src, include_gps, select.as_deref())?
        }
        crate::cli::ImageAction::Convert {
            path,
            stdin,
            format,
            quality,
            out,
            strip_exif,
            keep_exif,
        } => {
            let src = image_source(path, stdin)?;
            image_local::convert(
                &src,
                &format,
                quality,
                out.as_deref(),
                strip_exif,
                keep_exif,
            )?
        }
        crate::cli::ImageAction::Resize {
            path,
            stdin,
            width,
            height,
            keep_aspect,
            out,
            format,
            quality,
        } => {
            let src = image_source(path, stdin)?;
            image_local::resize(
                &src,
                width,
                height,
                keep_aspect,
                out.as_deref(),
                format.as_deref(),
                quality,
            )?
        }
        crate::cli::ImageAction::Download {
            url,
            out,
            max_bytes,
            require_image,
            allow_non_image,
        } => {
            let require = if allow_non_image {
                false
            } else {
                require_image
            };
            // One-shot local HTTP; reuse shared Tokio runtime budget.
            crate::runtime_util::block_on_io(image_local::download_image(
                &url,
                out.as_deref(),
                max_bytes,
                require,
            ))?
        }
        crate::cli::ImageAction::Exif {
            path,
            stdin,
            include_gps,
            select,
        } => {
            let src = image_source(path, stdin)?;
            let limits = image_local::ImageLimits::from_xdg();
            let (bytes, p) = src.load_bytes(limits)?;
            let map = image_local::read_exif_map(&bytes, include_gps)?;
            let full = serde_json::json!({
                "action": "exif",
                "path": p.as_ref().map(|x| x.display().to_string()),
                "count": map.len(),
                "exif": map,
                "include_gps": include_gps,
                "engine": "kamadak-exif",
            });
            image_local::project_fields(full, select.as_deref())
        }
    };
    emit_ok(data, json, |d| {
        let action = d.get("action").and_then(|v| v.as_str()).unwrap_or("image");
        let path = d.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let format = d.get("format").and_then(|v| v.as_str()).unwrap_or("");
        let w = d
            .get("width")
            .and_then(|v| v.as_u64())
            .map(|n| n.to_string())
            .unwrap_or_default();
        let h = d
            .get("height")
            .and_then(|v| v.as_u64())
            .map(|n| n.to_string())
            .unwrap_or_default();
        if w.is_empty() {
            crate::output::writeln_stdout(format!(
                "ok image action={action} format={format} path={path}"
            ))?;
        } else {
            crate::output::writeln_stdout(format!(
                "ok image action={action} format={format} w={w} h={h} path={path}"
            ))?;
        }
        Ok(())
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
