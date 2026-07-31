// SPDX-License-Identifier: MIT OR Apache-2.0
//! Browser sandbox classification (Snap / Flatpak / none).

use std::path::Path;

/// How a resolved browser binary is packaged (affects automation reliability).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserSandbox {
    /// System / portable install (APT, RPM, MSI, dmg, etc.).
    None,
    /// Snap confinement (`/snap/…` or `$SNAP`).
    Snap,
    /// Flatpak confinement (`/var/lib/flatpak/…`, `~/.var/app/…`, `$FLATPAK_ID`).
    Flatpak,
}

impl BrowserSandbox {
    /// Human-readable id for doctor JSON.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Snap => "snap",
            Self::Flatpak => "flatpak",
        }
    }

    /// True when distribution sandbox may block CDP automation.
    pub fn is_restricted(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Classify a browser executable path (and process env) for sandbox warnings.
pub fn detect_browser_sandbox(path: &Path) -> BrowserSandbox {
    let s = path.to_string_lossy();
    if s.contains("/snap/") || s.starts_with("/snap/") || std::env::var_os("SNAP").is_some() {
        // Prefer path prefix when both set (host CLI launching snap chrome).
        if s.contains("/snap/") {
            return BrowserSandbox::Snap;
        }
    }
    if s.contains("/var/lib/flatpak/")
        || s.contains("/.local/share/flatpak/")
        || s.contains("/.var/app/")
    {
        return BrowserSandbox::Flatpak;
    }
    if std::env::var_os("FLATPAK_ID").is_some() {
        return BrowserSandbox::Flatpak;
    }
    if std::env::var_os("SNAP").is_some() && s.contains("snap") {
        return BrowserSandbox::Snap;
    }
    BrowserSandbox::None
}

/// Emit a local warning when the resolved browser is snap/flatpak confined.
pub fn warn_if_sandboxed_browser(path: &Path) {
    match detect_browser_sandbox(path) {
        BrowserSandbox::None => {}
        BrowserSandbox::Snap => {
            tracing::warn!(
                path = %path.display(),
                "Chrome/Chromium resolved under Snap; CDP automation may fail. Prefer APT/RPM install or: config set chrome_path /path/to/chrome"
            );
        }
        BrowserSandbox::Flatpak => {
            tracing::warn!(
                path = %path.display(),
                "Chrome/Chromium resolved under Flatpak; host /tmp and user-data-dir may be blocked. Prefer system package or config set chrome_path"
            );
        }
    }
}
