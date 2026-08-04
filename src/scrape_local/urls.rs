// SPDX-License-Identifier: MIT OR Apache-2.0
//! URL list file helpers.

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::error::{CliError, ErrorKind};

/// Read URLs file (one URL per line, # comments).
///
/// Checks metadata size before reading: this list is user-supplied input to
/// `batch-scrape`, and it was the one reader in the product without a ceiling
/// while `sg`, local parse and JSON manifests all had one. An unbounded
/// `read_to_string` on external input is how a one-shot process gets OOM-killed
/// by a file it was only asked to look at.
///
/// # Errors
///
/// [`ErrorKind::Usage`] when the file exceeds `max_urls_file_bytes` or holds no
/// URLs; [`ErrorKind::Io`] when it cannot be read.
pub fn read_urls_file(path: &Path) -> Result<Vec<String>, CliError> {
    let max = crate::xdg::policy::policy_u64(crate::xdg::policy::key::MAX_URLS_FILE_BYTES);
    let meta = fs::metadata(path).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("stat urls file {}: {e}", path.display()),
        )
    })?;
    if meta.len() > max {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!(
                "urls file {} is {} bytes, over the {max} byte ceiling",
                path.display(),
                meta.len()
            ),
            crate::i18n::suggestion_key("urls_file_too_large", None),
        ));
    }
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
