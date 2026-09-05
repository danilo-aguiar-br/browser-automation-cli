// SPDX-License-Identifier: MIT OR Apache-2.0
//! Residual scavengers (FINALIZE + BORN cross-run GC).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::constants::STALE_MIN_AGE_SECS;
use super::discover::{
    discover_owned_chromium_tmp_side_channels, discover_stale_singleton_candidates_in_roots,
};
use super::roots::residual_scan_roots;
use super::wipe::wipe_safe_candidates;

/// FINALIZE scavenger (GAP-A002): wipe **owned** Chromium tmp leftovers that are
/// safe to remove — our uid, chromium/cli marker name, no live process holding
/// the path, and either created after `not_before` or Singleton-only / empty.
///
/// Never touches Flatpak/host Chrome profiles outside temp marker patterns.
pub fn scavenge_owned_chromium_tmp_orphans(
    profile: Option<&Path>,
    chrome_pid: Option<u32>,
    not_before: SystemTime,
) -> Vec<PathBuf> {
    let candidates = discover_owned_chromium_tmp_side_channels(profile, chrome_pid, not_before);
    wipe_safe_candidates(&candidates)
}

/// Cross-run GC of stale Singleton-only Chromium tmp + CLI marker orphans (RES-02).
///
/// Automatic BORN/FINALIZE hygiene: removes disk litter that survives healthy
/// process DIE when Chromium side-channels sit outside the marker profile.
///
/// **Never** deletes `com.google.Chrome.*` host Flatpak temp noise.
pub fn scavenge_stale_singleton_orphans() -> Vec<PathBuf> {
    scavenge_stale_singleton_orphans_with_min_age(Duration::from_secs(STALE_MIN_AGE_SECS))
}

/// Same as [`scavenge_stale_singleton_orphans`] with an explicit age floor.
///
/// Production uses [`STALE_MIN_AGE_SECS`].
/// Tests may pass `Duration::ZERO` to force immediate GC of fixture paths.
pub fn scavenge_stale_singleton_orphans_with_min_age(min_age: Duration) -> Vec<PathBuf> {
    scavenge_stale_singleton_orphans_in_roots(&residual_scan_roots(), min_age)
}

/// [`scavenge_stale_singleton_orphans_with_min_age`] over explicit roots.
///
/// The age floor is what keeps the default roots safe: a profile younger than
/// [`STALE_MIN_AGE_SECS`] is never a
/// candidate, so the gap between `create_dir_all` and Chrome appearing in the
/// process table cannot be collected. A caller that lowers the floor to zero
/// **removes that protection** and must therefore also narrow the roots to
/// paths it owns; otherwise it deletes the in-flight profile of any concurrent
/// invocation on the host, including sibling test binaries.
pub fn scavenge_stale_singleton_orphans_in_roots(
    roots: &[PathBuf],
    min_age: Duration,
) -> Vec<PathBuf> {
    let candidates = discover_stale_singleton_candidates_in_roots(roots, min_age);
    let wiped = wipe_safe_candidates(&candidates);
    if !wiped.is_empty() {
        tracing::debug!(
            count = wiped.len(),
            min_age_secs = min_age.as_secs(),
            "scavenge_stale_singleton_orphans wiped residual paths"
        );
    }
    wiped
}
