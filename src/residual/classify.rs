// SPDX-License-Identifier: MIT OR Apache-2.0
//! Name and shape classifiers for residual Chromium / CLI marker paths.

use std::path::Path;
use std::time::{Duration, SystemTime};

use super::constants::{
    CHROMIUM_TMP_DOT_PREFIX, CHROMIUM_TMP_PREFIX, CLI_CHROME_MARKER_PREFIX, MTIME_SKEW_SECS,
    SINGLETON_MAX_BYTES, SINGLETON_MAX_ENTRIES,
};

pub(crate) fn is_chromium_tmp_name(name: &str) -> bool {
    name.starts_with(CHROMIUM_TMP_PREFIX) || name.starts_with(CHROMIUM_TMP_DOT_PREFIX)
}

pub(crate) fn is_google_chrome_tmp_name(name: &str) -> bool {
    name.starts_with(".com.google.Chrome.") || name.starts_with("com.google.Chrome.")
}

pub(crate) fn is_singleton_only_or_small(path: &Path) -> bool {
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
            || n == super::owner::OWNER_PID_FILE
            || n.starts_with(".org.chromium")
            || n.ends_with(".lock"))
        {
            only_singleton = false;
        }
    }
    only_singleton || count == 0
}

pub(crate) fn path_older_than(path: &Path, now: SystemTime, min_age: Duration) -> bool {
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
    false
}

pub(crate) fn owned_by_current_user(path: &Path) -> bool {
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

pub(crate) fn created_or_modified_after(path: &Path, not_before: SystemTime) -> bool {
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

pub(crate) fn path_references(path: &Path, needle: &str) -> bool {
    if path.display().to_string().contains(needle) {
        return true;
    }
    if let Ok(target) = std::fs::read_link(path) {
        if target.to_string_lossy().contains(needle) {
            return true;
        }
    }
    if let Ok(meta) = path.metadata() {
        if meta.is_file() && meta.len() <= SINGLETON_MAX_BYTES {
            if let Ok(bytes) = std::fs::read(path) {
                if String::from_utf8_lossy(&bytes).contains(needle) {
                    return true;
                }
            }
        }
    }
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

pub(crate) fn age_since(t: SystemTime) -> Duration {
    SystemTime::now().duration_since(t).unwrap_or_default()
}

/// Substrings that mark a process cmdline as a Chromium-family browser (GAP-052).
///
/// Linux, macOS, and Windows shapes are listed together so residual wipe does
/// not depend on the host OS of the agent that left the profile.
const BROWSER_CMDLINE_MARKERS: &[&str] = &[
    "chromium",
    "google-chrome",
    "/chrome",
    "\\chrome",
    "chrome.exe",
    "msedge.exe",
    "brave.exe",
    "\0--type=",
    " --type=",
    "--user-data-dir=",
];

/// Substrings that mark a text tool that only *mentions* a path (not a browser).
const TEXT_TOOL_CMDLINE_MARKERS: &[&str] = &[
    "rg ",
    "grep ",
    "atomwrite",
    "sed ",
    "nvim",
    "code ",
    "cursor ",
];

#[inline]
fn cmdline_matches_any(cmd: &str, markers: &[&str]) -> bool {
    markers.iter().any(|m| cmd.contains(m))
}

/// True when a process cmdline looks like a Chrome/Chromium instance using our
/// temp profile marker (not a shell/agent that only mentions the string).
pub(crate) fn is_live_cli_chrome_cmdline(cmd: &str) -> bool {
    if !cmd.contains(CLI_CHROME_MARKER_PREFIX) {
        return false;
    }
    cmdline_is_browser(cmd)
}

/// True when `cmd` is a browser process that actually holds `path` (GAP-052).
///
/// Used only as the fallback for candidates without an owner-pid marker. The
/// browser shape requirement is what keeps an editor or an `rg` invocation that
/// merely mentions the path from pinning it forever.
pub(crate) fn cmdline_holds_path(cmd: &str, path: &str) -> bool {
    cmd.contains(path) && cmdline_is_browser(cmd)
}

/// Shape test shared by the marker and the path-holder predicates.
fn cmdline_is_browser(cmd: &str) -> bool {
    if !cmdline_matches_any(cmd, BROWSER_CMDLINE_MARKERS) {
        return false;
    }
    !cmdline_matches_any(cmd, TEXT_TOOL_CMDLINE_MARKERS)
}
