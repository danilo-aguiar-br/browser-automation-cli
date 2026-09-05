// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lightpanda binary discovery, launch, readiness, and error formatting.
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::native::cdp::spawn::{reserve_loopback_port, LaunchLogBuffer};
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
    build_lightpanda_serve_args_with(port, proxy, resolve_lightpanda_session_timeout_secs())
}

/// Parameterized core: the same argv against an explicit session timeout.
///
/// Exists so a test can assert a LITERAL. Asserting the facade forced the test
/// to spell the expected timeout as `resolve_lightpanda_session_timeout_secs()`
/// — the very call the function under test makes — so the two sides moved
/// together and the assertion held for any value, including a wrong one.
pub(crate) fn build_lightpanda_serve_args_with(
    port: u16,
    proxy: Option<&str>,
    timeout_secs: u64,
) -> Vec<String> {
    let mut args = vec![
        "serve".to_string(),
        "--host".to_string(),
        crate::constants::LOOPBACK_HOST.to_string(),
        "--port".to_string(),
        port.to_string(),
        "--timeout".to_string(),
        timeout_secs.to_string(),
    ];

    if let Some(proxy) = proxy {
        args.push("--http_proxy".to_string());
        args.push(proxy.to_string());
    }

    args
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
///
/// # Errors
///
/// Returns a human-readable reason when `options.executable_path` is `None`
/// and [`find_lightpanda`] finds nothing, when the resolved path is not a
/// spawn-safe binary (`.bat` / `.cmd` / `.ps1` or missing), when no loopback
/// port can be reserved, when the `serve` process cannot be spawned, when the
/// stdout/stderr drainers cannot be started, or when the DevTools endpoint
/// does not answer before `lightpanda_startup_timeout` — the last case carries
/// the last probe error plus the captured process output. Every failure after
/// the spawn reaps the child before returning.
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
        None => reserve_loopback_port()?,
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

/// Take the child's pipes and drain both, reaping the child on failure.
///
/// Wraps the engine-agnostic drainer so the Lightpanda contract of "never leave
/// a half-started child behind" is preserved at this call site.
pub(crate) fn start_log_drainers(
    child: &mut Child,
) -> Result<(LaunchLogBuffer, Vec<std::thread::JoinHandle<()>>), String> {
    crate::native::cdp::spawn::start_log_drainers(child).map_err(|e| {
        kill_and_reap(child);
        e.replace("engine", "Lightpanda")
    })
}

/// Poll the Lightpanda CDP endpoint until it answers, the child dies, or timeout.
pub(crate) async fn wait_for_lightpanda_ready(
    child: &mut Child,
    port: u16,
    logs: &LaunchLogBuffer,
    startup_timeout: Duration,
) -> Result<String, String> {
    crate::native::cdp::spawn::wait_for_cdp_ready(
        "Lightpanda",
        child,
        port,
        logs,
        crate::native::cdp::spawn::ReadinessBudget {
            startup: startup_timeout,
            ready_slice: lightpanda_ready_slice(),
            poll_interval: lightpanda_poll_interval(),
            discovery_timeout: lightpanda_discovery_timeout(),
        },
    )
    .await
}

/// Render a Lightpanda launch failure with probe error and captured output.
///
/// Production formatting now happens inside
/// [`crate::native::cdp::spawn::wait_for_cdp_ready`]; this stays as the named
/// entry point the Lightpanda tests assert the message shape through.
#[cfg(test)]
pub(crate) fn lightpanda_launch_error(
    message: &str,
    logs: &LaunchLogBuffer,
    last_probe_error: Option<&str>,
) -> String {
    crate::native::cdp::spawn::launch_error("Lightpanda", message, logs, last_probe_error)
}
