// SPDX-License-Identifier: MIT OR Apache-2.0
//! SVG rasterisation via `resvg` / `usvg` / `tiny-skia` (GAP-IMG-092), behind
//! the `image-svg` feature.
//!
//! [`super::svg_sanitize`] runs first and unconditionally; this module never
//! sees bytes that failed it.

use image::DynamicImage;

#[cfg(feature = "image-svg")]
use super::svg_sanitize::sanitize;
use super::svg_sanitize::SvgReport;
use crate::error::{CliError, ErrorKind};

/// True when this build can rasterise SVG.
#[must_use]
pub const fn raster_available() -> bool {
    cfg!(feature = "image-svg")
}

/// A rasterised SVG plus what the sanitiser measured on the way in.
pub struct RasterizedSvg {
    /// RGBA raster of the document.
    pub image: DynamicImage,
    /// Sanitiser findings, surfaced in the envelope.
    pub report: SvgReport,
    /// Raster width in pixels.
    pub width: u32,
    /// Raster height in pixels.
    pub height: u32,
}

/// Cheap probe: does this look like an SVG document?
///
/// Magic-byte detection does not apply — SVG is XML, so the "magic" is a token
/// scan over the first bytes. The window is bounded so a large non-SVG file is
/// rejected without a full read.
#[must_use]
pub fn looks_like_svg(bytes: &[u8]) -> bool {
    const WINDOW: usize = 1024;
    let head = &bytes[..bytes.len().min(WINDOW)];
    let Ok(text) = std::str::from_utf8(head) else {
        return false;
    };
    let lower = text.trim_start().to_ascii_lowercase();
    lower.starts_with("<svg") || (lower.starts_with("<?xml") && lower.contains("<svg"))
}

/// Rasterise an SVG source at `scale` (1.0 = the document's intrinsic size).
#[cfg(not(feature = "image-svg"))]
pub fn rasterize(_src: &[u8], _scale: f32) -> Result<RasterizedSvg, CliError> {
    Err(CliError::with_suggestion(
        ErrorKind::Config,
        "svg rasterisation requires the `image-svg` Cargo feature, which is off in this build",
        crate::i18n::suggestion_key("image_feature_disabled", None),
    ))
}

/// Rasterise an SVG source at `scale` (1.0 = the document's intrinsic size).
///
/// The output pixel count is checked against `image_max_pixels` *before* a
/// pixmap is allocated, so a document declaring a 100 000 px viewBox is refused
/// rather than sized into an out-of-memory abort.
#[cfg(feature = "image-svg")]
pub fn rasterize(src: &[u8], scale: f32) -> Result<RasterizedSvg, CliError> {
    use resvg::tiny_skia;
    use resvg::usvg;

    let report = sanitize(src)?;
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };

    // Default options resolve no external files: `usvg::Options` starts with an
    // empty resources dir and this build never populates a font/image loader
    // that could reach the network.
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(src, &options)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("svg parse: {e}")))?;

    let size = tree.size();
    let width = (f64::from(size.width()) * f64::from(scale))
        .round()
        .max(1.0);
    let height = (f64::from(size.height()) * f64::from(scale))
        .round()
        .max(1.0);
    if width > f64::from(u32::MAX) || height > f64::from(u32::MAX) {
        return Err(CliError::with_suggestion(
            ErrorKind::Data,
            format!("svg raster size {width}x{height} overflows u32"),
            crate::i18n::suggestion_key("image_too_large", None),
        ));
    }
    let (width, height) = (width as u32, height as u32);
    crate::image_local::ImageLimits::from_xdg().check_dimensions(width, height)?;

    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Data,
            format!("svg raster allocation failed for {width}x{height}"),
            crate::i18n::suggestion_key("image_too_large", None),
        )
    })?;
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let buffer = image::RgbaImage::from_raw(width, height, pixmap.take()).ok_or_else(|| {
        CliError::new(
            ErrorKind::Data,
            "svg raster buffer does not match the requested dimensions",
        )
    })?;
    Ok(RasterizedSvg {
        image: DynamicImage::ImageRgba8(buffer),
        report,
        width,
        height,
    })
}
