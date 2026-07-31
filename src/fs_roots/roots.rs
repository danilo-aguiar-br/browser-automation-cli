// SPDX-License-Identifier: MIT OR Apache-2.0
//! Which roots are allowed for this invocation.

use std::path::PathBuf;

/// Default roots for this host, in precedence order.
///
/// Missing directories are skipped rather than failing: a host without an XDG
/// cache dir must still be able to read from the working directory.
pub fn default_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    for dir in [
        crate::xdg::data_dir().ok(),
        crate::xdg::state_dir().ok(),
        crate::xdg::cache_dir().ok(),
    ]
    .into_iter()
    .flatten()
    {
        roots.push(dir);
    }
    roots.push(std::env::temp_dir());
    roots
}

/// Operator-configured extra roots from XDG `allowed_roots`.
pub fn configured_roots() -> Vec<PathBuf> {
    crate::xdg::resolve_allowed_roots()
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

/// Every root in effect for this invocation.
pub fn effective_roots() -> Vec<PathBuf> {
    let mut roots = default_roots();
    roots.extend(configured_roots());
    roots
}
