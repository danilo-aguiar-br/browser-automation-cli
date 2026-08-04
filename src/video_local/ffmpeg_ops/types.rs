// SPDX-License-Identifier: MIT OR Apache-2.0
//! Convert options and result types for path→path ffmpeg ops.

use std::path::PathBuf;

use super::super::validate::{normalize_codec, OutputContainer};

/// Options for container convert / remux.
#[derive(Debug, Clone)]
pub struct ConvertOpts {
    /// Target container.
    pub format: OutputContainer,
    /// Normalized video codec name (`copy`, `h264`, …).
    pub video_codec: String,
    /// Normalized audio codec name (`copy`, `aac`, …).
    pub audio_codec: String,
    /// CRF for re-encode paths.
    pub crf: u8,
    /// Opt out of MP4-family faststart (default applies faststart).
    pub no_faststart: bool,
    /// Strip container metadata when true.
    pub strip_metadata: bool,
    /// Drop all audio tracks when true.
    pub drop_audio: bool,
}

impl ConvertOpts {
    /// Build options from CLI flags (defaults: stream copy intent, XDG CRF, faststart on).
    #[allow(clippy::too_many_arguments)]
    pub fn from_flags(
        format: OutputContainer,
        video_codec: Option<&str>,
        audio_codec: Option<&str>,
        crf: Option<u8>,
        no_faststart: bool,
        strip_metadata: bool,
        drop_audio: bool,
    ) -> Self {
        let v = video_codec
            .map(normalize_codec)
            .unwrap_or_else(|| "copy".into());
        let a = audio_codec
            .map(normalize_codec)
            .unwrap_or_else(|| "copy".into());
        Self {
            format,
            video_codec: v,
            audio_codec: a,
            crf: crf.unwrap_or_else(crate::xdg::resolve_video_default_crf),
            no_faststart,
            strip_metadata,
            drop_audio,
        }
    }
}

/// Result of a path→path ffmpeg op (agent envelope fields).
#[derive(Debug, Clone)]
pub struct ConvertResult {
    /// Output media path.
    pub path_out: PathBuf,
    /// Output size in bytes.
    pub bytes_out: u64,
    /// SHA-256 of the output file (streamed hash).
    pub sha256_out: String,
    /// True when both A/V used stream copy.
    pub stream_copy: bool,
    /// Effective video codec / encoder name.
    pub video_codec: String,
    /// Effective audio codec / encoder name.
    pub audio_codec: String,
    /// Whether metadata was stripped.
    pub metadata_stripped: bool,
    /// True when copy was upgraded to re-encode automatically.
    pub auto_reencoded: bool,
    /// Machine reason for auto re-encode.
    pub reencode_reason: Option<String>,
    /// Whether +faststart was applied.
    pub faststart_applied: bool,
    /// Duration seconds when known.
    pub duration_secs: Option<f64>,
}
