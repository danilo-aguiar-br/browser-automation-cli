// SPDX-License-Identifier: MIT OR Apache-2.0
//! XDG-backed ceilings for local video ops.

use crate::error::{CliError, ErrorKind};

/// Input / download limits resolved from XDG (or compile-time defaults).
#[derive(Debug, Clone, Copy)]
pub struct VideoLimits {
    /// Max bytes for stdin materialization and path pre-check.
    pub max_input_bytes: usize,
    /// Max HTTP body for download.
    pub max_download_bytes: usize,
}

impl VideoLimits {
    /// Load from XDG product config (flags + XDG only; never product env).
    #[must_use]
    pub fn from_xdg() -> Self {
        Self {
            max_input_bytes: crate::xdg::resolve_video_max_input_bytes(),
            max_download_bytes: crate::xdg::resolve_video_download_max_bytes(),
        }
    }

    /// Fail closed when `len` exceeds the input ceiling.
    pub fn check_input_len(self, len: u64) -> Result<(), CliError> {
        let max = self.max_input_bytes as u64;
        if len > max {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("video input {len} bytes exceeds video_max_input_bytes {max}"),
                crate::i18n::suggestion_key("video_too_large", None),
            ));
        }
        Ok(())
    }
}
