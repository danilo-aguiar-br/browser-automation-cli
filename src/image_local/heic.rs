// SPDX-License-Identifier: MIT OR Apache-2.0
//! HEIC decode via `heif-oxide` (GAP-IMG-091), behind the `image-heic` feature.
//!
//! # Decode only, permanently
//!
//! HEIC wraps HEVC. No pure-Rust HEVC **encoder** exists, and one is not a
//! weekend project: it is a patent-encumbered, block-partitioned codec whose
//! reference encoder is hundreds of thousands of lines of C. That is a physical
//! limit of the ecosystem, not a scope decision, so `image convert --format heic`
//! stays rejected with an honest message rather than silently emitting something
//! else.

use image::DynamicImage;

use crate::error::{CliError, ErrorKind};

/// True when this build can decode HEIC.
#[must_use]
pub const fn decode_available() -> bool {
    cfg!(feature = "image-heic")
}

/// HEIC encode is unavailable in every configuration of this build.
#[must_use]
pub const fn encode_available() -> bool {
    false
}

#[cfg(not(feature = "image-heic"))]
fn feature_off() -> CliError {
    CliError::with_suggestion(
        ErrorKind::Config,
        "heic decode requires the `image-heic` Cargo feature, which is off in this build",
        crate::i18n::suggestion_key("image_feature_disabled", None),
    )
}

/// Decode HEIC bytes into a `DynamicImage`.
#[cfg(not(feature = "image-heic"))]
pub fn decode_bytes(_bytes: &[u8]) -> Result<DynamicImage, CliError> {
    Err(feature_off())
}

/// Decode HEIC bytes into a `DynamicImage`, applying the stored orientation.
///
/// 10- and 12-bit sources are down-shifted to 8 bits per channel: the rest of
/// the pipeline (encode, resize, hashing) is 8-bit, and silently keeping 16-bit
/// precision here would be lost at the first re-encode anyway.
#[cfg(feature = "image-heic")]
pub fn decode_bytes(bytes: &[u8]) -> Result<DynamicImage, CliError> {
    use heif_oxide::Pixels;

    let decoded = heif_oxide::decode_bytes(bytes)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("heic decode: {e}")))?;
    let (w, h) = (decoded.width, decoded.height);
    crate::image_local::ImageLimits::from_xdg().check_dimensions(w, h)?;

    let bad_buffer = || {
        CliError::new(
            ErrorKind::Data,
            format!("heic decode produced a buffer that does not match {w}x{h}"),
        )
    };
    // heif-oxide scales 10/12-bit samples to the full u16 range, so `>> 8` is a
    // plain range narrowing rather than a lossy guess.
    let narrow = |v: Vec<u16>| -> Vec<u8> { v.into_iter().map(|s| (s >> 8) as u8).collect() };

    Ok(match decoded.pixels {
        Pixels::Rgb8(buf) => {
            DynamicImage::ImageRgb8(image::RgbImage::from_raw(w, h, buf).ok_or_else(bad_buffer)?)
        }
        Pixels::Rgba8(buf) => {
            DynamicImage::ImageRgba8(image::RgbaImage::from_raw(w, h, buf).ok_or_else(bad_buffer)?)
        }
        Pixels::Rgb16(buf) => DynamicImage::ImageRgb8(
            image::RgbImage::from_raw(w, h, narrow(buf)).ok_or_else(bad_buffer)?,
        ),
        Pixels::Rgba16(buf) => DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(w, h, narrow(buf)).ok_or_else(bad_buffer)?,
        ),
    })
}

/// Reject HEIC encode with a message that names the real blocker.
pub fn encode_unsupported() -> CliError {
    CliError::with_suggestion(
        ErrorKind::Usage,
        "heic encode is not available: no pure-Rust HEVC encoder exists (decode only)",
        crate::i18n::suggestion_key("image_heic_encode_unavailable", None),
    )
}
