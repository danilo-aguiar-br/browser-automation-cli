// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lightpanda binary discovery, launch, readiness, and error formatting.
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::super::discovery::discover_cdp_url_with_timeout;
use crate::xdg::resolve_lightpanda_session_timeout_secs;

use super::process::{
    kill_and_reap, lightpanda_discovery_timeout, lightpanda_poll_interval, lightpanda_ready_slice,
    lightpanda_startup_timeout, LightpandaProcess,
};

/// Launch knobs for the Lightpanda engine.
///
/// Deliberately narrower than [`LaunchOptions`](crate::native::cdp::chrome::LaunchOptions):
/// Lightpanda is a lighter engine and does not accept most Chrome switches.
#[derive(Default)]
pub struct LightpandaLaunchOptions {
    /// Absolute path to the binary. `None` runs host discovery.
    pub executable_path: Option<String>,
    /// Proxy URL to serve through.
    pub proxy: Option<String>,
    /// TCP port to bind. `None` picks a free one, which is what keeps two
    /// concurrent invocations from fighting over a fixed port.
    pub port: Option<u16>,
}

pub(crate) fn build_lightpanda_serve_args(port: u16, proxy: Option<&str>) -> Vec<String> {
    let mut args = vec![
        "serve".to_string(),
        "--host".to_string(),
        crate::constants::LOOPBACK_HOST.to_string(),
        "--port".to_string(),
        port.to_string(),
        "--timeout".to_string(),
        resolve_lightpanda_session_timeout_secs().to_string(),
    ];

    if let Some(proxy) = proxy {
        args.push("--http_proxy".to_string());
        args.push(proxy.to_string());
    }

    args
}

/// Bounded stdout/stderr ring for launch diagnostics.
///
/// # Interior mutability
///
/// Reader threads push lines while the launcher snapshots on failure. Uses
/// `std::sync::Mutex` (short critical sections, no `.await`). Poison recovered
/// via `into_inner` so a panic in one stream cannot hide the other.
#[derive(Clone, Default)]
pub(crate) struct LaunchLogBuffer {
    stdout: Arc<Mutex<VecDeque<String>>>,
    stderr: Arc<Mutex<VecDeque<String>>>,
}

impl LaunchLogBuffer {
    pub(crate) fn push_stdout(&self, line: String) {
        push_bounded(&self.stdout, line);
    }

    pub(crate) fn push_stderr(&self, line: String) {
        push_bounded(&self.stderr, line);
    }

    pub(crate) fn snapshot_stdout(&self) -> Vec<String> {
        crate::sync_util::lock_recover(&self.stdout)
            .iter()
            .cloned()
            .collect()
    }

    fn snapshot_stderr(&self) -> Vec<String> {
        crate::sync_util::lock_recover(&self.stderr)
            .iter()
            .cloned()
            .collect()
    }
}

pub(crate) fn push_bounded(buffer: &Mutex<VecDeque<String>>, line: String) {
    let mut guard = crate::sync_util::lock_recover(buffer);
    if guard.len()
        >= crate::xdg::policy::policy_usize(crate::xdg::policy::key::LIGHTPANDA_MAX_LOG_LINES)
    {
        guard.pop_front();
    }
    guard.push_back(line);
}

/// Locate a Lightpanda binary on the host, or `None` when there is none.
pub fn find_lightpanda() -> Option<PathBuf> {
    // Pure PATH walk — never shell out to `which`/`where` (multiplatform rules).
    if let Some(p) = crate::platform::which_bin("lightpanda") {
        return Some(p);
    }

    if let Some(home) = dirs::home_dir() {
        let candidates = [
            home.join(".lightpanda/lightpanda"),
            home.join(".local/bin/lightpanda"),
        ];
        for c in &candidates {
            if crate::platform::is_executable_file(c) {
                return Some(c.clone());
            }
        }
    }

    None
}

/// Spawn Lightpanda and wait until its DevTools endpoint answers.
///
/// Readiness is polled rather than assumed: the process exists before it can
/// serve, and connecting too early fails in a way that looks like "no engine".
pub async fn launch_lightpanda(
    options: &LightpandaLaunchOptions,
) -> Result<LightpandaProcess, String> {
    let binary_path = match &options.executable_path {
        Some(p) => PathBuf::from(p),
        None => find_lightpanda().ok_or(
            "Lightpanda not found. Install it from https://lightpanda.io/docs/open-source/installation or use --executable-path.",
        )?,
    };
    if !crate::platform::is_spawn_safe_binary(&binary_path) {
        return Err(format!(
            "Lightpanda path is not a safe spawn binary (reject .bat/.cmd/.ps1 or missing): {}",
            binary_path.display()
        ));
    }

    let port = match options.port {
        Some(p) => p,
        None => TcpListener::bind((crate::constants::LOOPBACK_HOST, 0))
            .and_then(|l| l.local_addr())
            .map(|a| a.port())
            .map_err(|e| format!("Failed to find an available port for Lightpanda: {e}"))?,
    };
    let args = build_lightpanda_serve_args(port, options.proxy.as_deref());

    // Explicit Stdio: null stdin (automation), piped stdout/stderr for dual drainers.
    let mut child = Command::new(&binary_path)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to launch Lightpanda at {binary_path:?}: {e}"))?;

    let (log_buffer, log_drainers) = match start_log_drainers(&mut child) {
        Ok(v) => v,
        Err(e) => {
            // start_log_drainers already reaps on its internal error paths; re-reap is harmless
            // only if the child is still live (e.g. unexpected error shape).
            kill_and_reap(&mut child);
            return Err(e);
        }
    };

    let ws_url = match wait_for_lightpanda_ready(
        &mut child,
        port,
        &log_buffer,
        lightpanda_startup_timeout(),
    )
    .await
    {
        Ok(url) => url,
        Err(e) => {
            kill_and_reap(&mut child);
            return Err(e);
        }
    };

    Ok(LightpandaProcess {
        child: Some(child),
        ws_url,
        log_drainers,
    })
}

pub(crate) fn start_log_drainers(
    child: &mut Child,
) -> Result<(LaunchLogBuffer, Vec<std::thread::JoinHandle<()>>), String> {
    let stdout = child.stdout.take().ok_or_else(|| {
        kill_and_reap(child);
        "Failed to capture Lightpanda stdout".to_string()
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        kill_and_reap(child);
        "Failed to capture Lightpanda stderr".to_string()
    })?;

    let logs = LaunchLogBuffer::default();
    let stdout_logs = logs.clone();
    let stderr_logs = logs.clone();

    let stdout_handle =
        std::thread::spawn(move || drain_reader(stdout, move |line| stdout_logs.push_stdout(line)));
    let stderr_handle =
        std::thread::spawn(move || drain_reader(stderr, move |line| stderr_logs.push_stderr(line)));

    Ok((logs, vec![stdout_handle, stderr_handle]))
}

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

pub(crate) async fn wait_for_lightpanda_ready(
    child: &mut Child,
    port: u16,
    logs: &LaunchLogBuffer,
    startup_timeout: Duration,
) -> Result<String, String> {
    let deadline = std::time::Instant::now() + startup_timeout;
    let mut last_probe_error = None;

    loop {
        if let Ok(Some(status)) = child.try_wait() {
            // Give the drainer threads a brief window to flush the last log lines
            // before we snapshot them.  This is best-effort: lines written just
            // before exit may still be missing, but the most useful output (early
            // startup errors) will already be in the buffer.
            tokio::time::sleep(lightpanda_ready_slice()).await;
            return Err(lightpanda_launch_error(
                &format!("Lightpanda exited before CDP became ready (status: {status})"),
                logs,
                last_probe_error.as_deref(),
            ));
        }

        match discover_cdp_url_with_timeout(
            crate::constants::LOOPBACK_HOST,
            port,
            None,
            lightpanda_discovery_timeout(),
        )
        .await
        {
            Ok(ws_url) => return Ok(ws_url),
            Err(err) => last_probe_error = Some(err),
        }

        if std::time::Instant::now() >= deadline {
            return Err(lightpanda_launch_error(
                &format!(
                    "Timed out after {}ms waiting for Lightpanda CDP endpoint on port {}",
                    startup_timeout.as_millis(),
                    port
                ),
                logs,
                last_probe_error.as_deref(),
            ));
        }

        tokio::time::sleep(lightpanda_poll_interval()).await;
    }
}

pub(crate) fn lightpanda_launch_error(
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
            "Lightpanda stderr (last {} lines):\n  {}",
            stderr_lines.len(),
            stderr_lines.join("\n  ")
        ));
    }

    if !stdout_lines.is_empty() {
        details.push(format!(
            "Lightpanda stdout (last {} lines):\n  {}",
            stdout_lines.len(),
            stdout_lines.join("\n  ")
        ));
    }

    if details.is_empty() {
        format!("{message} (no stdout/stderr output from Lightpanda)")
    } else {
        format!("{}\n{}", message, details.join("\n"))
    }
}
