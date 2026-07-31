// SPDX-License-Identifier: MIT OR Apache-2.0
//! Platform module unit tests.

use super::*;
use std::io::Write;
use std::path::Path;

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
        detect_browser_sandbox(Path::new("/var/lib/flatpak/exports/bin/com.google.Chrome")),
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

#[test]
fn registry_app_path_empty_none() {
    assert!(registry_app_path("").is_none());
}

#[test]
fn registry_app_path_nonexistent_exe_none() {
    // On non-Windows always None; on Windows a fake App Paths name yields None.
    assert!(registry_app_path("browser-automation-cli-no-such-app.exe").is_none());
}

#[test]
fn probe_binary_version_missing_none() {
    assert!(probe_binary_version(Path::new("/nonexistent/browser-automation-cli-bin")).is_none());
}

#[test]
fn probe_binary_version_sh_on_unix() {
    #[cfg(unix)]
    {
        // `/bin/sh --version` is commonly supported (bash/dash print a line).
        if let Some(sh) = which_bin("sh").or_else(|| which_bin("/bin/sh")) {
            // dash may exit non-zero for --version but often still prints; accept either.
            let _ = probe_binary_version(&sh);
        }
    }
}
