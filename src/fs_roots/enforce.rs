// SPDX-License-Identifier: MIT OR Apache-2.0
//! Canonicalisation and the allowed-root decision.

use std::path::{Path, PathBuf};

use crate::error::{CliError, ErrorKind};

use super::{effective_roots, PathUse};

/// Canonicalise `path`, tolerating a not-yet-created final component.
///
/// A write target usually does not exist yet, so the parent is canonicalised
/// and the file name re-appended. This still defeats `..` and symlinked
/// parents, which is the property the check depends on.
fn canonical_for_check(path: &Path) -> Result<PathBuf, CliError> {
    if let Ok(c) = path.canonicalize() {
        return Ok(c);
    }
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    let name = path.file_name();
    match (parent, name) {
        (Some(parent), Some(name)) => {
            let base = parent.canonicalize().map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::NoInput,
                    format!("cannot resolve parent directory of {}: {e}", path.display()),
                    crate::i18n::suggestion_key("file_path_invalid", None),
                )
            })?;
            Ok(base.join(name))
        }
        _ => Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!("cannot resolve path: {}", path.display()),
            crate::i18n::suggestion_key("file_path_invalid", None),
        )),
    }
}

/// True when `candidate` is inside `root` after both are canonical.
fn is_within(candidate: &Path, root: &Path) -> bool {
    let Ok(root) = root.canonicalize() else {
        return false;
    };
    candidate.starts_with(&root)
}

/// Enforce the allowed-root policy for one local path.
///
/// `allow_outside` comes from `--allow-outside-roots` and is the only way out.
pub fn ensure_within_roots(
    path: &Path,
    use_kind: PathUse,
    allow_outside: bool,
) -> Result<PathBuf, CliError> {
    let canonical = canonical_for_check(path)?;
    if allow_outside {
        tracing::warn!(
            target: "browser_automation_cli::fs_roots",
            path = %canonical.display(),
            use_kind = use_kind.as_str(),
            "allowed-root policy bypassed by --allow-outside-roots"
        );
        return Ok(canonical);
    }
    let roots = effective_roots();
    if roots.iter().any(|r| is_within(&canonical, r)) {
        return Ok(canonical);
    }
    // The argv is correct; the PATH is outside policy. Returning `usage` would
    // send the agent into an argv-correction loop that cannot converge, which
    // is the GAP-020 failure class. This is a policy refusal with a known
    // remediation, so it is `capability-disabled` (exit 64).
    Err(CliError::with_suggestion(
        ErrorKind::CapabilityDisabled,
        format!(
            "{} path outside allowed roots: {}",
            use_kind.as_str(),
            canonical.display()
        ),
        crate::i18n::suggestion_key("path_outside_roots", None),
    ))
}

/// Enforce the policy on the local path carried by a `file://` URL.
///
/// Non-`file:` URLs pass through untouched: this gate is only about the local
/// scheme, network policy lives in [`crate::robots`] and `crate::net::ssrf`.
pub fn ensure_file_url_allowed(url: &str, allow_outside: bool) -> Result<(), CliError> {
    let trimmed = url.trim();
    if !trimmed.to_ascii_lowercase().starts_with("file:") {
        return Ok(());
    }
    let parsed = url::Url::parse(trimmed).map_err(|e| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            format!("invalid file URL: {e}"),
            crate::i18n::suggestion_key("url_absolute_http", None),
        )
    })?;
    let path = parsed.to_file_path().map_err(|()| {
        CliError::with_suggestion(
            ErrorKind::Usage,
            format!("file URL has no local path: {trimmed}"),
            crate::i18n::suggestion_key("url_absolute_http", None),
        )
    })?;
    ensure_within_roots(&path, PathUse::Read, allow_outside)?;
    Ok(())
}
