// SPDX-License-Identifier: MIT OR Apache-2.0
//! Dual-root residual scan paths (OS temp + XDG chrome-profiles) — DRY helper.

use std::path::PathBuf;

/// Roots scanned for CLI marker profiles and invocation-owned side-channels.
///
/// - OS `temp_dir`: Chromium Singleton / side-channel orphans (product law scan).
/// - XDG `chrome_profiles_dir`: product ephemeral profiles (Tier-4 path migration).
pub(crate) fn residual_scan_roots() -> Vec<PathBuf> {
    let mut roots = vec![std::env::temp_dir()];
    if let Ok(p) = crate::xdg::chrome_profiles_dir() {
        roots.push(p);
    }
    roots
}
