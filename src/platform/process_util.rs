// SPDX-License-Identifier: MIT OR Apache-2.0
//! Safe external process capture (timeout, explicit Stdio, BatBadBut defense).
//!
//! Product law: every short-lived domain child (lighthouse, ffmpeg, doctor probe)
//! uses this helper so stdin/stdout/stderr policy, wall-clock deadline, and reap
//! are DRY. Long-lived Lightpanda keeps its own RAII + dual drainers.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use super::path_util::is_executable_file;

/// Failure modes for [`run_capture_with_timeout`].
#[derive(Debug)]
pub enum ProcessCaptureError {
    /// `Command::spawn` failed.
    Spawn(std::io::Error),
    /// Child exceeded the wall-clock deadline and was killed+reaped.
    Timeout,
    /// Wait / pipe drain failed after spawn.
    Wait(std::io::Error),
}

impl std::fmt::Display for ProcessCaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(e) => write!(f, "spawn failed: {e}"),
            Self::Timeout => write!(f, "process timed out"),
            Self::Wait(e) => write!(f, "wait failed: {e}"),
        }
    }
}

impl std::error::Error for ProcessCaptureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn(e) | Self::Wait(e) => Some(e),
            Self::Timeout => None,
        }
    }
}

/// True when `path` is a regular executable that is safe to spawn with untrusted args.
///
/// Rejects Windows script hosts (`.bat` / `.cmd` / `.ps1`) as defense-in-depth for
/// CVE-2024-24576 (BatBadBut), even on Rust ≥ 1.77.2. Domain tools must be native
/// binaries (or non-script Unix executables).
pub fn is_spawn_safe_binary(path: &Path) -> bool {
    if !is_executable_file(path) {
        return false;
    }
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        // Non-UTF8 names: allow on Unix if executable; on Windows reject unknown.
        return !cfg!(windows);
    };
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".bat") || lower.ends_with(".cmd") || lower.ends_with(".ps1") {
        return false;
    }
    true
}

/// True when `arg` contains an embedded NUL (Unix truncates; reject).
pub fn arg_contains_nul(arg: impl AsRef<OsStr>) -> bool {
    arg.as_ref().as_encoded_bytes().contains(&0)
}

/// Run `cmd` with explicit capture Stdio and a wall-clock timeout.
///
/// # Stdio policy (rules: explicit, never inherit in automation)
///
/// - `stdin` = [`Stdio::null`] — do not steal the agent/parent stdin
/// - `stdout` = [`Stdio::piped`] — capture for status / diagnostics
/// - `stderr` = [`Stdio::piped`] — capture for error messages
///
/// # Lifecycle
///
/// Spawns, polls [`std::process::Child::try_wait`] until exit or deadline, then
/// either [`Child::wait_with_output`](std::process::Child::wait_with_output) or
/// `kill`+`wait`. Every path reaps the child (no zombies).
///
/// Callers on the Tokio runtime must wrap this in `spawn_blocking`.
pub fn run_capture_with_timeout(
    cmd: &mut Command,
    timeout: Duration,
) -> Result<Output, ProcessCaptureError> {
    // Explicit Stdio — do not rely on Command::output defaults.
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(ProcessCaptureError::Spawn)?;
    let deadline = Instant::now() + timeout;
    let poll = Duration::from_millis(crate::constants::PLATFORM_CHILD_POLL_MS);

    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                // Reaped status is retained; wait_with_output drains pipes.
                return child.wait_with_output().map_err(ProcessCaptureError::Wait);
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(poll);
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProcessCaptureError::Timeout);
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProcessCaptureError::Wait(e));
            }
        }
    }
}

/// Wait for `child` to exit until `timeout`, then kill+reap if still alive.
///
/// Used by long-lived children (Lightpanda) after cooperative Browser.close.
pub fn wait_child_or_kill(child: &mut std::process::Child, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    let poll = Duration::from_millis(crate::constants::PLATFORM_CHILD_POLL_MS);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(poll);
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn rejects_windows_script_extensions() {
        assert!(!is_spawn_safe_binary(Path::new("lighthouse.bat")));
        assert!(!is_spawn_safe_binary(Path::new("tool.CMD")));
        assert!(!is_spawn_safe_binary(Path::new("run.ps1")));
    }

    #[test]
    fn arg_nul_detection() {
        assert!(arg_contains_nul("a\0b"));
        assert!(!arg_contains_nul("https://example.com"));
    }

    #[cfg(unix)]
    #[test]
    fn timeout_kills_sleep() {
        let mut cmd = Command::new("/bin/sleep");
        cmd.arg("30");
        let err = run_capture_with_timeout(&mut cmd, Duration::from_millis(200)).unwrap_err();
        assert!(matches!(err, ProcessCaptureError::Timeout));
    }

    #[cfg(unix)]
    #[test]
    fn capture_true_succeeds() {
        let mut cmd = Command::new("/bin/true");
        let out = run_capture_with_timeout(&mut cmd, Duration::from_secs(2)).expect("true");
        assert!(out.status.success());
    }

    #[test]
    fn missing_path_not_safe() {
        assert!(!is_spawn_safe_binary(Path::new(
            "/no/such/browser-automation-cli-bin-xyz"
        )));
        let _ = PathBuf::from("x");
    }
}
