// SPDX-License-Identifier: MIT OR Apache-2.0
//! Input validation helpers for CLI flags and paths.

use std::path::{Component, Path};

/// Windows reserved device names (case-insensitive, with or without extension).
///
/// See: https://learn.microsoft.com/windows/win32/fileio/naming-a-file
const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Check if a session name is valid (alphanumeric, hyphens, and underscores only).
pub fn is_valid_session_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Convert arbitrary caller-provided text into a valid session-name component.
pub fn sanitize_session_component(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_sep = false;

    for c in value.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            last_was_sep = false;
        } else if c == '-' || c == '_' {
            if !out.is_empty() && !last_was_sep {
                out.push(c);
                last_was_sep = true;
            }
        } else if !out.is_empty() && !last_was_sep {
            out.push('-');
            last_was_sep = true;
        }
    }

    while out.ends_with(['-', '_']) {
        out.pop();
    }

    out
}

/// Generate error message for invalid session name
pub fn session_name_error(name: &str) -> String {
    format!(
        "Invalid session name '{}'. Only alphanumeric characters, hyphens, and underscores are allowed.",
        name
    )
}

/// Reject path strings that attempt directory traversal via `..` components.
///
/// Absolute paths are allowed (caller decides root policy). Empty paths fail.
/// On all targets, also rejects Windows reserved device basenames (`CON`, `NUL`, …)
/// so agents cannot create unportable artifact names when sharing scripts cross-OS.
pub fn reject_path_traversal(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("path must not be empty".into());
    }
    for c in path.components() {
        if matches!(c, Component::ParentDir) {
            return Err(format!(
                "path traversal rejected (`..` component): {}",
                path.display()
            ));
        }
    }
    reject_windows_reserved_basename(path)?;
    Ok(())
}

/// Same as [`reject_path_traversal`] for stringy CLI inputs.
pub fn reject_path_traversal_str(path: &str) -> Result<(), String> {
    reject_path_traversal(Path::new(path))
}

/// Reject basenames that are Windows reserved device names (`NUL`, `COM1`, …).
///
/// Applied on every OS so path contracts stay portable (rules multiplataforma).
pub fn reject_windows_reserved_basename(path: &Path) -> Result<(), String> {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Ok(());
    };
    // Strip one extension: `nul.txt` and `CON` both reserved.
    let stem = name
        .split_once('.')
        .map(|(s, _)| s)
        .unwrap_or(name);
    let upper = stem.to_ascii_uppercase();
    if WINDOWS_RESERVED_NAMES.iter().any(|r| *r == upper) {
        return Err(format!(
            "Windows reserved device name rejected in path basename: {}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn sanitize_session_component_produces_valid_component() {
        let value = sanitize_session_component("Next Dev Loop: /Users/me/worktree!");

        assert_eq!(value, "next-dev-loop-users-me-worktree");
        assert!(is_valid_session_name(&value));
    }

    #[test]
    fn sanitize_session_component_trims_separators() {
        assert_eq!(sanitize_session_component(" --Agent__ "), "agent");
    }

    #[test]
    fn path_traversal_rejected() {
        assert!(reject_path_traversal(Path::new("../etc/passwd")).is_err());
        assert!(reject_path_traversal(Path::new("foo/../../bar")).is_err());
        assert!(reject_path_traversal(Path::new("safe/out.png")).is_ok());
        assert!(reject_path_traversal(Path::new("/abs/safe")).is_ok());
        assert!(reject_path_traversal_str("").is_err());
        let p = PathBuf::from("a/b/c");
        assert!(reject_path_traversal(&p).is_ok());
    }

    #[test]
    fn windows_reserved_basenames_rejected() {
        assert!(reject_path_traversal(Path::new("NUL")).is_err());
        assert!(reject_path_traversal(Path::new("nul.txt")).is_err());
        assert!(reject_path_traversal(Path::new("out/CON")).is_err());
        assert!(reject_path_traversal(Path::new("com1")).is_err());
        assert!(reject_path_traversal(Path::new("report.png")).is_ok());
        assert!(reject_windows_reserved_basename(Path::new("aux.log")).is_err());
    }
}
