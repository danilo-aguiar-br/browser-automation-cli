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
/// [`crate::fs_roots::ensure_read_allowed`] when `path` falls outside the
/// allowed roots (GAP-026); [`ErrorKind::Usage`] when the file exceeds
/// `max_urls_file_bytes` or holds no URLs; [`ErrorKind::Io`] when it cannot be
/// read.
pub fn read_urls_file(path: &Path) -> Result<Vec<String>, CliError> {
    // GAP-026, read axis. The doc above already called this list "user-supplied
    // input", and the size ceiling exists for exactly that reason — but only the
    // SIZE axis was ever bounded, never the LOCATION one.
    //
    // MEASURED 2026-08-31: `batch-scrape --urls-file /etc/passwd` exited 0 with
    // `ok: true` and all 59 lines echoed back inside `data.errors[].error`,
    // because every unfetchable line is reported verbatim. That turns this flag
    // into a file-disclosure oracle for any readable path. The control that
    // proves the policy was active: `parse` refused the same path with exit 64.
    //
    // The check sits at entry, before `metadata`, so a refused path is never
    // even stat'd — the same ordering `write_bytes_atomic` uses on the write
    // axis. It belongs here rather than in `json_util`, because this is where an
    // operator-supplied path stops being trustworthy; the shared readers also
    // serve the product reading its own config, which lives outside the roots.
    crate::fs_roots::ensure_read_allowed(path)?;
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
