// SPDX-License-Identifier: MIT OR Apache-2.0
//! Encode DynamicImage to supported formats with atomic disk write.

use std::io::Cursor;
use std::path::Path;

use image::{DynamicImage, ImageFormat};

use super::atomic::write_bytes_atomic;
use crate::error::{CliError, ErrorKind};

/// An output format this build can encode.
///
/// AVIF is deliberately *not* an `image::ImageFormat`: the `image` crate routes
/// AVIF encode through its own optional backend, while this crate encodes with
/// `ravif` directly so the C-free feature wiring stays under our control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// A format handled by the `image` crate.
    Image(ImageFormat),
    /// AVIF via `ravif` (requires the `image-avif` feature).
    Avif,
}

impl OutputFormat {
    /// Stable lowercase wire name.
    #[must_use]
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Image(f) => format_wire_name(f),
            Self::Avif => "avif",
        }
    }

    /// True when the `quality` knob actually reaches the encoder.
    ///
    /// The local WebP encoder is lossless-only and PNG/GIF are lossless by
    /// construction, so reporting `quality_applied` honestly matters.
    #[must_use]
    pub fn quality_applies(self) -> bool {
        matches!(self, Self::Image(ImageFormat::Jpeg) | Self::Avif)
    }
}

/// Parse a CLI format token into an [`OutputFormat`].
pub fn parse_output_format(name: &str) -> Result<OutputFormat, CliError> {
    match name.trim().to_ascii_lowercase().as_str() {
        "png" => Ok(OutputFormat::Image(ImageFormat::Png)),
        "jpeg" | "jpg" => Ok(OutputFormat::Image(ImageFormat::Jpeg)),
        "webp" => Ok(OutputFormat::Image(ImageFormat::WebP)),
        "gif" => Ok(OutputFormat::Image(ImageFormat::Gif)),
        "avif" => {
            if crate::image_local::avif_encode_available() {
                Ok(OutputFormat::Avif)
            } else {
                Err(CliError::with_suggestion(
                    ErrorKind::Config,
                    "avif encode requires the `image-avif` Cargo feature, which is off in this build",
                    crate::i18n::suggestion_key("image_feature_disabled", None),
                ))
            }
        }
        "heic" | "heif" => Err(super::heic::encode_unsupported()),
        other => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unsupported output image format: {other}"),
            crate::i18n::suggestion_key("use_listed_value", None),
        )),
    }
}

/// Wire name for an `ImageFormat`.
#[must_use]
pub fn format_wire_name(fmt: ImageFormat) -> &'static str {
    match fmt {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpeg",
        ImageFormat::WebP => "webp",
        ImageFormat::Gif => "gif",
        _ => "unknown",
    }
}

/// Encode image into a memory buffer.
pub fn encode_to_vec(
    img: &DynamicImage,
    format: OutputFormat,
    quality: u8,
) -> Result<Vec<u8>, CliError> {
    let quality = quality.clamp(1, 100);
    let format = match format {
        OutputFormat::Avif => return super::avif::encode_to_vec(img, quality),
        OutputFormat::Image(f) => f,
    };
    let mut buf = Cursor::new(Vec::new());
    match format {
        ImageFormat::Jpeg => {
            let rgb = img.to_rgb8();
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
            enc.encode(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| CliError::new(ErrorKind::Data, format!("jpeg encode: {e}")))?;
        }
        ImageFormat::Png => {
            img.write_to(&mut buf, ImageFormat::Png)
                .map_err(|e| CliError::new(ErrorKind::Data, format!("png encode: {e}")))?;
        }
        ImageFormat::Gif => {
            img.write_to(&mut buf, ImageFormat::Gif)
                .map_err(|e| CliError::new(ErrorKind::Data, format!("gif encode: {e}")))?;
        }
        ImageFormat::WebP => {
            // docs.rs image 0.25: WebPEncoder is lossless-only (no quality knob without libwebp).
            img.write_to(&mut buf, ImageFormat::WebP)
                .map_err(|e| CliError::new(ErrorKind::Data, format!("webp encode: {e}")))?;
            let _ = quality; // intentionally unused: quality_applied=false in convert/resize envelopes
        }
        other => {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                format!("encode format not implemented: {other:?}"),
                crate::i18n::suggestion_key("use_listed_value", None),
            ));
        }
    }
    Ok(buf.into_inner())
}

/// Encode and atomically write to `path`.
pub fn encode_to_path(
    img: &DynamicImage,
    path: &Path,
    format: OutputFormat,
    quality: u8,
) -> Result<usize, CliError> {
    let bytes = encode_to_vec(img, format, quality)?;
    write_bytes_atomic(path, &bytes)?;
    Ok(bytes.len())
}
