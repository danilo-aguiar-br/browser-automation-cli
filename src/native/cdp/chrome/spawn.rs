// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chrome self-spawn: own the child, then connect over CDP.
//!
//! # Why not `Browser::launch`
//!
//! `chromiumoxide::Browser::launch` forks Chrome internally and keeps the
//! `Child` private, so the product never learns the pid. `chrome_pid()` therefore
//! returned `None`, the lifecycle ledger recorded no residual kill target, and a
//! `SIGKILL` of the CLI orphaned the whole browser tree. Forking here restores
//! the pid *and* the process group, and the connection is then established with
//! `Browser::connect_with_config`.
//!
//! # HandlerConfig parity
//!
//! `Browser::connect` alone would silently swap the handler configuration:
//! `HandlerConfig::default()` has `viewport: None`, while the `BrowserConfig`
//! builder defaults to `Some(Viewport { 800x600 })`. Losing the viewport changes
//! what every screenshot and layout query sees. [`handler_config`] therefore
//! reconstructs the exact values the builder path produced.

use std::path::PathBuf;
use std::time::Duration;

use chromiumoxide::browser::Browser;
use chromiumoxide::handler::viewport::Viewport;
use chromiumoxide::handler::HandlerConfig;
use chromiumoxide::Handler as OxideHandler;

use super::process::ChromeProcess;
use super::{build_chrome_args, find_chrome, LaunchOptions};
use crate::native::cdp::spawn::{
    reserve_loopback_port, spawn_guarded, start_log_drainers, wait_for_cdp_ready, ReadinessBudget,
    SpawnRequest,
};

/// A self-spawned Chrome plus the CDP handles bound to it.
pub struct ChromeLaunch {
    /// Live browser handle used to issue commands.
    pub browser: Browser,
    /// Event pump. It must be driven for the connection to make progress.
    pub handler: OxideHandler,
    /// The child this invocation owns and must reap.
    pub process: ChromeProcess,
    /// DevTools websocket URL of this browser.
    pub ws_url: String,
    /// Temp user-data-dir created for this one-shot (cleanup after FINALIZE).
    pub temp_user_data_dir: Option<PathBuf>,
}

/// Startup budget for the readiness poll.
fn startup_timeout() -> Duration {
    crate::xdg::policy::policy_secs(crate::xdg::policy::key::CHROME_STARTUP_TIMEOUT_SECS)
}

/// Handler configuration equal to what `Browser::launch` derived from `BrowserConfig`.
///
/// `ignore_https_errors` is `true` on both paths: the builder default is `true`
/// and the previous code never overrode it. The product enforces the operator's
/// choice separately via `Security.setIgnoreCertificateErrors`, so keeping the
/// handler value as-is preserves behaviour exactly rather than tightening it as
/// a side effect of the migration.
fn handler_config(options: &LaunchOptions) -> HandlerConfig {
    let (width, height) = options.viewport_size.unwrap_or((
        crate::xdg::policy::policy_u32(crate::xdg::policy::key::DEFAULT_VIEWPORT_WIDTH),
        crate::xdg::policy::policy_u32(crate::xdg::policy::key::DEFAULT_VIEWPORT_HEIGHT),
    ));
    HandlerConfig {
        ignore_https_errors: true,
        ignore_invalid_messages: true,
        viewport: Some(Viewport {
            width,
            height,
            ..Viewport::default()
        }),
        ..HandlerConfig::default()
    }
}

/// Replace the wildcard debugging port with the one reserved for this launch.
///
/// `build_chrome_args` emits `--remote-debugging-port=0` so Chrome picks a port
/// and announces it on stderr. That announcement is the only channel
/// `Browser::launch` had; a self-spawn does better by choosing the port up front
/// and proving it with a CDP probe.
fn pin_debugging_port(args: &mut Vec<String>, port: u16) {
    args.retain(|a| !a.starts_with("--remote-debugging-port="));
    args.push(format!("--remote-debugging-port={port}"));
    // Bind loopback only: the port is an unauthenticated full-control CDP socket.
    args.retain(|a| !a.starts_with("--remote-debugging-address="));
    args.push(format!(
        "--remote-debugging-address={}",
        crate::constants::LOOPBACK_HOST
    ));
}

/// BORN: fork Chrome, wait for CDP, and attach.
pub async fn launch_self_spawned(options: &LaunchOptions) -> Result<ChromeLaunch, String> {
    let mut chrome_args = build_chrome_args(options)?;

    if let Some(ref dir) = chrome_args.temp_user_data_dir {
        crate::concurrency::create_dir_all_blocking(dir.clone())
            .await
            .map_err(|e| format!("Failed to create temp profile dir: {e}"))?;
        // GAP-052: stamp the owning CLI pid so residual GC resolves liveness by
        // exact pid instead of substring-matching whole command lines.
        if let Err(e) = crate::residual::write_owner_pid(dir) {
            tracing::debug!(error = %e, dir = %dir.display(), "owner-pid marker not written");
        }
    }

    let executable = options
        .executable_path
        .as_ref()
        .map(PathBuf::from)
        .or_else(find_chrome)
        .ok_or_else(|| {
            "Chrome/Chromium not found. Install it or set: config set chrome_path <PATH>"
                .to_string()
        })?;
    if !crate::platform::is_spawn_safe_binary(&executable) {
        return Err(format!(
            "Chrome path is not a safe spawn binary (reject .bat/.cmd/.ps1 or missing): {}",
            executable.display()
        ));
    }

    let port = reserve_loopback_port()?;
    pin_debugging_port(&mut chrome_args.args, port);

    let request = SpawnRequest {
        program: executable,
        args: chrome_args.args.clone(),
    };
    // The fork itself blocks; keep it off the Tokio worker (the guard thread is
    // what actually owns the child, so `spawn_blocking` only carries the wait).
    let guarded = tokio::task::spawn_blocking(move || spawn_guarded(request))
        .await
        .map_err(|e| format!("Chrome spawn task failed: {e}"))??;
    let mut child = guarded.child;
    let pgid = guarded.pgid;

    let (logs, drainers) = match start_log_drainers(&mut child) {
        Ok(v) => v,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };

    let ws_url = match wait_for_cdp_ready(
        "Chrome",
        &mut child,
        port,
        &logs,
        ReadinessBudget {
            startup: startup_timeout(),
            // Poll cadence is engine-independent: these name a readiness slice,
            // not a Lightpanda policy, and the Chrome budget above is what
            // actually differs between the two engines.
            ready_slice: Duration::from_millis(crate::constants::LIGHTPANDA_READY_SLICE_MS),
            poll_interval: Duration::from_millis(crate::constants::LIGHTPANDA_POLL_INTERVAL_MS),
            discovery_timeout: Duration::from_millis(
                crate::constants::LIGHTPANDA_DISCOVERY_TIMEOUT_MS,
            ),
        },
    )
    .await
    {
        Ok(url) => url,
        Err(e) => {
            // Reap through the owner so the drainers are joined exactly once.
            ChromeProcess::new(child, pgid, drainers).kill();
            return Err(e);
        }
    };

    let process = ChromeProcess::new(child, pgid, drainers);
    let (browser, handler) = Browser::connect_with_config(&ws_url, handler_config(options))
        .await
        .map_err(|e| format!("chromiumoxide Browser::connect_with_config: {e}"))?;

    Ok(ChromeLaunch {
        browser,
        handler,
        process,
        ws_url,
        temp_user_data_dir: chrome_args.temp_user_data_dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_debugging_port_replaces_the_wildcard() {
        let mut args = vec![
            "--remote-debugging-port=0".to_string(),
            "--no-first-run".to_string(),
        ];
        pin_debugging_port(&mut args, 45_123);
        assert!(args.iter().any(|a| a == "--remote-debugging-port=45123"));
        assert!(
            !args.iter().any(|a| a == "--remote-debugging-port=0"),
            "the wildcard port must not survive: Chrome honors the last switch \
             but leaving both makes the argv ambiguous"
        );
        assert!(args.iter().any(|a| a == "--no-first-run"));
    }

    #[test]
    fn pin_debugging_port_forces_loopback_bind() {
        let mut args = vec!["--remote-debugging-address=0.0.0.0".to_string()];
        pin_debugging_port(&mut args, 1);
        assert!(!args.iter().any(|a| a.ends_with("0.0.0.0")));
        assert!(args.iter().any(|a| a
            == &format!(
                "--remote-debugging-address={}",
                crate::constants::LOOPBACK_HOST
            )));
    }

    #[test]
    fn handler_config_keeps_a_viewport() {
        let cfg = handler_config(&LaunchOptions::default());
        let viewport = cfg
            .viewport
            .expect("connect must not drop the viewport that Browser::launch set");
        assert!(viewport.width > 0 && viewport.height > 0);
    }

    #[test]
    fn handler_config_honors_explicit_viewport_size() {
        let options = LaunchOptions {
            viewport_size: Some((1280, 720)),
            ..Default::default()
        };
        let viewport = handler_config(&options).viewport.expect("viewport present");
        assert_eq!((viewport.width, viewport.height), (1280, 720));
    }
}
