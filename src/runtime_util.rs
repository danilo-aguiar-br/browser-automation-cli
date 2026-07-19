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
//! Budgets are **ceilings for regression detection**, not SLOs for trading.
//! Re-measure with `scripts/latency-baseline.sh` after runtime changes.
//!
//! # Runtime flavours
//!
//! | Helper | Flavour | Use |
//! |--------|---------|-----|
//! | [`block_on_browser_timeout`](crate::browser::block_on_browser_timeout) | multi-thread, **budgeted** workers | CDP event fan-out |
//! | [`block_on_io`] | multi-thread, budgeted (I/O pipelines) | HTTP scrape / batch / crawl |
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

/// Drive an async I/O future to completion on a budgeted multi-thread runtime.
///
/// Use for HTTP scrape, batch scrape, crawl, and other non-CDP async entered
/// from synchronous CLI handlers. Prefer
/// [`crate::browser::block_on_browser_timeout`] for Chrome CDP.
pub fn block_on_io<F, T>(fut: F) -> Result<T, CliError>
where
    F: std::future::Future<Output = Result<T, CliError>>,
{
    let rt = build_io_runtime()?;
    rt.block_on(fut)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn worker_budget_is_bounded() {
        assert!(browser_worker_threads() >= 2);
        assert!(browser_worker_threads() <= 8);
        assert!(browser_max_blocking_threads() <= 16);
        assert!(crate::concurrency::effective_limit() <= crate::concurrency::HARD_CAP);
    }
}
