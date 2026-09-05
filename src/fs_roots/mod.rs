// SPDX-License-Identifier: MIT OR Apache-2.0
//! Allowed-root containment for local reads and artifact writes (GAP-026).
//!
//! # Threat
//!
//! `goto file:///etc/passwd` succeeded with exit `0`, and pairing it with
//! `grab --path` let rendered host content be written anywhere the process
//! could reach. Nothing bounded either side.
//!
//! # Policy
//!
//! A local path is accepted when its **canonical** form lies under one of the
//! allowed roots. Canonicalisation is what makes the check resistant to `..`
//! and to symlinks pointing out of a root.
//!
//! | Layer | Source |
//! |-------|--------|
//! | default roots | cwd, XDG artifacts/state/data/cache, system temp |
//! | extra roots | XDG `allowed_roots` (platform-separated list) |
//! | escape | `--allow-outside-roots` on the same invocation |
//!
//! The escape flag is deliberately named for the risk it takes: there is no
//! silent bypass and no product environment variable.

mod enforce;
mod roots;

#[cfg(test)]
mod tests;

pub use enforce::{ensure_file_url_allowed, ensure_within_roots};
pub use roots::{configured_roots, default_roots, effective_roots};

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::CliError;

/// Process-wide `--allow-outside-roots`, published once at CLI boot.
///
/// A global keeps every artifact writer from growing a new parameter; it is set
/// from argv only, never from a product environment variable.
static ALLOW_OUTSIDE: AtomicBool = AtomicBool::new(false);

/// Publish the `--allow-outside-roots` decision for this process.
pub fn set_allow_outside_roots(allow: bool) {
    ALLOW_OUTSIDE.store(allow, Ordering::Relaxed);
}

/// Whether this invocation may step outside the allowed roots.
fn allow_outside_roots() -> bool {
    ALLOW_OUTSIDE.load(Ordering::Relaxed)
}

/// Enforce the read policy using the process-wide escape switch.
pub fn ensure_read_allowed(path: &Path) -> Result<PathBuf, CliError> {
    ensure_within_roots(path, PathUse::Read, allow_outside_roots())
}

/// Enforce the write policy using the process-wide escape switch.
pub fn ensure_write_allowed(path: &Path) -> Result<PathBuf, CliError> {
    ensure_within_roots(path, PathUse::Write, allow_outside_roots())
}

/// Enforce the `file://` policy using the process-wide escape switch.
pub fn ensure_file_url_allowed_default(url: &str) -> Result<(), CliError> {
    ensure_file_url_allowed(url, allow_outside_roots())
}

/// Which side of the boundary a path sits on, for error text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathUse {
    /// Reading a local resource (`file://` navigation, `parse`, fixtures).
    Read,
    /// Writing a produced artifact (screenshot, PDF, HAR, dump).
    Write,
}

impl PathUse {
    fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}
