// SPDX-License-Identifier: MIT OR Apache-2.0
//! SIMD resize via `fast_image_resize` (GAP-IMG-095), with a pure `image`
//! fallback when the `image-simd-resize` feature is off.
//!
//! Both paths take the same Lanczos3 kernel, so switching the feature changes
//! throughput and not output geometry. `image::imageops::FilterType::Lanczos3`
//! and `fast_image_resize::FilterType::Lanczos3` are the same windowed-sinc
//! filter; pixel values can differ in the last bit from rounding order, which
//! is why callers compare dimensions and never hashes across the two paths.

use image::DynamicImage;

use crate::error::CliError;

/// Name of the resize backend actually used, for the envelope.
#[must_use]
pub const fn backend() -> &'static str {
    if cfg!(feature = "image-simd-resize") {
        "fast_image_resize"
    } else {
        "image::imageops"
    }
}

/// Resize `src` to exactly `dst_w` x `dst_h`.
#[cfg(feature = "image-simd-resize")]
pub fn resize_exact(src: &DynamicImage, dst_w: u32, dst_h: u32) -> Result<DynamicImage, CliError> {
    use fast_image_resize::images::Image;
    use fast_image_resize::{
        FilterType, IntoImageView, PixelType, ResizeAlg, ResizeOptions, Resizer,
    };

    use crate::error::ErrorKind;

    // `IntoImageView` is implemented for DynamicImage by the `image` feature.
    // An unsupported colour type (e.g. 32-bit float) yields None; converting to
    // RGBA8 first is cheaper to reason about than enumerating every variant.
    let rgba = DynamicImage::ImageRgba8(src.to_rgba8());
    let pixel_type = rgba.pixel_type().ok_or_else(|| {
        CliError::new(
            ErrorKind::Data,
            "resize: unsupported pixel layout after RGBA8 conversion",
        )
    })?;
    debug_assert_eq!(pixel_type, PixelType::U8x4);

    let mut dst = Image::new(dst_w, dst_h, pixel_type);
    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
    resizer
        .resize(&rgba, &mut dst, &options)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("simd resize failed: {e}")))?;

    let buffer = image::RgbaImage::from_raw(dst_w, dst_h, dst.into_vec()).ok_or_else(|| {
        CliError::new(
            ErrorKind::Data,
            "resize: output buffer does not match target dimensions",
        )
    })?;
    Ok(DynamicImage::ImageRgba8(buffer))
}

/// Resize `src` to exactly `dst_w` x `dst_h` using `image::imageops`.
#[cfg(not(feature = "image-simd-resize"))]
pub fn resize_exact(src: &DynamicImage, dst_w: u32, dst_h: u32) -> Result<DynamicImage, CliError> {
    Ok(src.resize_exact(dst_w, dst_h, image::imageops::FilterType::Lanczos3))
}
