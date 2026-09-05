// SPDX-License-Identifier: MIT OR Apache-2.0
//! Magic-first image decode with resource limits.

use std::io::Cursor;
use std::path::Path;

use image::{DynamicImage, ImageReader};

use super::limits::ImageLimits;
use super::magic::{detect_format, DetectedFormat};
use crate::error::{CliError, ErrorKind};

/// Decoded image plus detected format and raw byte length.
pub struct DecodedImage {
    /// Pixel buffer.
    pub image: DynamicImage,
    /// Magic-detected format.
    pub format: DetectedFormat,
    /// Input byte length.
    pub bytes: usize,
}

/// Load and decode from a filesystem path (magic-first; extension ignored).
pub fn decode_path(path: &Path, limits: ImageLimits) -> Result<DecodedImage, CliError> {
    // GAP-026: bound the operator-supplied path before touching it.
    crate::fs_roots::ensure_read_allowed(path)?;
    let meta =
        std::fs::metadata(path).map_err(|e| crate::image_local::magic::io_open_err(path, &e))?;
    let len = meta.len() as usize;
    limits.check_input_len(len)?;
    let bytes = std::fs::read(path)
        .map_err(|e| crate::image_local::magic::io_path_err(path, "read", &e))?;
    decode_bytes(&bytes, limits)
}

/// Decode from an in-memory buffer (magic-first).
pub fn decode_bytes(bytes: &[u8], limits: ImageLimits) -> Result<DecodedImage, CliError> {
    limits.check_input_len(bytes.len())?;
    let format = detect_format(bytes)?;
    if !format.is_supported() {
        return Err(super::magic::undecodable(format));
    }
    // HEIC and SVG decode outside the `image` crate, so they short-circuit here
    // before the ImageReader path.
    match format {
        DetectedFormat::Heic => {
            let image = super::heic::decode_bytes(bytes)?;
            let (w, h) = image.dimensions();
            limits.check_dimensions(w, h)?;
            return Ok(DecodedImage {
                image,
                format,
                bytes: bytes.len(),
            });
        }
        DetectedFormat::Svg => {
            let raster = super::svg::rasterize(bytes, 1.0)?;
            return Ok(DecodedImage {
                image: raster.image,
                format,
                bytes: bytes.len(),
            });
        }
        _ => {}
    }
    let image_format = format.to_image_format().ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Data,
            format!("format {} has no decoder", format.as_str()),
            crate::i18n::suggestion_key("image_format_unsupported", None),
        )
    })?;
    let mut reader = ImageReader::new(Cursor::new(bytes));
    reader.set_format(image_format);
    reader.limits(limits.to_image_limits());
    let image = reader.decode().map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Data,
            format!("image decode failed: {e}"),
            crate::i18n::suggestion_key("image_magic_invalid", None),
        )
    })?;
    let (w, h) = image.dimensions();
    limits.check_dimensions(w, h)?;
    Ok(DecodedImage {
        image,
        format,
        bytes: bytes.len(),
    })
}

use image::GenericImageView;
