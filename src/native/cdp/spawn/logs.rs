// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bounded stdout/stderr rings shared by every self-spawned CDP engine.
//!
//! Extracted from the Lightpanda launcher so the Chrome self-spawn path reports
//! startup failures with the same evidence instead of growing a second copy.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::process::Child;
use std::sync::{Arc, Mutex};

/// Bounded stdout/stderr ring for launch diagnostics.
///
/// # Interior mutability
///
/// Reader threads push lines while the launcher snapshots on failure. Uses
/// `std::sync::Mutex` (short critical sections, no `.await`). Poison is
/// recovered via [`crate::sync_util::lock_recover`] so a panic in one stream
/// cannot hide the other.
#[derive(Clone, Default)]
pub struct LaunchLogBuffer {
    stdout: Arc<Mutex<VecDeque<String>>>,
    stderr: Arc<Mutex<VecDeque<String>>>,
}

impl LaunchLogBuffer {
    /// Record one stdout line, evicting the oldest when the ring is full.
    pub fn push_stdout(&self, line: String) {
        push_bounded(&self.stdout, line);
    }

    /// Record one stderr line, evicting the oldest when the ring is full.
    pub fn push_stderr(&self, line: String) {
        push_bounded(&self.stderr, line);
    }

    /// Copy of the retained stdout lines, oldest first.
    #[must_use]
    pub fn snapshot_stdout(&self) -> Vec<String> {
        crate::sync_util::lock_recover(&self.stdout)
            .iter()
            .cloned()
            .collect()
    }

    /// Copy of the retained stderr lines, oldest first.
    #[must_use]
    pub fn snapshot_stderr(&self) -> Vec<String> {
        crate::sync_util::lock_recover(&self.stderr)
            .iter()
            .cloned()
            .collect()
    }
}

/// Push `line`, dropping the oldest entry once the configured cap is reached.
pub(crate) fn push_bounded(buffer: &Mutex<VecDeque<String>>, line: String) {
    let mut guard = crate::sync_util::lock_recover(buffer);
    if guard.len()
        >= crate::xdg::policy::policy_usize(crate::xdg::policy::key::LIGHTPANDA_MAX_LOG_LINES)
    {
        guard.pop_front();
    }
    guard.push_back(line);
}

/// Take the child's piped stdout/stderr and drain both on dedicated threads.
///
/// Returns the shared ring plus the join handles the owner must join **after**
/// the child is reaped, so the pipes reach EOF instead of blocking the readers.
pub(crate) fn start_log_drainers(
    child: &mut Child,
) -> Result<(LaunchLogBuffer, Vec<std::thread::JoinHandle<()>>), String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture engine stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture engine stderr".to_string())?;

    let logs = LaunchLogBuffer::default();
    let stdout_logs = logs.clone();
    let stderr_logs = logs.clone();

    let stdout_handle =
        std::thread::spawn(move || drain_reader(stdout, move |line| stdout_logs.push_stdout(line)));
    let stderr_handle =
        std::thread::spawn(move || drain_reader(stderr, move |line| stderr_logs.push_stderr(line)));

    Ok((logs, vec![stdout_handle, stderr_handle]))
}

/// Join drainer threads, waiting at most `grace` for all of them together.
///
/// A drainer ends only when EVERY process holding the child's stdout or
/// stderr closes it, and a descendant can inherit those: Fedora's
/// `chromium-browser.sh` routes both through `cat` helpers. Measured: an
/// unbounded join held a `--timeout 10` command for 30 seconds. A thread still
/// running at the deadline is left detached; it blocks only on a pipe that
/// dies with this one-shot process.
pub(crate) fn join_drainers_within(
    handles: Vec<std::thread::JoinHandle<()>>,
    grace: std::time::Duration,
) {
    let deadline = std::time::Instant::now() + grace;
    let poll = std::time::Duration::from_millis(crate::xdg::policy::policy_u64(
        crate::xdg::policy::key::PLATFORM_CHILD_POLL_MS,
    ));
    while handles.iter().any(|h| !h.is_finished()) {
        if std::time::Instant::now() >= deadline {
            tracing::warn!(
                target: "browser_automation_cli::spawn",
                grace_ms = u64::try_from(grace.as_millis()).unwrap_or(u64::MAX),
                "log drainers still blocked at teardown; left detached \
                 (a descendant of the engine may still hold its output open)"
            );
            return;
        }
        std::thread::sleep(poll);
    }
    for handle in handles {
        let _ = handle.join();
    }
}

/// Read `reader` line by line until EOF or the first I/O error.
pub(crate) fn drain_reader<R, F>(reader: R, mut push: F)
where
    R: std::io::Read,
    F: FnMut(String),
{
    for line in BufReader::new(reader).lines() {
        match line {
            Ok(line) => push(line),
            Err(_) => break,
        }
    }
}

/// Render a launch failure with the probe error and whatever the engine printed.
pub(crate) fn launch_error(
    engine: &str,
    message: &str,
    logs: &LaunchLogBuffer,
    last_probe_error: Option<&str>,
) -> String {
    let stdout_lines = logs.snapshot_stdout();
    let stderr_lines = logs.snapshot_stderr();
    let mut details = Vec::new();

    if let Some(err) = last_probe_error {
        details.push(format!("Last probe error: {err}"));
    }

    if !stderr_lines.is_empty() {
        details.push(format!(
            "{engine} stderr (last {} lines):\n  {}",
            stderr_lines.len(),
            stderr_lines.join("\n  ")
        ));
    }

    if !stdout_lines.is_empty() {
        details.push(format!(
            "{engine} stdout (last {} lines):\n  {}",
            stdout_lines.len(),
            stdout_lines.join("\n  ")
        ));
    }

    if details.is_empty() {
        format!("{message} (no stdout/stderr output from {engine})")
    } else {
        format!("{}\n{}", message, details.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Teardown returns on time while a child still holds the drained pipe.
    ///
    /// Reproduces the measured hang one layer out from the DevTools pipe: the
    /// `sleep` keeps its stdout open, so the drainer never sees end-of-file and
    /// an unbounded join would wait for as long as `sleep` lives.
    #[test]
    #[cfg(unix)]
    fn drainers_are_left_detached_when_the_output_stays_open() {
        let Some(program) = crate::platform::which_bin("sleep") else {
            crate::test_utils::skip_unit_test("log_drainers", "sleep not found on this host.");
            return;
        };
        let mut holder = std::process::Command::new(program)
            .arg("30")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("sleep spawns");
        let (_logs, drainers) = start_log_drainers(&mut holder).expect("drainers start");
        let grace = std::time::Duration::from_millis(300);
        let started = std::time::Instant::now();
        join_drainers_within(drainers, grace);
        let elapsed = started.elapsed();
        let _ = holder.kill();
        let _ = holder.wait();
        assert!(
            elapsed < grace + std::time::Duration::from_secs(2),
            "the join blocked for {elapsed:?} while the output was still open"
        );
    }
}
