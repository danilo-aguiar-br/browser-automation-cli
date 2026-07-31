// SPDX-License-Identifier: MIT OR Apache-2.0
//! Machine-readable residual disk hygiene report (doctor / agents).

use std::time::Duration;

use serde::Serialize;

use super::classify::{is_live_cli_chrome_cmdline, path_older_than};
use super::constants::STALE_MIN_AGE_SECS;
use super::discover::{
    count_chromium_singleton_shaped, discover_stale_singleton_candidates,
    list_cli_chrome_marker_dirs,
};
use super::proc::{index_live_processes, path_has_live_process};

/// Machine-readable residual disk hygiene report (doctor / agents).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResidualDiskReport {
    /// Count of `browser-automation-cli-chrome-*` dirs under the scanned roots.
    pub cli_marker_dirs: usize,
    /// Count of Chromium Singleton-only tmp dirs that look orphaned.
    pub chromium_tmp_singleton_orphans: usize,
    /// Count of paths that would be wiped by stale GC right now.
    pub scavenge_safe_candidates: usize,
    /// Live browser *processes* holding a CLI marker profile.
    ///
    /// One invocation contributes many, because Chrome spawns renderer, GPU and
    /// utility children that all carry the same `--user-data-dir`. Kept for
    /// existing agent parsers; prefer [`Self::sibling_live_processes`] to count
    /// concurrent invocations.
    pub live_cli_marker_processes: usize,
    /// Live sibling *invocations*: distinct marker profiles with a live holder
    /// (GAP-002).
    ///
    /// **Informational.** A concurrent invocation is healthy, not a defect, so
    /// this never fails `doctor`.
    pub sibling_live_processes: usize,
    /// Marker dirs above the age floor whose owner pid is dead (GAP-006).
    ///
    /// **This is the real defect signal**: residue that no live process can still
    /// be holding and that healthy DIE should have removed.
    pub orphan_marker_dirs: usize,
    /// True when the host process table could not be enumerated (GAP-045).
    ///
    /// While true the collector refuses every wipe, so the residue counts above
    /// are expected to grow rather than converge to zero.
    pub process_table_unavailable: bool,
}

/// Snapshot residual disk hygiene without mutating the filesystem.
#[must_use]
pub fn residual_disk_report() -> ResidualDiskReport {
    let markers = list_cli_chrome_marker_dirs();
    let age_floor = Duration::from_secs(STALE_MIN_AGE_SECS);
    let stale = discover_stale_singleton_candidates(age_floor);
    let index = index_live_processes();
    let now = std::time::SystemTime::now();

    // Raw process count for the legacy field: every browser-like process holding a
    // marker profile. One invocation contributes many of these, because Chrome
    // spawns renderer/GPU/utility children that all carry the same
    // `--user-data-dir`. Cmdlines that merely *mention* the marker (editors, `rg`,
    // residual scripts) are filtered out.
    let live_marker_procs = index
        .as_ref()
        .map(|idx| {
            idx.cmdlines()
                .iter()
                .filter(|cmd| is_live_cli_chrome_cmdline(cmd))
                .count()
        })
        .unwrap_or(0);

    // Sibling *invocations*: distinct marker profiles with a live holder. This is
    // the number an operator expects to read, not the Chrome subprocess count.
    let sibling_live_processes = index
        .as_ref()
        .map(|idx| {
            markers
                .iter()
                .filter(|dir| path_has_live_process(dir, idx))
                .count()
        })
        .unwrap_or(0);

    // A marker dir is an orphan only when it is past the age floor AND no live
    // process holds it — neither a live owner pid nor an orphaned browser still
    // writing to the profile. Unknown liveness is never counted as an orphan.
    let orphan_marker_dirs = index
        .as_ref()
        .map(|idx| {
            markers
                .iter()
                .filter(|dir| {
                    path_older_than(dir, now, age_floor) && !path_has_live_process(dir, idx)
                })
                .count()
        })
        .unwrap_or(0);

    // Count chromium singleton-shaped dirs (including those younger than the floor).
    let orphans = count_chromium_singleton_shaped();
    ResidualDiskReport {
        cli_marker_dirs: markers.len(),
        chromium_tmp_singleton_orphans: orphans,
        scavenge_safe_candidates: stale.len(),
        live_cli_marker_processes: live_marker_procs,
        sibling_live_processes,
        orphan_marker_dirs,
        process_table_unavailable: index.is_none(),
    }
}
