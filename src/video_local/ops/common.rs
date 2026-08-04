// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared helpers for video ops envelopes.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::super::ffmpeg_ops::ConvertResult;
use super::super::magic::DetectedContainer;
use crate::error::CliError;
use crate::xdg;

pub(super) fn extension_hint(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
}

pub(super) fn extension_mismatch(container: DetectedContainer, ext: Option<&str>) -> bool {
    let Some(ext) = ext else {
        return false;
    };
    match container {
        DetectedContainer::Mp4 | DetectedContainer::IsoBmffUnknown => {
            !matches!(ext, "mp4" | "m4v" | "mov" | "m4a")
        }
        DetectedContainer::Mov => !matches!(ext, "mov" | "qt"),
        DetectedContainer::M4v => ext != "m4v",
        DetectedContainer::Avi => ext != "avi",
        DetectedContainer::MatroskaOrWebm => !matches!(ext, "mkv" | "webm" | "mka"),
        DetectedContainer::MpegPs => !matches!(ext, "mpg" | "mpeg" | "vob"),
        DetectedContainer::Asf => !matches!(ext, "wmv" | "asf"),
    }
}

pub(super) fn sha256_path_head(path: &Path, _max: u64) -> Result<(String, u64), CliError> {
    use std::io::Read;
    let meta =
        std::fs::metadata(path).map_err(|e| crate::video_local::magic::io_open_err(path, &e))?;
    let len = meta.len();
    let mut f =
        std::fs::File::open(path).map_err(|e| crate::video_local::magic::io_open_err(path, &e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| crate::video_local::magic::io_open_err(path, &e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok((format!("{:x}", hasher.finalize()), len))
}

pub(super) fn default_out_path(ext: &str) -> PathBuf {
    let dir = xdg::cache_dir().unwrap_or_else(|_| PathBuf::from("."));
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    dir.join(format!("video-{stamp}.{ext}"))
}

pub(super) fn convert_envelope(
    action: &str,
    path_in: &Path,
    container_in: &str,
    container_out: &str,
    drop_audio: bool,
    res: &ConvertResult,
) -> Value {
    // Agent-native CLEAN STDOUT: omit Option keys when None (never emit JSON null).
    let mut map = serde_json::Map::new();
    map.insert("action".into(), json!(action));
    map.insert("path_in".into(), json!(path_in.display().to_string()));
    map.insert("path_out".into(), json!(res.path_out.display().to_string()));
    map.insert("container_in".into(), json!(container_in));
    map.insert("container_out".into(), json!(container_out));
    map.insert("stream_copy".into(), json!(res.stream_copy));
    map.insert("auto_reencoded".into(), json!(res.auto_reencoded));
    if let Some(ref reason) = res.reencode_reason {
        map.insert("reencode_reason".into(), json!(reason));
    }
    map.insert("video_codec".into(), json!(res.video_codec));
    map.insert("audio_codec".into(), json!(res.audio_codec));
    map.insert(
        "tracks_mapped".into(),
        json!(if drop_audio { "video_only" } else { "all" }),
    );
    map.insert("drop_audio".into(), json!(drop_audio));
    map.insert("faststart_applied".into(), json!(res.faststart_applied));
    map.insert("metadata_stripped".into(), json!(res.metadata_stripped));
    if let Some(d) = res.duration_secs {
        map.insert("duration_secs".into(), json!(d));
    }
    map.insert("bytes_out".into(), json!(res.bytes_out));
    map.insert("sha256_out".into(), json!(res.sha256_out));
    map.insert("engine".into(), json!("ffmpeg"));
    map.insert(
        "ffmpeg_timeout_secs".into(),
        json!(xdg::resolve_ffmpeg_timeout_secs()),
    );
    Value::Object(map)
}

/// Project video envelopes with agent-friendly select aliases.
///
/// Unknown keys are ignored; `action` is always retained.
/// Aliases resolve to the first present canonical key:
/// - `format` → `container` | `container_out`
/// - `bytes` | `size` → `size_bytes` | `bytes_out`
/// - `duration` → `duration_secs`
/// - `path` → `path` | `path_out` (when only out path exists)
pub(crate) fn project_fields(value: Value, select: Option<&str>) -> Value {
    let Some(sel) = select.map(str::trim).filter(|s| !s.is_empty()) else {
        return value;
    };
    let Some(obj) = value.as_object() else {
        return value;
    };
    let mut expanded = Vec::new();
    for key in sel.split([',', ' ']) {
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        if obj.contains_key(key) {
            expanded.push(key.to_string());
            continue;
        }
        let mapped = match key {
            "format" => first_present(obj, &["container", "container_out"]),
            "bytes" | "size" => first_present(obj, &["size_bytes", "bytes_out"]),
            "duration" => first_present(obj, &["duration_secs"]),
            "path" => first_present(obj, &["path", "path_out"]),
            "format_in" => first_present(obj, &["container_in"]),
            "format_out" => first_present(obj, &["container_out"]),
            _ => None,
        };
        if let Some(m) = mapped {
            expanded.push(m.to_string());
        } else {
            expanded.push(key.to_string());
        }
    }
    let joined = expanded.join(",");
    crate::json_util::project_fields_plain(value, Some(joined.as_str()))
}

fn first_present<'a>(obj: &serde_json::Map<String, Value>, keys: &[&'a str]) -> Option<&'a str> {
    keys.iter().copied().find(|k| obj.contains_key(*k))
}
