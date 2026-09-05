// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local document parse, QR, image, and spreadsheet write handlers (no Chrome).

use std::path::Path;

use crate::commands::common::{emit_ok, emit_ok_summary};
use crate::error::CliError;
use crate::image_local::{self, ImageSource};

pub(crate) fn handle_parse(
    path: &Path,
    redact_pii: bool,
    formats: &[String],
    json: bool,
) -> Result<(), CliError> {
    let data = crate::scrape_local::parse_file_formats(path, redact_pii, formats)?;
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
    emit_ok_summary(data, json, "qr")
}

/// Resolve `--path` / `--stdin` / `--paths-file` for the `image` family.
fn image_inputs(
    path: Option<std::path::PathBuf>,
    stdin: bool,
    paths_file: Option<std::path::PathBuf>,
) -> Result<crate::commands::media::MediaInputs<ImageSource>, CliError> {
    crate::commands::media::resolve("image", path, stdin, paths_file, ImageSource::Path, || {
        ImageSource::Stdin
    })
}

pub(crate) fn handle_image(action: crate::cli::ImageAction, json: bool) -> Result<(), CliError> {
    let data = match action {
        crate::cli::ImageAction::Info {
            path,
            stdin,
            paths_file,
            include_gps,
            select,
        } => {
            let inputs = image_inputs(path, stdin, paths_file)?;
            crate::commands::media::run("image", inputs, |src| {
                image_local::info(src, include_gps, select.as_deref())
            })?
        }
        crate::cli::ImageAction::Convert {
            path,
            stdin,
            paths_file,
            format,
            quality,
            out,
            strip_exif,
            keep_exif,
        } => {
            let inputs = image_inputs(path, stdin, paths_file)?;
            crate::commands::media::run_producing(
                "image",
                inputs,
                out.as_deref(),
                |_| format.to_ascii_lowercase(),
                |src, dest| {
                    image_local::convert(src, &format, quality, dest, strip_exif, keep_exif)
                },
            )?
        }
        crate::cli::ImageAction::Resize {
            path,
            stdin,
            paths_file,
            width,
            height,
            keep_aspect,
            out,
            format,
            quality,
        } => {
            let inputs = image_inputs(path, stdin, paths_file)?;
            crate::commands::media::run_producing(
                "image",
                inputs,
                out.as_deref(),
                // Resize without `--format` keeps the single-input default, the
                // XDG image format, so the batch derives the same extension.
                |_| match format.as_deref() {
                    Some(f) => f.to_ascii_lowercase(),
                    None => crate::xdg::resolve_image_default_format(),
                },
                |src, dest| {
                    image_local::resize(
                        src,
                        width,
                        height,
                        keep_aspect,
                        dest,
                        format.as_deref(),
                        quality,
                    )
                },
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
            paths_file,
            include_gps,
            select,
        } => {
            let inputs = image_inputs(path, stdin, paths_file)?;
            crate::commands::media::run("image", inputs, |src| {
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
                Ok(image_local::project_fields(full, select.as_deref()))
            })?
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
    force: bool,
    json: bool,
) -> Result<(), CliError> {
    if !force && out.exists() {
        return Err(CliError::with_suggestion(
            crate::error::ErrorKind::Usage,
            format!("{} already exists; refusing to overwrite", out.display()),
            crate::i18n::suggestion_key("sheet-write-exists", None),
        ));
    }
    // `-o` is required, so the workbook this overwrites is always named in argv.
    let data = crate::etd::with_target(
        crate::sheet_local::sheet_write(input, out, sheet)?,
        &out.display().to_string(),
        crate::etd::TargetSource::Argv,
    );
    emit_ok(data, json, |d| {
        crate::output::writeln_stdout(format!(
            "ok sheet-write path={} rows={}",
            d.get("path").and_then(|v| v.as_str()).unwrap_or(""),
            d.get("rows").and_then(|v| v.as_u64()).unwrap_or(0)
        ))?;
        Ok(())
    })
}
