// SPDX-License-Identifier: MIT OR Apache-2.0
//! Magic-byte image format detection (never trust file extensions).

use crate::constants::IMAGE_MAGIC_PROBE_BYTES;
use crate::error::{CliError, ErrorKind};

/// Formats this build can detect. Decode support is per-format and, for HEIC,
/// per-Cargo-feature — see [`DetectedFormat::is_supported`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedFormat {
    /// ISO/IEC 10918-1 JPEG (`FF D8 FF`).
    Jpeg,
    /// PNG signature (8 bytes).
    Png,
    /// RIFF container with WEBP fourcc.
    Webp,
    /// GIF87a or GIF89a.
    Gif,
    /// HEIF brand `avif` / `avis`. Encode-only: no pure-Rust AV1 decoder exists
    /// without a C assembler, so decode stays rejected (see `image_local::avif`).
    Avif,
    /// HEIF brand `heic` / `heix` / `mif1`. Decode-only, and only when the
    /// `image-heic` feature is on (see `image_local::heic`).
    Heic,
    /// SVG document (XML, not magic bytes). Rasterised behind `image-svg`.
    Svg,
}

impl DetectedFormat {
    /// Stable lowercase wire name for envelopes.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::Webp => "webp",
            Self::Gif => "gif",
            Self::Avif => "avif",
            Self::Heic => "heic",
            Self::Svg => "svg",
        }
    }

    /// True when this build can decode the format at all.
    ///
    /// HEIC and SVG are gated on Cargo features, so this is a runtime answer
    /// rather than a constant: a caller must not assume detection implies decode.
    #[must_use]
    pub fn is_supported(self) -> bool {
        match self {
            Self::Jpeg | Self::Png | Self::Webp | Self::Gif => true,
            Self::Heic => crate::image_local::heic_decode_available(),
            Self::Svg => crate::image_local::svg_raster_available(),
            Self::Avif => false,
        }
    }

    /// True when this build can *encode* to the format.
    #[must_use]
    pub fn is_encodable(self) -> bool {
        match self {
            Self::Jpeg | Self::Png | Self::Webp | Self::Gif => true,
            Self::Avif => crate::image_local::avif_encode_available(),
            // No pure-Rust HEVC encoder exists; SVG is vector output, not raster.
            Self::Heic | Self::Svg => false,
        }
    }

    /// Map to `image::ImageFormat` when supported.
    pub fn to_image_format(self) -> Option<image::ImageFormat> {
        match self {
            Self::Jpeg => Some(image::ImageFormat::Jpeg),
            Self::Png => Some(image::ImageFormat::Png),
            Self::Webp => Some(image::ImageFormat::WebP),
            Self::Gif => Some(image::ImageFormat::Gif),
            // Decoded outside the `image` crate (heif-oxide / resvg) or not at all.
            Self::Avif | Self::Heic | Self::Svg => None,
        }
    }
}

/// Probe at least [`IMAGE_MAGIC_PROBE_BYTES`] from the start of `bytes`.
pub fn detect_format(bytes: &[u8]) -> Result<DetectedFormat, CliError> {
    if bytes.len() < 3 {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "image too short for magic probe ({} bytes; need ≥ {IMAGE_MAGIC_PROBE_BYTES})",
                bytes.len()
            ),
            crate::i18n::suggestion_key("image_magic_invalid", None),
        ));
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Ok(DetectedFormat::Jpeg);
    }
    if bytes.len() >= 8 && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Ok(DetectedFormat::Png);
    }
    if bytes.len() >= 6 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        return Ok(DetectedFormat::Gif);
    }
    if bytes.len() >= 12 && bytes[0..4] == *b"RIFF" && bytes[8..12] == *b"WEBP" {
        return Ok(DetectedFormat::Webp);
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"avif" || brand == b"avis" {
            return Ok(DetectedFormat::Avif);
        }
        if brand == b"heic" || brand == b"heix" || brand == b"mif1" || brand == b"heif" {
            return Ok(DetectedFormat::Heic);
        }
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!(
                "unsupported HEIF brand '{}'",
                String::from_utf8_lossy(brand)
            ),
            crate::i18n::suggestion_key("image_format_unsupported", None),
        ));
    }
    if crate::image_local::looks_like_svg(bytes) {
        return Ok(DetectedFormat::Svg);
    }
    let preview: String = bytes
        .iter()
        .take(8)
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    Err(CliError::with_suggestion(
        ErrorKind::Data,
        format!("unrecognized image magic bytes: [{preview}]"),
        crate::i18n::suggestion_key("image_magic_invalid", None),
    ))
}

/// Reject a format that was detected but cannot be decoded in this build.
///
/// The message names the concrete blocker (missing feature vs. absent codec) so
/// an agent can act on it instead of retrying the same call.
pub(crate) fn undecodable(fmt: DetectedFormat) -> CliError {
    let reason = match fmt {
        DetectedFormat::Avif => {
            "no pure-Rust AV1 decoder exists without a C assembler; encode is supported, decode is not"
        }
        DetectedFormat::Heic => "rebuild with the `image-heic` Cargo feature to decode HEIC",
        DetectedFormat::Svg => "rebuild with the `image-svg` Cargo feature to rasterise SVG",
        _ => "format has no decoder in this build",
    };
    CliError::with_suggestion(
        ErrorKind::Data,
        format!("cannot decode {}: {reason}", fmt.as_str()),
        crate::i18n::suggestion_key("image_format_unsupported", None),
    )
}

/// True when path bytes match expected grab/screenshot format name.
pub fn verify_format_name(bytes: &[u8], format: &str) -> bool {
    let Ok(detected) = detect_format(bytes) else {
        return false;
    };
    match format {
        "png" => detected == DetectedFormat::Png,
        "jpeg" | "jpg" => detected == DetectedFormat::Jpeg,
        "webp" => detected == DetectedFormat::Webp,
        "gif" => detected == DetectedFormat::Gif,
        "avif" => detected == DetectedFormat::Avif,
        "heic" => detected == DetectedFormat::Heic,
        "svg" => detected == DetectedFormat::Svg,
        _ => !bytes.is_empty(),
    }
}

/// Map filesystem path I/O errors to agent-native [`CliError`] (suggestion on permission).
///
/// `op` is a short English verb (`open`, `read`, `stat`, `mkdir`, `rename`, …).
/// Suggestion reuses `ffmpeg_io_failed` (path R/W guidance already EN+PT SSOT).
pub(crate) fn io_path_err(path: &std::path::Path, op: &str, e: &std::io::Error) -> CliError {
    let msg = format!("image {op} {}: {e}", path.display());
    let lower = e.to_string().to_ascii_lowercase();
    if e.kind() == std::io::ErrorKind::PermissionDenied || lower.contains("permission") {
        CliError::with_suggestion(
            ErrorKind::Io,
            msg,
            crate::i18n::suggestion_key("ffmpeg_io_failed", None),
        )
    } else {
        CliError::new(ErrorKind::Io, msg)
    }
}

/// Map filesystem open/read errors (`op = "open"`).
pub(crate) fn io_open_err(path: &std::path::Path, e: &std::io::Error) -> CliError {
    io_path_err(path, "open", e)
}
