// SPDX-License-Identifier: MIT OR Apache-2.0
//! Container × codec validation and smart stream-copy / re-encode planning.

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

fn incompat(out: OutputContainer, codec: &str, detail: &str) -> CliError {
    CliError::with_suggestion(
        ErrorKind::Usage,
        format!(
            "codec '{codec}' is incompatible with container '{}': {detail}",
            out.as_str()
        ),
        crate::i18n::suggestion_key("video_codec_container_mismatch", None),
    )
}

/// Default encoder *wire* names (normalized) for re-encode paths.
#[must_use]
pub fn default_video_wire(out: OutputContainer) -> &'static str {
    match out {
        OutputContainer::Webm => "vp9",
        _ => "h264",
    }
}

/// Default audio wire name for re-encode paths.
#[must_use]
pub fn default_audio_wire(out: OutputContainer) -> &'static str {
    match out {
        OutputContainer::Webm => "opus",
        _ => "aac",
    }
}

/// Default encoder names for ffmpeg when re-encoding.
#[must_use]
pub fn default_video_encoder(out: OutputContainer) -> &'static str {
    match out {
        OutputContainer::Webm => crate::constants::SCREENCAST_FFMPEG_VCODEC_WEBM,
        OutputContainer::Mp4
        | OutputContainer::M4v
        | OutputContainer::Mov
        | OutputContainer::Avi => crate::constants::SCREENCAST_FFMPEG_VCODEC_MP4,
        OutputContainer::Mkv => crate::constants::SCREENCAST_FFMPEG_VCODEC_MP4,
    }
}

/// Default audio encoder for re-encode paths.
#[must_use]
pub fn default_audio_encoder(out: OutputContainer) -> &'static str {
    match out {
        OutputContainer::Webm => "libopus",
        _ => "aac",
    }
}

/// Map wire codec name to ffmpeg encoder argument.
#[must_use]
pub fn video_encoder_arg(wire: &str) -> String {
    match normalize_codec(wire).as_str() {
        "copy" => "copy".into(),
        "h264" => "libx264".into(),
        "hevc" => "libx265".into(),
        "vp8" => "libvpx".into(),
        "vp9" => "libvpx-vp9".into(),
        "av1" => "libaom-av1".into(),
        other => other.to_string(),
    }
}

/// Map wire audio codec to ffmpeg encoder argument.
#[must_use]
pub fn audio_encoder_arg(wire: &str) -> String {
    match normalize_codec(wire).as_str() {
        "copy" => "copy".into(),
        "aac" => "aac".into(),
        "opus" => "libopus".into(),
        "vorbis" => "libvorbis".into(),
        "mp3" => "libmp3lame".into(),
        "ac3" => "ac3".into(),
        other => other.to_string(),
    }
}

/// Effective codec plan after smart copy / re-encode resolution.
#[derive(Debug, Clone)]
pub struct CodecPlan {
    /// ffmpeg `-c:v` argument.
    pub video_ffmpeg: String,
    /// ffmpeg `-c:a` argument (`none` when drop_audio).
    pub audio_ffmpeg: String,
    /// True when both tracks are stream-copy (or video-only copy with drop_audio).
    pub stream_copy: bool,
    /// True when at least one track was upgraded from copy to re-encode.
    pub auto_reencoded: bool,
    /// Machine reason when `auto_reencoded` (null otherwise).
    pub reencode_reason: Option<&'static str>,
}

/// Resolve stream-copy vs re-encode using optional input stream codecs from ffprobe.
///
/// Agent-first: prefer copy when muxable; otherwise re-encode with container defaults
/// so `video convert --format webm` works on H.264 sources without manual flags.
pub fn resolve_effective_codecs(
    out: OutputContainer,
    want_video: &str,
    want_audio: &str,
    drop_audio: bool,
    input_video: Option<&str>,
    input_audio: Option<&str>,
    probe_ok: bool,
) -> Result<CodecPlan, CliError> {
    let mut auto = false;
    let mut reason: Option<&'static str> = None;

    let want_v = normalize_codec(want_video);
    let want_a = normalize_codec(want_audio);

    // Explicit non-copy must be valid for the container.
    if want_v != "copy" {
        validate_codec_for_container(out, &want_v, None)?;
    }
    if !drop_audio && want_a != "copy" {
        validate_codec_for_container(out, "copy", Some(&want_a))?;
    }

    let v_wire = if want_v == "copy" {
        match input_video.map(normalize_codec) {
            Some(c) if video_codec_muxable(out, &c) => "copy".to_string(),
            Some(_) => {
                auto = true;
                reason = Some("copy_incompatible_with_container");
                default_video_wire(out).to_string()
            }
            None if !probe_ok && matches!(out, OutputContainer::Webm) => {
                // Conservative: H.264-in-MP4 is the common agent input; never copy into WebM blind.
                auto = true;
                reason = Some("ffprobe_unavailable");
                default_video_wire(out).to_string()
            }
            None => "copy".to_string(),
        }
    } else {
        want_v
    };

    let a_wire = if drop_audio {
        "none".to_string()
    } else if want_a == "copy" {
        match input_audio.map(normalize_codec) {
            Some(c) if audio_codec_muxable(out, &c) => "copy".to_string(),
            Some(_) => {
                auto = true;
                if reason.is_none() {
                    reason = Some("copy_incompatible_with_container");
                }
                default_audio_wire(out).to_string()
            }
            None if !probe_ok && matches!(out, OutputContainer::Webm) => {
                auto = true;
                if reason.is_none() {
                    reason = Some("ffprobe_unavailable");
                }
                default_audio_wire(out).to_string()
            }
            None => "copy".to_string(),
        }
    } else {
        want_a
    };

    let video_ffmpeg = if v_wire == "copy" {
        "copy".into()
    } else if v_wire == default_video_wire(out) {
        default_video_encoder(out).to_string()
    } else {
        video_encoder_arg(&v_wire)
    };
    let audio_ffmpeg = if a_wire == "none" {
        "none".into()
    } else if a_wire == "copy" {
        "copy".into()
    } else if a_wire == default_audio_wire(out) {
        default_audio_encoder(out).to_string()
    } else {
        audio_encoder_arg(&a_wire)
    };

    // Final safety: non-copy args must still be valid (encode path).
    if video_ffmpeg != "copy" && !video_codec_muxable(out, &v_wire) {
        return Err(incompat(
            out,
            &v_wire,
            "video codec not accepted for this container",
        ));
    }
    if !drop_audio && audio_ffmpeg != "copy" && !audio_codec_muxable(out, &a_wire) {
        return Err(incompat(
            out,
            &a_wire,
            "audio codec not accepted for this container",
        ));
    }

    let stream_copy = video_ffmpeg == "copy" && (audio_ffmpeg == "copy" || drop_audio);
    Ok(CodecPlan {
        video_ffmpeg,
        audio_ffmpeg,
        stream_copy,
        auto_reencoded: auto,
        reencode_reason: if auto { reason } else { None },
    })
}
