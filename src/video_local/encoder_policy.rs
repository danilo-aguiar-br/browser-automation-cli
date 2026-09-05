// SPDX-License-Identifier: MIT OR Apache-2.0
//! Encoder policy: stream-copy when possible, otherwise pick a defensible default.
//!
//! # Why this is separate from [`super::container_matrix`]
//!
//! The matrix states what a container accepts, and that never changes. This
//! module decides what to DO about it, and every decision here is a product
//! choice that could reasonably be made differently:
//!
//! - prefer copy over re-encode whenever the input already muxes;
//! - when it does not, re-encode to the container default rather than refuse,
//!   so `video convert --format webm` works on an H.264 source with no flags;
//! - when ffprobe is unavailable, never copy blind into WebM, because H.264 in
//!   MP4 is the common agent input and a blind copy would produce a file no
//!   player accepts.
//!
//! Those are conservative defaults, not facts about containers. Separating them
//! means changing a default cannot accidentally rewrite the compatibility table.

use crate::error::CliError;

use super::container_matrix::{
    audio_codec_muxable, incompat, normalize_codec, validate_codec_for_container,
    video_codec_muxable, OutputContainer,
};

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

    // Final validation: non-copy args must still be valid (encode path). Worded
    // without the word "safety" on purpose: `clippy::unnecessary_safety_comment`
    // keys on that word and fires on this line, which holds no `unsafe` block.
    // The lint is not enabled in this crate, so the rename costs nothing today
    // and removes one false positive from the day it is.
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
