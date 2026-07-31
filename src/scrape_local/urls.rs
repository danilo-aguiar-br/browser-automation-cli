// SPDX-License-Identifier: MIT OR Apache-2.0
//! URL list file helpers.

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::error::{CliError, ErrorKind};

/// Read URLs file (one URL per line, # comments).
pub fn read_urls_file(path: &Path) -> Result<Vec<String>, CliError> {
    let raw = fs::read_to_string(path).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("read urls file {}: {e}", path.display()),
        )
    })?;
    let mut urls = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        urls.push(line.to_string());
    }
    if urls.is_empty() {
        return Err(CliError::new(ErrorKind::Usage, "urls file has no URLs"));
    }
    Ok(urls)
}

/// Stable sorted map helper for tests.
#[allow(dead_code)]
pub fn sorted_keys(v: &Value) -> Vec<String> {
    v.as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}
