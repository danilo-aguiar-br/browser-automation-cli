//! Microbenchmarks for agent-facing CPU paths (clap + compact JSON).
//!
//! Criterion reports confidence intervals (not mean-only). For **wall-clock
//! process P50/P99** use `scripts/latency-baseline.sh` (rules_rust_latencia_reduzir).
//! Never treat microbench alone as proof of end-to-end latency.

use browser_automation_cli::cli::Cli;
use browser_automation_cli::envelope::{ErrorBody, ErrorEnvelope, SuccessEnvelope};
use browser_automation_cli::json_util;
use clap::{CommandFactory, Parser};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;

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
        data: json!({"status": "ok", "n": 1}),
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
        error: ErrorBody {
            kind: "software".into(),
            message: "example".into(),
            exit_code: 70,
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
    envelope_error_compact
);
criterion_main!(benches);
