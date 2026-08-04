// SPDX-License-Identifier: MIT OR Apache-2.0
//! Output format validation and audio codec planning (copy vs re-encode).

use crate::error::{CliError, ErrorKind};

/// Output audio format selected by the agent (`--format`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// MPEG Layer III.
    Mp3,
    /// MPEG-4 audio (AAC typically).
    M4a,
    /// AAC elementary (ADTS mux when bare).
    Aac,
    /// OGG container (Vorbis default).
    Ogg,
    /// Opus in OGG.
    Opus,
    /// FLAC lossless.
    Flac,
    /// WAV PCM.
    Wav,
}

impl OutputFormat {
    /// Wire extension / format name for ffmpeg `-f` and paths.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::M4a => "ipod", // ffmpeg muxer for m4a
            Self::Aac => "adts",
            Self::Ogg => "ogg",
            Self::Opus => "ogg",
            Self::Flac => "flac",
            Self::Wav => "wav",
        }
    }

    /// Preferred file extension for default output paths.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Aac => "aac",
            Self::Ogg => "ogg",
            Self::Opus => "opus",
            Self::Flac => "flac",
            Self::Wav => "wav",
        }
    }

    /// Whether MP4-family faststart applies.
    #[must_use]
    pub fn supports_faststart(self) -> bool {
        matches!(self, Self::M4a)
    }
}

/// Parse `--format` for audio convert.
pub fn parse_output_format(raw: &str) -> Result<OutputFormat, CliError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "mp3" => Ok(OutputFormat::Mp3),
        "m4a" | "mp4" => Ok(OutputFormat::M4a),
        "aac" => Ok(OutputFormat::Aac),
        "ogg" => Ok(OutputFormat::Ogg),
        "opus" => Ok(OutputFormat::Opus),
        "flac" => Ok(OutputFormat::Flac),
        "wav" | "wave" => Ok(OutputFormat::Wav),
        other => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("unsupported audio output format '{other}'"),
            crate::i18n::suggestion_key("audio_format_unsupported", None),
        )),
    }
}

/// Normalize codec name from ffprobe / agent flags.
#[must_use]
pub fn normalize_codec(name: &str) -> String {
    let n = name.trim().to_ascii_lowercase();
    match n.as_str() {
        "aac" | "mp4a" => "aac".into(),
        "opus" | "libopus" => "opus".into(),
        "vorbis" | "libvorbis" => "vorbis".into(),
        "mp3" | "libmp3lame" | "mp3float" => "mp3".into(),
        "flac" => "flac".into(),
        "pcm_s16le" | "pcm_s24le" | "pcm_s32le" | "pcm_f32le" | "pcm" => "pcm".into(),
        "alac" => "alac".into(),
        "copy" => "copy".into(),
        other => other.to_string(),
    }
}

/// True when codec is lossy (for lossy→lossy agent warning).
#[must_use]
pub fn is_lossy_codec(codec: &str) -> bool {
    matches!(
        normalize_codec(codec).as_str(),
        "mp3" | "aac" | "opus" | "vorbis" | "ac3" | "eac3"
    )
}

/// Default ffmpeg audio encoder for an output format (when not stream-copy).
#[must_use]
pub fn default_encoder(format: OutputFormat, codec_flag: Option<&str>) -> String {
    if let Some(c) = codec_flag {
        let n = normalize_codec(c);
        if n != "copy" {
            return match n.as_str() {
                "mp3" => "libmp3lame".into(),
                "aac" => "aac".into(),
                "opus" => "libopus".into(),
                "vorbis" => "libvorbis".into(),
                "flac" => "flac".into(),
                "pcm" => "pcm_s16le".into(),
                other => other.to_string(),
            };
        }
    }
    match format {
        OutputFormat::Mp3 => "libmp3lame".into(),
        OutputFormat::M4a | OutputFormat::Aac => "aac".into(),
        OutputFormat::Ogg => "libvorbis".into(),
        OutputFormat::Opus => "libopus".into(),
        OutputFormat::Flac => "flac".into(),
        OutputFormat::Wav => "pcm_s16le".into(),
    }
}

/// Whether stream copy of `in_codec` can mux into `out`.
#[must_use]
pub fn codec_copy_muxable(out: OutputFormat, in_codec: &str) -> bool {
    let c = normalize_codec(in_codec);
    match out {
        OutputFormat::Mp3 => c == "mp3",
        OutputFormat::M4a => matches!(c.as_str(), "aac" | "alac"),
        OutputFormat::Aac => c == "aac",
        OutputFormat::Ogg => matches!(c.as_str(), "vorbis" | "opus" | "flac"),
        OutputFormat::Opus => c == "opus",
        OutputFormat::Flac => c == "flac",
        OutputFormat::Wav => c == "pcm",
    }
}

/// Plan for convert: ffmpeg codec string + flags.
#[derive(Debug, Clone)]
pub struct AudioPlan {
    /// ffmpeg `-c:a` value.
    pub audio_ffmpeg: String,
    /// True when using stream copy.
    pub stream_copy: bool,
    /// True when intent was copy but upgraded to re-encode.
    pub auto_reencoded: bool,
    /// Machine reason for auto re-encode.
    pub reencode_reason: Option<&'static str>,
    /// Lossy input → lossy output recompress.
    pub lossy_transcode: bool,
}

/// Resolve effective audio codec plan.
pub fn resolve_audio_plan(
    format: OutputFormat,
    codec_flag: Option<&str>,
    in_codec: Option<&str>,
    probe_ok: bool,
) -> AudioPlan {
    let want_copy = codec_flag
        .map(|c| normalize_codec(c) == "copy")
        .unwrap_or(true);

    let in_lossy = in_codec.map(is_lossy_codec).unwrap_or(false);
    let out_enc = default_encoder(format, codec_flag.filter(|c| normalize_codec(c) != "copy"));
    let out_lossy = is_lossy_codec(&out_enc);

    if want_copy && probe_ok {
        if let Some(ic) = in_codec {
            if codec_copy_muxable(format, ic) {
                return AudioPlan {
                    audio_ffmpeg: "copy".into(),
                    stream_copy: true,
                    auto_reencoded: false,
                    reencode_reason: None,
                    lossy_transcode: false,
                };
            }
            return AudioPlan {
                audio_ffmpeg: out_enc,
                stream_copy: false,
                auto_reencoded: true,
                reencode_reason: Some("codec_not_muxable"),
                lossy_transcode: in_lossy && out_lossy,
            };
        }
    }

    if want_copy && !probe_ok {
        return AudioPlan {
            audio_ffmpeg: out_enc,
            stream_copy: false,
            auto_reencoded: true,
            reencode_reason: Some("probe_unavailable"),
            lossy_transcode: out_lossy, // unknown input; flag if out lossy
        };
    }

    AudioPlan {
        audio_ffmpeg: out_enc,
        stream_copy: false,
        auto_reencoded: false,
        reencode_reason: None,
        lossy_transcode: in_lossy && out_lossy,
    }
}
