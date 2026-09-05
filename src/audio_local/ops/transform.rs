// SPDX-License-Identifier: MIT OR Apache-2.0
//! convert / trim agent envelopes for audio.

use std::path::Path;

use serde_json::{json, Value};

use super::super::ffmpeg_ops::{convert_path, trim_path, ConvertOpts};
use super::super::limits::AudioLimits;
use super::super::magic::probe_path_magic;
use super::super::validate::parse_output_format;
use super::common::{convert_envelope, resolve_out_path};
use super::source::AudioSource;
use crate::error::CliError;
use crate::xdg;

/// Convert audio format / codec (path→path via ffmpeg).
pub fn convert(
    source: &AudioSource,
    format: &str,
    out: Option<&Path>,
    opts: ConvertOpts,
    select: Option<&str>,
) -> Result<Value, CliError> {
    let limits = AudioLimits::from_xdg();
    let (path_in, _temp) = source.resolve_path(limits)?;
    let magic = probe_path_magic(&path_in)?;
    let container = parse_output_format(format)?;
    let mut opts = opts;
    opts.format = container;
    let out_path = resolve_out_path(out, container.extension())?;
    let res = convert_path(&path_in, &out_path, &opts)?;
    let full = convert_envelope(
        "convert",
        &path_in,
        magic.as_str(),
        container.extension(),
        &res,
    );
    Ok(super::common::project_fields(full, select))
}

/// Trim a time range (path→path).
#[allow(clippy::too_many_arguments)]
pub fn trim(
    source: &AudioSource,
    start: f64,
    duration: Option<f64>,
    to: Option<f64>,
    out: Option<&Path>,
    format: Option<&str>,
    codec: Option<&str>,
    bitrate: Option<&str>,
    select: Option<&str>,
) -> Result<Value, CliError> {
    let limits = AudioLimits::from_xdg();
    let (path_in, _temp) = source.resolve_path(limits)?;
    let magic = probe_path_magic(&path_in)?;
    let fmt = format.map(|s| s.to_string()).unwrap_or_else(|| {
        // Prefer magic-based extension default, else XDG audio default.
        match magic.as_str() {
            "wav" | "flac" | "ogg" | "mp3" | "m4a" | "aiff" | "aac" => magic.as_str().into(),
            _ => xdg::resolve_audio_default_format(),
        }
    });
    let container = parse_output_format(&fmt)?;
    let br = bitrate
        .map(|s| s.to_string())
        .unwrap_or_else(xdg::resolve_audio_default_bitrate);
    let out_path = resolve_out_path(out, container.extension())?;
    let res = trim_path(
        &path_in, &out_path, start, duration, to, container, codec, &br,
    )?;
    let mut full = convert_envelope(
        "trim",
        &path_in,
        magic.as_str(),
        container.extension(),
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
