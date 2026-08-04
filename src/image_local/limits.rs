// SPDX-License-Identifier: MIT OR Apache-2.0
//! Image decode / download ceilings from XDG resolve helpers.

/// Resolved ceilings for one image operation.
#[derive(Debug, Clone, Copy)]
pub struct ImageLimits {
    /// Max input bytes (file or HTTP body).
    pub max_input_bytes: usize,
    /// Max `width * height` after decode headers.
    pub max_pixels: u64,
    /// Default lossy quality 1..=100.
    pub default_quality: u8,
}

impl ImageLimits {
    /// Load from XDG (`config set image_*`) with named constant defaults.
    #[must_use]
    pub fn from_xdg() -> Self {
        Self {
            max_input_bytes: crate::xdg::resolve_image_max_input_bytes(),
            max_pixels: crate::xdg::resolve_image_max_pixels(),
            default_quality: crate::xdg::resolve_image_default_quality(),
        }
    }

    /// Build `image::Limits` for the decoder.
    pub fn to_image_limits(self) -> image::Limits {
        let mut lim = image::Limits::default();
        lim.max_image_width = Some(
            (self.max_pixels.min(u64::from(u32::MAX)))
                .try_into()
                .unwrap_or(u32::MAX),
        );
        lim.max_image_height = lim.max_image_width;
        lim.max_alloc = Some(
            self.max_pixels
                .saturating_mul(4)
                .max(self.max_input_bytes as u64),
        );
        lim
    }

    /// Reject when `width * height` exceeds the pixel ceiling.
    pub fn check_dimensions(self, width: u32, height: u32) -> Result<(), crate::error::CliError> {
        let pixels = match u64::from(width).checked_mul(u64::from(height)) {
            Some(p) => p,
            None => {
                return Err(crate::error::CliError::with_suggestion(
                    crate::error::ErrorKind::Data,
                    format!("image dimensions overflow: {width}x{height}"),
                    crate::i18n::suggestion_key("image_too_large", None),
                ));
            }
        };
        if pixels > self.max_pixels {
            return Err(crate::error::CliError::with_suggestion(
                crate::error::ErrorKind::Data,
                format!(
                    "image pixels {pixels} exceed image_max_pixels {}",
                    self.max_pixels
                ),
                crate::i18n::suggestion_key("image_too_large", None),
            ));
        }
        Ok(())
    }

    /// Reject oversized byte buffers before decode.
    pub fn check_input_len(self, len: usize) -> Result<(), crate::error::CliError> {
        if len > self.max_input_bytes {
            return Err(crate::error::CliError::with_suggestion(
                crate::error::ErrorKind::Data,
                format!(
                    "image input {len} bytes exceeds image_max_input_bytes {}",
                    self.max_input_bytes
                ),
                crate::i18n::suggestion_key("image_too_large", None),
            ));
        }
        Ok(())
    }
}
