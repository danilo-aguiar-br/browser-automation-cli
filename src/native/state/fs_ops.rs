// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::validation::sanitize_session_component;

use super::crypto::decrypt_data;
use super::types::StorageState;

pub(crate) fn is_state_file(path: &std::path::Path) -> bool {
    let fname = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    fname.ends_with(".json")
        || fname.ends_with(".json.enc")
        || fname.ends_with(".json.previous")
        || fname.ends_with(".json.enc.previous")
}

pub(crate) fn is_encrypted_state(path: &std::path::Path) -> bool {
    let path = path.to_string_lossy();
    path.ends_with(".json.enc") || path.ends_with(".json.enc.previous")
}

/// List saved state files with their metadata.
///
/// # Errors
///
/// Fails with `"Failed to read sessions dir: …"` when the state directory
/// exists but cannot be read. A directory that does not exist is not an
/// error: it answers with an empty list, because "no sessions yet" is a
/// legitimate state. Per-file metadata failures are absorbed as zeroes.
pub fn state_list() -> Result<Value, String> {
    let dir = get_sessions_dir();
    if !dir.exists() {
        return Ok(json!({ "files": [], "directory": dir.to_string_lossy() }));
    }

    let mut files = Vec::new();

    let entries = fs::read_dir(&dir).map_err(|e| format!("Failed to read sessions dir: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if is_state_file(&path) {
            let metadata = fs::metadata(&path).ok();
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let encrypted = is_encrypted_state(&path);

            files.push(json!({
                "filename": filename,
                "path": path.to_string_lossy(),
                "size": size,
                "modified": modified,
                "encrypted": encrypted,
            }));
        }
    }

    Ok(json!({ "files": files, "directory": dir.to_string_lossy() }))
}

/// Summarise one state file without loading it into a browser.
///
/// Reports counts and origins, never the cookie or storage VALUES: showing a
/// state file must not print the credentials it exists to carry.
///
/// # Errors
///
/// For an encrypted file (`.json.enc`), fails with
/// `"Encrypted state file requires config set encryption_key (XDG config)"`
/// when no key is configured, with the decryption error when the key is wrong
/// or the file is corrupt, and with `"Decrypted state is not valid UTF-8: …"`
/// when the plaintext is not text.
///
/// For either kind, fails with `"Failed to read state file: …"` on I/O
/// failure and `"Invalid state file: …"` when the JSON does not deserialize
/// into a `StorageState`.
pub fn state_show(path: &str) -> Result<Value, String> {
    let encrypted = is_encrypted_state(std::path::Path::new(path));
    let json_str = if encrypted {
        let key = crate::xdg::encryption_key().ok_or_else(|| {
            "Encrypted state file requires config set encryption_key (XDG config)".to_string()
        })?;
        let data = fs::read(path).map_err(|e| format!("Failed to read state file: {e}"))?;
        let decrypted = decrypt_data(&data, &key)?;
        String::from_utf8(decrypted)
            .map_err(|e| format!("Decrypted state is not valid UTF-8: {e}"))?
    } else {
        fs::read_to_string(path).map_err(|e| format!("Failed to read state file: {e}"))?
    };

    let state: StorageState =
        crate::json_util::from_str(&json_str).map_err(|e| format!("Invalid state file: {e}"))?;

    let metadata = fs::metadata(path).ok();
    let filename = std::path::Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    Ok(json!({
        "filename": filename,
        "path": path,
        "size": metadata.as_ref().map(|m| m.len()).unwrap_or(0),
        "modified": metadata.as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0),
        "encrypted": encrypted,
        "summary": format!("{} cookies, {} origins", state.cookies.len(), state.origins.len()),
        "state": state,
    }))
}

/// Delete one state file, or every one when `path` is `None`.
///
/// # Errors
///
/// Fails with `"Failed to delete state: …"` only when `path` is `Some` and
/// that file cannot be removed — missing, or permission denied. The
/// delete-everything form cannot fail: a missing directory answers
/// `deleted: 0`, an unreadable one is skipped, and each individual removal is
/// best-effort, so the reported count can exceed the files actually gone.
pub fn state_clear(path: Option<&str>) -> Result<Value, String> {
    if let Some(p) = path {
        fs::remove_file(p).map_err(|e| format!("Failed to delete state: {e}"))?;
        return Ok(json!({ "deleted": p }));
    }

    let dir = get_sessions_dir();
    if !dir.exists() {
        return Ok(json!({ "deleted": 0 }));
    }

    let mut count = 0;
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_state_file(&path) {
                let _ = fs::remove_file(&path);
                count += 1;
            }
        }
    }

    Ok(json!({ "deleted": count }))
}

/// Delete state files older than `max_age_days`.
///
/// # Errors
///
/// Never returns `Err`. A missing state directory answers `cleaned: 0`, an
/// unreadable one is skipped, and each removal is best-effort — a file whose
/// metadata or deletion fails is counted as kept. The `Result` is kept so
/// this shares the dispatch signature of its neighbours.
pub fn state_clean(max_age_days: u64) -> Result<Value, String> {
    let dir = get_sessions_dir();
    if !dir.exists() {
        return Ok(json!({ "cleaned": 0, "keptCount": 0, "days": max_age_days }));
    }

    let now = std::time::SystemTime::now();
    let max_age = std::time::Duration::from_secs(max_age_days * 86400);
    let mut deleted = 0;
    let mut kept = 0;

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_state_file(&path) {
                continue;
            }

            if let Ok(metadata) = fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > max_age {
                            let _ = fs::remove_file(&path);
                            deleted += 1;
                            continue;
                        }
                    }
                }
            }
            kept += 1;
        }
    }

    Ok(json!({ "cleaned": deleted, "keptCount": kept, "days": max_age_days }))
}

/// Rename a state file, keeping it inside the state directory.
///
/// # Errors
///
/// Fails with `"State file not found: <path>"` when `old_path` does not
/// exist, and with `"Failed to rename state: …"` when the rename is refused —
/// permission denied, or a cross-device move. An existing destination is
/// **not** an error: `fs::rename` overwrites it, so a `new_name` that is
/// already taken silently replaces that state file.
pub fn state_rename(old_path: &str, new_name: &str) -> Result<Value, String> {
    let old = PathBuf::from(old_path);
    if !old.exists() {
        return Err(format!("State file not found: {old_path}"));
    }

    let fallback = PathBuf::from(".");
    let dir = old.parent().unwrap_or(&fallback);
    let new_path = dir.join(format!("{new_name}.json"));

    fs::rename(&old, &new_path).map_err(|e| format!("Failed to rename state: {e}"))?;

    Ok(json!({
        "renamed": true,
        "from": old_path,
        "to": new_path.to_string_lossy(),
    }))
}

/// Directory holding saved state files, under XDG state.
pub fn get_state_dir() -> PathBuf {
    // XDG only — no product state under OS temp (anti-hardcode path law).
    let Ok(base) = crate::xdg::state_dir() else {
        return PathBuf::from("state-unconfigured");
    };

    if let Ok(cfg) = crate::xdg::load_config() {
        if let Some(namespace) = cfg.namespace {
            let namespace = sanitize_session_component(&namespace);
            if !namespace.is_empty() {
                return base.join("namespaces").join(namespace);
            }
        }
    }

    base
}

/// Directory holding named sessions, under XDG state.
pub fn get_sessions_dir() -> PathBuf {
    get_state_dir().join("sessions")
}
