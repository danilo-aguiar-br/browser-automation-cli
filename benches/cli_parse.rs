//! Microbenchmarks for agent-facing CPU paths (clap + compact JSON).
//!
//! Criterion reports confidence intervals (not mean-only). For **wall-clock
//! process P50/P99** use `scripts/latency-baseline.sh` (rules_rust_latencia_reduzir).
//! Never treat microbench alone as proof of end-to-end latency.

// `criterion_group!` and `criterion_main!` expand to a public function and a
// `main` this file neither names nor can document, and the expansion unwraps
// internally. The package-wide policy in `[lints]` reaches benches too — which
// is the point of moving it out of `src/lib.rs` — so the exemption is stated
// here rather than left to a green gate that never saw the target. An attribute
// on the macro invocation does NOT work: it applies to the invocation, not to
// the items the expansion produces, and rustc rejects it as unused.
#![allow(missing_docs, clippy::unwrap_used)]

use browser_automation_cli::cli::Cli;
use browser_automation_cli::envelope::{ErrorBody, ErrorEnvelope, SuccessEnvelope};
use browser_automation_cli::json_util;
use clap::{CommandFactory, Parser};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;

/// Cost of the per-step runtime that `batch`, `crawl` and `offline` pay.
///
/// # Why this benchmark exists
///
/// Those three loops call `block_on_io` once per item, and each call builds a
/// runtime and tears it down. Hoisting a single runtime out of the loops was
/// blocked by a real constraint — a detached signal task per call — which
/// `AbortOnDrop` has since removed, so the change became POSSIBLE. Possible is
/// not the same as worthwhile, and the open question was never answered with a
/// number: is the runtime a meaningful share of a step, or noise next to the
/// network I/O every one of those steps performs?
///
/// This measures the runtime alone, so that question stops being argued from
/// intuition. Compare the result against the cost of one HTTP request before
/// restructuring anything.
fn io_runtime_lifecycle(c: &mut Criterion) {
    c.bench_function("build_and_shutdown_io_runtime", |b| {
        b.iter(|| {
            let rt = browser_automation_cli::runtime_util::build_io_runtime().unwrap();
            browser_automation_cli::runtime_util::shutdown_runtime(black_box(rt));
        });
    });
}

fn parse_doctor_json(c: &mut Criterion) {
    c.bench_function("parse_doctor_offline_quick_json", |b| {
        b.iter(|| {
            let cli = Cli::try_parse_from(black_box([
                "browser-automation-cli",
                "--json",
                "doctor",
                "--offline",
                "--quick",
            ]));
            black_box(cli).expect("parse");
        });
    });
}

fn command_factory_build(c: &mut Criterion) {
    c.bench_function("command_factory_cli_tree", |b| {
        b.iter(|| {
            let cmd = Cli::command();
            black_box(cmd);
        });
    });
}

fn debug_assert_tree(c: &mut Criterion) {
    c.bench_function("command_debug_assert", |b| {
        b.iter(|| {
            browser_automation_cli::command_factory_debug_assert();
        });
    });
}

/// Compact success envelope encode — agent stdout hot path (not pretty).
fn envelope_success_compact(c: &mut Criterion) {
    let env = SuccessEnvelope {
        schema_version: 1,
        ok: true,
        correlation_id: None,
        data: json!({"status": "ok", "n": 1}),
        // The unreduced hot path: the field is skipped when absent, so this
        // still benchmarks the exact bytes an unflagged invocation emits.
        agent_ops: None,
    };
    c.bench_function("envelope_success_to_compact_string", |b| {
        b.iter(|| {
            let s = json_util::to_compact_string(black_box(&env)).expect("encode");
            black_box(s);
        });
    });
}

/// Compact error envelope encode — cold path but still agent contract.
fn envelope_error_compact(c: &mut Criterion) {
    let env = ErrorEnvelope {
        schema_version: 1,
        ok: false,
        correlation_id: None,
        error: ErrorBody {
            kind: "software".into(),
            message: "example".into(),
            exit_code: 70,
            // Serialised by this benchmark, so it belongs to the shape being
            // measured: a field left out here would make the benchmark time an
            // envelope the product never emits.
            retryable: false,
            suggestion: Some("retry".into()),
        },
        data: None,
    };
    c.bench_function("envelope_error_to_compact_string", |b| {
        b.iter(|| {
            let s = json_util::to_compact_string(black_box(&env)).expect("encode");
            black_box(s);
        });
    });
}

criterion_group!(
    benches,
    parse_doctor_json,
    command_factory_build,
    debug_assert_tree,
    envelope_success_compact,
    envelope_error_compact,
    io_runtime_lifecycle
);
criterion_main!(benches);
