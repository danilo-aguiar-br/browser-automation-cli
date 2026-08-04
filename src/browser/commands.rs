// SPDX-License-Identifier: MIT OR Apache-2.0
//! Public one-shot browser command entry points and runtime helpers.

// GAP-046 lote 3 pending: wire-type docs land per file; the allow is removed
// as each file is documented, so it can never silence a new module.
#![allow(missing_docs)]
use serde_json::Value;

use crate::error::CliError;
use crate::lifecycle::Lifecycle;

use super::session::{CaptureOpts, OneShotSession};
use super::support::{finish, launch_marked};

pub async fn run_goto(life: &Lifecycle, url: &str) -> Result<Value, CliError> {
    run_goto_with_robots(
        life,
        url,
        CaptureOpts::default(),
        crate::robots::RobotsPolicy::Honor,
    )
    .await
}

pub async fn run_goto_with_robots(
    life: &Lifecycle,
    url: &str,
    capture: CaptureOpts,
    robots: crate::robots::RobotsPolicy,
) -> Result<Value, CliError> {
    run_goto_with_options(life, url, capture, robots, None, None, None).await
}

/// One-shot goto with tool-ref navigation options (init script, beforeunload, timeout).
#[allow(clippy::too_many_arguments)]
pub async fn run_goto_with_options(
    life: &Lifecycle,
    url: &str,
    capture: CaptureOpts,
    robots: crate::robots::RobotsPolicy,
    init_script: Option<&str>,
    handle_before_unload: Option<&str>,
    navigation_timeout_ms: Option<u64>,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = session
        .goto_with_options(
            url,
            robots,
            init_script,
            handle_before_unload,
            navigation_timeout_ms,
        )
        .await;
    finish(life, session, work).await
}

pub async fn run_scrape(
    life: &Lifecycle,
    url: &str,
    robots: crate::robots::RobotsPolicy,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    run_scrape_wait(life, url, robots, capture, 0).await
}

/// Browser scrape with optional post-navigation wait (base waitFor parity, ms).
pub async fn run_scrape_wait(
    life: &Lifecycle,
    url: &str,
    robots: crate::robots::RobotsPolicy,
    capture: CaptureOpts,
    wait_ms: u64,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = session.scrape_with_wait(url, robots, &[], wait_ms).await;
    finish(life, session, work).await
}

pub async fn run_goto_capture(
    life: &Lifecycle,
    url: &str,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    run_goto_with_robots(life, url, capture, crate::robots::RobotsPolicy::Honor).await
}

pub async fn run_view(
    life: &Lifecycle,
    verbose: bool,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = async {
        let _ = session
            .goto("about:blank", crate::robots::RobotsPolicy::Honor)
            .await?;
        session.view(verbose).await
    }
    .await;
    finish(life, session, work).await
}

pub async fn run_press(
    life: &Lifecycle,
    target: &str,
    dblclick: bool,
    include_snapshot: bool,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = async {
        let _ = session
            .goto("about:blank", crate::robots::RobotsPolicy::Honor)
            .await?;
        session.press(target, dblclick, include_snapshot).await
    }
    .await;
    finish(life, session, work).await
}

pub async fn run_write(
    life: &Lifecycle,
    target: &str,
    value: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = async {
        let _ = session
            .goto("about:blank", crate::robots::RobotsPolicy::Honor)
            .await?;
        session.write(target, value, include_snapshot).await
    }
    .await;
    finish(life, session, work).await
}

pub async fn run_keys(
    life: &Lifecycle,
    key: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = async {
        let _ = session
            .goto("about:blank", crate::robots::RobotsPolicy::Honor)
            .await?;
        session.keys(key, include_snapshot).await
    }
    .await;
    finish(life, session, work).await
}

// Mirrors the clap argument surface 1:1; grouping into a struct would add an
// indirection that has to be kept in sync with argv by hand.
#[allow(clippy::too_many_arguments)]
pub async fn run_type(
    life: &Lifecycle,
    target: Option<&str>,
    text: &str,
    clear: bool,
    submit: Option<&str>,
    focus_only: bool,
    include_snapshot: bool,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = async {
        let _ = session
            .goto("about:blank", crate::robots::RobotsPolicy::Honor)
            .await?;
        session
            .type_text(target, text, clear, submit, focus_only, include_snapshot)
            .await
    }
    .await;
    finish(life, session, work).await
}

pub async fn run_with_session<F, Fut>(
    life: &Lifecycle,
    capture: CaptureOpts,
    work: F,
) -> Result<Value, CliError>
where
    F: FnOnce(OneShotSession) -> Fut,
    Fut: std::future::Future<Output = Result<(OneShotSession, Value), CliError>>,
{
    let session = launch_marked(life, capture).await?;
    match work(session).await {
        Ok((session, value)) => finish(life, session, Ok(value)).await,
        Err(e) => {
            // Session was dropped inside `work` without explicit Browser.close.
            // Keep residual ledger flags so FINALIZE can SIGTERM→grace→SIGKILL
            // (rules: never clear ownership without reap). Do **not** mark_closed.
            Err(e)
        }
    }
}

/// Block on tokio multi-thread runtime for one-shot browser work.
///
/// # Workload
///
/// **I/O-bound** (Chrome CDP + network). See [`block_on_browser_timeout`].
pub fn block_on_browser<F, T>(fut: F) -> Result<T, CliError>
where
    F: std::future::Future<Output = Result<T, CliError>>,
{
    block_on_browser_timeout(fut, 0)
}

/// Like `block_on_browser`, but abort with `ErrorKind::Timeout` when `timeout_secs > 0`.
///
/// Also races the work against the active [`crate::lifecycle::Lifecycle`] cancel
/// token (SIGINT/SIGTERM via in-runtime signal watch → exit **130**).
///
/// # Graceful shutdown (one-shot)
///
/// Delegates to [`crate::runtime_util::block_on_with_shutdown`]:
///
/// 1. **Detect** — [`crate::browser::shutdown_signal`] (SIGINT/SIGTERM; Windows Ctrl-C/Break/Close).
/// 2. **Signal** — first OS signal cancels the shared `CancellationToken`(tokio_util::sync::CancellationToken).
/// 3. **Force** — second OS signal runs residual `Lifecycle::finalize` so a
///    stuck Browser.close cannot hang the process forever.
/// 4. **Bias** — `select!` is `biased` with **cancel polled first**.
///
/// # Workload classification (rules_rust_paralelismo + economia + latencia)
///
/// - **Class:** I/O-bound (CDP WebSocket + Chrome subprocess events).
/// - **Runtime:** [`crate::runtime_util::build_browser_runtime`] — multi-thread
///   workers from [`crate::concurrency::browser_worker_threads`] (budget /
///   `--max-concurrency`, hard-capped at 8) + capped blocking pool.
/// - **CDP fan-out:** snapshot/screenshot resolve batches use
///   [`crate::concurrency::join_bounded`] (never unbounded `join_all`).
/// - **CPU off async:** structural scan / multi-file work uses Rayon via
///   [`crate::concurrency`]; never call Rayon on a Tokio worker without
///   `spawn_blocking`.
/// - **Latency:** measure agent meta P50/P99 via `scripts/latency-baseline.sh`.
///   Chrome boot is external WCET (seconds).
/// - **No daemon:** product law is BORN→EXECUTE→FINALIZE→DIE; amortizing Chrome
///   boot via a long-lived process is **forbidden**.
/// - **Subprocess:** Chrome via chromiumoxide; residual kill is ledger + Job
///   Object / SIGTERM→grace→SIGKILL — not `systemd-run --scope` as product default.
/// - **N/A (product law):** PGO/BOLT, isolcpus, mlockall, remote permit metrics,
///   loom CI full, systemd MemoryMax scopes — ops/HFT, not agent one-shot default.
pub fn block_on_browser_timeout<F, T>(fut: F, timeout_secs: u64) -> Result<T, CliError>
where
    F: std::future::Future<Output = Result<T, CliError>>,
{
    // Shared detect → signal → await path (cancel-first `biased` select).
    // See [`crate::runtime_util::block_on_with_shutdown`].
    let rt = crate::runtime_util::build_browser_runtime()?;
    crate::runtime_util::block_on_with_shutdown(&rt, fut, timeout_secs)
}
