// SPDX-License-Identifier: MIT OR Apache-2.0
//! Discovery of CLI marker dirs and invocation-owned Chromium side-channels.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::classify::{
    age_since, created_or_modified_after, is_chromium_tmp_name, is_google_chrome_tmp_name,
    is_singleton_only_or_small, owned_by_current_user, path_older_than, path_references,
};
use super::constants::{
    CLI_CHROME_MARKER_PREFIX, INVOCATION_SIDE_CHANNEL_WINDOW_SECS, SINGLETON_MAX_BYTES,
};
use super::roots::residual_scan_roots;

/// Resolve the `/tmp/org.chromium.Chromium.*` directory Chrome made for THIS
/// launch, by reading the symlink Chrome leaves inside our own profile.
///
/// # Why the heuristic scan was not enough
///
/// A unix socket path is capped near 108 bytes, and this product's profile
/// path — an XDG cache dir plus a UUID — blows past that. So Chrome puts the
/// real `SingletonSocket` in a short `/tmp` directory and drops a symlink to
/// it in the profile. That directory is never inside the profile, so wiping
/// the profile does not touch it.
///
/// [`discover_owned_chromium_tmp_side_channels`] tried to claim it by scanning
/// and matching, and measured against a live launch it claimed nothing: the
/// directory names the pid nowhere, names the profile nowhere, and Chrome
/// creates it during startup — before the launch timestamp the scan compares
/// against. The result was one leaked directory per browser launch, counted by
/// `chromium_tmp_singleton_orphans` and cleaned by nobody.
///
/// The symlink removes every guess. It is written by Chrome, it lives in a
/// profile this process created under a UUID no other process can name, and it
/// points at exactly one directory. Ownership is proven rather than inferred.
///
/// # Why the target is still checked
///
/// The return value is fed to a recursive delete. A symlink is attacker-
/// writable in principle, so the target must look like a Chromium temp
/// directory, be owned by this uid, and sit under a scan root this product
/// already claims. A path that fails any of those is returned as `None` and
/// left alone — leaking a directory is a smaller harm than deleting one that
/// belongs to somebody else.
///
/// Must be called while the profile still exists: FINALIZE wipes the profile,
/// and the symlink goes with it.
#[must_use]
pub fn owned_chromium_tmp_dir_via_profile(profile: &Path) -> Option<PathBuf> {
    let target = std::fs::read_link(profile.join("SingletonSocket")).ok()?;
    let dir = target.parent()?;
    if !is_chromium_tmp_name(&dir.file_name()?.to_string_lossy()) {
        return None;
    }
    if !residual_scan_roots().iter().any(|r| dir.starts_with(r)) {
        return None;
    }
    if !owned_by_current_user(dir) {
        return None;
    }
    Some(dir.to_path_buf())
}

/// Discover Chromium side-channel paths that belong to this launch (GAP-020).
///
/// Scans OS temp (Chromium side-channels) and XDG chrome-profiles (product
/// ephemeral profiles after Tier-4 path migration).
pub fn discover_owned_chromium_tmp_side_channels(
    profile: Option<&Path>,
    chrome_pid: Option<u32>,
    not_before: SystemTime,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let roots = residual_scan_roots();
    let pid_s = chrome_pid.map(|p| p.to_string());
    let profile_s = profile.map(|p| p.display().to_string());

    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for ent in entries.flatten() {
            let name = ent.file_name();
            let name = name.to_string_lossy();
            let is_chromium_tmp = is_chromium_tmp_name(&name);
            let is_cli_marker = name.starts_with(CLI_CHROME_MARKER_PREFIX);
            if !is_chromium_tmp && !is_cli_marker {
                continue;
            }
            let path = ent.path();
            if !owned_by_current_user(&path) {
                continue;
            }
            if !created_or_modified_after(&path, not_before) {
                continue;
            }
            if is_cli_marker {
                // Always ours when marker + recent + our uid.
                out.push(path);
                continue;
            }
            // Chromium tmp: require pid or profile reference when possible.
            if let Some(ref pid) = pid_s {
                if path_references(&path, pid) {
                    out.push(path);
                    continue;
                }
            }
            if let Some(ref prof) = profile_s {
                if path_references(&path, prof) {
                    out.push(path);
                    continue;
                }
            }
            // Recent + our uid + chromium tmp pattern created within the tight window:
            // still only if younger than a tight window (owned launch noise).
            if age_since(not_before) < Duration::from_secs(INVOCATION_SIDE_CHANNEL_WINDOW_SECS) {
                // Conservative: only empty lock files / small singleton sockets.
                if let Ok(meta) = path.metadata() {
                    if meta.len() <= SINGLETON_MAX_BYTES {
                        out.push(path);
                    }
                }
            }
        }
    }
    out
}

/// Collect residual marker profile dirs (OS temp + XDG chrome-profiles).
///
/// Healthy DIE leaves this empty. OS temp scan remains for Chromium side-channels;
/// XDG cache holds product ephemeral profiles after Tier-4 path migration.
pub fn list_cli_chrome_marker_dirs() -> Vec<PathBuf> {
    list_cli_chrome_marker_dirs_in_roots(&residual_scan_roots())
}

/// [`list_cli_chrome_marker_dirs`] restricted to an explicit set of roots.
///
/// The default roots are **shared with every other process on the host**, so a
/// caller that wants to reason about profiles it alone created must be able to
/// name where they live. Tests use this to snapshot a sandbox root instead of
/// the user's real cache.
pub fn list_cli_chrome_marker_dirs_in_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for ent in entries.flatten() {
            let name = ent.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(CLI_CHROME_MARKER_PREFIX) {
                out.push(ent.path());
            }
        }
    }
    out
}

/// Stale GC candidates across **every** scanned root (GAP-003).
///
/// Previously this scanned `std::env::temp_dir()` only, while marker discovery
/// scanned [`residual_scan_roots`]. Profiles created under the XDG cache root
/// were therefore observed by the report but never collected. Both the age floor
/// and the liveness check (applied by the wipe stage) hold for the XDG root too,
/// so the same policy is safe there.
pub(crate) fn discover_stale_singleton_candidates(min_age: Duration) -> Vec<PathBuf> {
    discover_stale_singleton_candidates_in_roots(&residual_scan_roots(), min_age)
}

/// [`discover_stale_singleton_candidates`] restricted to an explicit set of roots.
pub(crate) fn discover_stale_singleton_candidates_in_roots(
    roots: &[PathBuf],
    min_age: Duration,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let now = SystemTime::now();
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for ent in entries.flatten() {
            let name = ent.file_name();
            let name = name.to_string_lossy();
            let is_cli_marker = name.starts_with(CLI_CHROME_MARKER_PREFIX);
            let is_chromium_tmp = is_chromium_tmp_name(&name);
            // Explicitly exclude host Google Chrome Flatpak temp prefixes.
            if is_google_chrome_tmp_name(&name) {
                continue;
            }
            if !is_cli_marker && !is_chromium_tmp {
                continue;
            }
            let path = ent.path();
            if !owned_by_current_user(&path) {
                continue;
            }
            // Chromium side-channels must be Singleton-shaped. Marker dirs are ours
            // by name, so a full profile also qualifies; the age floor below and the
            // owner-pid liveness check in the wipe stage are what protect a live
            // sibling invocation.
            if !is_cli_marker && !is_singleton_only_or_small(&path) {
                continue;
            }
            if !path_older_than(&path, now, min_age) {
                continue;
            }
            out.push(path);
        }
    }
    out
}

/// Count Singleton-shaped Chromium side-channels across every scanned root.
pub(crate) fn count_chromium_singleton_shaped() -> usize {
    let mut n = 0usize;
    for root in residual_scan_roots() {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for ent in entries.flatten() {
            let name = ent.file_name();
            let name = name.to_string_lossy();
            if !is_chromium_tmp_name(&name) {
                continue;
            }
            let path = ent.path();
            if owned_by_current_user(&path) && is_singleton_only_or_small(&path) {
                n += 1;
            }
        }
    }
    n
}
