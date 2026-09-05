// SPDX-License-Identifier: MIT OR Apache-2.0
//! Container × codec compatibility: what a container is allowed to mux.
//!
//! # Why this is separate from [`super::encoder_policy`]
//!
//! This module answers one question and it has no opinion: *can codec X sit
//! inside container Y?* The answer is dictated by the container specs, so it is
//! the same for every command, every flag and every input — a fact table.
//!
//! Choosing WHAT to encode when the answer is "no" is a different question with
//! a different owner: it depends on the caller's flags, on whether ffprobe ran,
//! and on which default the product prefers. That is a policy, it changes when
//! the product changes its mind, and it lives in [`super::encoder_policy`].
//!
//! Keeping them in one file made the policy read like part of the spec, and the
//! spec read like something a flag could bend. Neither is true.

use crate::error::{CliError, ErrorKind};

/// Output container selected by the agent (`--format`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputContainer {
    /// MPEG-4 Part 14.
    Mp4,
    /// WebM (Matroska subset).
    Webm,
    /// Matroska.
    Mkv,
    /// QuickTime MOV.
    Mov,
    /// AVI RIFF.
    Avi,
    /// Apple M4V.
    M4v,
}

impl OutputContainer {
    /// Wire extension / format name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Webm => "webm",
            Self::Mkv => "mkv",
            Self::Mov => "mov",
            Self::Avi => "avi",
            Self::M4v => "m4v",
        }
    }

    /// True when `-movflags +faststart` is meaningful.
    #[must_use]
    pub fn supports_faststart(self) -> bool {
        matches!(self, Self::Mp4 | Self::M4v | Self::Mov)
    }
}

/// Parse `--format` for video convert.
pub fn parse_output_container(raw: &str) -> Result<OutputContainer, CliError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "mp4" => Ok(OutputContainer::Mp4),
        "webm" => Ok(OutputContainer::Webm),
        "mkv" | "matroska" => Ok(OutputContainer::Mkv),
        "mov" | "qt" => Ok(OutputContainer::Mov),
        "avi" => Ok(OutputContainer::Avi),
        "m4v" => Ok(OutputContainer::M4v),
        other => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unsupported video output format '{other}'"),
            crate::i18n::suggestion_key("video_format_unsupported", None),
        )),
    }
}

/// Normalize codec name from ffprobe / agent flags.
#[must_use]
pub fn normalize_codec(name: &str) -> String {
    let n = name.trim().to_ascii_lowercase();
    match n.as_str() {
        "h264" | "avc" | "avc1" | "libx264" => "h264".into(),
        "hevc" | "h265" | "hev1" | "hvc1" | "libx265" => "hevc".into(),
        "vp8" | "libvpx" => "vp8".into(),
        "vp9" | "libvpx-vp9" => "vp9".into(),
        "av1" | "libaom-av1" | "libsvtav1" | "rav1e" => "av1".into(),
        "aac" | "mp4a" => "aac".into(),
        "opus" | "libopus" => "opus".into(),
        "vorbis" | "libvorbis" => "vorbis".into(),
        "mp3" | "libmp3lame" => "mp3".into(),
        "ac3" | "eac3" => "ac3".into(),
        "copy" => "copy".into(),
        other => other.to_string(),
    }
}

/// Whether a *real* (non-copy) video codec can mux into `out`.
#[must_use]
pub fn video_codec_muxable(out: OutputContainer, codec: &str) -> bool {
    let v = normalize_codec(codec);
    match out {
        OutputContainer::Webm => matches!(v.as_str(), "vp8" | "vp9" | "av1"),
        OutputContainer::Mp4 | OutputContainer::M4v | OutputContainer::Mov => {
            matches!(v.as_str(), "h264" | "hevc" | "av1" | "vp9" | "mpeg4")
        }
        OutputContainer::Mkv | OutputContainer::Avi => true,
    }
}

/// Whether a *real* (non-copy) audio codec can mux into `out`.
#[must_use]
pub fn audio_codec_muxable(out: OutputContainer, codec: &str) -> bool {
    let a = normalize_codec(codec);
    match out {
        OutputContainer::Webm => matches!(a.as_str(), "vorbis" | "opus"),
        OutputContainer::Mp4 | OutputContainer::M4v | OutputContainer::Mov => {
            matches!(a.as_str(), "aac" | "mp3" | "ac3" | "opus" | "alac")
        }
        OutputContainer::Mkv | OutputContainer::Avi => true,
    }
}

/// Validate that chosen video/audio codecs are acceptable for `out`.
pub fn validate_codec_for_container(
    out: OutputContainer,
    video_codec: &str,
    audio_codec: Option<&str>,
) -> Result<(), CliError> {
    let v = normalize_codec(video_codec);
    if v != "copy" && !video_codec_muxable(out, &v) {
        return Err(incompat(
            out,
            &v,
            match out {
                OutputContainer::Webm => {
                    "WebM accepts only VP8, VP9, or AV1 video (not H.264/H.265)"
                }
                OutputContainer::Mp4 | OutputContainer::M4v | OutputContainer::Mov => {
                    "MP4/MOV/M4V prefer H.264, HEVC, AV1, or VP9"
                }
                _ => "codec not accepted for this container",
            },
        ));
    }
    if let Some(a) = audio_codec {
        let a = normalize_codec(a);
        if a != "copy" && !audio_codec_muxable(out, &a) {
            return Err(incompat(
                out,
                &a,
                match out {
                    OutputContainer::Webm => "WebM accepts only Vorbis or Opus audio",
                    OutputContainer::Mp4 | OutputContainer::M4v | OutputContainer::Mov => {
                        "MP4/MOV audio typically AAC/MP3/AC-3/Opus"
                    }
                    _ => "audio codec not accepted for this container",
                },
            ));
        }
    }
    Ok(())
}

/// The single incompatibility error, shared with the encoder policy.
///
/// The policy re-checks its own resolved arguments before handing them to
/// ffmpeg, and a second wording of the same refusal would let two paths disagree
/// about what the product considers impossible.
pub(super) fn incompat(out: OutputContainer, codec: &str, detail: &str) -> CliError {
    CliError::with_suggestion(
        ErrorKind::Usage,
        format!(
            "codec '{codec}' is incompatible with container '{}': {detail}",
            out.as_str()
        ),
        crate::i18n::suggestion_key("video_codec_container_mismatch", None),
    )
}
