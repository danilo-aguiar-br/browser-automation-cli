// SPDX-License-Identifier: MIT OR Apache-2.0
//! Resolve optional ffmpeg / ffprobe binaries (XDG then PATH).

use std::path::PathBuf;

use crate::error::{CliError, ErrorKind};
use crate::platform::{is_spawn_safe_binary, which_bin};

/// Absolute path to a spawn-safe `ffmpeg` binary, if available.
#[must_use]
pub fn resolve_ffmpeg_bin() -> Option<PathBuf> {
    crate::xdg::ffmpeg_path_from_config()
        .map(PathBuf::from)
        .filter(|p| is_spawn_safe_binary(p))
        .or_else(|| which_bin("ffmpeg").filter(|p| is_spawn_safe_binary(p)))
}

/// Absolute path to a spawn-safe `ffprobe` binary, if available.
///
/// Prefers a sibling of the resolved ffmpeg path (`…/ffprobe`), then PATH.
#[must_use]
pub fn resolve_ffprobe_bin() -> Option<PathBuf> {
    if let Some(ff) = resolve_ffmpeg_bin() {
        if let Some(parent) = ff.parent() {
            let sibling = parent.join(if cfg!(windows) {
                "ffprobe.exe"
            } else {
                "ffprobe"
            });
            if is_spawn_safe_binary(&sibling) {
                return Some(sibling);
            }
        }
    }
    which_bin("ffprobe").filter(|p| is_spawn_safe_binary(p))
}

/// Require ffmpeg or return agent-friendly Unavailable.
pub fn require_ffmpeg() -> Result<PathBuf, CliError> {
    resolve_ffmpeg_bin().ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Unavailable,
            "ffmpeg not found (set XDG ffmpeg_path or install on PATH)",
            crate::i18n::suggestion_key("ffmpeg_missing", None),
        )
    })
}

/// Require ffprobe or return agent-friendly Unavailable.
pub fn require_ffprobe() -> Result<PathBuf, CliError> {
    resolve_ffprobe_bin().ok_or_else(|| {
        CliError::with_suggestion(
            ErrorKind::Unavailable,
            "ffprobe not found (install ffmpeg suite or set ffmpeg_path next to ffprobe)",
            crate::i18n::suggestion_key("ffmpeg_missing", None),
        )
    })
}
