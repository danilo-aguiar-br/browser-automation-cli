// SPDX-License-Identifier: MIT OR Apache-2.0
//! Audio trim path→path via ffmpeg.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use super::super::validate::{default_encoder, OutputFormat};
use super::atomic::{
    cleanup_partials, ensure_parent, ffmpeg_fail, finalize_partial, map_spawn_err, partial_path,
    sha256_file,
};
use super::types::ConvertResult;
use crate::error::{CliError, ErrorKind};
use crate::platform::run_capture_with_timeout;
use crate::video_local::{duration_secs, primary_audio_codec, probe_path, require_ffmpeg};

/// Trim a time range from audio (path→path). Prefers stream copy when possible.
#[allow(clippy::too_many_arguments)]
pub fn trim_path(
    input: &Path,
    output: &Path,
    start: f64,
    duration: Option<f64>,
    end: Option<f64>,
    format: OutputFormat,
    codec: Option<&str>,
    bitrate: &str,
) -> Result<ConvertResult, CliError> {
    if start < 0.0 {
        return Err(CliError::new(
            ErrorKind::Usage,
            "audio trim --start must be >= 0",
        ));
    }
    let dur = match (duration, end) {
        (Some(d), _) if d > 0.0 => Some(d),
        (None, Some(e)) if e > start => Some(e - start),
        (None, None) => None,
        _ => {
            return Err(CliError::new(
                ErrorKind::Usage,
                "audio trim: pass --duration > 0 or --to > --start",
            ));
        }
    };

    let (in_a, probe_dur) = match probe_path(input) {
        Ok(raw) => (primary_audio_codec(&raw), duration_secs(&raw)),
        Err(_) => (None, None),
    };

    // Prefer copy when input codec muxable; else re-encode.
    let use_copy = codec
        .map(|c| c.eq_ignore_ascii_case("copy"))
        .unwrap_or(true)
        && in_a
            .as_deref()
            .map(|c| super::super::validate::codec_copy_muxable(format, c))
            .unwrap_or(false);

    let enc = if use_copy {
        "copy".to_string()
    } else {
        default_encoder(format, codec)
    };

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
        .arg(start.to_string())
        .arg("-i")
        .arg(input)
        .arg("-vn")
        .arg("-map")
        .arg("0:a:0?");
    if let Some(d) = dur {
        cmd.arg("-t").arg(d.to_string());
    }
    cmd.arg("-c:a").arg(&enc);
    if enc == "libvorbis" {
        cmd.arg("-q:a")
            .arg(crate::constants::DEFAULT_VORBIS_QUALITY);
    } else if enc != "copy" && enc != "flac" && enc != "pcm_s16le" {
        cmd.arg("-b:a").arg(bitrate);
    }
    cmd.arg("-f").arg(format.as_str());
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
        .map_err(|e| crate::audio_local::magic::io_path_err(output, "stat", &e))?;
    Ok(ConvertResult {
        path_out: output.to_path_buf(),
        bytes_out: meta.len(),
        sha256_out: sha256_file(output)?,
        stream_copy: use_copy,
        audio_codec: enc,
        metadata_stripped: false,
        auto_reencoded: !use_copy,
        reencode_reason: if use_copy {
            None
        } else {
            Some("trim_reencode".into())
        },
        lossy_transcode: false,
        faststart_applied: false,
        duration_secs: dur.or(probe_dur),
    })
}
