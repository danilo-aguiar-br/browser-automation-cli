// SPDX-License-Identifier: MIT OR Apache-2.0
//! AVIF encode via `ravif` (GAP-IMG-090), behind the `image-avif` feature.
//!
//! # Why encode only
//!
//! AVIF decode has no pure-Rust path as of 2026. Both `rav1d` 1.1 and
//! `rav1d-safe` 0.5.7 declare `cc` **and** `nasm-rs` as unconditional
//! build-dependencies — no feature combination removes them, because they are
//! plain build-deps rather than optional ones. The remaining decoders
//! (`libavif`, `dav1d-sys`) are FFI. Under this crate's no-C-toolchain rule the
//! honest outcome is encode-only, so [`super::magic`] keeps rejecting AVIF
//! input and this module refuses to pretend otherwise.

use std::path::Path;

use image::DynamicImage;

use super::atomic::write_bytes_atomic;
use crate::error::{CliError, ErrorKind};

/// Error surfaced when AVIF is requested from a build without `image-avif`.
#[cfg(not(feature = "image-avif"))]
pub fn encode_to_vec(_img: &DynamicImage, _quality: u8) -> Result<Vec<u8>, CliError> {
    Err(CliError::with_suggestion(
        ErrorKind::Config,
        "avif encode requires the `image-avif` Cargo feature, which is off in this build",
        crate::i18n::suggestion_key("image_feature_disabled", None),
    ))
}

/// Encode `img` to an AVIF byte buffer.
///
/// `quality` is the 1..=100 scale shared with JPEG/WebP; `ravif` takes the same
/// range as `f32`. Encoder speed comes from XDG `image_avif_speed`.
/// Images with an alpha channel take the RGBA path so transparency survives.
#[cfg(feature = "image-avif")]
pub fn encode_to_vec(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, CliError> {
    use rgb::FromSlice;

    let quality = f32::from(quality.clamp(1, 100));
    let speed = crate::xdg::resolve_image_avif_speed();
    let encoder = ravif::Encoder::new()
        .with_quality(quality)
        .with_speed(speed)
        // rav1e is compiled without its NASM kernels here, so tile threading is
        // the only lever that keeps large frames off the multi-second path.
        .with_num_threads(Some(crate::concurrency::rayon_threads()));

    let map_err = |e: ravif::Error| CliError::new(ErrorKind::Data, format!("avif encode: {e}"));

    let encoded = if img.color().has_alpha() {
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width() as usize, rgba.height() as usize);
        let buf = imgref::Img::new(rgba.as_raw().as_rgba(), w, h);
        encoder.encode_rgba(buf).map_err(map_err)?
    } else {
        let rgb = img.to_rgb8();
        let (w, h) = (rgb.width() as usize, rgb.height() as usize);
        let buf = imgref::Img::new(rgb.as_raw().as_rgb(), w, h);
        encoder.encode_rgb(buf).map_err(map_err)?
    };
    Ok(encoded.avif_file)
}

/// Encode and atomically write AVIF bytes to `path`.
pub fn encode_to_path(img: &DynamicImage, path: &Path, quality: u8) -> Result<usize, CliError> {
    let bytes = encode_to_vec(img, quality)?;
    write_bytes_atomic(path, &bytes)?;
    Ok(bytes.len())
}

/// True when this build can emit AVIF.
#[must_use]
pub const fn encode_available() -> bool {
    cfg!(feature = "image-avif")
}

/// AVIF decode is unavailable in every configuration of this build.
///
/// Kept as an explicit constant so envelopes can report the asymmetry instead of
/// letting an agent infer that encode support implies decode support.
#[must_use]
pub const fn decode_available() -> bool {
    false
}
