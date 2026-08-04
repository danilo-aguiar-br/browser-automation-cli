// SPDX-License-Identifier: MIT OR Apache-2.0
//! Name and shape classifiers for residual Chromium / CLI marker paths.

use std::path::Path;
use std::time::{Duration, SystemTime};

use super::constants::{
    CHROMIUM_TMP_DOT_PREFIX, CHROMIUM_TMP_PREFIX, CLI_CHROME_MARKER_PREFIX, MTIME_SKEW_SECS,
    SINGLETON_MAX_BYTES, SINGLETON_MAX_ENTRIES,
};
use super::proc::ProcessEntry;

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
///
/// A denylist can only ever exclude what someone thought of. This one covers
/// seven tools and never covered `bash`, `sh`, `python3` or `node`, which is how
/// a shell script carrying `--user-data-dir=<marker> --type=renderer` passed as a
/// live browser. It survives only inside [`cmdline_is_browser`], whose sole
/// remaining job is the PROTECTIVE fallback where over-matching is harmless.
const TEXT_TOOL_CMDLINE_MARKERS: &[&str] = &[
    "rg ",
    "grep ",
    "atomwrite",
    "sed ",
    "nvim",
    "code ",
    "cursor ",
];

/// Executable file-name fragments that identify a Chromium-family browser.
///
/// Matched with `contains` against the lower-cased file name, not with equality,
/// because one browser ships under many names: `google-chrome-stable`,
/// `chromium-browser`, `chrome.exe`, and macOS's `Google Chrome Helper (Renderer)`
/// would each need their own entry otherwise, and the list would rot one release
/// at a time.
///
/// `contains` is safe HERE and was not safe on argv: this input is the kernel's
/// answer to "what binary is this", which the process cannot rewrite about
/// itself. A shell that merely *mentions* `chromium` has `bash` as its executable.
const BROWSER_EXE_TOKENS: &[&str] = &[
    "chrome",
    "chromium",
    "msedge",
    "microsoft-edge",
    "brave",
    "thorium",
    "vivaldi",
    "opera",
];

#[inline]
fn cmdline_matches_any(cmd: &str, markers: &[&str]) -> bool {
    markers.iter().any(|m| cmd.contains(m))
}

/// Lower-cased executable file name with any `.exe` suffix removed.
fn exe_file_name(exe: &Path) -> Option<String> {
    let name = exe.file_name()?.to_str()?.to_ascii_lowercase();
    Some(name.strip_suffix(".exe").unwrap_or(&name).to_string())
}

/// True when the kernel-reported executable is a Chromium-family browser.
///
/// Known and accepted blind spot: sandbox wrappers report the wrapper as the
/// executable — Flatpak's tree root is `bwrap`, Snap's is `snap`. Those roots
/// answer `false` here, so the STRICT counts under-report them. That is the safe
/// direction for every consumer of this predicate: under-reporting a verdict
/// field cannot fail a healthy host, and under-reporting a kill candidate cannot
/// signal an innocent process.
pub(crate) fn exe_is_browser(exe: &Path) -> bool {
    exe_file_name(exe).is_some_and(|name| BROWSER_EXE_TOKENS.iter().any(|t| name.contains(t)))
}

/// STRICT polarity: the process is positively identified as a browser.
///
/// Unknown executable answers `false`. Every caller of this predicate either
/// fails a host or hands a pid to the reaper, so ignorance must never be read as
/// evidence.
pub(crate) fn entry_is_browser_strict(entry: &ProcessEntry) -> bool {
    entry.exe.as_deref().is_some_and(exe_is_browser)
}

/// STRICT polarity: a live browser running against one of our marker profiles.
///
/// Feeds `live_cli_marker_processes`, `foreign_root_orphans`,
/// `ghost_marker_processes` and the reaper's candidate discovery.
pub(crate) fn entry_is_live_cli_chrome_strict(entry: &ProcessEntry) -> bool {
    entry.cmdline.contains(CLI_CHROME_MARKER_PREFIX) && entry_is_browser_strict(entry)
}

/// PROTECTIVE polarity: some live process may be holding `path`.
///
/// The union of both signals, so this is never less protective than the argv-only
/// predicate it replaced. Deleting a profile that a live sibling is still writing
/// to is the expensive mistake here; keeping a directory one run too long is not.
/// That asymmetry is why the strict identity is an *addition* and not a
/// replacement on this path.
pub(crate) fn entry_holds_path_protective(entry: &ProcessEntry, path: &str) -> bool {
    if entry.cmdline.contains(path) && entry_is_browser_strict(entry) {
        return true;
    }
    cmdline_holds_path(&entry.cmdline, path)
}

/// True when `entry` is a live CLI of this product, i.e. a process that could own
/// a Chrome tree.
///
/// Used by the reparenting proof: a browser whose parent satisfies this is still
/// owned and must never be reaped, while one whose parent does not has outlived
/// its launcher.
///
/// The argv fallback still excludes browser-shaped command lines, because our
/// marker prefix *contains* the product binary name — every Chrome child carries
/// `--user-data-dir=browser-automation-cli-chrome-…` and would otherwise look
/// like an owner of itself. The kernel path needs no such trick, and it also
/// fixes the mirror-image bug: `browser-automation-cli scrape https://chromium.org`
/// used to read as a browser and turn a real owner into a non-owner, which marked
/// an owned tree as orphaned.
pub(crate) fn entry_is_owning_cli(entry: &ProcessEntry) -> bool {
    if let Some(exe) = entry.exe.as_deref() {
        return exe_file_name(exe).is_some_and(|n| n == crate::constants::PRODUCT_BIN_NAME);
    }
    !cmdline_is_browser(&entry.cmdline)
        && entry.cmdline.contains(crate::constants::PRODUCT_BIN_NAME)
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
