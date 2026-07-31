// SPDX-License-Identifier: MIT OR Apache-2.0
//! Puppeteer/Playwright cache path helpers and tilde expand (L/M/W).
use std::path::{Path, PathBuf};

pub(crate) fn find_puppeteer_chrome() -> Option<PathBuf> {
    let mut search_dirs = Vec::new();
    if let Ok(bd) = crate::xdg::browsers_dir() {
        search_dirs.push(bd);
    }
    if let Some(home) = dirs::home_dir() {
        // Optional local caches under home (not env-driven).
        search_dirs.push(home.join(".cache/puppeteer/chrome"));
        search_dirs.push(home.join(".cache/ms-playwright"));
    }
    for dir in &search_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut matches: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    let path = e.path();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with("chromium-") {
                        let linux = path.join("chrome-linux64/chrome");
                        if linux.exists() {
                            return Some(linux);
                        }
                        let win = path.join("chrome-win64/chrome.exe");
                        if win.exists() {
                            return Some(win);
                        }
                    }
                    let candidate = build_puppeteer_binary_path(&path);
                    if candidate.exists() {
                        Some(candidate)
                    } else {
                        None
                    }
                })
                .collect();
            matches.sort();
            matches.reverse();
            if let Some(p) = matches.into_iter().next() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn build_puppeteer_binary_path(version_dir: &Path) -> PathBuf {
    version_dir.join("chrome-linux64/chrome")
}

#[cfg(target_os = "macos")]
fn build_puppeteer_binary_path(version_dir: &Path) -> PathBuf {
    let arm = version_dir.join(
        "chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
    );
    if arm.exists() {
        return arm;
    }
    version_dir.join(
        "chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
    )
}

#[cfg(target_os = "windows")]
fn build_puppeteer_binary_path(version_dir: &Path) -> PathBuf {
    version_dir.join(r"chrome-win64\chrome.exe")
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn build_puppeteer_binary_path(version_dir: &Path) -> PathBuf {
    version_dir.join("chrome")
}

pub(crate) fn find_playwright_chromium() -> Option<PathBuf> {
    // Home-local caches only (no PLAYWRIGHT_BROWSERS_PATH product env).
    let mut search_dirs = Vec::new();
    if let Some(home) = dirs::home_dir() {
        search_dirs.push(home.join(".cache/ms-playwright"));
    }
    if let Ok(bd) = crate::xdg::browsers_dir() {
        search_dirs.push(bd);
    }
    for dir in &search_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut matches: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| n.starts_with("chromium-"))
                        .unwrap_or(false)
                })
                .filter_map(|e| {
                    let candidate = build_playwright_binary_path(&e.path());
                    if candidate.exists() {
                        Some(candidate)
                    } else {
                        None
                    }
                })
                .collect();
            matches.sort();
            matches.reverse();
            if let Some(p) = matches.into_iter().next() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn build_playwright_binary_path(chromium_dir: &Path) -> PathBuf {
    let standard = chromium_dir.join("chrome-linux/chrome");
    if standard.exists() {
        return standard;
    }
    chromium_dir.join("chrome-linux64/chrome")
}

#[cfg(target_os = "macos")]
fn build_playwright_binary_path(chromium_dir: &Path) -> PathBuf {
    chromium_dir.join("chrome-mac/Chromium.app/Contents/MacOS/Chromium")
}

#[cfg(target_os = "windows")]
fn build_playwright_binary_path(chromium_dir: &Path) -> PathBuf {
    chromium_dir.join("chrome-win/chrome.exe")
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn build_playwright_binary_path(chromium_dir: &Path) -> PathBuf {
    chromium_dir.join("chrome")
}

pub(crate) fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix('~') {
        if let Some(home) = dirs::home_dir() {
            return home
                .join(rest.strip_prefix('/').unwrap_or(rest))
                .to_string_lossy()
                .to_string();
        }
    }
    path.to_string()
}
