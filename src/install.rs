// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chrome cache discovery for one-shot launches (no download / no apt).
//!
//! PRD: system Chrome/Chromium only — no embedded BrowserFetcher in MVP.
//!
//! # Workload
//!
//! **I/O-light sequential:** few version directories under XDG browsers cache.
//! Rayon would cost more than a handful of `read_dir` entries (PAR-71).

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Cache directory for optional pre-placed Chrome builds (XDG cache).
///
/// Path: `$XDG_CACHE_HOME/browser-automation-cli/browsers` (via `directories`).
pub fn get_browsers_dir() -> PathBuf {
    crate::xdg::browsers_dir().unwrap_or_else(|_| {
        std::env::temp_dir()
            .join("browser-automation-cli")
            .join("browsers")
    })
}

/// Find a Chrome binary previously placed under the product browsers cache.
pub fn find_installed_chrome() -> Option<PathBuf> {
    let browsers_dir = get_browsers_dir();
    // Debug diagnostics only when stderr is a terminal and --debug was used by the process
    // (tracing layer). Avoid reading product env vars at runtime.
    let debug = false;

    if debug {
        let _ = writeln!(
            io::stderr(),
            "[chrome-search] browsers_dir={}",
            browsers_dir.display()
        );
    }

    if !browsers_dir.exists() {
        if debug {
            let _ = writeln!(io::stderr(), "[chrome-search] browsers_dir does not exist");
        }
        return None;
    }

    let entries = match fs::read_dir(&browsers_dir) {
        Ok(entries) => entries,
        Err(e) => {
            let _ = writeln!(
                io::stderr(),
                "Warning: cannot read Chrome cache directory {}: {}",
                browsers_dir.display(),
                e
            );
            return None;
        }
    };

    let mut versions: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let matches = e
                .file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("chrome-"));
            if debug {
                let _ = writeln!(
                    io::stderr(),
                    "[chrome-search] entry {:?} matches={}",
                    e.file_name(),
                    matches
                );
            }
            matches
        })
        .collect();

    versions.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

    for entry in versions {
        let dir = entry.path();
        if let Some(bin) = chrome_binary_in_dir(&dir) {
            let exists = bin.exists();
            if debug {
                let _ = writeln!(
                    io::stderr(),
                    "[chrome-search] candidate {} exists={}",
                    bin.display(),
                    exists
                );
            }
            if exists {
                return Some(bin);
            }
        } else if debug {
            let _ = writeln!(
                io::stderr(),
                "[chrome-search] no binary found in {}",
                dir.display()
            );
        }
    }

    if debug {
        let _ = writeln!(io::stderr(), "[chrome-search] no installed Chrome found");
    }
    None
}

fn chrome_binary_in_dir(dir: &Path) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let app =
            dir.join("Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing");
        if app.exists() {
            return Some(app);
        }
        let inner = dir.join(
            "chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
        );
        if inner.exists() {
            return Some(inner);
        }
        let inner_x64 = dir.join(
            "chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
        );
        if inner_x64.exists() {
            return Some(inner_x64);
        }
        None
    }

    #[cfg(target_os = "linux")]
    {
        let bin = dir.join("chrome");
        if bin.exists() {
            return Some(bin);
        }
        let inner = dir.join("chrome-linux64/chrome");
        if inner.exists() {
            return Some(inner);
        }
        None
    }

    #[cfg(target_os = "windows")]
    {
        let bin = dir.join("chrome.exe");
        if bin.exists() {
            return Some(bin);
        }
        let inner = dir.join("chrome-win64/chrome.exe");
        if inner.exists() {
            return Some(inner);
        }
        None
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = dir;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn get_browsers_dir_ends_with_product_path() {
        let dir = get_browsers_dir();
        let s = dir.to_string_lossy();
        assert!(
            s.contains("browser-automation-cli") && s.contains("browsers"),
            "unexpected browsers dir: {s}"
        );
    }

    #[test]
    fn chrome_binary_in_dir_none_for_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(chrome_binary_in_dir(tmp.path()).is_none());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn chrome_binary_in_dir_finds_linux_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("chrome-linux64");
        fs::create_dir_all(&nested).unwrap();
        let bin = nested.join("chrome");
        fs::write(&bin, b"x").unwrap();
        assert_eq!(chrome_binary_in_dir(tmp.path()).as_ref(), Some(&bin));
    }
}
