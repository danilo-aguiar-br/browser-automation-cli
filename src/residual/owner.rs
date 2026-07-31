// SPDX-License-Identifier: MIT OR Apache-2.0
//! Owner-PID marker for CLI Chrome profiles (GAP-052).
//!
//! The BORN phase stamps the owning CLI process id inside every marker profile
//! it allocates. The collector then resolves that exact pid against the live
//! process table instead of substring-matching whole command lines.
//!
//! Substring matching over `/proc/*/cmdline` is unusable as a liveness signal:
//! any editor, `rg`, or shell that merely mentions the profile path pins the
//! directory forever and residual GC never converges.

use std::path::{Path, PathBuf};

/// File name of the owner-pid marker inside a CLI profile directory.
pub const OWNER_PID_FILE: &str = ".browser-automation-cli-owner-pid";

/// Path of the owner-pid marker for `dir`.
#[must_use]
pub fn owner_pid_path(dir: &Path) -> PathBuf {
    dir.join(OWNER_PID_FILE)
}

/// Stamp the current process id into `dir` as its owner.
///
/// Best-effort: a failure only degrades the collector back to the cmdline
/// fallback, so the error is returned for tracing but never fatal.
pub fn write_owner_pid(dir: &Path) -> std::io::Result<()> {
    std::fs::write(owner_pid_path(dir), std::process::id().to_string())
}

/// Read the owner pid recorded for `dir`, if any.
///
/// Returns `None` for paths that carry no marker (Chromium side-channels and
/// profiles created before this marker existed).
#[must_use]
pub fn read_owner_pid(dir: &Path) -> Option<u32> {
    let raw = std::fs::read_to_string(owner_pid_path(dir)).ok()?;
    raw.trim().parse::<u32>().ok()
}

/// True when `dir` carries an owner-pid marker.
#[must_use]
pub fn has_owner_pid(dir: &Path) -> bool {
    owner_pid_path(dir).is_file()
}
