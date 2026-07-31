// SPDX-License-Identifier: MIT OR Apache-2.0
//! Safe residual wipe with a single live-process index + map_cpu (PAR-89/90).

use std::path::{Path, PathBuf};

use super::classify::is_singleton_only_or_small;
use super::constants::CLI_CHROME_MARKER_PREFIX;
use super::proc::{index_live_processes, path_has_live_process, LiveProcessIndex};

/// Wipe the candidates that provably hold no live process.
///
/// **GAP-045 (fail-closed).** When the host process table cannot be enumerated
/// the function wipes **nothing**. Deleting a profile because liveness is
/// unknown corrupts a live sibling invocation; leaving residue on disk only
/// costs a later GC pass.
pub(crate) fn wipe_safe_candidates(candidates: &[PathBuf]) -> Vec<PathBuf> {
    let Some(index) = index_live_processes() else {
        tracing::warn!(
            candidates = candidates.len(),
            "residual wipe refused: live process table unavailable on this host"
        );
        return Vec::new();
    };
    wipe_safe_candidates_with_index(candidates, &index)
}

/// [`wipe_safe_candidates`] against an explicit index (tests, alternate probes).
pub(crate) fn wipe_safe_candidates_with_index(
    candidates: &[PathBuf],
    index: &LiveProcessIndex,
) -> Vec<PathBuf> {
    // PAR-89: index live processes **once**, then map_cpu candidate checks against
    // the shared index (never N full process scans under Rayon).
    let wipeable: Vec<PathBuf> = crate::concurrency::map_cpu(candidates, |path| {
        if path_has_live_process(path, index) {
            return None;
        }
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let is_cli_marker = name.starts_with(CLI_CHROME_MARKER_PREFIX);
        let is_singletonish = is_singleton_only_or_small(path);
        if is_cli_marker || is_singletonish {
            Some(path.clone())
        } else {
            None
        }
    })
    .into_iter()
    .flatten()
    .collect();
    // PAR-90: independent remove_dir_all/remove_file on disjoint paths.
    crate::concurrency::map_cpu(&wipeable, |path| {
        wipe_path(path);
        path.clone()
    })
}

pub(crate) fn wipe_path(path: &Path) {
    if !path.exists() {
        return;
    }
    if path.is_dir() {
        let _ = std::fs::remove_dir_all(path);
    } else {
        let _ = std::fs::remove_file(path);
    }
}
