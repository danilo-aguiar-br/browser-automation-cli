// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;

use serde_json::Value;

use crate::validation::is_valid_session_name;

use super::fs_ops::{
    get_sessions_dir, state_clean, state_clear, state_list, state_rename, state_show,
};

/// Locate the auto-saved state file for a named session, if one exists.
pub fn find_auto_state_file(session_name: &str) -> Option<String> {
    if !is_valid_session_name(session_name) {
        return None;
    }

    let dir = get_sessions_dir();
    if !dir.exists() {
        return None;
    }
    let prefix = format!("{session_name}-");
    let mut best_path: Option<(String, std::time::SystemTime)> = None;

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let fname = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let is_match = fname.starts_with(&prefix)
                && (fname.ends_with(".json") || fname.ends_with(".json.enc"));
            if !is_match {
                continue;
            }
            let modified = fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::UNIX_EPOCH);
            if best_path.as_ref().is_none_or(|(_, t)| modified > *t) {
                best_path = Some((path.to_string_lossy().to_string(), modified));
            }
        }
    }
    best_path.map(|(p, _)| p)
}

/// Dispatch a state management command from its JSON payload.
/// Returns `Some(result)` for recognised state_* actions, `None` otherwise.
pub fn dispatch_state_command(cmd: &Value) -> Option<Result<Value, String>> {
    let action = cmd.get("action").and_then(|v| v.as_str())?;
    match action {
        "state_list" => Some(state_list()),
        "state_show" => Some(
            cmd.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'path' parameter".to_string())
                .and_then(state_show),
        ),
        "state_clear" => {
            let path = cmd.get("path").and_then(|v| v.as_str());
            Some(state_clear(path))
        }
        "state_clean" => {
            let days = cmd.get("days").and_then(|v| v.as_u64()).unwrap_or(30);
            Some(state_clean(days))
        }
        "state_rename" => Some(
            cmd.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'path' parameter".to_string())
                .and_then(|path| {
                    cmd.get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing 'name' parameter".to_string())
                        .and_then(|name| state_rename(path, name))
                }),
        ),
        _ => None,
    }
}
