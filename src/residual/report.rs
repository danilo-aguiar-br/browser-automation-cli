// SPDX-License-Identifier: MIT OR Apache-2.0
//! Machine-readable residual disk hygiene report (doctor / agents).

use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;

use super::classify::{entry_is_live_cli_chrome_strict, path_older_than};
use super::discover::{
    count_chromium_singleton_shaped, discover_stale_singleton_candidates,
    list_cli_chrome_marker_dirs,
};
use super::proc::{index_live_processes, path_has_live_process};
use super::roots::residual_scan_roots;

/// Age floor before a dead-owner marker profile is treated as collectable.
pub(crate) fn orphan_age_floor() -> Duration {
    crate::xdg::policy::policy_secs(crate::xdg::policy::key::RESIDUAL_ORPHAN_MIN_AGE_SECS)
}

/// Machine-readable residual disk hygiene report (doctor / agents).
///
/// # Which field is the contract
///
/// [`Self::cli_marker_dirs`] is the field the agent contract requires to be zero
/// after a healthy DIE when this process is the only concurrent invocation, and
/// it stays that way. [`Self::sibling_live_processes`] is **not** a replacement
/// for it: it answers a different question — how many concurrent invocations are
/// running — and is informational, so a live sibling never fails `doctor`.
/// [`Self::live_cli_marker_processes`] is a legacy process count (Chrome spawns
/// many children per invocation); agents MUST NOT require it to be zero.
///
/// Defect signals that fail `residual_disk`:
/// - [`Self::orphan_marker_dirs`]: profile past the age floor with no live holder
/// - [`Self::ghost_marker_processes`]: live CLI Chrome whose marker `--user-data-dir`
///   no longer exists as a directory (dir-based counts are blind to this)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResidualDiskReport {
    /// Roots this report actually scanned.
    ///
    /// Without this the zero is unfalsifiable. `chrome_profiles_dir()` derives
    /// from `cache_dir()`, so an invocation with a different `XDG_CACHE_HOME`
    /// scans a different tree and honestly reports zero while profiles sit in the
    /// other one. Measured: the same binary on the same host reported
    /// `cli_marker_dirs: 0` in one session and `2` in another for exactly this
    /// reason. Emitting the roots turns "zero" into "zero, here".
    pub scanned_roots: Vec<String>,
    /// Count of `browser-automation-cli-chrome-*` dirs under the scanned roots.
    pub cli_marker_dirs: usize,
    /// Count of Chromium Singleton-only tmp dirs that look orphaned.
    pub chromium_tmp_singleton_orphans: usize,
    /// Count of paths that would be wiped by stale GC right now.
    pub scavenge_safe_candidates: usize,
    /// Live browser processes holding a CLI marker profile, deduplicated by pid.
    ///
    /// One invocation still contributes many, because Chrome spawns renderer, GPU
    /// and utility children that all carry the same `--user-data-dir`. What this
    /// no longer does is count the same pid more than once: the previous
    /// implementation counted raw command-line strings and reported 368 where 22
    /// processes existed. Prefer [`Self::sibling_live_processes`] to count
    /// concurrent invocations.
    ///
    /// Membership is decided by the KERNEL-reported executable, not by argv, so a
    /// shell or script that merely carries `--user-data-dir=<marker>` is excluded.
    /// The accepted cost is that sandbox wrappers under-report: a Flatpak tree
    /// root runs as `bwrap` and a Snap one as `snap`, and neither answers to the
    /// browser allowlist. Under-reporting here cannot fail a healthy host, which
    /// is why the trade goes this way.
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
    /// Marker PROFILES outside the scanned roots that a live browser still holds.
    ///
    /// These are invisible to every other count in this report by construction,
    /// which is what made a stale-cache-home orphan look like a clean host. A
    /// non-zero value means residue exists that this invocation cannot collect,
    /// because it is not looking at the tree that holds it.
    ///
    /// Counted per directory, not per process. It used to count entries, so one
    /// invocation with a renderer, a GPU child and a utility child reported three
    /// orphans for one directory — the same Chrome-subprocess inflation already
    /// fixed in the fields above, left alive in the one nobody re-read.
    pub foreign_root_orphans: usize,
    /// Live browser pids whose `--user-data-dir` is a CLI marker path that is
    /// **not** a directory any more (NC-RESIDUAL-ORPHAN-PROC-NO-DIR).
    ///
    /// Dir-based residual counts (`cli_marker_dirs`, `orphan_marker_dirs`) walk
    /// the filesystem. When DIE (or a foreign wipe) removes the profile while
    /// Chrome children stay alive, those counts report zero and the host looks
    /// clean. This field is the process-table backstop: marker-shaped
    /// `--user-data-dir` + live browser + missing dir = fail `residual_disk`.
    pub ghost_marker_processes: usize,
    /// True when the host process table could not be enumerated (GAP-045).
    ///
    /// While true the collector refuses every wipe, so the residue counts above
    /// are expected to grow rather than converge to zero.
    pub process_table_unavailable: bool,
}

/// Extract the `--user-data-dir=` value from a browser command line.
///
/// Both `--user-data-dir=/path` and `--user-data-dir /path` shapes appear in the
/// wild; only the first is emitted by this product, but a foreign-root probe that
/// missed the second would under-report.
pub(crate) fn user_data_dir_of(cmdline: &str) -> Option<&str> {
    if let Some(rest) = cmdline.split("--user-data-dir=").nth(1) {
        return Some(rest.split_whitespace().next().unwrap_or(rest));
    }
    let mut parts = cmdline.split_whitespace();
    while let Some(part) = parts.next() {
        if part == "--user-data-dir" {
            return parts.next();
        }
    }
    None
}

/// True when `dir` is not inside any scanned root.
pub(crate) fn outside_roots(dir: &str, roots: &[PathBuf]) -> bool {
    let path = std::path::Path::new(dir);
    !roots.iter().any(|root| path.starts_with(root))
}

/// True when `dir` looks like a product CLI marker profile that is missing on disk.
///
/// Used by [`residual_disk_report`] to count ghost holders: live browsers whose
/// `--user-data-dir` still names a marker path after the directory is gone.
pub(crate) fn is_missing_cli_marker_dir(dir: &str) -> bool {
    let path = std::path::Path::new(dir);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.starts_with(super::constants::CLI_CHROME_MARKER_PREFIX) && !path.is_dir()
}

/// Snapshot residual disk hygiene without mutating the filesystem.
#[must_use]
pub fn residual_disk_report() -> ResidualDiskReport {
    let roots = residual_scan_roots();
    let markers = list_cli_chrome_marker_dirs();
    let age_floor = orphan_age_floor();
    let stale = discover_stale_singleton_candidates(age_floor);
    let index = index_live_processes();
    let now = std::time::SystemTime::now();

    // Browser processes holding a marker profile, counted once per pid.
    //
    // Identity comes from the kernel-reported executable, not from argv. A
    // process that merely *mentions* the marker — an editor, `rg`, a residual
    // script, or a shell carrying `--user-data-dir=<marker> --type=renderer` —
    // is not a browser no matter what its command line says.
    let live_marker_procs = index
        .as_ref()
        .map(|idx| idx.count_distinct_pids(entry_is_live_cli_chrome_strict))
        .unwrap_or(0);

    // Marker-holding browsers whose profile is not under any scanned root: real
    // residue that every other count in this report is blind to.
    //
    // Deduplicated by PROFILE, not by process. The field is named for orphans —
    // directories — but it used to `.count()` entries, so one invocation with a
    // renderer, a GPU child and a utility child reported three orphans where one
    // directory existed. That is the same inflation already fixed twice in this
    // module, surviving in the one field nobody re-read.
    let foreign_root_orphans = index
        .as_ref()
        .map(|idx| {
            idx.entries()
                .iter()
                .filter(|entry| entry_is_live_cli_chrome_strict(entry))
                .filter_map(|entry| user_data_dir_of(&entry.cmdline))
                .filter(|dir| outside_roots(dir, &roots))
                .collect::<std::collections::HashSet<&str>>()
                .len()
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

    // Ghost holders: live CLI Chrome whose marker profile dir is gone. Dir walks
    // cannot see these; without this count, doctor reports residual-zero while
    // Chrome children keep running against a deleted --user-data-dir.
    let ghost_marker_processes = index
        .as_ref()
        .map(|idx| {
            idx.count_distinct_pids(|entry| {
                entry_is_live_cli_chrome_strict(entry)
                    && user_data_dir_of(&entry.cmdline).is_some_and(is_missing_cli_marker_dir)
            })
        })
        .unwrap_or(0);

    // Count chromium singleton-shaped dirs (including those younger than the floor).
    let orphans = count_chromium_singleton_shaped();
    ResidualDiskReport {
        scanned_roots: roots.iter().map(|p| p.display().to_string()).collect(),
        cli_marker_dirs: markers.len(),
        chromium_tmp_singleton_orphans: orphans,
        scavenge_safe_candidates: stale.len(),
        live_cli_marker_processes: live_marker_procs,
        sibling_live_processes,
        orphan_marker_dirs,
        foreign_root_orphans,
        ghost_marker_processes,
        process_table_unavailable: index.is_none(),
    }
}
