// SPDX-License-Identifier: MIT OR Apache-2.0
//! Escape hatch for marker profiles that an orphaned browser pins forever.
//!
//! # The deadlock this breaks
//!
//! `orphan_marker_dirs` only counts a profile when **no** live process holds it,
//! and `wipe_safe_candidates` refuses to delete a path with a live holder. When
//! the owning CLI is hard-killed, its Chrome survives and keeps the profile in
//! its `--user-data-dir`, so:
//!
//! - the browser protects the profile, because it is a live holder, and
//! - the profile protects the browser, because nothing ever kills a process the
//!   GC is not allowed to touch.
//!
//! Neither side ever yields, so the residue is permanent by construction. The
//! only way out is to break the symmetry: prove the profile is abandoned, kill
//! the browser that pins it, then collect the profile.
//!
//! # What counts as proof
//!
//! All three must hold, and each one alone is insufficient:
//!
//! 1. the profile carries an owner-pid marker — so it is a profile *this product*
//!    created, not a pre-marker leftover or a foreign Chromium directory;
//! 2. that owner pid is **dead** — a live owner is a healthy concurrent
//!    invocation and is never touched, which is what keeps a sibling safe;
//! 3. the profile is older than the configured age floor — which covers the
//!    window between `create_dir_all` and the owner appearing in the process
//!    table, where a brand-new invocation would otherwise look abandoned.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::classify::path_older_than;
use super::discover::list_cli_chrome_marker_dirs;
use super::owner::read_owner_pid;
use super::proc::{index_live_processes, LiveProcessIndex};
use super::report::user_data_dir_of;

/// What a reconciliation pass actually did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReconcileOutcome {
    /// Browser pids signalled because their owning CLI was already gone.
    pub killed_pids: Vec<u32>,
    /// Marker profiles removed after their pinning browser was reaped.
    pub wiped_paths: Vec<PathBuf>,
}

impl ReconcileOutcome {
    /// True when the pass changed nothing (the healthy steady state).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.killed_pids.is_empty() && self.wiped_paths.is_empty()
    }
}

/// Marker dirs whose owner pid is provably dead and that are past `min_age`.
///
/// Split out from the reaping so the decision is testable without signalling
/// anything.
#[must_use]
pub fn abandoned_marker_dirs(
    markers: &[PathBuf],
    index: &LiveProcessIndex,
    now: SystemTime,
    min_age: Duration,
) -> Vec<PathBuf> {
    markers
        .iter()
        .filter(|dir| {
            match read_owner_pid(dir) {
                // (1) ours, and (2) the owner is gone, plus (3) the age floor.
                Some(owner) => !index.contains_pid(owner) && path_older_than(dir, now, min_age),
                // No marker: profiles created before GAP-052 existed, and any
                // profile whose marker did not survive. Falling closed here left
                // legacy orphans permanently unreapable — measured on 2026-08-02,
                // a tree from an earlier build survived every 0.1.7 invocation
                // because its directory carries only `Local State` and
                // `BrowserMetrics-spare.pma`.
                //
                // The marker is one proof of abandonment, not the only one. The
                // substitute has to be at least as strong, so it demands both:
                None => {
                    path_older_than(dir, now, min_age.saturating_mul(LEGACY_AGE_FLOOR_FACTOR))
                        && browsers_are_reparented(dir, index)
                }
            }
        })
        .cloned()
        .collect()
}

/// Age multiplier applied when a profile carries no owner-pid marker.
///
/// Without the marker the age floor is the weaker half of the proof, so it is
/// widened rather than dropped. A concurrent invocation that somehow lost its
/// marker still gets an order of magnitude more room than a marked one.
const LEGACY_AGE_FLOOR_FACTOR: u32 = 10;

/// True when every browser pinning `dir` has been reparented away from a CLI.
///
/// # Why this substitutes for the owner marker
///
/// A Chrome tree has exactly one root: the process the CLI spawned. Its children
/// (zygote, GPU, network, renderers) report that root as their parent, so they
/// are identified and skipped by looking for a parent inside the same set.
///
/// If the root's own parent is absent from the process table, or is present but
/// is not this product, then no CLI owns the tree — which is precisely the
/// definition of the orphan this pass exists to collect. That is the same fact
/// the owner-pid marker would have established, read from the kernel instead of
/// from a file.
///
/// Conservative on purpose: an empty pin set returns `false`, because "no browser
/// holds this" is a question for the ordinary stale-GC path, not for a reaper
/// that signals processes.
#[must_use]
fn browsers_are_reparented(dir: &Path, index: &LiveProcessIndex) -> bool {
    let pins = browsers_pinning(dir, index);
    if pins.is_empty() {
        return false;
    }
    let pin_set: std::collections::HashSet<u32> = pins.iter().copied().collect();
    pins.iter().all(|pid| {
        let Some(entry) = index.entries().iter().find(|e| e.pid == *pid) else {
            return false;
        };
        match entry.ppid {
            // Child of another process in the same tree: not a root, says nothing.
            Some(ppid) if pin_set.contains(&ppid) => true,
            // Root: orphaned unless its parent is a live CLI of this product.
            Some(ppid) => !index
                .entries()
                .iter()
                .any(|e| e.pid == ppid && super::classify::entry_is_owning_cli(e)),
            // No parent reported: nothing can be proven, so refuse.
            None => false,
        }
    })
}

/// Marker profiles named by a live browser's `--user-data-dir`, wherever they live.
///
/// # Why directory listing is not enough
///
/// `list_cli_chrome_marker_dirs` walks `residual_scan_roots()`, and one of those
/// roots is derived from `XDG_CACHE_HOME`. Two invocations under different cache
/// homes therefore form mutually blind islands: each lists its own profiles and
/// neither can see the other's.
///
/// Measured on 2026-08-02: an orphaned tree under
/// `~/.cache/browser-automation-cli/chrome-profiles/` survived repeated 0.1.7
/// invocations whose `XDG_CACHE_HOME` pointed elsewhere. `foreign_root_orphans`
/// reported it correctly — detection already reads command lines — but the reaper
/// never received the path, so the report named residue that nothing could act on.
/// Detection and action must share a scope, or the report becomes a complaint.
///
/// The process table is the one view that is not scoped to a cache home, so the
/// browser's own argv is what closes the gap. The safety proof is unchanged:
/// [`abandoned_marker_dirs`] still demands an owner-pid marker, a dead owner and
/// the age floor, and it reads that marker from the profile itself, which works
/// for any path.
#[must_use]
pub fn marker_dirs_from_live_cmdlines(index: &LiveProcessIndex) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for entry in index.entries() {
        if !super::classify::entry_is_live_cli_chrome_strict(entry) {
            continue;
        }
        let Some(dir) = user_data_dir_of(&entry.cmdline) else {
            continue;
        };
        // Belt and braces: the cmdline filter already required the marker, but a
        // profile path that does not carry the product prefix is not ours to reap.
        let path = PathBuf::from(dir);
        let is_ours = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(super::constants::CLI_CHROME_MARKER_PREFIX));
        if is_ours && !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

/// Live pids whose `--user-data-dir` is exactly `dir`, browser-shaped or not.
///
/// Matching on the parsed switch value rather than on a substring is what stops
/// this from selecting an unrelated process that merely mentions the path.
///
/// # Why this is deliberately NOT filtered by browser identity
///
/// It is tempting to require a real browser here, since the result feeds a
/// reaper. Doing so inverts the risk. `browsers_are_reparented` uses this set
/// as the TOPOLOGY of the Chrome tree: a pid is recognised as a child, rather
/// than a root, by finding its parent inside the same set. Under Flatpak the
/// tree root is `bwrap` and under Snap it is `snap`, neither of which is a
/// browser executable. Filtering the root out leaves its children with no parent
/// in the set, so every one of them is re-read as a root, every root looks
/// orphaned, and a perfectly alive tree becomes a target.
///
/// Identity belongs at the point of destruction, not at the point of proof.
/// See `browsers_reapable`.
#[must_use]
pub fn browsers_pinning(dir: &Path, index: &LiveProcessIndex) -> Vec<u32> {
    let needle = dir.display().to_string();
    index
        .entries()
        .iter()
        .filter(|entry| user_data_dir_of(&entry.cmdline) == Some(needle.as_str()))
        .map(|entry| entry.pid)
        .collect()
}

/// The subset of [`browsers_pinning`] that is positively identified as a browser.
///
/// Only these may be signalled. `browsers_pinning` answers "what shape is this
/// tree", which needs every pid; this answers "what may I kill", which needs
/// proof. Conflating the two is how a process whose only crime was carrying
/// `--user-data-dir=<marker>` in its argv reached `kill_unix_tree`.
#[must_use]
pub fn browsers_reapable(dir: &Path, index: &LiveProcessIndex) -> Vec<u32> {
    let needle = dir.display().to_string();
    index
        .entries()
        .iter()
        .filter(|entry| user_data_dir_of(&entry.cmdline) == Some(needle.as_str()))
        .filter(|entry| super::classify::entry_is_browser_strict(entry))
        .map(|entry| entry.pid)
        .collect()
}

/// BORN reconciliation: reap browser trees whose owning CLI already died.
///
/// Runs on every invocation, so residue left by *previous* invocations is
/// collected retroactively instead of accumulating until an operator notices.
/// Best-effort: it never panics and it never touches a profile with a live owner.
pub fn reconcile_abandoned_profiles() -> ReconcileOutcome {
    reconcile_abandoned_profiles_with_min_age(super::report::orphan_age_floor())
}

/// [`reconcile_abandoned_profiles`] with an explicit age floor.
///
/// Tests pass a short floor against a sandbox root. Production must not lower it:
/// the floor is the only thing protecting an invocation whose Chrome has not yet
/// been observed in the process table.
pub fn reconcile_abandoned_profiles_with_min_age(min_age: Duration) -> ReconcileOutcome {
    let Some(index) = index_live_processes() else {
        // Fail-closed (GAP-045): with no process table we cannot prove an owner
        // is dead, and killing on a guess is worse than leaving residue.
        tracing::debug!("residual reconcile skipped: live process table unavailable");
        return ReconcileOutcome::default();
    };
    // Union of the two views: directories we can list under our own roots, plus
    // directories a live browser names in its argv regardless of cache home.
    // Neither is a superset of the other — the first finds profiles whose browser
    // already exited, the second finds profiles outside our roots.
    let mut markers = list_cli_chrome_marker_dirs();
    for dir in marker_dirs_from_live_cmdlines(&index) {
        if !markers.contains(&dir) {
            markers.push(dir);
        }
    }
    let abandoned = abandoned_marker_dirs(&markers, &index, SystemTime::now(), min_age);

    let mut outcome = ReconcileOutcome::default();
    for dir in abandoned {
        let pinning = browsers_pinning(&dir, &index);
        let reapable = browsers_reapable(&dir, &index);

        // All-or-nothing. If any pid holds this profile without being a
        // recognisable browser, the pass declines the whole directory: it does
        // not kill, and it does not wipe.
        //
        // Killing only the identified subset would leave the unidentified holder
        // running against a directory this pass then deleted — which is the exact
        // definition of `ghost_marker_processes`, so a "partial cleanup" would
        // manufacture the very defect the report exists to catch. Leaving residue
        // for one more run is recoverable; signalling a process we cannot name is
        // not.
        if pinning.len() != reapable.len() {
            tracing::debug!(
                dir = %dir.display(),
                pinning = pinning.len(),
                reapable = reapable.len(),
                "residual reconcile skipped: unidentified process holds this profile"
            );
            continue;
        }

        for pid in reapable {
            reap_orphan_browser(pid);
            outcome.killed_pids.push(pid);
        }
        // Wipe after the reap so the browser cannot recreate Singleton files
        // between the delete and its own exit.
        super::wipe::wipe_path(&dir);
        if !dir.exists() {
            outcome.wiped_paths.push(dir);
        }
    }
    if !outcome.is_empty() {
        tracing::debug!(
            killed = outcome.killed_pids.len(),
            wiped = outcome.wiped_paths.len(),
            "residual reconcile reaped abandoned browser profiles"
        );
    }
    outcome
}

/// Signal one orphaned browser tree, using whatever the host provides.
fn reap_orphan_browser(pid: u32) {
    #[cfg(unix)]
    {
        // The orphan's group is unknown here (its launcher is gone), so the
        // pid-tree walk is the correct escalation.
        crate::lifecycle::kill_unix_tree(
            pid,
            None,
            Duration::from_secs(crate::constants::FINALIZE_CHILD_GRACE_SECS),
        );
    }
    #[cfg(windows)]
    {
        crate::win_job::terminate_pid(pid);
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
    }
}
