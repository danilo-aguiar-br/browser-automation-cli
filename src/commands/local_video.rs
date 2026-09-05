// SPDX-License-Identifier: MIT OR Apache-2.0
//! Dispatch handlers for `video` (local media; no Chrome).

use crate::cli::VideoAction;
use crate::commands::common::emit_ok;
use crate::error::CliError;
use crate::video_local::{self, ConvertOpts, VideoSource};

/// Resolve `--path` / `--stdin` / `--paths-file` for the `video` family.
fn video_inputs(
    path: Option<std::path::PathBuf>,
    stdin: bool,
    paths_file: Option<std::path::PathBuf>,
) -> Result<crate::commands::media::MediaInputs<VideoSource>, CliError> {
    crate::commands::media::resolve("video", path, stdin, paths_file, VideoSource::Path, || {
        VideoSource::Stdin
    })
}

/// Run a `video` subcommand and write the agent envelope.
pub(crate) fn handle_video(action: VideoAction, json: bool) -> Result<(), CliError> {
    let data = match action {
        VideoAction::Info {
            path,
            stdin,
            paths_file,
            select,
        } => {
            let inputs = video_inputs(path, stdin, paths_file)?;
            crate::commands::media::run("video", inputs, |src| {
                video_local::info(src, select.as_deref())
            })?
        }
        VideoAction::Download {
            url,
            out,
            max_bytes,
            require_video,
            allow_non_video,
            select,
        } => {
            let require = if allow_non_video {
                false
            } else {
                require_video
            };
            let v = crate::runtime_util::block_on_io(video_local::download_video(
                &url,
                out.as_deref(),
                max_bytes,
                require,
            ))?;
            crate::video_local::project_fields(v, select.as_deref())
        }
        VideoAction::Convert {
            path,
            stdin,
            paths_file,
            format,
            out,
            video_codec,
            audio_codec,
            crf,
            no_faststart,
            strip_metadata,
            drop_audio,
            select,
        } => {
            let inputs = video_inputs(path, stdin, paths_file)?;
            let fmt = format.unwrap_or_else(crate::xdg::resolve_video_default_container);
            let container = video_local::parse_output_container(&fmt)?;
            let opts = ConvertOpts::from_flags(
                container,
                video_codec.as_deref(),
                audio_codec.as_deref(),
                crf,
                no_faststart,
                strip_metadata,
                drop_audio,
            );
            crate::commands::media::run_producing(
                "video",
                inputs,
                out.as_deref(),
                |_| fmt.clone(),
                |src, dest| video_local::convert(src, &fmt, dest, opts.clone(), select.as_deref()),
            )?
        }
        VideoAction::ToMp3 {
            path,
            stdin,
            paths_file,
            out,
            bitrate,
            audio_stream,
            select,
        } => {
            let inputs = video_inputs(path, stdin, paths_file)?;
            crate::commands::media::run_producing(
                "video",
                inputs,
                out.as_deref(),
                // The action names its own output format; nothing to infer.
                |_| "mp3".to_string(),
                |src, dest| {
                    video_local::to_mp3(
                        src,
                        dest,
                        bitrate.as_deref(),
                        audio_stream,
                        select.as_deref(),
                    )
                },
            )?
        }
        VideoAction::Trim {
            path,
            stdin,
            paths_file,
            start,
            duration,
            to,
            out,
            format,
            video_codec,
            audio_codec,
            select,
        } => {
            let inputs = video_inputs(path, stdin, paths_file)?;
            crate::commands::media::run_producing(
                "video",
                inputs,
                out.as_deref(),
                // Trim keeps the container it was handed, so absent `--format`
                // the target is a property of each input, not of the run.
                |input| match format.as_deref() {
                    Some(f) => f.to_ascii_lowercase(),
                    None => crate::commands::media::input_ext(
                        input,
                        &crate::xdg::resolve_video_default_container(),
                    ),
                },
                |src, dest| {
                    video_local::trim(
                        src,
                        start,
                        duration,
                        to,
                        dest,
                        format.as_deref(),
                        video_codec.as_deref(),
                        audio_codec.as_deref(),
                        select.as_deref(),
                    )
                },
            )?
        }
        VideoAction::Thumbnail {
            path,
            stdin,
            paths_file,
            at,
            out,
            select,
        } => {
            let inputs = video_inputs(path, stdin, paths_file)?;
            crate::commands::media::run_producing(
                "video",
                inputs,
                out.as_deref(),
                // A frame is an image; the single-input default is `.png` too.
                |_| "png".to_string(),
                |src, dest| video_local::thumbnail(src, at, dest, select.as_deref()),
            )?
        }
        VideoAction::Manifest {
            path,
            stdin,
            paths_file,
            base_url,
            select,
        } => {
            let inputs = video_inputs(path, stdin, paths_file)?;
            crate::commands::media::run("video", inputs, |src| {
                // Read under the manifest cap, not the video one: a playlist is
                // a small text document and never needs a temp file on disk.
                let body = src.load_bytes(crate::xdg::resolve_manifest_max_bytes())?;
                let parsed = video_local::parse_manifest(&body, base_url.as_deref())?;
                Ok(video_local::project_fields(parsed, select.as_deref()))
            })?
        }
    };
    emit_ok(data, json, |d| {
        let action = d.get("action").and_then(|v| v.as_str()).unwrap_or("video");
        // A manifest envelope has no path: it is parsed from bytes and writes
        // nothing. Printing `path=` empty would read as a missing file.
        if action == "manifest" {
            let kind = d.get("kind").and_then(|v| v.as_str()).unwrap_or("unknown");
            let mut line = format!("ok video action=manifest kind={kind}");
            for (label, key) in [
                ("variants", "variant_count"),
                ("segments", "segment_count"),
                ("representations", "representation_count"),
            ] {
                if let Some(n) = d.get(key).and_then(serde_json::Value::as_u64) {
                    line.push_str(&format!(" {label}={n}"));
                }
            }
            return crate::output::writeln_stdout(line);
        }
        let path = d
            .get("path")
            .or_else(|| d.get("path_out"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let mut line = format!("ok video action={action} path={path}");
        if let Some(ar) = d.get("auto_reencoded").and_then(|v| v.as_bool()) {
            line.push_str(&format!(" auto_reencoded={ar}"));
        }
        if let Some(b) = d.get("bytes_out").and_then(|v| v.as_u64()) {
            line.push_str(&format!(" bytes={b}"));
        }
        crate::output::writeln_stdout(line)?;
        Ok(())
    })
}
