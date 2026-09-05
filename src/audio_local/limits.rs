// SPDX-License-Identifier: MIT OR Apache-2.0
//! XDG-backed ceilings for local audio ops.
//!
//! # Why this is not shared with `video_local::limits`
//!
//! The two modules look like copies and are deliberately kept apart. Audited on
//! 2026-08-31: folding them into one type parameterised by a domain name would
//! remove about a dozen lines and cost more than it saves.
//!
//! Nothing here is actually common. The two XDG keys differ, the error text
//! names the key the operator has to change, and the i18n suggestion key differs
//! — so the "shared" function would take the domain word, both resolvers and the
//! suggestion key as arguments, which is more machinery at every call site than
//! the duplication it replaces.
//!
//! The separate types also carry a guarantee: [`AudioLimits`] and
//! `VideoLimits` cannot be substituted for one another, so a video ceiling can
//! never be applied to audio input by a misplaced argument. A single
//! `MediaLimits` would make that a runtime concern.
//!
//! Revisit only if a THIRD domain arrives, or if the two stop diverging in the
//! keys they resolve.

use crate::error::{CliError, ErrorKind};

/// Input / download limits resolved from XDG (or compile-time defaults).
#[derive(Debug, Clone, Copy)]
pub struct AudioLimits {
    /// Max bytes for stdin materialization and path pre-check.
    pub max_input_bytes: usize,
    /// Max HTTP body for download.
    pub max_download_bytes: usize,
}

impl AudioLimits {
    /// Load from XDG product config (flags + XDG only; never product env).
    #[must_use]
    pub fn from_xdg() -> Self {
        Self {
            max_input_bytes: crate::xdg::resolve_audio_max_input_bytes(),
            max_download_bytes: crate::xdg::resolve_audio_download_max_bytes(),
        }
    }

    /// Fail closed when `len` exceeds the input ceiling.
    pub fn check_input_len(self, len: u64) -> Result<(), CliError> {
        let max = self.max_input_bytes as u64;
        if len > max {
            return Err(CliError::with_suggestion(
                ErrorKind::Data,
                format!("audio input {len} bytes exceeds audio_max_input_bytes {max}"),
                crate::i18n::suggestion_key("audio_too_large", None),
            ));
        }
        Ok(())
    }
}
