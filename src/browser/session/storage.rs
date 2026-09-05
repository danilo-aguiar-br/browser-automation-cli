// SPDX-License-Identifier: MIT OR Apache-2.0
//! Portable auth state: cookies + localStorage + sessionStorage (GAP-034 pillar 1).
//!
//! # Why this exists
//!
//! The one-shot model drops the browser at DIE, so every script has to repeat
//! its authentication prefix. Exporting the storage state and importing it in
//! the next invocation removes that prefix without introducing a resident
//! service: the process count is unchanged, only the work is shorter.
//!
//! # Contract
//!
//! | Rule | Reason |
//! |------|--------|
//! | `--path` is required | an implicit XDG file is how GAP-009 leaked across invocations |
//! | path obeys [`crate::fs_roots`] | same containment as any other artifact (GAP-026) |
//! | file mode `0600` on unix | cookies and tokens are secrets |
//!
//! The payload is the existing [`crate::native::state`] `StorageState` shape,
//! so a file written here is readable by the state tooling and vice versa.

use std::path::Path;

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::OneShotSession;

impl OneShotSession {
    /// Export cookies and per-origin storage to an explicit path.
    ///
    /// # Errors
    ///
    /// Fails when `path` falls outside the roots
    /// [`crate::fs_roots`] allows, and when the session has no active page.
    ///
    /// Fails with [`ErrorKind::Browser`] —
    /// `"storage export failed: …"`, carrying the `navigate_first`
    /// suggestion — when cookies cannot be read or the file cannot be written,
    /// and with [`ErrorKind::Io`] when the
    /// written file cannot be restricted to mode `0600`. That last failure is
    /// surfaced rather than warned about: the artifact holds cookies and
    /// tokens, so a world-readable one is not an acceptable success.
    ///
    /// Also propagates the summary read of the file just written — a state
    /// file that cannot be re-read or exceeds the `max_json_file_bytes`
    /// ceiling.
    pub async fn storage_export(&mut self, path: &Path) -> Result<Value, CliError> {
        let target = crate::fs_roots::ensure_write_allowed(path)?;
        let session_id = self.session_id()?;
        // save_state merges this with the live frame-tree origins, so an empty
        // set still captures every origin the current page reaches.
        let visited = rustc_hash::FxHashSet::default();
        let written = crate::native::state::save_state(
            &self.manager.client,
            &session_id,
            Some(&target.display().to_string()),
            None,
            "storage-export",
            &visited,
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Browser,
                format!("storage export failed: {e}"),
                crate::i18n::suggestion_key("navigate_first", None),
            )
        })?;

        restrict_to_owner(Path::new(&written))?;
        let summary = summarize(Path::new(&written))?;
        Ok(json!({
            "ok": true,
            "path": written,
            "cookies": summary.0,
            "origins": summary.1,
            "mode": "0600",
        }))
    }

    /// Import cookies and per-origin storage from an explicit path.
    ///
    /// # Errors
    ///
    /// Fails when `path` falls outside the roots
    /// [`crate::fs_roots`] allows, with
    /// [`ErrorKind::NoInput`] —
    /// `"storage file not found: …"` — when it is missing or is a directory,
    /// and when the session has no active page.
    ///
    /// Fails with [`ErrorKind::Data`] —
    /// `"storage import failed: …"` — when the file is not a readable
    /// `StorageState`, when an encrypted state has no configured key, or when
    /// a per-origin navigation is refused.
    ///
    /// A partial restore is possible without an error: individual
    /// `localStorage` / `sessionStorage` writes are best-effort, so an origin
    /// that refuses storage is skipped silently. Call
    /// [`storage_live_counts`](Self::storage_live_counts) to see what actually
    /// landed.
    pub async fn storage_import(&mut self, path: &Path) -> Result<Value, CliError> {
        let source = crate::fs_roots::ensure_read_allowed(path)?;
        if !source.is_file() {
            return Err(CliError::with_suggestion(
                ErrorKind::NoInput,
                format!("storage file not found: {}", source.display()),
                crate::i18n::suggestion_key("file_path_invalid", None),
            ));
        }
        let session_id = self.session_id()?;
        let summary = summarize(&source)?;
        crate::native::state::load_state(
            &self.manager.client,
            &session_id,
            &source.display().to_string(),
        )
        .await
        .map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Data,
                format!("storage import failed: {e}"),
                crate::i18n::suggestion_key("file_path_invalid", None),
            )
        })?;
        Ok(json!({
            "ok": true,
            "path": source.display().to_string(),
            "cookies": summary.0,
            "origins": summary.1,
        }))
    }

    /// Count storage entries currently live on the page.
    ///
    /// Reported after `storage import --url` so the caller sees that the state
    /// actually landed, instead of trusting that the write succeeded.
    ///
    /// # Errors
    ///
    /// Propagates [`eval`](Self::eval): no active page, a refused
    /// `Runtime.evaluate`, or a JavaScript exception — which is what an origin
    /// that denies storage access raises when `localStorage` is touched.
    pub async fn storage_live_counts(&mut self) -> Result<Value, CliError> {
        let js = r#"(() => ({
            localStorage: localStorage.length,
            sessionStorage: sessionStorage.length,
            cookies: document.cookie ? document.cookie.split(';').filter(s => s.trim()).length : 0
        }))()"#;
        self.eval(js, None, None, None).await
    }
}

/// Owner-only permission for a file holding session secrets.
fn restrict_to_owner(path: &Path) -> Result<(), CliError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|e| {
            CliError::new(
                ErrorKind::Io,
                format!("cannot restrict {} to owner: {e}", path.display()),
            )
        })?;
    }
    #[cfg(not(unix))]
    {
        // Windows inherits the parent ACL; the artifact still lives under an
        // allowed root, which is the portable half of the guarantee.
        let _ = path;
    }
    Ok(())
}

/// Count cookies and origins in a state file without re-parsing at call sites.
fn summarize(path: &Path) -> Result<(usize, usize), CliError> {
    let value: Value =
        crate::json_util::read_json_value_file(path, crate::xdg::resolve_max_json_file_bytes())
            .map_err(|e| {
                CliError::new(
                    e.kind(),
                    format!("storage state {}: {}", path.display(), e.message()),
                )
            })?;
    let cookies = value
        .get("cookies")
        .and_then(|c| c.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let origins = value
        .get("origins")
        .and_then(|o| o.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    Ok((cookies, origins))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_reads_counts() {
        let tmp = tempfile::Builder::new()
            .prefix("bac-storage-")
            .tempdir()
            .expect("scratch dir");
        let path = tmp.path().join("state.json");
        std::fs::write(
            &path,
            br#"{"cookies":[{"name":"a"},{"name":"b"}],"origins":[{"origin":"https://x"}]}"#,
        )
        .expect("write");
        assert_eq!(summarize(&path).expect("summary"), (2, 1));
    }

    #[test]
    fn summarize_tolerates_missing_sections() {
        let tmp = tempfile::Builder::new()
            .prefix("bac-storage-")
            .tempdir()
            .expect("scratch dir");
        let path = tmp.path().join("empty.json");
        std::fs::write(&path, b"{}").expect("write");
        assert_eq!(summarize(&path).expect("summary"), (0, 0));
    }

    #[cfg(unix)]
    #[test]
    fn restrict_to_owner_sets_0600() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::Builder::new()
            .prefix("bac-storage-")
            .tempdir()
            .expect("scratch dir");
        let path = tmp.path().join("secret.json");
        std::fs::write(&path, b"{}").expect("write");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("chmod");
        restrict_to_owner(&path).expect("restrict");
        let mode = std::fs::metadata(&path).expect("meta").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "state file must be owner-only");
    }
}
