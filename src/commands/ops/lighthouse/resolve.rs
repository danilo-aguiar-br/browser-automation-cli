// SPDX-License-Identifier: MIT OR Apache-2.0
//! External lighthouse binary resolution and spawn-safety checks.

use crate::error::{CliError, ErrorKind};
use std::path::Path;

/// Where the lighthouse binary was resolved from (agent-honest; GAP-A010 / LH-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LighthouseSource {
    /// Explicit `--lighthouse-path` flag.
    Flag,
    /// XDG `config set lighthouse_path`.
    Xdg,
    /// Found on PATH via which-equivalent.
    Path,
    /// Local e2e mock script (`mock-lighthouse`).
    Mock,
}

impl LighthouseSource {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::Xdg => "xdg",
            Self::Path => "path",
            Self::Mock => "mock",
        }
    }
}

fn ensure_spawn_safe_lighthouse(path: &Path) -> Result<(), CliError> {
    if crate::platform::is_spawn_safe_binary(path) {
        return Ok(());
    }
    if path.is_file() {
        return Err(CliError::with_suggestion(
            ErrorKind::Usage,
            format!(
                "lighthouse path is not a safe spawn binary (reject .bat/.cmd/.ps1): {}",
                path.display()
            ),
            crate::i18n::suggestion_key("binary_unsafe_windows", None),
        ));
    }
    Err(CliError::with_suggestion(
        ErrorKind::Usage,
        format!("lighthouse path not found: {}", path.display()),
        crate::i18n::suggestion_key("external_binary_path", None),
    ))
}

pub(crate) fn resolve_lighthouse_binary(
    cli_path: Option<&Path>,
) -> Result<(std::path::PathBuf, LighthouseSource), CliError> {
    if let Some(p) = cli_path {
        ensure_spawn_safe_lighthouse(p)?;
        let source = if p
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.contains("mock-lighthouse"))
        {
            LighthouseSource::Mock
        } else {
            LighthouseSource::Flag
        };
        return Ok((p.to_path_buf(), source));
    }
    if let Some(xdg) = crate::xdg::lighthouse_path_from_config().filter(|s| !s.is_empty()) {
        let p = Path::new(&xdg);
        ensure_spawn_safe_lighthouse(p)?;
        let source = if xdg.contains("mock-lighthouse") {
            LighthouseSource::Mock
        } else {
            LighthouseSource::Xdg
        };
        return Ok((p.to_path_buf(), source));
    }
    if let Some(p) = which_lighthouse() {
        let path = Path::new(&p).to_path_buf();
        ensure_spawn_safe_lighthouse(&path)?;
        return Ok((path, LighthouseSource::Path));
    }
    Err(CliError::with_suggestion(
        ErrorKind::Unavailable,
        "lighthouse binary not found on PATH or XDG lighthouse_path",
        crate::i18n::suggestion_key("lighthouse_missing", None),
    ))
}

pub(crate) fn which_lighthouse() -> Option<String> {
    crate::platform::which_bin("lighthouse").map(|p| p.display().to_string())
}
