// SPDX-License-Identifier: MIT OR Apache-2.0
//! ffprobe JSON metadata (path-only; no full media load in-process).

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use serde_json::{json, Value};

use super::ffmpeg_bin::require_ffprobe;
use crate::error::{CliError, ErrorKind};
use crate::platform::{run_capture_with_timeout, ProcessCaptureError};

/// Run ffprobe and return parsed JSON (`format` + `streams`).
pub fn probe_path(path: &Path) -> Result<Value, CliError> {
    let bin = require_ffprobe()?;
    let timeout = Duration::from_secs(crate::xdg::resolve_ffmpeg_timeout_secs());
    let path_owned = path.to_path_buf();
    let out = run_capture_with_timeout(
        Command::new(&bin)
            .arg("-v")
            .arg("quiet")
            .arg("-print_format")
            .arg("json")
            .arg("-show_format")
            .arg("-show_streams")
            .arg(&path_owned),
        timeout,
    )
    .map_err(|e| match e {
        ProcessCaptureError::Timeout => CliError::with_suggestion(
            ErrorKind::Timeout,
            format!(
                "ffprobe timed out after {}s",
                crate::xdg::resolve_ffmpeg_timeout_secs()
            ),
            crate::i18n::suggestion_key("ffmpeg_timeout", None),
        ),
        other => CliError::new(ErrorKind::Unavailable, format!("ffprobe spawn: {other}")),
    })?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(CliError::new(
            ErrorKind::Data,
            format!("ffprobe failed: {}", err.trim()),
        ));
    }
    let raw = String::from_utf8_lossy(&out.stdout);
    crate::json_util::value_from_str(&raw)
        .map_err(|e| CliError::new(ErrorKind::Data, format!("ffprobe JSON parse: {e}")))
}

/// Compact agent-facing stream list from raw ffprobe JSON.
pub fn compact_streams(probe: &Value) -> Value {
    let mut streams = Vec::new();
    if let Some(arr) = probe.get("streams").and_then(|s| s.as_array()) {
        for s in arr {
            let codec_type = s
                .get("codec_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let codec = s
                .get("codec_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let mut entry = json!({
                "index": s.get("index").cloned().unwrap_or(json!(null)),
                "type": codec_type,
                "codec": codec,
            });
            if codec_type == "video" {
                if let Some(w) = s.get("width") {
                    entry["width"] = w.clone();
                }
                if let Some(h) = s.get("height") {
                    entry["height"] = h.clone();
                }
                if let Some(r) = s.get("r_frame_rate").and_then(|v| v.as_str()) {
                    entry["fps"] = json!(parse_rate(r));
                }
                if let Some(p) = s.get("profile") {
                    entry["profile"] = p.clone();
                }
                if let Some(l) = s.get("level") {
                    entry["level"] = l.clone();
                }
            }
            if codec_type == "audio" {
                if let Some(ch) = s.get("channels") {
                    entry["channels"] = ch.clone();
                }
                if let Some(sr) = s.get("sample_rate") {
                    entry["sample_rate"] = sr.clone();
                }
            }
            streams.push(entry);
        }
    }
    Value::Array(streams)
}

/// Duration seconds from ffprobe format section when present.
pub fn duration_secs(probe: &Value) -> Option<f64> {
    probe
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|s| s.parse().ok())
        .or_else(|| {
            probe
                .get("format")
                .and_then(|f| f.get("duration"))
                .and_then(|d| d.as_f64())
        })
}

/// First video stream codec_name from ffprobe JSON.
#[must_use]
pub fn primary_video_codec(probe: &Value) -> Option<String> {
    probe
        .get("streams")
        .and_then(|s| s.as_array())
        .into_iter()
        .flatten()
        .find(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("video"))
        .and_then(|s| {
            s.get("codec_name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
}

/// First audio stream codec_name from ffprobe JSON.
#[must_use]
pub fn primary_audio_codec(probe: &Value) -> Option<String> {
    probe
        .get("streams")
        .and_then(|s| s.as_array())
        .into_iter()
        .flatten()
        .find(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("audio"))
        .and_then(|s| {
            s.get("codec_name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
}

fn parse_rate(r: &str) -> Option<f64> {
    let mut parts = r.split('/');
    let n: f64 = parts.next()?.parse().ok()?;
    let d: f64 = parts.next().unwrap_or("1").parse().ok()?;
    if d == 0.0 {
        None
    } else {
        Some(n / d)
    }
}
