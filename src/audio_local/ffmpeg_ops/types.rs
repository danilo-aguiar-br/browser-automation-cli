// SPDX-License-Identifier: MIT OR Apache-2.0
//! Convert options and result types for path→path audio ffmpeg ops.

use std::path::PathBuf;

use super::super::validate::OutputFormat;

/// Options for audio convert / remux.
#[derive(Debug, Clone)]
pub struct ConvertOpts {
    /// Target format.
    pub format: OutputFormat,
    /// Explicit codec intent (`copy`, `mp3`, `aac`, …) or None for smart plan.
    pub codec: Option<String>,
    /// Bitrate for lossy encode (e.g. `192k`).
    pub bitrate: String,
    /// Optional sample rate Hz.
    pub sample_rate: Option<u32>,
    /// Optional channel count.
    pub channels: Option<u32>,
    /// Audio stream index (default 0).
    pub audio_stream: Option<u32>,
    /// Strip container metadata when true.
    pub strip_metadata: bool,
}

/// Result of a path→path audio ffmpeg op.
#[derive(Debug, Clone)]
pub struct ConvertResult {
    /// Output media path.
    pub path_out: PathBuf,
    /// Output size in bytes.
    pub bytes_out: u64,
    /// SHA-256 of the output file.
    pub sha256_out: String,
    /// True when stream copy used.
    pub stream_copy: bool,
    /// Effective audio codec / encoder name.
    pub audio_codec: String,
    /// Whether metadata was stripped.
    pub metadata_stripped: bool,
    /// True when copy was upgraded to re-encode automatically.
    pub auto_reencoded: bool,
    /// Machine reason for auto re-encode.
    pub reencode_reason: Option<String>,
    /// Lossy→lossy recompress flag.
    pub lossy_transcode: bool,
    /// Whether +faststart was applied.
    pub faststart_applied: bool,
    /// Duration seconds when known.
    pub duration_secs: Option<f64>,
}
