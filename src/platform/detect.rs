// SPDX-License-Identifier: MIT OR Apache-2.0
//! Host environment detectors (WSL, container, CI, Termux).
//!
//! Product law: these read host OS markers only — never product config knobs.

use std::path::{Path, PathBuf};

pub(crate) fn detect_wsl() -> bool {
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

pub(crate) fn detect_container() -> bool {
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

pub(crate) fn detect_ci() -> bool {
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

pub(crate) fn detect_termux() -> bool {
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
