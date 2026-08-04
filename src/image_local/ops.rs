// SPDX-License-Identifier: MIT OR Apache-2.0
//! High-level image info / convert / resize operations.

use std::io::Read;
use std::path::{Path, PathBuf};

use image::GenericImageView;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::decode::{decode_bytes, decode_path};
use super::encode::{encode_to_path, parse_output_format};
use super::exif::read_exif_map;
use super::limits::ImageLimits;
use super::magic::{detect_format, DetectedFormat};
use crate::error::{CliError, ErrorKind};
use crate::xdg;

/// Where image bytes come from for one-shot ops.
#[derive(Debug, Clone)]
pub enum ImageSource {
    /// Filesystem path.
    Path(PathBuf),
    /// Read stdin to EOF (bounded by image_max_input_bytes).
    Stdin,
}

impl ImageSource {
    /// Load raw bytes + optional path label.
    pub fn load_bytes(&self, limits: ImageLimits) -> Result<(Vec<u8>, Option<PathBuf>), CliError> {
        match self {
            Self::Path(p) => {
                let meta = std::fs::metadata(p)
                    .map_err(|e| crate::image_local::magic::io_open_err(p, &e))?;
                limits.check_input_len(meta.len() as usize)?;
                let b = std::fs::read(p)
                    .map_err(|e| crate::image_local::magic::io_path_err(p, "read", &e))?;
                Ok((b, Some(p.clone())))
            }
            Self::Stdin => {
                let mut buf = Vec::new();
                let stdin = std::io::stdin();
                let mut handle = stdin.lock();
                // Cap read to max_input_bytes + 1 to detect overflow.
                let mut chunk = [0u8; 64 * 1024];
                loop {
                    let n = handle.read(&mut chunk).map_err(|e| {
                        CliError::new(ErrorKind::Io, format!("image stdin read: {e}"))
                    })?;
                    if n == 0 {
                        break;
                    }
                    if buf.len().saturating_add(n) > limits.max_input_bytes {
                        return Err(CliError::with_suggestion(
                            ErrorKind::Data,
                            format!(
                                "stdin image exceeds image_max_input_bytes {}",
                                limits.max_input_bytes
                            ),
                            crate::i18n::suggestion_key("image_too_large", None),
                        ));
                    }
                    buf.extend_from_slice(&chunk[..n]);
                }
                if buf.is_empty() {
                    return Err(CliError::new(ErrorKind::NoInput, "empty stdin for image"));
                }
                Ok((buf, None))
            }
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

fn default_out_path(format: &str) -> Result<PathBuf, CliError> {
    let dir = xdg::cache_dir().unwrap_or_else(|_| PathBuf::from("."));
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    Ok(dir.join(format!("image-{stamp}.{format}")))
}

/// Probe format, dimensions, optional EXIF (no pixel dump).
///
/// `select` is an optional CSV of field names to project (agent-native anti-token).
pub fn info(
    source: &ImageSource,
    include_gps: bool,
    select: Option<&str>,
) -> Result<Value, CliError> {
    let limits = ImageLimits::from_xdg();
    let (bytes, path) = source.load_bytes(limits)?;
    let fmt = detect_format(&bytes)?;
    let decoded = decode_bytes(&bytes, limits)?;
    let (w, h) = decoded.image.dimensions();
    let has_alpha = decoded.image.color().has_alpha();
    let exif = read_exif_map(&bytes, include_gps).unwrap_or_default();
    let iptc = super::iptc::read_iptc_map(&bytes).unwrap_or_default();
    let xmp = super::xmp::read_xmp_map(&bytes).unwrap_or_default();
    // Real animation frame count (GAP-IMG-093). A malformed GIF must not sink
    // `info`, so a walk failure degrades to the single decoded frame — but the
    // envelope says so via `frame_count_exact`. Reporting a silent `1` would
    // reintroduce exactly the lie this gap existed to remove.
    let (frame_count, frame_count_exact) = if fmt == DetectedFormat::Gif {
        match super::gif_frames::frame_count(&bytes) {
            Ok(n) => (u64::from(n), true),
            Err(e) => {
                tracing::debug!(error = %e.message(), "gif frame walk failed; reporting decoded frame only");
                (1, false)
            }
        }
    } else {
        // Every other format this build decodes is single-frame by construction.
        (1, true)
    };
    let full = json!({
        "action": "info",
        "format": fmt.as_str(),
        "width": w,
        "height": h,
        "bytes": bytes.len(),
        "path": path.as_ref().map(|p| p.display().to_string()),
        "magic_ok": true,
        "has_alpha": has_alpha,
        "frame_count": frame_count,
        "frame_count_exact": frame_count_exact,
        "animated": frame_count > 1,
        "exif": exif,
        "iptc": iptc,
        "xmp": xmp,
        "sha256": sha256_hex(&bytes),
        "engine": "image",
    });
    Ok(project_fields(full, select))
}

/// Project a JSON object to a subset of keys (CSV). Unknown keys are ignored.
/// Always retains `action` for agent routing (agent-native anti-token).
///
/// Agent-friendly aliases (input only; output uses canonical keys):
/// - `tags` → `exif`
/// - `tag_count` → `count`
pub(crate) fn project_fields(value: Value, select: Option<&str>) -> Value {
    crate::json_util::project_fields(
        value,
        select,
        &[
            ("tags", "exif"),
            ("tag_count", "count"),
            ("frames", "frame_count"),
        ],
    )
}

/// Convert to another format; pixel re-encode always drops EXIF (agent-honest).
pub fn convert(
    source: &ImageSource,
    format: &str,
    quality: Option<u8>,
    out: Option<&Path>,
    strip_exif: bool,
    keep_exif: bool,
) -> Result<Value, CliError> {
    let limits = ImageLimits::from_xdg();
    let (bytes, _path) = source.load_bytes(limits)?;
    let decoded = decode_bytes(&bytes, limits)?;
    let out_fmt = parse_output_format(format)?;
    let q = quality.unwrap_or(limits.default_quality);
    let wire = out_fmt.wire_name();
    let out_path = match out {
        Some(p) => p.to_path_buf(),
        None => default_out_path(wire)?,
    };
    // Pixel re-encode always drops EXIF APP1/chunks; keep_exif cannot be honored yet.
    let written = encode_to_path(&decoded.image, &out_path, out_fmt, q)?;
    let out_bytes = std::fs::read(&out_path).unwrap_or_default();
    let magic_ok = detect_format(&out_bytes)
        .map(|f| f.as_str() == wire || (wire == "jpeg" && f.as_str() == "jpeg"))
        .unwrap_or(false);
    let (w, h) = decoded.image.dimensions();
    // image 0.25 WebPEncoder is lossless-only (docs.rs); quality reaches jpeg and avif.
    let quality_applied = out_fmt.quality_applies();
    Ok(json!({
        "action": "convert",
        "format": wire,
        "width": w,
        "height": h,
        "bytes": written,
        "path": out_path.display().to_string(),
        "magic_ok": magic_ok,
        "quality": q,
        "quality_applied": quality_applied,
        "exif_stripped": true,
        "strip_exif_requested": strip_exif && !keep_exif,
        "keep_exif_requested": keep_exif,
        "keep_exif_honored": false,
        "sha256": sha256_hex(&out_bytes),
        "engine": if wire == "avif" { "ravif" } else { "image" },
        "src_format": decoded.format.as_str(),
    }))
}

/// Resize with optional aspect-preserving width-only scale.
pub fn resize(
    source: &ImageSource,
    width: u32,
    height: Option<u32>,
    keep_aspect: bool,
    out: Option<&Path>,
    format: Option<&str>,
    quality: Option<u8>,
) -> Result<Value, CliError> {
    if width == 0 {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            "image resize requires --width > 0",
            crate::i18n::suggestion_key("use_listed_value", None),
        ));
    }
    let limits = ImageLimits::from_xdg();
    let (bytes, _) = source.load_bytes(limits)?;
    let decoded = decode_bytes(&bytes, limits)?;
    let (src_w, src_h) = decoded.image.dimensions();
    let (dst_w, dst_h) = if keep_aspect || height.is_none() {
        let h = height.unwrap_or_else(|| {
            let scale = width as f64 / src_w.max(1) as f64;
            ((src_h as f64) * scale).round().max(1.0) as u32
        });
        if keep_aspect {
            let scale = (width as f64 / src_w.max(1) as f64).min(h as f64 / src_h.max(1) as f64);
            (
                ((src_w as f64) * scale).round().max(1.0) as u32,
                ((src_h as f64) * scale).round().max(1.0) as u32,
            )
        } else {
            (width, h)
        }
    } else {
        (width, height.unwrap_or(src_h).max(1))
    };
    limits.check_dimensions(dst_w, dst_h)?;
    // GAP-IMG-095: SIMD path when `image-simd-resize` is on, imageops otherwise.
    let resized = super::resize_simd::resize_exact(&decoded.image, dst_w, dst_h)?;
    let out_fmt_name = format
        .map(|s| s.to_string())
        .unwrap_or_else(crate::xdg::resolve_image_default_format);
    let out_fmt = parse_output_format(&out_fmt_name)?;
    let wire = out_fmt.wire_name();
    let q = quality.unwrap_or(limits.default_quality);
    let out_path = match out {
        Some(p) => p.to_path_buf(),
        None => default_out_path(wire)?,
    };
    let written = encode_to_path(&resized, &out_path, out_fmt, q)?;
    let out_bytes = std::fs::read(&out_path).unwrap_or_default();
    let quality_applied = out_fmt.quality_applies();
    Ok(json!({
        "action": "resize",
        "format": wire,
        "width": dst_w,
        "height": dst_h,
        "src_width": src_w,
        "src_height": src_h,
        "bytes": written,
        "path": out_path.display().to_string(),
        "magic_ok": detect_format(&out_bytes).is_ok(),
        "keep_aspect": keep_aspect,
        "quality": q,
        "quality_applied": quality_applied,
        "sha256": sha256_hex(&out_bytes),
        "engine": "image",
        "resize_backend": super::resize_simd::backend(),
    }))
}

/// Decode path helper used by QR (magic-first).
pub fn decode_path_for_qr(path: &Path) -> Result<image::DynamicImage, CliError> {
    Ok(decode_path(path, ImageLimits::from_xdg())?.image)
}
