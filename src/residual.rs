// SPDX-License-Identifier: MIT OR Apache-2.0
//! Owned residual path discovery for one-shot Chrome (GAP-017 / GAP-020 / RES-01…12).
//!
//! Never wipes host Chromes (Flatpak/VS Code). Only paths that are:
//! - under our marker prefix `browser-automation-cli-chrome-*`, or
//! - Singleton files inside our profile, or
//! - `/tmp/org.chromium.Chromium.*` / `/tmp/.org.chromium.Chromium.*` that are
//!   Singleton-only (or empty), owned by this uid, with no live process holding
//!   the path — either from this invocation window or cross-run stale GC.
//!
//! Host Flatpak tmp noise (`.com.google.Chrome.*` / `com.google.Chrome.*`) is
//! **never** deleted by cross-run GC (product law).
//!
//! # Workload
//!
//! **I/O-bound** temp/`/proc` scan during BORN and FINALIZE. Discovery of temp
//! entries stays sequential (directory iterator).
//!
//! **PAR-89:** live-process checks use a **single** `/proc` cmdline index
//! (`index_proc_cmdlines`) then [`crate::concurrency::map_cpu`] over candidates
//! (threshold 32). Never rescan `/proc` inside each parallel task (that would
//! be O(N×P) thrashing under Rayon).
//!
//! **PAR-90:** independent wipe of disjoint paths uses `map_cpu` when large.
//! Subprocess Chrome itself is never unbounded-forked (one launch per one-shot
//! process). The brief `std::thread::sleep` settle in browser residual path is
//! on the **sync** FINALIZE path (not a Tokio worker).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::Serialize;

/// Prefix for CLI-owned temp Chrome user-data-dir profiles.
pub const CLI_CHROME_MARKER_PREFIX: &str = "browser-automation-cli-chrome-";

/// Chromium side-channel prefix under the process temp dir.
pub const CHROMIUM_TMP_PREFIX: &str = "org.chromium.Chromium.";

/// Hidden Chromium side-channel prefix under the process temp dir.
pub const CHROMIUM_TMP_DOT_PREFIX: &str = ".org.chromium.Chromium.";

/// Max file/dir payload size treated as Singleton-only residue (bytes).
pub const SINGLETON_MAX_BYTES: u64 = 4096;

/// Max directory entries allowed when classifying Singleton-only residue.
pub const SINGLETON_MAX_ENTRIES: usize = 8;

/// Tight window after launch for unattributed chromium tmp (seconds).
pub const INVOCATION_SIDE_CHANNEL_WINDOW_SECS: u64 = 5;

/// Skew allowance when comparing mtime/ctime to `not_before` (seconds).
pub const MTIME_SKEW_SECS: u64 = 2;

/// Minimum age for cross-run stale Singleton GC (seconds).
///
/// Avoids racing a Chromium process still creating files under the same uid.
pub const STALE_MIN_AGE_SECS: u64 = 60;

/// Machine-readable residual disk hygiene report (doctor / agents).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResidualDiskReport {
    /// Count of `browser-automation-cli-chrome-*` dirs under temp.
    pub cli_marker_dirs: usize,
    /// Count of Chromium Singleton-only tmp dirs that look orphaned.
    pub chromium_tmp_singleton_orphans: usize,
    /// Count of paths that would be wiped by stale GC right now.
    pub scavenge_safe_candidates: usize,
    /// Live processes whose cmdline contains the CLI chrome marker prefix.
    pub live_cli_marker_processes: usize,
}

/// Discover Chromium side-channel paths in the process temp dir that belong to
/// this launch (GAP-020).
pub fn discover_owned_chromium_tmp_side_channels(
    profile: Option<&Path>,
    chrome_pid: Option<u32>,
    not_before: SystemTime,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let tmp = std::env::temp_dir();
    let Ok(entries) = std::fs::read_dir(&tmp) else {
        return out;
    };
    let pid_s = chrome_pid.map(|p| p.to_string());
    let profile_s = profile.map(|p| p.display().to_string());

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
    out
}

/// Collect residual marker profile dirs left under temp (should be empty after healthy DIE).
pub fn list_cli_chrome_marker_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let tmp = std::env::temp_dir();
    let Ok(entries) = std::fs::read_dir(tmp) else {
        return out;
    };
    for ent in entries.flatten() {
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(CLI_CHROME_MARKER_PREFIX) {
            out.push(ent.path());
        }
    }
    out
}

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
    wipe_safe_candidates(candidates)
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
/// Production uses [`STALE_MIN_AGE_SECS`]. Tests may pass `Duration::ZERO` to
/// force immediate GC of fixture paths.
pub fn scavenge_stale_singleton_orphans_with_min_age(min_age: Duration) -> Vec<PathBuf> {
    let candidates = discover_stale_singleton_candidates(min_age);
    let wiped = wipe_safe_candidates(candidates);
    if !wiped.is_empty() {
        tracing::debug!(
            count = wiped.len(),
            min_age_secs = min_age.as_secs(),
            "scavenge_stale_singleton_orphans wiped residual paths"
        );
    }
    wiped
}

/// Snapshot residual disk hygiene without mutating the filesystem.
pub fn residual_disk_report() -> ResidualDiskReport {
    let markers = list_cli_chrome_marker_dirs();
    let stale = discover_stale_singleton_candidates(Duration::from_secs(STALE_MIN_AGE_SECS));
    let proc_index = index_proc_cmdlines();
    // Count only browser-like processes that hold our marker profile.
    // Avoid false positives from agent/shell cmdlines that merely *mention*
    // the marker string (grep, find, editors, residual scripts).
    let live_cli = proc_index
        .iter()
        .filter(|cmd| is_live_cli_chrome_cmdline(cmd))
        .count();
    // Count chromium singleton-shaped dirs (including those younger than age floor).
    let orphans = count_chromium_singleton_shaped();
    ResidualDiskReport {
        cli_marker_dirs: markers.len(),
        chromium_tmp_singleton_orphans: orphans,
        scavenge_safe_candidates: stale.len(),
        live_cli_marker_processes: live_cli,
    }
}

fn discover_stale_singleton_candidates(min_age: Duration) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let tmp = std::env::temp_dir();
    let Ok(entries) = std::fs::read_dir(&tmp) else {
        return out;
    };
    let now = SystemTime::now();
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
        if !is_cli_marker && !is_singleton_only_or_small(&path) {
            continue;
        }
        if is_cli_marker {
            // Marker dirs: wipe when empty/small or fully Singleton-shaped; age floor applies.
            if !is_singleton_only_or_small(&path) && dir_entry_count(&path) > 0 {
                // Non-empty full profile still young may be live CLI launch — require no live holder below.
            }
        }
        if !path_older_than(&path, now, min_age) {
            continue;
        }
        out.push(path);
    }
    out
}

fn count_chromium_singleton_shaped() -> usize {
    let tmp = std::env::temp_dir();
    let Ok(entries) = std::fs::read_dir(tmp) else {
        return 0;
    };
    let mut n = 0usize;
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
    n
}

fn wipe_safe_candidates(candidates: Vec<PathBuf>) -> Vec<PathBuf> {
    // PAR-89: index /proc cmdlines **once**, then map_cpu candidate checks against
    // the shared index (never N full /proc scans under Rayon).
    let proc_index = index_proc_cmdlines();
    let wipeable: Vec<PathBuf> = crate::concurrency::map_cpu(&candidates, |path| {
        if path_has_live_process(path, &proc_index) {
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

fn is_chromium_tmp_name(name: &str) -> bool {
    name.starts_with(CHROMIUM_TMP_PREFIX) || name.starts_with(CHROMIUM_TMP_DOT_PREFIX)
}

fn is_google_chrome_tmp_name(name: &str) -> bool {
    name.starts_with(".com.google.Chrome.") || name.starts_with("com.google.Chrome.")
}

fn is_singleton_only_or_small(path: &Path) -> bool {
    if !path.is_dir() {
        if let Ok(meta) = path.metadata() {
            return meta.len() <= SINGLETON_MAX_BYTES;
        }
        return false;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    let mut count = 0usize;
    let mut only_singleton = true;
    for ent in entries.flatten() {
        count += 1;
        if count > SINGLETON_MAX_ENTRIES {
            return false;
        }
        let n = ent.file_name();
        let n = n.to_string_lossy();
        if !(n.starts_with("Singleton")
            || n == "DevToolsActivePort"
            || n.starts_with(".org.chromium")
            || n.ends_with(".lock"))
        {
            only_singleton = false;
        }
    }
    only_singleton || count == 0
}

fn dir_entry_count(path: &Path) -> usize {
    std::fs::read_dir(path)
        .map(|it| it.flatten().count())
        .unwrap_or(0)
}

fn path_older_than(path: &Path, now: SystemTime, min_age: Duration) -> bool {
    let Ok(meta) = path.metadata() else {
        return false;
    };
    let modified = meta.modified().ok();
    let created = meta.created().ok();
    let age = |t: SystemTime| now.duration_since(t).unwrap_or_default();
    if let Some(m) = modified {
        if age(m) >= min_age {
            return true;
        }
    }
    if let Some(c) = created {
        if age(c) >= min_age {
            return true;
        }
    }
    // If neither timestamp is available, treat as not stale (safe).
    false
}

/// One-shot index of live process cmdlines under `/proc` (unix).
///
/// **PAR-89:** call **once** per scavenge, then pass the slice into
/// [`path_has_live_process`]. Never rebuild inside `map_cpu` tasks.
#[cfg(unix)]
pub fn index_proc_cmdlines() -> Vec<String> {
    let mut out = Vec::new();
    let Ok(proc) = std::fs::read_dir("/proc") else {
        return out;
    };
    for ent in proc.flatten() {
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if !name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let cmdline = ent.path().join("cmdline");
        if let Ok(bytes) = std::fs::read(cmdline) {
            // cmdline is NUL-separated; keep as lossy string for substring search.
            out.push(String::from_utf8_lossy(&bytes).into_owned());
        }
    }
    out
}

#[cfg(not(unix))]
pub fn index_proc_cmdlines() -> Vec<String> {
    Vec::new()
}

/// True if any indexed cmdline contains `path` (best-effort; no external `rg`).
fn path_has_live_process(path: &Path, proc_index: &[String]) -> bool {
    let needle = path.display().to_string();
    if needle.is_empty() {
        return false;
    }
    proc_index.iter().any(|cmd| cmd.contains(&needle))
}

/// True when a process cmdline looks like a Chrome/Chromium instance using our
/// temp profile marker (not a shell/agent that only mentions the string).
fn is_live_cli_chrome_cmdline(cmd: &str) -> bool {
    if !cmd.contains(CLI_CHROME_MARKER_PREFIX) {
        return false;
    }
    // Chrome multiproc children always carry a browser binary path or --type=.
    let looks_like_browser = cmd.contains("chromium")
        || cmd.contains("google-chrome")
        || cmd.contains("/chrome")
        || cmd.contains("\0--type=")
        || cmd.contains(" --type=")
        || cmd.contains("--user-data-dir=");
    if !looks_like_browser {
        return false;
    }
    // Exclude pure text tools / editors that might embed the marker path in argv.
    let looks_like_text_tool = cmd.contains("rg ")
        || cmd.contains("grep ")
        || cmd.contains("atomwrite")
        || cmd.contains("sed ")
        || cmd.contains("nvim")
        || cmd.contains("code ")
        || cmd.contains("cursor ");
    !looks_like_text_tool
}

fn wipe_path(path: &Path) {
    if !path.exists() {
        return;
    }
    if path.is_dir() {
        let _ = std::fs::remove_dir_all(path);
    } else {
        let _ = std::fs::remove_file(path);
    }
}

fn owned_by_current_user(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let Ok(meta) = path.metadata() else {
            return false;
        };
        // SAFETY:
        // - Contract: compare path owner uid to the real uid of this process.
        // - Invariant: `getuid` has no preconditions and returns the caller's real uid.
        // - Used only to refuse deleting residual paths not owned by the current user.
        // - See: `man 2 getuid`; `MetadataExt::uid` for the file side.
        meta.uid() == unsafe { libc::getuid() }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        true
    }
}

fn created_or_modified_after(path: &Path, not_before: SystemTime) -> bool {
    let Ok(meta) = path.metadata() else {
        return false;
    };
    let modified = meta.modified().ok();
    let created = meta.created().ok();
    let skew = Duration::from_secs(MTIME_SKEW_SECS);
    let threshold = not_before.checked_sub(skew).unwrap_or(not_before);
    if let Some(m) = modified {
        if m >= threshold {
            return true;
        }
    }
    if let Some(c) = created {
        if c >= threshold {
            return true;
        }
    }
    false
}

fn path_references(path: &Path, needle: &str) -> bool {
    if path.display().to_string().contains(needle) {
        return true;
    }
    // Symlink target may encode hostname-pid.
    if let Ok(target) = std::fs::read_link(path) {
        if target.to_string_lossy().contains(needle) {
            return true;
        }
    }
    // Small text files may contain the pid.
    if let Ok(meta) = path.metadata() {
        if meta.is_file() && meta.len() <= SINGLETON_MAX_BYTES {
            if let Ok(bytes) = std::fs::read(path) {
                if String::from_utf8_lossy(&bytes).contains(needle) {
                    return true;
                }
            }
        }
    }
    // Directory: check Singleton* symlink targets / small files.
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for ent in entries.flatten() {
                let p = ent.path();
                if path_references(&p, needle) {
                    return true;
                }
            }
        }
    }
    false
}

fn age_since(t: SystemTime) -> Duration {
    SystemTime::now().duration_since(t).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime};

    #[test]
    fn marker_scan_finds_and_filters() {
        let tmp = std::env::temp_dir();
        let dir = tmp.join(format!(
            "browser-automation-cli-chrome-test-{}",
            uuid::Uuid::new_v4()
        ));
        let _ = fs::create_dir_all(&dir);
        let found = list_cli_chrome_marker_dirs();
        assert!(
            found.iter().any(|p| p == &dir),
            "expected marker dir in list"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_respects_not_before() {
        let future = SystemTime::now() + Duration::from_secs(3600);
        let found = discover_owned_chromium_tmp_side_channels(None, None, future);
        assert!(
            found.is_empty(),
            "future not_before must yield no side channels"
        );
    }

    #[test]
    fn stale_gc_removes_singleton_only_fixture() {
        let tmp = std::env::temp_dir();
        let dir = tmp.join(format!(
            "org.chromium.Chromium.{}",
            &uuid::Uuid::new_v4().to_string()[..8]
        ));
        let _ = fs::create_dir_all(&dir);
        // Singleton-shaped contents.
        let cookie = dir.join("SingletonCookie");
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink("12345", &cookie);
        #[cfg(not(unix))]
        let _ = fs::write(&cookie, b"12345");
        let sock = dir.join("SingletonSocket");
        // Regular empty file standing in for a dead socket.
        let _ = fs::write(&sock, b"");

        assert!(is_singleton_only_or_small(&dir));
        // Age floor zero in tests so we do not depend on utimensat/filetime.
        let wiped = scavenge_stale_singleton_orphans_with_min_age(Duration::ZERO);
        assert!(
            wiped.iter().any(|p| p == &dir) || !dir.exists(),
            "stale singleton fixture must be wiped: wiped={wiped:?} exists={}",
            dir.exists()
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn residual_disk_report_is_finite() {
        let r = residual_disk_report();
        // Just ensure fields are accessible and non-panicking.
        let _ = r.cli_marker_dirs
            + r.chromium_tmp_singleton_orphans
            + r.scavenge_safe_candidates
            + r.live_cli_marker_processes;
    }

    #[test]
    fn google_chrome_tmp_names_excluded_from_stale_gc_list() {
        assert!(is_google_chrome_tmp_name(".com.google.Chrome.XYZ"));
        assert!(is_google_chrome_tmp_name("com.google.Chrome.XYZ"));
        assert!(!is_google_chrome_tmp_name("org.chromium.Chromium.XYZ"));
    }

    #[test]
    fn live_cli_chrome_cmdline_ignores_shell_mentions() {
        let shell = "bash -c ls /tmp/browser-automation-cli-chrome-abc";
        assert!(!is_live_cli_chrome_cmdline(shell));
        let chrome = "/usr/bin/chromium-browser --user-data-dir=/tmp/browser-automation-cli-chrome-abc --headless=new";
        assert!(is_live_cli_chrome_cmdline(chrome));
    }
}
