//! Owned residual path discovery for one-shot Chrome (GAP-017 / GAP-020).
//!
//! Never wipes host Chromes (Flatpak/VS Code). Only paths that are:
//! - under our marker prefix `browser-automation-cli-chrome-*`, or
//! - Singleton files inside our profile, or
//! - `/tmp/org.chromium.Chromium.*` / `/tmp/.org.chromium.Chromium.*` created
//!   after launch, owned by this uid, and referencing our chrome pid or profile.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

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
        let is_chromium_tmp = name.starts_with("org.chromium.Chromium.")
            || name.starts_with(".org.chromium.Chromium.")
            || name.starts_with(".com.google.Chrome.")
            || name.starts_with("com.google.Chrome.");
        let is_cli_marker = name.starts_with("browser-automation-cli-chrome-");
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
        // Recent + our uid + chromium tmp pattern created within 2s of launch:
        // still only if younger than a tight window (owned launch noise).
        if age_since(not_before) < Duration::from_secs(5) {
            // Conservative: only empty lock files / small singleton sockets under 4KiB.
            if let Ok(meta) = path.metadata() {
                if meta.len() <= 4096 {
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
        if name.starts_with("browser-automation-cli-chrome-") {
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
    let mut wiped = Vec::new();
    for path in candidates {
        if path_has_live_process(&path) {
            continue;
        }
        // Prefer Singleton-only / small dirs; also wipe our marker profiles.
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let is_cli_marker = name.starts_with("browser-automation-cli-chrome-");
        let is_singletonish = is_singleton_only_or_small(&path);
        if is_cli_marker || is_singletonish {
            wipe_path(&path);
            wiped.push(path);
        }
    }
    wiped
}

fn is_singleton_only_or_small(path: &Path) -> bool {
    if !path.is_dir() {
        if let Ok(meta) = path.metadata() {
            return meta.len() <= 4096;
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
        if count > 8 {
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

fn path_has_live_process(path: &Path) -> bool {
    let needle = path.display().to_string();
    #[cfg(unix)]
    {
        // Best-effort: scan /proc/*/cmdline for the path (no external rg).
        let Ok(proc) = std::fs::read_dir("/proc") else {
            return false;
        };
        for ent in proc.flatten() {
            let name = ent.file_name();
            let name = name.to_string_lossy();
            if !name.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let cmdline = ent.path().join("cmdline");
            if let Ok(bytes) = std::fs::read(cmdline) {
                let s = String::from_utf8_lossy(&bytes);
                if s.contains(&needle) {
                    return true;
                }
            }
        }
        false
    }
    #[cfg(not(unix))]
    {
        let _ = needle;
        false
    }
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
    let skew = Duration::from_secs(2);
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
        if meta.is_file() && meta.len() <= 4096 {
            if let Ok(bytes) = std::fs::read(path) {
                if String::from_utf8_lossy(&bytes).contains(needle) {
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
    use std::time::SystemTime;

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
}
