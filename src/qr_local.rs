// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot QR encode/decode (no Chrome, pure-Rust path).
//!
//! # Workload
//!
//! **CPU-light** single payload. Encode builds one matrix; decode usually finds
//! one grid. Multi-grid decode loops are O(grids) with grids typically 1 —
//! coordination overhead of Rayon exceeds gain (rules: never parallelize when
//! cost ≪ spawn). Sequential is intentional, not an omission.

use std::fs;
use std::path::{Path, PathBuf};

use image::Luma;
use qrcode::QrCode;
use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::xdg;

/// Encode text to PNG/SVG/terminal and optional path.
pub fn encode(text: &str, format: &str, path: Option<&Path>) -> Result<Value, CliError> {
    if text.is_empty() {
        return Err(CliError::new(ErrorKind::Usage, "qr encode requires --text"));
    }
    let code = QrCode::new(text.as_bytes())
        .map_err(|e| CliError::new(ErrorKind::Data, format!("qr encode: {e}")))?;
    let fmt = format.trim().to_ascii_lowercase();
    match fmt.as_str() {
        "png" | "image" => {
            let img = code.render::<Luma<u8>>().quiet_zone(true).build();
            let out = resolve_out_path(path, "qr.png")?;
            img.save(&out).map_err(|e| {
                CliError::new(
                    ErrorKind::Io,
                    format!("write qr png {}: {e}", out.display()),
                )
            })?;
            Ok(json!({
                "action": "encode",
                "format": "png",
                "path": out.display().to_string(),
                "bytes": text.len(),
                "engine": "qrcode",
            }))
        }
        "svg" => {
            let svg = code
                .render::<qrcode::render::svg::Color>()
                .quiet_zone(true)
                .build();
            let out = resolve_out_path(path, "qr.svg")?;
            fs::write(&out, svg.as_bytes()).map_err(|e| {
                CliError::new(
                    ErrorKind::Io,
                    format!("write qr svg {}: {e}", out.display()),
                )
            })?;
            Ok(json!({
                "action": "encode",
                "format": "svg",
                "path": out.display().to_string(),
                "engine": "qrcode",
            }))
        }
        "terminal" | "text" | "unicode" => {
            let s = code
                .render::<char>()
                .quiet_zone(true)
                .module_dimensions(2, 1)
                .build();
            Ok(json!({
                "action": "encode",
                "format": "terminal",
                "matrix": s,
                "engine": "qrcode",
            }))
        }
        other => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unknown qr format: {other}"),
            "Use png|svg|terminal",
        )),
    }
}

/// Decode QR payload from an image file.
pub fn decode(path: &Path) -> Result<Value, CliError> {
    let img = image::open(path).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("qr decode open {}: {e}", path.display()),
        )
    })?;
    let luma = img.to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(luma);
    let grids = prepared.detect_grids();
    if grids.is_empty() {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            "no QR code detected in image",
            "Use a clear PNG/JPEG of a QR with quiet zone",
        ));
    }
    let mut payloads = Vec::new();
    for g in grids {
        match g.decode() {
            Ok((_meta, content)) => payloads.push(content),
            Err(e) => payloads.push(format!("decode_error:{e}")),
        }
    }
    Ok(json!({
        "action": "decode",
        "path": path.display().to_string(),
        "count": payloads.len(),
        "payloads": payloads,
        "engine": "rqrr",
    }))
}

fn resolve_out_path(path: Option<&Path>, default_name: &str) -> Result<PathBuf, CliError> {
    if let Some(p) = path {
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| CliError::new(ErrorKind::Io, format!("create qr dir: {e}")))?;
            }
        }
        return Ok(p.to_path_buf());
    }
    let dir = xdg::cache_dir().unwrap_or_else(|_| PathBuf::from("."));
    fs::create_dir_all(&dir).ok();
    Ok(dir.join(default_name))
}
