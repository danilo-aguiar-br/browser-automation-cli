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

use super::args::{materialize_profile_dir, profile_postmortem, reassert_profile_dir};
use super::process::ChromeProcess;
use super::{build_chrome_args, find_chrome, LaunchOptions};
use crate::native::cdp::pipe::PipeBridge;
use crate::native::cdp::spawn::{launch_error, spawn_guarded, start_log_drainers, SpawnRequest};

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
    ///
    /// The private X server, when one was started, is NOT here: it lives
    /// inside [`ChromeProcess`], which is what reaps the browser. Ownership is
    /// what enforces the teardown order — kill the display first and Chrome
    /// dies with it.
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

/// Tear a failed launch down on a blocking thread, not on the runtime thread.
///
/// The teardown blocks for up to two bounded graces (bridge threads, then log
/// drainers). Run inline, it held the thread the `--timeout` select runs on, so
/// a `--timeout 6` launch was measured ending at 9.1 s with exit 69 instead of
/// 124. Awaiting a blocking task leaves that select free to fire; the task
/// still runs to completion, because the runtime waits for blocking tasks
/// before it shuts down.
async fn kill_off_the_runtime(mut process: ChromeProcess) {
    let _ = tokio::task::spawn_blocking(move || process.kill()).await;
}

/// BORN: fork Chrome, wait for CDP, and attach.
///
/// # Errors
///
/// Returns a human-readable reason when `build_chrome_args` rejects the
/// options, when the temp profile directory cannot be created, when neither
/// `options.executable_path` nor [`find_chrome`] locates a browser, when the
/// resolved path is not a spawn-safe binary (`.bat` / `.cmd` / `.ps1` or
/// missing), when no loopback port can be reserved, when the fork itself fails
/// or its `spawn_blocking` task panics, when the stdio drainers cannot be
/// started, when Chrome does not announce a DevTools endpoint inside
/// [`CHROME_STARTUP_TIMEOUT_SECS`](crate::xdg::policy::key::CHROME_STARTUP_TIMEOUT_SECS),
/// or when `Browser::connect_with_config` cannot attach to the announced
/// websocket. A missing private X server is **not** an error: the launch
/// degrades to a plain headed window and warns.
///
/// Every failure after the fork reaps the child before returning, so a failed
/// launch leaves no residual Chrome.
pub async fn launch_self_spawned(options: &LaunchOptions) -> Result<ChromeLaunch, String> {
    // A headed Chrome on Linux draws into a private X server when one can be
    // started, so the window is genuinely rendered and genuinely invisible.
    // Failure here degrades to a plain headed window rather than failing the
    // launch: the display sharpens the disguise, it is not what makes the
    // browser work, and a missing optional package must not become an outage.
    //
    // Started BEFORE the argv is built, so the X11 pin in `build_chrome_args`
    // follows whether the server actually came up. Deciding the pin first was
    // measured forcing X11 onto a launch with no X server at all.
    let xvfb = if crate::native::cdp::xvfb::should_use_private_display(
        options.headless,
        options.no_xvfb,
    ) {
        match crate::native::cdp::xvfb::start_private_display() {
            Ok(guard) => Some(guard),
            Err(reason) => {
                tracing::warn!(
                    target: "browser_automation_cli::xvfb",
                    reason = %reason,
                    "private display unavailable; launching headed on the current display"
                );
                None
            }
        }
    } else {
        None
    };
    crate::browser_policy::record_display_outcome(options.headless, xvfb.is_some());
    let options = &LaunchOptions {
        private_display: xvfb.is_some(),
        ..options.clone()
    };

    let mut chrome_args = build_chrome_args(options)?;

    materialize_profile_dir(&chrome_args).await?;

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

    // DevTools over a pipe: no TCP listener for other local processes to find.
    // See `native::cdp::pipe` for the unauthenticated port this replaces.
    let (pipe_child, pipe_parent) = crate::native::cdp::pipe::create()
        .map_err(|e| format!("DevTools pipe could not be created: {e}"))?;
    crate::native::cdp::pipe::use_pipe_transport(&mut chrome_args.args, &pipe_child);
    crate::native::cdp::chrome::publish_launch_args(&chrome_args.args);

    // `XAUTHORITY` carries the cookie the private server now demands.
    let envs = xvfb
        .as_ref()
        .map(|g| {
            vec![
                ("DISPLAY".to_string(), g.display_value()),
                ("XAUTHORITY".to_string(), g.xauthority_value()),
            ]
        })
        .unwrap_or_default();
    // `--ozone-platform=x11` is what keeps the window inside Xvfb; see
    // `pin_x11_for_private_display`. The variable is dropped as well so the
    // child sees no Wayland session the switch contradicts. Only when the
    // private display actually started, so the degraded path still reaches a
    // real screen.
    let env_remove = if xvfb.is_some() {
        vec!["WAYLAND_DISPLAY".to_string()]
    } else {
        Vec::new()
    };

    // Re-establish the precondition immediately before the fork.
    //
    // Materialization happened before the executable lookup, the port reservation
    // and the Xvfb start, so the directory has been sitting on disk unattended
    // for that whole stretch. Meanwhile `Lifecycle::new` runs residual GC over
    // `chrome_profiles_dir` at the BORN of EVERY invocation, and `cargo test`
    // runs test binaries concurrently, so a sibling process sweeps the very
    // ground this launch is standing on. The age floors make that sweep safe in
    // the ordinary case; this call makes the precondition true rather than
    // probable. It is not a retry — nothing is relaunched, and a genuine I/O
    // failure still surfaces as itself.
    reassert_profile_dir(&chrome_args)?;

    let request = SpawnRequest {
        envs,
        env_remove,
        debug_pipe: Some(pipe_child),
        ..SpawnRequest::new(executable, chrome_args.args.clone())
    };
    // The fork itself blocks; keep it off the Tokio worker (the guard thread is
    // what actually owns the child, so `spawn_blocking` only carries the wait).
    //
    // The owner is built INSIDE the blocking task. A `std::process::Child`
    // does not kill on drop, so a launch cancelled while this future was
    // suspended used to leave a live Chrome with no owner until the process
    // exited; a `ChromeProcess` dropped with the task's output kills it.
    let (process, logs) = tokio::task::spawn_blocking(move || {
        let guarded = spawn_guarded(request)?;
        let mut child = guarded.child;
        match start_log_drainers(&mut child) {
            Ok((logs, drainers)) => Ok((ChromeProcess::new(child, guarded.pgid, drainers), logs)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(e)
            }
        }
    })
    .await
    .map_err(|e| format!("Chrome spawn task failed: {e}"))??;
    // The display is handed to the process that renders into it, so teardown
    // order is fixed by ownership rather than by call-site discipline: Chrome
    // is reaped first, the server afterwards.
    let process = process.with_private_display(xvfb);

    // The accept clock starts here, while readiness gets its own full budget
    // below; twice the budget leaves the client at least one whole budget to
    // connect even when Chrome answers at the last moment.
    let bridge = match PipeBridge::start(pipe_parent, startup_timeout().saturating_mul(2)).await {
        Ok(bridge) => bridge,
        Err(e) => {
            kill_off_the_runtime(process).await;
            return Err(e);
        }
    };
    // Owned by the process before the first wait on it, so every exit path —
    // failure, cancellation, FINALIZE — tears it down after Chrome is reaped.
    let mut process = process.with_transport(bridge);
    let ready = match process.transport_mut() {
        Some(bridge) => bridge
            .wait_ready(startup_timeout())
            .await
            .map(|()| bridge.ws_url().to_string()),
        None => Err("DevTools pipe bridge vanished before readiness".to_string()),
    };
    let ws_url = match ready {
        Ok(url) => url,
        Err(e) => {
            // Reap through the owner so the drainers are joined exactly once.
            kill_off_the_runtime(process).await;
            return Err(format!(
                "{}{}",
                launch_error("Chrome", &e, &logs, None),
                profile_postmortem(&chrome_args)
            ));
        }
    };

    let (browser, handler) =
        match Browser::connect_with_config(&ws_url, handler_config(options)).await {
            Ok(pair) => pair,
            Err(e) => {
                kill_off_the_runtime(process).await;
                return Err(format!("chromiumoxide Browser::connect_with_config: {e}"));
            }
        };

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
