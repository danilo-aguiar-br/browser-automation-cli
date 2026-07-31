// SPDX-License-Identifier: MIT OR Apache-2.0
//! System Chrome discovery (PATH, known paths, Windows ProgramFiles OS env).
//!
//! # Product law
//!
//! `std::env::var_os` for `ProgramFiles` / `LOCALAPPDATA` is **OS platform
//! path resolution**, not product configuration. Product knobs use XDG
//! `config set chrome_path` only.
use std::io::Write;
use std::path::PathBuf;

/// Locate a usable Chrome or Chromium on the host.
///
/// Honours the configured `chrome_path` first, then well-known install
/// locations including Flatpak and Snap wrappers. `None` means the host has
/// none, which is an `unavailable` error and not a crash.
pub fn find_chrome() -> Option<PathBuf> {
    if let Some(p) = crate::xdg::chrome_path_from_config() {
        let path = PathBuf::from(p);
        if crate::platform::is_executable_file(&path) {
            crate::platform::warn_if_sandboxed_browser(&path);
            return Some(path);
        }
    }
    if let Some(p) = crate::install::find_installed_chrome() {
        if crate::platform::is_executable_file(&p) {
            crate::platform::warn_if_sandboxed_browser(&p);
            return Some(p);
        }
    }

    let cache_dir = crate::install::get_browsers_dir();
    if cache_dir.exists() {
        let _ = writeln!(
            std::io::stderr(),
            "Warning: Chrome cache directory exists ({}) but no Chrome binary found inside. \
             Falling back to system Chrome (product browsers cache empty).",
            cache_dir.display()
        );
    }

    if let Some(p) = find_chrome_registry() {
        crate::platform::warn_if_sandboxed_browser(&p);
        return Some(p);
    }

    if let Some(p) = find_chrome_on_path() {
        crate::platform::warn_if_sandboxed_browser(&p);
        return Some(p);
    }

    if let Some(p) = find_chrome_known_paths() {
        crate::platform::warn_if_sandboxed_browser(&p);
        return Some(p);
    }

    if let Some(p) = super::tooling::find_puppeteer_chrome() {
        if crate::platform::is_executable_file(&p) {
            return Some(p);
        }
    }
    if let Some(p) = super::tooling::find_playwright_chromium() {
        if crate::platform::is_executable_file(&p) {
            return Some(p);
        }
    }

    None
}

/// Windows App Paths registry (no-op on other OSes).
fn find_chrome_registry() -> Option<PathBuf> {
    const APP_PATH_EXES: &[&str] = &["chrome.exe", "msedge.exe", "brave.exe"];
    for name in APP_PATH_EXES {
        if let Some(p) = crate::platform::registry_app_path(name) {
            return Some(p);
        }
    }
    None
}

/// Resolve browser names via `$PATH` without shelling out.
fn find_chrome_on_path() -> Option<PathBuf> {
    // `chrome` is the common Windows PATH basename; Unix distros use google-chrome/chromium.
    const NAMES: &[&str] = &[
        "google-chrome",
        "google-chrome-stable",
        "google-chrome-beta",
        "google-chrome-unstable",
        "chromium",
        "chromium-browser",
        "chrome",
        "microsoft-edge",
        "microsoft-edge-stable",
        "msedge",
        "brave-browser",
        "brave-browser-stable",
        "brave",
    ];
    for name in NAMES {
        if let Some(p) = crate::platform::which_bin(name) {
            return Some(p);
        }
    }
    None
}

/// Absolute install layouts per OS (GAP-049: XDG `chrome_search_paths` overrides).
///
/// Precedence: XDG `chrome_path` (exact binary, handled by [`find_chrome`]) →
/// XDG `chrome_search_paths` (ordered list) → the built-in per-OS layout in
/// [`crate::constants::DEFAULT_CHROME_SEARCH_PATHS`].
fn find_chrome_known_paths() -> Option<PathBuf> {
    let configured = crate::xdg::resolve_chrome_search_paths();
    if !configured.is_empty() {
        let candidates: Vec<PathBuf> = configured.iter().map(PathBuf::from).collect();
        return crate::platform::first_existing_executable(candidates.iter().map(|p| p.as_path()));
    }

    let mut candidates: Vec<PathBuf> = crate::constants::DEFAULT_CHROME_SEARCH_PATHS
        .iter()
        .map(PathBuf::from)
        .collect();

    // Windows resolves install roots from OS path variables before the literals.
    #[cfg(target_os = "windows")]
    {
        const RELS: &[&str] = &[
            r"Google\Chrome\Application\chrome.exe",
            r"Google\Chrome Beta\Application\chrome.exe",
            r"Google\Chrome SxS\Application\chrome.exe", // Canary
            r"Microsoft\Edge\Application\msedge.exe",
            r"Microsoft\Edge Beta\Application\msedge.exe",
            r"BraveSoftware\Brave-Browser\Application\brave.exe",
        ];
        let mut expanded: Vec<PathBuf> = Vec::with_capacity(24);
        for key in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Some(base) = std::env::var_os(key) {
                let base = PathBuf::from(base);
                for rel in RELS {
                    expanded.push(base.join(rel));
                }
            }
        }
        expanded.append(&mut candidates);
        candidates = expanded;
    }

    if let Some(home) = dirs::home_dir() {
        for suffix in crate::constants::DEFAULT_CHROME_HOME_SUFFIXES {
            candidates.push(home.join(suffix));
        }
    }

    crate::platform::first_existing_executable(candidates.iter().map(|p| p.as_path()))
}
