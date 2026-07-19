// SPDX-License-Identifier: MIT OR Apache-2.0
//! Binary entry for `browser-automation-cli`.
//!
//! Delegates to [`browser_automation_cli::run`].
//!
//! # Resource economy & performance
//!
//! - Global allocator: **mimalloc** (rules_rust_economia_de_recursos — default
//!   for CLI/scripts; measure with `scripts/rss-baseline.sh` when comparing).
//! - Release profile: `opt-level=3`, `lto="fat"`, `codegen-units=1`,
//!   `panic="abort"`, `strip="symbols"` (rules_rust_eficiencia_e_performance).
//! - Latency profiling: `cargo build --profile release-prof` (`debug=1`, no strip).
//! - Local Linux link: mold via `.cargo/config.toml` (build-time only).
//! - Workload is one-shot **I/O-bound** (Chrome CDP / HTTP) with **bounded
//!   parallelism** via [`browser_automation_cli::concurrency`]
//!   (`--max-concurrency`, JoinSet / `join_bounded`, Rayon for CPU scans);
//!   runtimes in [`browser_automation_cli::runtime_util`] (budgeted multi-thread).
//! - Graceful shutdown (one-shot): SIGINT/SIGTERM → cancel/130; second signal
//!   forces residual finalize; residual SIGTERM→grace→SIGKILL; BrokenPipe 141.
//! - Hygiene: `scripts/perf-check.sh`, `scripts/latency-check.sh`,
//!   `scripts/latency-baseline.sh` (P50/P99 wall-clock; no GHA),
//!   `scripts/tracing-check.sh` (local tracing / rotation; no remote OTEL).
//! - Tracing: installed in lib `run()` via [`browser_automation_cli::telemetry`];
//!   `human_panic` hook is chained after the subscriber (panic → tracing error).

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> std::process::ExitCode {
    // Friendly panic reports in release builds (rules_rust_cli_com_clap).
    // With `panic = "abort"` the hook still runs before process abort.
    human_panic::setup_panic!(human_panic::Metadata::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
    .authors(env!("CARGO_PKG_AUTHORS"))
    .homepage(env!("CARGO_PKG_HOMEPAGE")));

    browser_automation_cli::run()
}
