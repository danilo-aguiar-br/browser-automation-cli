// SPDX-License-Identifier: MIT OR Apache-2.0
//! Host environment probe (WSL / container / CI / Termux / sandbox flags).

use super::detect::{detect_ci, detect_container, detect_termux, detect_wsl};

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
