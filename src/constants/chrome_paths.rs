// SPDX-License-Identifier: MIT OR Apache-2.0
//! Built-in Chrome/Chromium discovery layout per OS (GAP-049 named default).
//!
//! Operator override: XDG `config set chrome_search_paths <p1>:<p2>` (`;` on
//! Windows). The exact-binary override `chrome_path` still wins over both.

/// Absolute Chrome/Chromium install layout searched when `chrome_search_paths` is unset.
#[cfg(target_os = "linux")]
pub const DEFAULT_CHROME_SEARCH_PATHS: &[&str] = &[
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/google-chrome-beta",
    "/usr/bin/google-chrome-unstable",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/usr/bin/microsoft-edge",
    "/usr/bin/microsoft-edge-stable",
    "/usr/bin/brave-browser",
    "/opt/google/chrome/chrome",
    "/opt/google/chrome/google-chrome",
    "/opt/microsoft/msedge/msedge",
    "/snap/bin/chromium",
    "/snap/bin/chromium-browser",
    "/var/lib/flatpak/exports/bin/com.google.Chrome",
    "/var/lib/flatpak/exports/bin/org.chromium.Chromium",
    "/var/lib/flatpak/exports/bin/com.brave.Browser",
    "/var/lib/flatpak/exports/bin/com.microsoft.Edge",
];

/// Absolute Chrome/Chromium install layout searched when `chrome_search_paths` is unset.
#[cfg(target_os = "macos")]
pub const DEFAULT_CHROME_SEARCH_PATHS: &[&str] = &[
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Google Chrome Beta.app/Contents/MacOS/Google Chrome Beta",
    "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
];

/// Absolute Chrome/Chromium install layout searched when `chrome_search_paths` is unset.
///
/// Windows resolves `ProgramFiles` / `LOCALAPPDATA` first (OS path resolution,
/// not product config); these are the last-resort literals.
#[cfg(target_os = "windows")]
pub const DEFAULT_CHROME_SEARCH_PATHS: &[&str] = &[
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
];

/// Absolute Chrome/Chromium install layout searched when `chrome_search_paths` is unset.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub const DEFAULT_CHROME_SEARCH_PATHS: &[&str] = &[];

/// Home-relative discovery suffixes appended after the absolute layout.
#[cfg(target_os = "linux")]
pub const DEFAULT_CHROME_HOME_SUFFIXES: &[&str] = &[
    ".local/share/flatpak/exports/bin/com.google.Chrome",
    ".local/share/flatpak/exports/bin/org.chromium.Chromium",
    ".local/share/flatpak/exports/bin/com.brave.Browser",
];

/// Home-relative discovery suffixes appended after the absolute layout.
#[cfg(target_os = "macos")]
pub const DEFAULT_CHROME_HOME_SUFFIXES: &[&str] = &[
    "Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "Applications/Chromium.app/Contents/MacOS/Chromium",
    "Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
];

/// Home-relative discovery suffixes appended after the absolute layout.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub const DEFAULT_CHROME_HOME_SUFFIXES: &[&str] = &[];

/// Ordered discovery paths: XDG `chrome_search_paths` when set, else the built-in layout.
///
/// Returns owned strings so the operator list and the compile-time default
/// share one call shape at the single discovery call site.
pub fn chrome_search_paths() -> Vec<String> {
    let configured = crate::xdg::resolve_chrome_search_paths();
    if !configured.is_empty() {
        return configured;
    }
    DEFAULT_CHROME_SEARCH_PATHS
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}
