// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tokio runtime builders tuned for **one-shot CLI** with **bounded parallelism**.
//!
//! # Workload
//!
//! **Mista / I/O-bound** (Chrome CDP + HTTP). Async coordinates wait; CPU work
//! uses Rayon or `spawn_blocking` (see [`crate::concurrency`]).
//!
//! # Product latency model
//!
//! Wall time is dominated by **Chrome CDP / network I/O**, not Rust CPU. Rules
//! `rules_rust_latencia_reduzir` + `rules_rust_paralelismo` require:
//!
//! - **Bounded runtimes** — worker count from [`crate::concurrency::browser_worker_threads`]
//!   (not unbounded `num_cpus` on a one-shot agent process; hard-capped at 8).
//! - **No blocking work on async workers** without `spawn_blocking`.
//! - **No PGO/BOLT/isolcpus/mlockall** as product defaults (daemon/HFT ops;
//!   product law is BORN→EXECUTE→FINALIZE→DIE).
//!
//! # Latency budgets (agent-facing, host-local, release build)
//!
//! | Path | P99 budget (order of magnitude) | Notes |
//! |------|----------------------------------|-------|
//! | Clap parse + doctor offline quick | ≤ **50 ms** | No Chrome; meta path |
//! | `--help` cold | ≤ **80 ms** | First process image load |
//! | JSON envelope encode (small) | ≤ **100 µs** | Criterion / unit |
//! | Chrome launch + first CDP | **seconds** | External; not Rust hot path |
//!
//! Wall-clock baselines report **P50 / P99 / P999 / P9999** (never mean-only)
//! via `scripts/latency-baseline.sh`. Budgets are **ceilings for regression
//! detection**, not SLOs for trading.
//!
//! # N/A for this product (HFT / daemon ops — do not ship as defaults)
//!
//! PGO/BOLT product pipelines, `isolcpus` / `mlockall` / huge pages, kernel
//! bypass, TSC tick-to-trade, remote HDR telemetry. Workload is one-shot
//! **I/O-bound** (Chrome CDP); process dies after FINALIZE.
//!
//! # Runtime flavours
//!
//! | Helper | Flavour | Use |
//! |--------|---------|-----|
//! | [`block_on_browser_timeout`](crate::browser::block_on_browser_timeout) | multi-thread, **budgeted** workers | CDP event fan-out |
//! | [`block_on_io`](crate::runtime_util::block_on_io) | multi-thread, budgeted (I/O pipelines) | HTTP scrape / batch / crawl |
//!
//! Never create an unbounded `new_multi_thread()` without the concurrency budget.

use crate::concurrency::{browser_max_blocking_threads, browser_worker_threads};
use crate::error::{CliError, ErrorKind};

/// Thread name prefix for browser runtime workers (`bac-browser-0`, …).
pub const BROWSER_THREAD_NAME: &str = "bac-browser";

/// Thread name prefix for I/O multi-thread runtimes.
pub const IO_THREAD_NAME: &str = "bac-io";

/// Build the multi-thread runtime used for Chrome CDP sessions.
///
/// # Parallelism notes
///
/// - Workers: [`browser_worker_threads`] from process budget / `--max-concurrency`.
/// - Blocking pool: [`browser_max_blocking_threads`].
/// - Named threads for `perf` / `tokio-console` attribution.
pub fn build_browser_runtime() -> Result<tokio::runtime::Runtime, CliError> {
    let workers = browser_worker_threads();
    let blocking = browser_max_blocking_threads();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(workers)
        .max_blocking_threads(blocking)
        .thread_name(BROWSER_THREAD_NAME)
        .build()
        .map_err(|e| {
            CliError::new(
                ErrorKind::Software,
                format!("Failed to create browser tokio runtime: {e}"),
            )
        })
}

/// Build a multi-thread runtime for HTTP / offline async fan-out.
///
/// Uses the same budgeted worker count as the browser runtime so batch scrape
/// and crawl can drive concurrent sockets without a second unbounded pool.
pub fn build_io_runtime() -> Result<tokio::runtime::Runtime, CliError> {
    let workers = browser_worker_threads();
    let blocking = browser_max_blocking_threads();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(workers)
        .max_blocking_threads(blocking)
        .thread_name(IO_THREAD_NAME)
        .build()
        .map_err(|e| {
            CliError::new(
                ErrorKind::Software,
                format!("Failed to create I/O tokio runtime: {e}"),
            )
        })
}

/// Tear a runtime down under a hard deadline instead of an unbounded `Drop`.
///
/// # Why this is not the implicit drop
///
/// Dropping a [`tokio::runtime::Runtime`] waits for its blocking pool to drain,
/// and that wait has no upper bound. [`tokio::task::spawn_blocking`] tasks
/// **cannot be aborted once they start running**: calling `abort` on their
/// handle is documented as having no effect, and runtime shutdown "will wait
/// indefinitely for all started `spawn_blocking` to finish running".
///
/// This crate has ~48 `spawn_blocking` call sites, including PDF and HTML
/// parsing of attacker-sized bodies. So a SIGTERM arriving mid-parse takes the
/// cooperative path in [`block_on_with_shutdown`], returns exit 130 — and then
/// the process still sits in `Drop` until the parse finishes on its own. The
/// product law is BORN → EXECUTE → FINALIZE → **DIE**, and an unbounded drop
/// silently converts the last step into "eventually".
///
/// [`tokio::runtime::Runtime::shutdown_timeout`] stops waiting after the
/// deadline and lets the process exit. The threads are not killed — that is not
/// something Tokio can promise — but they stop being able to hold the exit
/// hostage, which is what residual-zero actually requires.
///
/// # Deadline
///
/// From the XDG knob `shutdown_deadline_secs` (default 30 s), the same budget
/// the browser-exit wait already honours. Zero means "do not wait at all", which
/// is a legitimate operator choice and is passed through unchanged.
pub fn shutdown_runtime(rt: tokio::runtime::Runtime) {
    let secs =
        crate::xdg::policy::policy_u64(crate::xdg::policy::key::DEFAULT_SHUTDOWN_DEADLINE_SECS);
    rt.shutdown_timeout(std::time::Duration::from_secs(secs));
}

/// Drive an async I/O future to completion on a budgeted multi-thread runtime.
///
/// Use for HTTP scrape, batch scrape, crawl, and other non-CDP async entered
/// from synchronous CLI handlers. Prefer
/// [`crate::browser::block_on_browser_timeout`] for Chrome CDP.
///
/// # Graceful shutdown
///
/// Same detect → signal → force path as browser work: SIGINT/SIGTERM (Unix) or
/// Ctrl-C/Break/Close (Windows) cancel the shared `CancellationToken` and
/// map to exit **130**. Previously this helper ignored OS signals (gap).
pub fn block_on_io<F, T>(fut: F) -> Result<T, CliError>
where
    F: std::future::Future<Output = Result<T, CliError>>,
{
    let rt = build_io_runtime()?;
    let out = block_on_with_shutdown(&rt, fut, 0);
    // Bounded teardown on BOTH paths: an error return is exactly when a blocking
    // parse is most likely still running. See [`shutdown_runtime`].
    shutdown_runtime(rt);
    out
}

/// Cancel-aware `Runtime::block_on` for one-shot CLI work (DRY for browser + I/O).
///
/// # Phases (rules_rust_encerramento_graceful_shutdown)
///
/// 1. **Detect** — [`crate::browser::shutdown_signal`] in a background task.
/// 2. **Signal** — first OS signal cancels the process `CancellationToken`.
/// 3. **Force** — second OS signal runs residual `Lifecycle::finalize`.
///
/// # Select bias
///
/// `biased` polls **cancel first**, then work, then optional timeout — so a
/// pending cancel wins over a Ready work future that completed in the same
/// poll wave (rules: prioritize cooperative shutdown).
pub fn block_on_with_shutdown<F, T>(
    rt: &tokio::runtime::Runtime,
    fut: F,
    timeout_secs: u64,
) -> Result<T, CliError>
where
    F: std::future::Future<Output = Result<T, CliError>>,
{
    use crate::browser::shutdown_signal;
    use crate::lifecycle::{current_cancel, current_lifecycle};

    let cancel = current_cancel();
    if cancel.is_cancelled() {
        return Err(crate::browser::cancelled_error());
    }

    // Capture lifecycle on this thread before entering the runtime: the signal
    // task runs on a worker thread and must not rely on thread-local CURRENT_LIFE.
    let life_for_signal = current_lifecycle();

    rt.block_on(async move {
        let cancel_for_signal = cancel.clone();
        // Bound to a NAMED guard, so the signal task dies with THIS call instead
        // of with the runtime. A bare `tokio::spawn` DETACHES: with one runtime
        // per call the teardown collected it, which hid the real constraint —
        // the primitive could not be called in a loop over a SHARED runtime,
        // because N calls would leave N tasks parked in `shutdown_signal()`.
        // That, and not the cost of the runtime, is what blocked hoisting a
        // single runtime out of the batch, crawl and offline loops.
        let _signal_task = AbortOnDrop::new(tokio::spawn(async move {
            let first = shutdown_signal().await;
            tracing::warn!(
                trigger = first.as_str(),
                "shutdown signal received; cooperative cancel (exit 130 path)"
            );
            cancel_for_signal.cancel();
            let second = shutdown_signal().await;
            tracing::warn!(
                trigger = second.as_str(),
                "second shutdown signal; forcing residual finalize"
            );
            if let Some(life) = life_for_signal {
                life.finalize();
            }
        }));

        if timeout_secs == 0 {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => Err(crate::browser::cancelled_error()),
                r = fut => r,
            }
        } else {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => Err(crate::browser::cancelled_error()),
                r = fut => r,
                _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)) => {
                    Err(CliError::with_suggestion(
                        ErrorKind::Timeout,
                        format!("operation exceeded --timeout {timeout_secs}s"),
                        crate::i18n::suggestion_key("raise_timeout", None),
                    ))
                }
            }
        }
    })
}

/// A spawned task that is aborted when this handle is dropped.
///
/// # Why dropping the bare `JoinHandle` is not enough
///
/// In Tokio, dropping a `JoinHandle` DETACHES the task; it does not cancel it.
/// A background loop whose only brake is an `abort()` call further down the
/// function therefore survives every path that never reaches that call — and
/// the path that matters here is cancellation.
///
/// On SIGINT, [`block_on_with_shutdown`] resolves its `select!` on the cancel
/// branch and DROPS the work future. Every `abort()` written after the awaited
/// work is skipped, the handle is dropped, and the loop keeps running detached
/// against a browser the process is trying to shut down. It stops only when
/// `shutdown_timeout` tears the runtime down, and a loop still issuing CDP
/// commands during FINALIZE is exactly what a clean shutdown must not do.
///
/// Wrapping the handle moves the abort into `Drop`, so it fires on all four
/// paths — success, error, timeout and abandonment — instead of the three a
/// human remembered to write.
///
/// `abort()` is still asynchronous: it schedules cancellation at the task's
/// next await point. That is sufficient here, because the loops this guards
/// await a sleep on every iteration.
pub(crate) struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl AbortOnDrop {
    /// Take ownership of `handle`, aborting its task when the guard is dropped.
    pub(crate) fn new(handle: tokio::task::JoinHandle<()>) -> Self {
        Self(handle)
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dropping the guard must stop the task; dropping a bare handle must not.
    ///
    /// Both halves are asserted, because only the contrast shows that the guard
    /// is doing the work. A test on the guard alone would still pass if Tokio
    /// had cancelled detached tasks all along, and would prove nothing.
    #[test]
    fn abort_on_drop_stops_a_loop_that_a_bare_handle_leaves_running() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let rt = build_browser_runtime().expect("browser rt");
        let slice = std::time::Duration::from_millis(5);
        let settle = std::time::Duration::from_millis(120);

        let spawn_counting_loop = |counter: Arc<AtomicUsize>| {
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(slice).await;
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            })
        };

        let (guarded, detached) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        rt.block_on(async {
            drop(AbortOnDrop::new(spawn_counting_loop(Arc::clone(&guarded))));
            drop(spawn_counting_loop(Arc::clone(&detached)));
            tokio::time::sleep(settle).await;
        });

        let (g, d) = (
            guarded.load(Ordering::SeqCst),
            detached.load(Ordering::SeqCst),
        );
        assert_eq!(
            g, 0,
            "guarded loop ran {g} times after its guard was dropped"
        );
        assert!(
            d > 0,
            "the detached loop never ran, so this test proves nothing about the guard"
        );
        shutdown_runtime(rt);
    }

    #[test]
    fn browser_runtime_builds() {
        let rt = build_browser_runtime().expect("browser rt");
        let n = rt.block_on(async { 1 + 1 });
        assert_eq!(n, 2);
    }

    #[test]
    fn io_runtime_block_on_io() {
        let v = block_on_io(async { Ok::<_, CliError>(42u32) }).expect("io");
        assert_eq!(v, 42);
    }

    #[test]
    fn block_on_io_respects_pre_cancelled_token() {
        let lc = crate::lifecycle::Lifecycle::new();
        lc.cancel.cancel();
        let err = block_on_io(async { Ok::<u32, CliError>(1) }).expect_err("must cancel");
        assert_eq!(err.kind(), ErrorKind::Cancelled);
        assert_eq!(err.exit_code(), 130);
    }

    #[test]
    fn worker_budget_is_bounded() {
        assert!(browser_worker_threads() >= 2);
        assert!(browser_worker_threads() <= 8);
        assert!(browser_max_blocking_threads() <= 16);
        assert!(crate::concurrency::effective_limit() <= crate::concurrency::HARD_CAP);
    }
}
