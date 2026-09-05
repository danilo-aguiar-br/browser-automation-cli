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

/// # Errors
///
/// Propagates [`run_goto_with_robots`] with no capture and robots honoured:
/// [`ErrorKind::Unavailable`](crate::error::ErrorKind::Unavailable) when
/// Chrome cannot be launched, a robots refusal, or a navigation failure.
pub async fn run_goto(life: &Lifecycle, url: &str) -> Result<Value, CliError> {
    run_goto_with_robots(
        life,
        url,
        CaptureOpts::default(),
        crate::robots::RobotsPolicy::Honor,
    )
    .await
}

/// # Errors
///
/// Propagates [`run_goto_with_options`] with no init script, no beforeunload
/// handling and the default navigation ceiling.
pub async fn run_goto_with_robots(
    life: &Lifecycle,
    url: &str,
    capture: CaptureOpts,
    robots: crate::robots::RobotsPolicy,
) -> Result<Value, CliError> {
    run_goto_with_options(life, url, capture, robots, None, None, None).await
}

/// One-shot goto with tool-ref navigation options (init script, beforeunload, timeout).
///
/// # Errors
///
/// Fails with
/// [`ErrorKind::Unavailable`](crate::error::ErrorKind::Unavailable) when
/// Chrome cannot be launched or the navigation exceeds its ceiling, and
/// otherwise propagates
/// [`goto_with_options`](crate::browser::OneShotSession::goto_with_options):
/// a `file:` URL outside the allowed roots, a robots refusal, a failed
/// `init_script` registration, or a browser-reported navigation error.
///
/// A shutdown failure is also surfaced: the session is always closed, and when
/// the work succeeded but FINALIZE did not, the close error is what the caller
/// receives.
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

/// # Errors
///
/// Propagates [`run_scrape_wait`] with no post-navigation wait: a failed
/// Chrome launch, a robots refusal, a navigation error, or an extraction
/// failure.
pub async fn run_scrape(
    life: &Lifecycle,
    url: &str,
    robots: crate::robots::RobotsPolicy,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    run_scrape_wait(life, url, robots, capture, 0).await
}

/// Browser scrape with optional post-navigation wait (base waitFor parity, ms).
///
/// # Errors
///
/// Propagates [`run_scrape_actions`] with an empty action list: a failed
/// Chrome launch, a robots refusal, a navigation error, or an extraction
/// failure.
pub async fn run_scrape_wait(
    life: &Lifecycle,
    url: &str,
    robots: crate::robots::RobotsPolicy,
    capture: CaptureOpts,
    wait_ms: u64,
) -> Result<Value, CliError> {
    run_scrape_actions(life, url, robots, capture, wait_ms, &[]).await
}

/// Browser scrape that first acts on the page.
///
/// # Why the steps run in THIS session
///
/// A step in one process and a scrape in another share nothing: the CLI is
/// one-shot, so the second invocation gets a fresh browser with no cookies,
/// no scroll position and no expanded accordion. Anything the actions
/// accomplished would be gone before the extraction started. Running them
/// here, between navigation and extraction, is the only ordering that makes
/// them mean anything.
///
/// # Why the steps are `run --script` steps
///
/// Reusing `commands::run::execute::execute_step` rather than writing
/// a second interpreter keeps ONE grammar for acting on a page. A second
/// dialect would drift from the first, and `record` output — which is
/// `run --script` lines — would stop being replayable here.
///
/// A failing step fails the scrape. These are preconditions the caller stated
/// for the extraction; scraping anyway would return a page that is not the one
/// that was asked for, labelled as success.
///
/// # Errors
///
/// Fails with
/// [`ErrorKind::Unavailable`](crate::error::ErrorKind::Unavailable) when
/// Chrome cannot be launched, and propagates the navigation errors of
/// [`goto`](crate::browser::OneShotSession::goto) — robots refusal, `file:`
/// URL outside the allowed roots, navigation timeout.
///
/// Fails with [`ErrorKind::Usage`](crate::error::ErrorKind::Usage) —
/// `"--action[N] has no `cmd` field"` — for a step that names no command, and
/// otherwise re-raises the failing step's own kind with the message prefixed
/// `"--action[N] (<cmd>): "`. The index is part of the contract: with several
/// actions, the bare step error does not say which one failed.
///
/// A failing action fails the whole scrape by design; extracting anyway would
/// return a page other than the one the caller described, labelled as success.
pub async fn run_scrape_actions(
    life: &Lifecycle,
    url: &str,
    robots: crate::robots::RobotsPolicy,
    capture: CaptureOpts,
    wait_ms: u64,
    actions: &[Value],
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = scrape_after_actions(&mut session, url, robots, wait_ms, actions).await;
    finish(life, session, work).await
}

/// Navigate, run each action in order, then extract.
async fn scrape_after_actions(
    session: &mut crate::browser::OneShotSession,
    url: &str,
    robots: crate::robots::RobotsPolicy,
    wait_ms: u64,
    actions: &[Value],
) -> Result<Value, CliError> {
    if actions.is_empty() {
        return session.scrape_with_wait(url, robots, &[], wait_ms).await;
    }
    // Navigate first: an action naming a selector needs a document to find it
    // in, and `scrape_with_wait` would navigate again afterwards anyway.
    session.goto(url, robots).await?;
    let flags = crate::commands::run::RunFlags::default();
    for (index, step) in actions.iter().enumerate() {
        let cmd = step
            .get("cmd")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CliError::with_suggestion(
                    crate::error::ErrorKind::Usage,
                    format!("--action[{index}] has no `cmd` field"),
                    crate::i18n::suggestion_key("use_listed_value", None),
                )
            })?
            .to_string();
        crate::commands::run::execute::execute_step(session, &cmd, step, robots, flags, None)
            .await
            .map_err(|e| {
                // The index is what makes the message actionable: with five
                // actions, "press failed" does not say which one.
                CliError::new(
                    e.kind(),
                    format!("--action[{index}] ({cmd}): {}", e.message()),
                )
            })?;
    }
    session.scrape_with_wait(url, robots, &[], wait_ms).await
}

/// # Errors
///
/// Propagates [`run_goto_with_robots`] with robots honoured: a failed Chrome
/// launch, a robots refusal, or a navigation error.
pub async fn run_goto_capture(
    life: &Lifecycle,
    url: &str,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    run_goto_with_robots(life, url, capture, crate::robots::RobotsPolicy::Honor).await
}

/// # Errors
///
/// Fails with
/// [`ErrorKind::Unavailable`](crate::error::ErrorKind::Unavailable) when
/// Chrome cannot be launched, on the `about:blank` navigation, and then
/// propagates [`view`](crate::browser::OneShotSession::view): a refused
/// accessibility walk, or an empty tree when the caller did not allow one.
///
/// Because this launches its own browser and lands on `about:blank`, the
/// snapshot describes a blank page unless a previous step in the same process
/// navigated elsewhere.
pub async fn run_view(
    life: &Lifecycle,
    verbose: bool,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = async {
        let _ = session
            .goto(
                crate::constants::ABOUT_BLANK,
                crate::robots::RobotsPolicy::Honor,
            )
            .await?;
        session.view(verbose).await
    }
    .await;
    finish(life, session, work).await
}

/// # Errors
///
/// Fails with
/// [`ErrorKind::Unavailable`](crate::error::ErrorKind::Unavailable) when
/// Chrome cannot be launched or the `about:blank` navigation fails, then
/// propagates [`press`](crate::browser::OneShotSession::press): `target`
/// resolves to no element, or the click is refused.
///
/// A `@eN` ref cannot survive into this process — refs live only in the
/// session that minted them — so a ref target fails as unknown here.
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
            .goto(
                crate::constants::ABOUT_BLANK,
                crate::robots::RobotsPolicy::Honor,
            )
            .await?;
        session.press(target, dblclick, include_snapshot).await
    }
    .await;
    finish(life, session, work).await
}

/// # Errors
///
/// Fails with
/// [`ErrorKind::Unavailable`](crate::error::ErrorKind::Unavailable) when
/// Chrome cannot be launched or the `about:blank` navigation fails, then
/// propagates [`write`](crate::browser::OneShotSession::write): `target`
/// resolves to no element, or the value cannot be assigned.
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
            .goto(
                crate::constants::ABOUT_BLANK,
                crate::robots::RobotsPolicy::Honor,
            )
            .await?;
        session.write(target, value, include_snapshot).await
    }
    .await;
    finish(life, session, work).await
}

/// # Errors
///
/// Fails with
/// [`ErrorKind::Unavailable`](crate::error::ErrorKind::Unavailable) when
/// Chrome cannot be launched or the `about:blank` navigation fails, then
/// propagates [`keys`](crate::browser::OneShotSession::keys): the key event
/// was refused by the browser.
pub async fn run_keys(
    life: &Lifecycle,
    key: &str,
    include_snapshot: bool,
    capture: CaptureOpts,
) -> Result<Value, CliError> {
    let mut session = launch_marked(life, capture).await?;
    let work = async {
        let _ = session
            .goto(
                crate::constants::ABOUT_BLANK,
                crate::robots::RobotsPolicy::Honor,
            )
            .await?;
        session.keys(key, include_snapshot).await
    }
    .await;
    finish(life, session, work).await
}

// Mirrors the clap argument surface 1:1; grouping into a struct would add an
// indirection that has to be kept in sync with argv by hand.
/// # Errors
///
/// Fails with
/// [`ErrorKind::Unavailable`](crate::error::ErrorKind::Unavailable) when
/// Chrome cannot be launched or the `about:blank` navigation fails, then
/// propagates [`type_text`](crate::browser::OneShotSession::type_text):
/// neither `target` nor `focus_only` was given, `target` resolves to no
/// element, or a key event was refused.
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
            .goto(
                crate::constants::ABOUT_BLANK,
                crate::robots::RobotsPolicy::Honor,
            )
            .await?;
        session
            .type_text(target, text, clear, submit, focus_only, include_snapshot)
            .await
    }
    .await;
    finish(life, session, work).await
}

/// # Errors
///
/// Fails with
/// [`ErrorKind::Unavailable`](crate::error::ErrorKind::Unavailable) when
/// Chrome cannot be launched, and otherwise returns whatever `work` produced.
///
/// The two failure paths differ in teardown, deliberately. When `work`
/// succeeds, FINALIZE runs and a failed shutdown becomes the returned error.
/// When `work` fails it has already dropped the session, so the residual
/// ledger is left armed rather than cleared — FINALIZE must still be able to
/// reap the browser, and marking it closed here would strand a live Chrome.
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
///
/// # Errors
///
/// Propagates [`block_on_browser_timeout`] with no timeout: a runtime that
/// cannot be built, a cancel signal, or whatever `fut` itself returned. With
/// `timeout_secs` at `0` there is no wall-clock ceiling here, so a hung future
/// is bounded only by the per-operation budgets inside it.
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
/// # Errors
///
/// Fails when [`crate::runtime_util::build_browser_runtime`] cannot create the
/// Tokio runtime, with
/// [`ErrorKind::Timeout`](crate::error::ErrorKind::Timeout) when
/// `timeout_secs` is non-zero and elapses first, and with
/// [`ErrorKind::Cancelled`](crate::error::ErrorKind::Cancelled) when SIGINT or
/// SIGTERM cancels the shared token — exit **130** for both signals, which is
/// the one-shot product contract rather than sysexits' 143.
///
/// Otherwise returns whatever `fut` produced. The runtime is torn down on
/// every path, including the failing ones.
pub fn block_on_browser_timeout<F, T>(fut: F, timeout_secs: u64) -> Result<T, CliError>
where
    F: std::future::Future<Output = Result<T, CliError>>,
{
    // Shared detect → signal → await path (cancel-first `biased` select).
    // See [`crate::runtime_util::block_on_with_shutdown`].
    let rt = crate::runtime_util::build_browser_runtime()?;
    let out = crate::runtime_util::block_on_with_shutdown(&rt, fut, timeout_secs);
    // Bounded teardown: dropping the runtime waits without limit for started
    // `spawn_blocking` tasks, which cannot be aborted. See
    // [`crate::runtime_util::shutdown_runtime`].
    crate::runtime_util::shutdown_runtime(rt);
    out
}
