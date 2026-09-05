// SPDX-License-Identifier: MIT OR Apache-2.0
//! to-mp3 / trim / thumbnail path→path ffmpeg ops.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use super::super::container_matrix::OutputContainer;
use super::super::encoder_policy::resolve_effective_codecs;
use super::super::ffmpeg_bin::require_ffmpeg;
use super::super::probe::{primary_audio_codec, primary_video_codec, probe_path};
use super::atomic::{
    cleanup_partials, ensure_parent, ffmpeg_fail, finalize_partial, map_spawn_err, partial_path,
    sha256_file,
};
use super::types::{ConvertOpts, ConvertResult};
use crate::error::{CliError, ErrorKind};
use crate::platform::run_capture_with_timeout;

/// Extract audio track to MP3 (re-encode with libmp3lame).
pub fn to_mp3_path(
    input: &Path,
    output: &Path,
    bitrate: &str,
    audio_stream: Option<u32>,
) -> Result<ConvertResult, CliError> {
    let bin = require_ffmpeg()?;
    let timeout = Duration::from_secs(crate::xdg::resolve_ffmpeg_timeout_secs());
    ensure_parent(output)?;
    let partial = partial_path(output);
    let _ = std::fs::remove_file(&partial);

    let mut cmd = Command::new(&bin);
    cmd.arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-i")
        .arg(input)
        .arg("-vn");
    if let Some(idx) = audio_stream {
        cmd.arg("-map").arg(format!("0:a:{idx}"));
    } else {
        cmd.arg("-map").arg("0:a:0?");
    }
    cmd.arg("-c:a")
        .arg("libmp3lame")
        .arg("-b:a")
        .arg(bitrate)
        .arg(&partial);

    let out = run_capture_with_timeout(&mut cmd, timeout).map_err(|e| {
        cleanup_partials(output, &partial);
        map_spawn_err(e, "to-mp3")
    })?;
    if !out.status.success() {
        cleanup_partials(output, &partial);
        return Err(ffmpeg_fail("to-mp3", &out.stderr));
    }
    if let Err(e) = finalize_partial(&partial, output) {
        cleanup_partials(output, &partial);
        return Err(e);
    }
    let meta = std::fs::metadata(output)
        .map_err(|e| crate::video_local::magic::io_path_err(output, "stat", &e))?;
    Ok(ConvertResult {
        path_out: output.to_path_buf(),
        bytes_out: meta.len(),
        sha256_out: sha256_file(output)?,
        stream_copy: false,
        video_codec: "none".into(),
        audio_codec: "libmp3lame".into(),
        metadata_stripped: false,
        auto_reencoded: false,
        reencode_reason: None,
        faststart_applied: false,
        duration_secs: None,
    })
}

/// Trim a time range (path→path). Prefers stream copy when codecs stay muxable.
#[allow(clippy::too_many_arguments)]
pub fn trim_path(
    input: &Path,
    output: &Path,
    start: f64,
    duration: Option<f64>,
    end: Option<f64>,
    video_codec: Option<&str>,
    audio_codec: Option<&str>,
    format_hint: OutputContainer,
) -> Result<ConvertResult, CliError> {
    if start < 0.0 {
        return Err(CliError::new(
            ErrorKind::Usage,
            "video trim --start must be >= 0",
        ));
    }
    let dur = match (duration, end) {
        (Some(d), _) if d > 0.0 => Some(d),
        (None, Some(e)) if e > start => Some(e - start),
        (None, None) => None,
        _ => {
            return Err(CliError::new(
                ErrorKind::Usage,
                "video trim: pass --duration > 0 or --to > --start",
            ));
        }
    };

    let opts = ConvertOpts::from_flags(
        format_hint,
        video_codec,
        audio_codec,
        None,
        false,
        false,
        false,
    );
    let (probe_ok, in_v, in_a) = match probe_path(input) {
        Ok(raw) => (true, primary_video_codec(&raw), primary_audio_codec(&raw)),
        Err(_) => (false, None, None),
    };
    let plan = resolve_effective_codecs(
        format_hint,
        &opts.video_codec,
        &opts.audio_codec,
        false,
        in_v.as_deref(),
        in_a.as_deref(),
        probe_ok,
    )?;

    let bin = require_ffmpeg()?;
    let timeout = Duration::from_secs(crate::xdg::resolve_ffmpeg_timeout_secs());
    ensure_parent(output)?;
    let partial = partial_path(output);
    let _ = std::fs::remove_file(&partial);

    let mut cmd = Command::new(&bin);
    cmd.arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-ss")
        .arg(format!("{start:.3}"))
        .arg("-i")
        .arg(input);
    if let Some(d) = dur {
        cmd.arg("-t").arg(format!("{d:.3}"));
    }
    cmd.arg("-map")
        .arg("0")
        .arg("-c:v")
        .arg(&plan.video_ffmpeg)
        .arg("-c:a")
        .arg(&plan.audio_ffmpeg);
    if plan.video_ffmpeg != "copy" {
        cmd.arg("-crf")
            .arg(crate::xdg::resolve_video_default_crf().to_string());
        cmd.arg("-pix_fmt")
            .arg(crate::constants::SCREENCAST_FFMPEG_PIX_FMT);
    }
    if !opts.no_faststart && format_hint.supports_faststart() {
        cmd.arg("-movflags").arg("+faststart");
    }
    cmd.arg(&partial);

    let out = run_capture_with_timeout(&mut cmd, timeout).map_err(|e| {
        cleanup_partials(output, &partial);
        map_spawn_err(e, "trim")
    })?;
    if !out.status.success() {
        cleanup_partials(output, &partial);
        return Err(ffmpeg_fail("trim", &out.stderr));
    }
    if let Err(e) = finalize_partial(&partial, output) {
        cleanup_partials(output, &partial);
        return Err(e);
    }
    let meta = std::fs::metadata(output)
        .map_err(|e| crate::video_local::magic::io_path_err(output, "stat", &e))?;
    Ok(ConvertResult {
        path_out: output.to_path_buf(),
        bytes_out: meta.len(),
        sha256_out: sha256_file(output)?,
        stream_copy: plan.stream_copy,
        video_codec: plan.video_ffmpeg,
        audio_codec: plan.audio_ffmpeg,
        metadata_stripped: false,
        auto_reencoded: plan.auto_reencoded,
        reencode_reason: plan.reencode_reason.map(str::to_string),
        faststart_applied: !opts.no_faststart && format_hint.supports_faststart(),
        duration_secs: dur,
    })
}

/// Extract a single video frame as an image file (png/jpg via extension).
pub fn thumbnail_path(
    input: &Path,
    output: &Path,
    at_secs: f64,
) -> Result<ConvertResult, CliError> {
    if at_secs < 0.0 {
        return Err(CliError::new(
            ErrorKind::Usage,
            "video thumbnail --at must be >= 0",
        ));
    }
    let bin = require_ffmpeg()?;
    let timeout = Duration::from_secs(crate::xdg::resolve_ffmpeg_timeout_secs());
    ensure_parent(output)?;
    let partial = partial_path(output);
    let _ = std::fs::remove_file(&partial);

    let mut cmd = Command::new(&bin);
    cmd.arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-ss")
        .arg(format!("{at_secs:.3}"))
        .arg("-i")
        .arg(input)
        .arg("-frames:v")
        .arg("1")
        .arg("-q:v")
        .arg("2")
        .arg(&partial);

    let out = run_capture_with_timeout(&mut cmd, timeout).map_err(|e| {
        cleanup_partials(output, &partial);
        map_spawn_err(e, "thumbnail")
    })?;
    if !out.status.success() {
        cleanup_partials(output, &partial);
        return Err(ffmpeg_fail("thumbnail", &out.stderr));
    }
    if let Err(e) = finalize_partial(&partial, output) {
        cleanup_partials(output, &partial);
        return Err(e);
    }
    let meta = std::fs::metadata(output)
        .map_err(|e| crate::video_local::magic::io_path_err(output, "stat", &e))?;
    Ok(ConvertResult {
        path_out: output.to_path_buf(),
        bytes_out: meta.len(),
        sha256_out: sha256_file(output)?,
        stream_copy: false,
        video_codec: "mjpeg_or_png".into(),
        audio_codec: "none".into(),
        metadata_stripped: false,
        auto_reencoded: false,
        reencode_reason: None,
        faststart_applied: false,
        duration_secs: Some(at_secs),
    })
}
