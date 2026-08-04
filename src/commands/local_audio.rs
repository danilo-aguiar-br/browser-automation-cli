// SPDX-License-Identifier: MIT OR Apache-2.0
//! Dispatch handlers for `audio` (local media; no Chrome).

use crate::audio_local::{self, AudioSource, ConvertOpts};
use crate::cli::AudioAction;
use crate::commands::common::emit_ok;
use crate::error::{CliError, ErrorKind};

fn audio_source(path: Option<std::path::PathBuf>, stdin: bool) -> Result<AudioSource, CliError> {
    match (path, stdin) {
        (Some(p), false) => Ok(AudioSource::Path(p)),
        (None, true) => Ok(AudioSource::Stdin),
        (Some(_), true) => Err(CliError::new(
            ErrorKind::Usage,
            "audio: pass either --path or --stdin, not both",
        )),
        (None, false) => Err(CliError::new(
            ErrorKind::Usage,
            "audio: require --path or --stdin",
        )),
    }
}

/// Run an `audio` subcommand and write the agent envelope.
pub(crate) fn handle_audio(action: AudioAction, json: bool) -> Result<(), CliError> {
    let data = match action {
        AudioAction::Info {
            path,
            stdin,
            select,
        } => {
            let src = audio_source(path, stdin)?;
            audio_local::info(&src, select.as_deref())?
        }
        AudioAction::Download {
            url,
            out,
            max_bytes,
            require_audio,
            allow_non_audio,
            select,
        } => {
            let require = if allow_non_audio {
                false
            } else {
                require_audio
            };
            crate::runtime_util::block_on_io(audio_local::download(
                &url,
                out.as_deref(),
                max_bytes,
                require,
                select.as_deref(),
            ))?
        }
        AudioAction::Convert {
            path,
            stdin,
            format,
            out,
            codec,
            bitrate,
            sample_rate,
            channels,
            audio_stream,
            strip_metadata,
            select,
        } => {
            let src = audio_source(path, stdin)?;
            let fmt = format.unwrap_or_else(crate::xdg::resolve_audio_default_format);
            let container = audio_local::parse_output_format(&fmt)?;
            let br = bitrate.unwrap_or_else(crate::xdg::resolve_audio_default_bitrate);
            let opts = ConvertOpts {
                format: container,
                codec,
                bitrate: br,
                sample_rate,
                channels,
                audio_stream,
                strip_metadata,
            };
            audio_local::convert(&src, &fmt, out.as_deref(), opts, select.as_deref())?
        }
        AudioAction::Trim {
            path,
            stdin,
            start,
            duration,
            to,
            out,
            format,
            codec,
            bitrate,
            select,
        } => {
            let src = audio_source(path, stdin)?;
            audio_local::trim(
                &src,
                start,
                duration,
                to,
                out.as_deref(),
                format.as_deref(),
                codec.as_deref(),
                bitrate.as_deref(),
                select.as_deref(),
            )?
        }
    };
    emit_ok(data, json, |d| {
        let action = d.get("action").and_then(|v| v.as_str()).unwrap_or("audio");
        let path = d
            .get("path")
            .or_else(|| d.get("path_out"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let mut line = format!("ok audio action={action} path={path}");
        if let Some(ar) = d.get("auto_reencoded").and_then(|v| v.as_bool()) {
            line.push_str(&format!(" auto_reencoded={ar}"));
        }
        if let Some(lt) = d.get("lossy_transcode").and_then(|v| v.as_bool()) {
            line.push_str(&format!(" lossy_transcode={lt}"));
        }
        if let Some(b) = d.get("bytes_out").and_then(|v| v.as_u64()) {
            line.push_str(&format!(" bytes={b}"));
        } else if let Some(b) = d.get("bytes").and_then(|v| v.as_u64()) {
            line.push_str(&format!(" bytes={b}"));
        }
        crate::output::writeln_stdout(line)?;
        Ok(())
    })
}
