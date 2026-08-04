// SPDX-License-Identifier: MIT OR Apache-2.0
//! convert / to-mp3 / trim / thumbnail agent envelopes.

use std::path::Path;

use serde_json::{json, Value};

use super::super::ffmpeg_ops::{convert_path, thumbnail_path, to_mp3_path, trim_path, ConvertOpts};
use super::super::limits::VideoLimits;
use super::super::magic::{probe_path_magic, DetectedContainer};
use super::super::validate::parse_output_container;
use super::common::{convert_envelope, default_out_path};
use super::source::VideoSource;
use crate::error::CliError;
use crate::xdg;

/// Convert container / codecs (path→path via ffmpeg).
pub fn convert(
    source: &VideoSource,
    format: &str,
    out: Option<&Path>,
    opts: ConvertOpts,
    select: Option<&str>,
) -> Result<Value, CliError> {
    let limits = VideoLimits::from_xdg();
    let (path_in, _temp) = source.resolve_path(limits)?;
    let magic = probe_path_magic(&path_in)?;
    let container = parse_output_container(format)?;
    let mut opts = opts;
    opts.format = container;
    let out_path = out
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| default_out_path(container.as_str()));
    let drop_audio = opts.drop_audio;
    let res = convert_path(&path_in, &out_path, &opts)?;
    let full = convert_envelope(
        "convert",
        &path_in,
        magic.as_str(),
        container.as_str(),
        drop_audio,
        &res,
    );
    Ok(super::common::project_fields(full, select))
}

/// Extract first audio stream to MP3.
pub fn to_mp3(
    source: &VideoSource,
    out: Option<&Path>,
    bitrate: Option<&str>,
    audio_stream: Option<u32>,
    select: Option<&str>,
) -> Result<Value, CliError> {
    let limits = VideoLimits::from_xdg();
    let (path_in, _temp) = source.resolve_path(limits)?;
    let magic = probe_path_magic(&path_in)?;
    let br = bitrate
        .map(|s| s.to_string())
        .unwrap_or_else(xdg::resolve_video_default_audio_bitrate);
    let out_path = out
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| default_out_path("mp3"));
    let res = to_mp3_path(&path_in, &out_path, &br, audio_stream)?;
    let full = json!({
        "action": "to_mp3",
        "path_in": path_in.display().to_string(),
        "path_out": res.path_out.display().to_string(),
        "container_in": magic.as_str(),
        "audio_codec": res.audio_codec,
        "bitrate": br,
        "audio_stream": audio_stream.unwrap_or(0),
        "bytes_out": res.bytes_out,
        "sha256_out": res.sha256_out,
        "engine": "ffmpeg",
        "ffmpeg_timeout_secs": xdg::resolve_ffmpeg_timeout_secs(),
    });
    Ok(super::common::project_fields(full, select))
}

/// Trim a time range (path→path).
#[allow(clippy::too_many_arguments)]
pub fn trim(
    source: &VideoSource,
    start: f64,
    duration: Option<f64>,
    to: Option<f64>,
    out: Option<&Path>,
    format: Option<&str>,
    video_codec: Option<&str>,
    audio_codec: Option<&str>,
    select: Option<&str>,
) -> Result<Value, CliError> {
    let limits = VideoLimits::from_xdg();
    let (path_in, _temp) = source.resolve_path(limits)?;
    let magic = probe_path_magic(&path_in)?;
    let fmt = format
        .map(|s| s.to_string())
        .unwrap_or_else(|| match magic {
            DetectedContainer::MatroskaOrWebm => "mkv".into(),
            DetectedContainer::Mov => "mov".into(),
            DetectedContainer::Avi => "avi".into(),
            DetectedContainer::M4v => "m4v".into(),
            _ => xdg::resolve_video_default_container(),
        });
    let container = parse_output_container(&fmt)?;
    let out_path = out
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| default_out_path(container.as_str()));
    let res = trim_path(
        &path_in,
        &out_path,
        start,
        duration,
        to,
        video_codec,
        audio_codec,
        container,
    )?;
    let mut full = convert_envelope(
        "trim",
        &path_in,
        magic.as_str(),
        container.as_str(),
        false,
        &res,
    );
    if let Some(obj) = full.as_object_mut() {
        obj.insert("start_secs".into(), json!(start));
        if let Some(d) = duration {
            obj.insert("duration_secs_req".into(), json!(d));
        }
        if let Some(t) = to {
            obj.insert("to_secs".into(), json!(t));
        }
    }
    Ok(super::common::project_fields(full, select))
}

/// Extract one frame as image path.
pub fn thumbnail(
    source: &VideoSource,
    at: Option<f64>,
    out: Option<&Path>,
    select: Option<&str>,
) -> Result<Value, CliError> {
    let limits = VideoLimits::from_xdg();
    let (path_in, _temp) = source.resolve_path(limits)?;
    let magic = probe_path_magic(&path_in)?;
    let at_secs = at.unwrap_or(0.0);
    let out_path = out
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| default_out_path("png"));
    let res = thumbnail_path(&path_in, &out_path, at_secs)?;
    let full = json!({
        "action": "thumbnail",
        "path_in": path_in.display().to_string(),
        "path_out": res.path_out.display().to_string(),
        "container_in": magic.as_str(),
        "at_secs": at_secs,
        "bytes_out": res.bytes_out,
        "sha256_out": res.sha256_out,
        "engine": "ffmpeg",
        "ffmpeg_timeout_secs": xdg::resolve_ffmpeg_timeout_secs(),
    });
    Ok(super::common::project_fields(full, select))
}
