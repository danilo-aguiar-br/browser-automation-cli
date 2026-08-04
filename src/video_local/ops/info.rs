// SPDX-License-Identifier: MIT OR Apache-2.0
//! `video info` probe envelope.

use serde_json::{json, Value};

use super::super::limits::VideoLimits;
use super::super::magic::probe_path_magic;
use super::super::probe::{compact_streams, duration_secs, probe_path};
use super::common::{extension_hint, extension_mismatch, sha256_path_head};
use super::source::VideoSource;
use crate::error::{CliError, ErrorKind};

/// Probe container + streams (ffprobe when available).
pub fn info(source: &VideoSource, select: Option<&str>) -> Result<Value, CliError> {
    let limits = VideoLimits::from_xdg();
    let (path, _temp) = source.resolve_path(limits)?;
    let container = probe_path_magic(&path)?;
    let ext = extension_hint(&path);
    let mismatch = extension_mismatch(container, ext.as_deref());
    let (sha, size) = sha256_path_head(&path, limits.max_input_bytes as u64)?;

    let mut engine = "magic";
    let mut streams = json!([]);
    let mut duration = Value::Null;
    let mut probe_note: Option<String> = None;

    match probe_path(&path) {
        Ok(raw) => {
            engine = "ffprobe";
            streams = compact_streams(&raw);
            duration = duration_secs(&raw).map(|d| json!(d)).unwrap_or(Value::Null);
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
    map.insert("streams".into(), streams);
    map.insert("engine".into(), json!(engine));
    if let Some(note) = probe_note {
        map.insert("probe_note".into(), json!(note));
    }
    let full = Value::Object(map);
    Ok(super::common::project_fields(full, select))
}
