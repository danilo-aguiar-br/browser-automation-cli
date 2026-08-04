// SPDX-License-Identifier: MIT OR Apache-2.0
//! Path→path audio convert / remux via ffmpeg (`-vn`).

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use super::super::validate::resolve_audio_plan;
use super::atomic::{
    cleanup_partials, ensure_parent, ffmpeg_fail, finalize_partial, map_spawn_err, partial_path,
    sha256_file,
};
use super::types::{ConvertOpts, ConvertResult};
use crate::error::CliError;
use crate::platform::run_capture_with_timeout;
use crate::video_local::{duration_secs, primary_audio_codec, probe_path, require_ffmpeg};

/// Convert/remux audio `input` to `output` (path→path; atomic partial; smart copy).
pub fn convert_path(
    input: &Path,
    output: &Path,
    opts: &ConvertOpts,
) -> Result<ConvertResult, CliError> {
    let (probe_ok, in_a, dur) = match probe_path(input) {
        Ok(raw) => (true, primary_audio_codec(&raw), duration_secs(&raw)),
        Err(_) => (false, None, None),
    };

    let plan = resolve_audio_plan(
        opts.format,
        opts.codec.as_deref(),
        in_a.as_deref(),
        probe_ok,
    );

    let faststart_applied = opts.format.supports_faststart() && !plan.stream_copy;

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
    if let Some(idx) = opts.audio_stream {
        cmd.arg("-map").arg(format!("0:a:{idx}"));
    } else {
        cmd.arg("-map").arg("0:a:0?");
    }
    cmd.arg("-c:a").arg(&plan.audio_ffmpeg);
    if !plan.stream_copy {
        if plan.audio_ffmpeg == "libvorbis" {
            // Quality mode is more reliable than -b:a for low sample rates (e.g. 8 kHz).
            cmd.arg("-q:a")
                .arg(crate::constants::DEFAULT_VORBIS_QUALITY);
        } else if plan.audio_ffmpeg != "flac" && plan.audio_ffmpeg != "pcm_s16le" {
            cmd.arg("-b:a").arg(&opts.bitrate);
        }
        if let Some(sr) = opts.sample_rate {
            cmd.arg("-ar").arg(sr.to_string());
        }
        if let Some(ch) = opts.channels {
            cmd.arg("-ac").arg(ch.to_string());
        }
    }
    if faststart_applied {
        cmd.arg("-movflags").arg("+faststart");
    }
    if opts.strip_metadata {
        cmd.arg("-map_metadata").arg("-1");
    }
    cmd.arg("-f").arg(opts.format.as_str());
    cmd.arg(&partial);

    let out = run_capture_with_timeout(&mut cmd, timeout).map_err(|e| {
        cleanup_partials(output, &partial);
        map_spawn_err(e, "convert")
    })?;
    if !out.status.success() {
        cleanup_partials(output, &partial);
        return Err(ffmpeg_fail("convert", &out.stderr));
    }
    if let Err(e) = finalize_partial(&partial, output) {
        cleanup_partials(output, &partial);
        return Err(e);
    }

    let meta = std::fs::metadata(output)
        .map_err(|e| crate::audio_local::magic::io_path_err(output, "stat", &e))?;
    let sha = sha256_file(output)?;
    Ok(ConvertResult {
        path_out: output.to_path_buf(),
        bytes_out: meta.len(),
        sha256_out: sha,
        stream_copy: plan.stream_copy,
        audio_codec: plan.audio_ffmpeg,
        metadata_stripped: opts.strip_metadata,
        auto_reencoded: plan.auto_reencoded,
        reencode_reason: plan.reencode_reason.map(str::to_string),
        lossy_transcode: plan.lossy_transcode,
        faststart_applied,
        duration_secs: dur,
    })
}
