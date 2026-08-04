// SPDX-License-Identifier: MIT OR Apache-2.0
//! Path→path container convert / remux via ffmpeg.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use super::super::ffmpeg_bin::require_ffmpeg;
use super::super::probe::{duration_secs, primary_audio_codec, primary_video_codec, probe_path};
use super::super::validate::resolve_effective_codecs;
use super::atomic::{
    cleanup_partials, ensure_parent, ffmpeg_fail, finalize_partial, map_spawn_err, partial_path,
    sha256_file,
};
use super::types::{ConvertOpts, ConvertResult};
use crate::error::CliError;
use crate::platform::run_capture_with_timeout;

/// Convert/remux `input` to `output` (path→path; atomic partial; smart copy).
pub fn convert_path(
    input: &Path,
    output: &Path,
    opts: &ConvertOpts,
) -> Result<ConvertResult, CliError> {
    let (probe_ok, in_v, in_a, dur) = match probe_path(input) {
        Ok(raw) => (
            true,
            primary_video_codec(&raw),
            primary_audio_codec(&raw),
            duration_secs(&raw),
        ),
        Err(_) => (false, None, None, None),
    };

    let plan = resolve_effective_codecs(
        opts.format,
        &opts.video_codec,
        &opts.audio_codec,
        opts.drop_audio,
        in_v.as_deref(),
        in_a.as_deref(),
        probe_ok,
    )?;

    let faststart_applied = !opts.no_faststart && opts.format.supports_faststart();

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
        .arg("-map")
        .arg("0");
    if opts.drop_audio {
        cmd.arg("-an");
    }
    cmd.arg("-c:v").arg(&plan.video_ffmpeg);
    if !opts.drop_audio {
        cmd.arg("-c:a").arg(&plan.audio_ffmpeg);
    }
    if plan.video_ffmpeg != "copy" {
        cmd.arg("-crf").arg(opts.crf.to_string());
        cmd.arg("-pix_fmt")
            .arg(crate::constants::SCREENCAST_FFMPEG_PIX_FMT);
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
        .map_err(|e| crate::video_local::magic::io_path_err(output, "stat", &e))?;
    let sha = sha256_file(output)?;
    Ok(ConvertResult {
        path_out: output.to_path_buf(),
        bytes_out: meta.len(),
        sha256_out: sha,
        stream_copy: plan.stream_copy,
        video_codec: plan.video_ffmpeg,
        audio_codec: if opts.drop_audio {
            "none".into()
        } else {
            plan.audio_ffmpeg
        },
        metadata_stripped: opts.strip_metadata,
        auto_reencoded: plan.auto_reencoded,
        reencode_reason: plan.reencode_reason.map(str::to_string),
        faststart_applied,
        duration_secs: dur,
    })
}
