// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cross-platform host helpers (PATH lookup, console, sandbox, environment).
//!
//! Product law: Chrome CDP is **host-only** (not WASM). Browser path override is
//! XDG `chrome_path` / CLI launch options — not product env vars.
//!
//! Rules: `docs_rules/rules_rust_multiplataforma_sistemas_operacionais.md`.

use std::path::{Path, PathBuf};

/// Result of probing the host for container / CI / WSL / Termux / sandbox env.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostEnvironment {
    /// Running under Windows Subsystem for Linux (`WSL_DISTRO_NAME` / `/proc` markers).
    pub wsl: bool,
    /// Docker / Podman / k8s container markers.
    pub container: bool,
    /// Common CI env keys present (local observability only; never product settings).
    pub ci: bool,
    /// Android Termux (`PREFIX` under `/data/data/com.termux`).
    pub termux: bool,
    /// Process is inside a Flatpak sandbox (`FLATPAK_ID`).
    pub flatpak: bool,
    /// Process is inside a Snap sandbox (`SNAP`).
    pub snap: bool,
}

impl HostEnvironment {
    /// Probe the current process environment and filesystem markers once.
    pub fn detect() -> Self {
        Self {
            wsl: detect_wsl(),
            container: detect_container(),
            ci: detect_ci(),
            termux: detect_termux(),
            flatpak: std::env::var_os("FLATPAK_ID").is_some(),
            snap: std::env::var_os("SNAP").is_some(),
        }
    }

    /// Compact label for doctor / diagnostics JSON.
    pub fn summary(&self) -> String {
        let mut tags = Vec::with_capacity(6);
        if self.wsl {
            tags.push("wsl");
        }
        if self.container {
            tags.push("container");
        }
        if self.ci {
            tags.push("ci");
        }
        if self.termux {
            tags.push("termux");
        }
        if self.flatpak {
            tags.push("flatpak");
        }
        if self.snap {
            tags.push("snap");
        }
        if tags.is_empty() {
            "host".into()
        } else {
            tags.join("+")
        }
    }
}

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

/// Locate an executable on `$PATH` without shelling out to `which`/`where`.
///
/// On Windows, also tries `{name}.exe`. Returns the first existing regular file.
pub fn which_bin(name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    // Absolute / relative path with separators: honor directly when executable.
    let as_path = Path::new(name);
    if as_path.components().count() > 1 || as_path.is_absolute() {
        if is_executable_file(as_path) {
            return Some(as_path.to_path_buf());
        }
        return None;
    }
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let with_exe = dir.join(format!("{name}.exe"));
            if is_executable_file(&with_exe) {
                return Some(with_exe);
            }
            let with_cmd = dir.join(format!("{name}.cmd"));
            if with_cmd.is_file() {
                return Some(with_cmd);
            }
            let with_bat = dir.join(format!("{name}.bat"));
            if with_bat.is_file() {
                return Some(with_bat);
            }
        }
    }
    None
}

/// True when `path` is a regular file and (on Unix) has any execute bit.
pub fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match path.metadata() {
            Ok(meta) => meta.permissions().mode() & 0o111 != 0,
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// First existing executable among candidate paths (skips missing / non-exec).
pub fn first_existing_executable<'a, I>(candidates: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = &'a Path>,
{
    for p in candidates {
        if is_executable_file(p) {
            return Some(p.to_path_buf());
        }
    }
    None
}

/// Configure Windows console for UTF-8 (CP 65001) and virtual terminal ANSI.
///
/// No-op on non-Windows. Failures are ignored (already UTF-8 / redirected handles).
pub fn configure_console() {
    #[cfg(windows)]
    {
        configure_console_windows();
    }
}

#[cfg(windows)]
fn configure_console_windows() {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleCP, SetConsoleMode, SetConsoleOutputCP,
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
    };

    // SAFETY: Win32 console APIs are process-wide and safe at single-threaded boot.
    // CP_UTF8 = 65001. VT mode enables ANSI on modern Windows Terminal / conhost.
    // See: https://learn.microsoft.com/windows/console/console-virtual-terminal-sequences
    const CP_UTF8: u32 = 65001;
    unsafe {
        let _ = SetConsoleOutputCP(CP_UTF8);
        let _ = SetConsoleCP(CP_UTF8);
        for nstd in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let h = GetStdHandle(nstd);
            if h == INVALID_HANDLE_VALUE || h.is_null() {
                continue;
            }
            let mut mode: u32 = 0;
            if GetConsoleMode(h, &mut mode) == 0 {
                continue;
            }
            let _ = SetConsoleMode(h, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}

fn detect_wsl() -> bool {
    if std::env::var_os("WSL_DISTRO_NAME").is_some() || std::env::var_os("WSL_INTEROP").is_some() {
        return true;
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(osrelease) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
            let lower = osrelease.to_ascii_lowercase();
            if lower.contains("microsoft") || lower.contains("wsl") {
                return true;
            }
        }
    }
    false
}

fn detect_container() -> bool {
    if Path::new("/.dockerenv").exists() || Path::new("/run/.containerenv").exists() {
        return true;
    }
    if std::env::var_os("KUBERNETES_SERVICE_HOST").is_some() {
        return true;
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(cgroup) = std::fs::read_to_string("/proc/1/cgroup") {
            if cgroup.contains("docker")
                || cgroup.contains("kubepods")
                || cgroup.contains("lxc")
                || cgroup.contains("containerd")
                || cgroup.contains("podman")
            {
                return true;
            }
        }
    }
    false
}

fn detect_ci() -> bool {
    // Observability only — product settings never bind to these keys.
    const KEYS: &[&str] = &[
        "CI",
        "GITHUB_ACTIONS",
        "GITLAB_CI",
        "BUILDKITE",
        "CIRCLECI",
        "TRAVIS",
        "APPVEYOR",
        "TF_BUILD",
        "JENKINS_URL",
    ];
    KEYS.iter().any(|k| std::env::var_os(k).is_some())
}

fn detect_termux() -> bool {
    if std::env::var_os("TERMUX_VERSION").is_some() {
        return true;
    }
    if let Some(prefix) = std::env::var_os("PREFIX") {
        let p = PathBuf::from(prefix);
        if p.starts_with("/data/data/com.termux") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn host_environment_detect_does_not_panic() {
        let env = HostEnvironment::detect();
        assert!(!env.summary().is_empty());
    }

    #[test]
    fn sandbox_none_for_ordinary_path() {
        assert_eq!(
            detect_browser_sandbox(Path::new("/usr/bin/google-chrome")),
            BrowserSandbox::None
        );
    }

    #[test]
    fn sandbox_snap_by_path() {
        assert_eq!(
            detect_browser_sandbox(Path::new("/snap/bin/chromium")),
            BrowserSandbox::Snap
        );
    }

    #[test]
    fn sandbox_flatpak_by_path() {
        assert_eq!(
            detect_browser_sandbox(Path::new(
                "/var/lib/flatpak/exports/bin/com.google.Chrome"
            )),
            BrowserSandbox::Flatpak
        );
        assert_eq!(
            detect_browser_sandbox(Path::new(
                "/home/u/.local/share/flatpak/exports/bin/com.google.Chrome"
            )),
            BrowserSandbox::Flatpak
        );
    }

    #[test]
    fn which_bin_empty_name_none() {
        assert!(which_bin("").is_none());
    }

    #[test]
    fn which_bin_finds_sh_on_unix() {
        #[cfg(unix)]
        {
            // `/bin/sh` or PATH `sh` almost always present on Unix CI hosts.
            let found = which_bin("sh").or_else(|| which_bin("/bin/sh"));
            assert!(found.is_some(), "expected sh on PATH or /bin/sh");
            assert!(is_executable_file(found.as_ref().unwrap()));
        }
    }

    #[test]
    fn first_existing_skips_missing() {
        let missing = Path::new("/nonexistent/browser-automation-cli-chrome-xyz");
        let mut tmp = tempfile::NamedTempFile::new().expect("tmp");
        writeln!(tmp, "x").ok();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tmp.as_file().metadata().unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(tmp.path(), perms).unwrap();
        }
        let found = first_existing_executable([missing, tmp.path()]);
        assert_eq!(found.as_deref(), Some(tmp.path()));
    }

    #[test]
    fn browser_sandbox_as_str() {
        assert_eq!(BrowserSandbox::None.as_str(), "none");
        assert!(BrowserSandbox::Snap.is_restricted());
        assert!(!BrowserSandbox::None.is_restricted());
    }
}
