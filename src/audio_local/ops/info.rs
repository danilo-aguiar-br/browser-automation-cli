// SPDX-License-Identifier: MIT OR Apache-2.0
//! `audio info` probe envelope.

use serde_json::{json, Value};

use super::super::limits::AudioLimits;
use super::super::magic::probe_path_magic;
use super::common::{extension_hint, extension_mismatch, sha256_path_head};
use super::source::AudioSource;
use crate::error::{CliError, ErrorKind};
use crate::video_local::{compact_streams, duration_secs, primary_audio_codec, probe_path};

/// Probe container + audio streams (ffprobe when available).
pub fn info(source: &AudioSource, select: Option<&str>) -> Result<Value, CliError> {
    let limits = AudioLimits::from_xdg();
    let (path, _temp) = source.resolve_path(limits)?;
    let container = probe_path_magic(&path)?;
    let ext = extension_hint(&path);
    let mismatch = extension_mismatch(container, ext.as_deref());
    let (sha, size) = sha256_path_head(&path, limits.max_input_bytes as u64)?;

    let mut engine = "magic";
    let mut streams = json!([]);
    let mut duration = Value::Null;
    let mut codec = Value::Null;
    let mut channels = Value::Null;
    let mut sample_rate = Value::Null;
    let mut probe_note: Option<String> = None;

    match probe_path(&path) {
        Ok(raw) => {
            engine = "ffprobe";
            streams = compact_streams(&raw);
            duration = duration_secs(&raw).map(|d| json!(d)).unwrap_or(Value::Null);
            codec = primary_audio_codec(&raw)
                .map(|c| json!(c))
                .unwrap_or(Value::Null);
            if let Some(arr) = raw.get("streams").and_then(|s| s.as_array()) {
                if let Some(a) = arr
                    .iter()
                    .find(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("audio"))
                {
                    if let Some(ch) = a.get("channels") {
                        channels = ch.clone();
                    }
                    if let Some(sr) = a.get("sample_rate") {
                        sample_rate = sr.clone();
                    }
                }
            }
        }
        Err(e) if e.kind() == ErrorKind::Unavailable => {
            probe_note = Some(e.message().to_string());
        }
        Err(e) => {
            probe_note = Some(e.message().to_string());
        }
    }

    let mut map = serde_json::Map::new();
    map.insert("action".into(), json!("info"));
    map.insert("path".into(), json!(path.display().to_string()));
    map.insert("container".into(), json!(container.as_str()));
    map.insert("magic_ok".into(), json!(true));
    map.insert("extension".into(), json!(ext));
    map.insert("extension_mismatch".into(), json!(mismatch));
    map.insert("size_bytes".into(), json!(size));
    map.insert("sha256".into(), json!(sha));
    map.insert("duration_secs".into(), duration);
    map.insert("audio_codec".into(), codec);
    map.insert("channels".into(), channels);
    map.insert("sample_rate".into(), sample_rate);
    map.insert("streams".into(), streams);
    map.insert("engine".into(), json!(engine));
    if let Some(note) = probe_note {
        map.insert("probe_note".into(), json!(note));
    }
    let full = Value::Object(map);
    Ok(super::common::project_fields(full, select))
}
