#!/usr/bin/env bash
# Local hygiene gate for rules_rust_logs_com_tracing_e_rotacao (one-shot CLI).
# No GitHub Actions / CD — run manually or from scripts/ci-check.sh.
#
# Usage:
#   ./scripts/tracing-check.sh
#   ./scripts/tracing-check.sh --inventory-only
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

INVENTORY_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --inventory-only) INVENTORY_ONLY=1 ;;
    -h|--help)
      sed -n '2,10p' "$0"
      exit 0
      ;;
  esac
done

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; fail=1; }

echo "== tracing-check (local-only logs / rotation / no remote telemetry) =="

# 1) Canonical stack declared
if rg -q 'tracing-subscriber' Cargo.toml && rg -q 'tracing-appender' Cargo.toml && rg -q 'tracing-error' Cargo.toml; then
  pass "tracing + subscriber + appender + error deps"
else
  bad "missing tracing stack in Cargo.toml"
fi

# 2) Explicit subscriber features (json + env-filter + tracing-log bridge)
if rg -n 'tracing-subscriber' Cargo.toml | rg -q 'env-filter' \
  && rg -n 'tracing-subscriber' Cargo.toml | rg -q 'json' \
  && rg -n 'tracing-subscriber' Cargo.toml | rg -q 'tracing-log'; then
  pass "subscriber features env-filter + json + tracing-log"
else
  bad "tracing-subscriber missing required features"
fi

# 3) Dedicated telemetry module + init
if [ -f src/telemetry.rs ] && rg -q 'fn init_telemetry' src/telemetry.rs; then
  pass "src/telemetry.rs init_telemetry"
else
  bad "missing telemetry module / init_telemetry"
fi

# 4) WorkerGuard held (no executable mem::forget — comments/docs may mention the ban)
if rg -n 'mem::forget\(|std::mem::forget\(' src/ --glob '*.rs' >/dev/null 2>&1; then
  bad "mem::forget(...) still present (prefer named TelemetryGuard drop)"
  rg -n 'mem::forget\(|std::mem::forget\(' src/ --glob '*.rs' || true
else
  pass "no mem::forget(...) (WorkerGuard via TelemetryGuard)"
fi

if rg -q 'TelemetryGuard|_telemetry|WorkerGuard' src/lib.rs src/telemetry.rs; then
  pass "TelemetryGuard / WorkerGuard lifecycle wired"
else
  bad "guard lifecycle not wired from run()"
fi

# 5) Rolling builder + max_log_files
if rg -q 'max_log_files' src/telemetry.rs && rg -q 'RollingFileAppender::builder|rolling::RollingFileAppender::builder' src/telemetry.rs; then
  pass "RollingFileAppender builder + max_log_files"
else
  bad "missing rolling Builder / max_log_files"
fi

# 6) ErrorLayer present
if rg -q 'ErrorLayer' src/telemetry.rs; then
  pass "ErrorLayer (SpanTrace)"
else
  bad "missing ErrorLayer"
fi

# 7) No product RUST_LOG *read* in telemetry (docs may mention the ban)
if rg -n 'env::var(_os)?\([^\)]*RUST_LOG|std::env::var.*"RUST_LOG"' src/telemetry.rs >/dev/null 2>&1; then
  bad "telemetry must not read RUST_LOG (XDG + argv only)"
else
  pass "no RUST_LOG env read in telemetry (product law)"
fi

# 8) No remote OTEL / slog / env_logger *usage* (allow ban docs + sg_local anti-telemetry scanner)
if rg -n 'use env_logger|env_logger::|use slog|slog::|opentelemetry::|OTEL_EXPORTER' src/ --glob '*.rs' >/dev/null 2>&1; then
  bad "remote/observability anti-pattern import/use in src"
  rg -n 'use env_logger|env_logger::|use slog|slog::|opentelemetry::|OTEL_EXPORTER' src/ --glob '*.rs' || true
else
  pass "no OTEL/env_logger/slog usage in src"
fi

# 9) Production eprintln/dbg for diagnostics (allow tests + build.rs)
prod_eprint=$(rg -n 'eprintln!|dbg!' src/ --glob '*.rs' \
  | rg -v '^\s*//|tests/|#\[cfg\(test\)\]|/tests\.rs' \
  | rg -v 'src/.*#\[cfg\(test\)\]' \
  || true)
# Filter: only non-test modules under src (exclude #[cfg(test)] blocks is hard; check known modules)
if rg -n 'eprintln!|dbg!' src/robots.rs src/native/browser.rs src/native/snapshot.rs src/telemetry.rs src/lib.rs 2>/dev/null | rg -v '^\s*//' >/dev/null 2>&1; then
  bad "eprintln!/dbg! in production hot modules"
  rg -n 'eprintln!|dbg!' src/robots.rs src/native/browser.rs src/native/snapshot.rs src/telemetry.rs src/lib.rs || true
else
  pass "no eprintln!/dbg! in telemetry/robots/native hot paths"
fi

# 10) Panic bridge after subscriber
if rg -q 'install_panic_tracing_bridge|target: "panic"' src/telemetry.rs; then
  pass "panic → tracing error bridge"
else
  bad "missing panic tracing bridge"
fi

if [ "$INVENTORY_ONLY" -eq 1 ]; then
  [ "$fail" -eq 0 ] || exit 65
  echo "OK (inventory only)"
  exit 0
fi

echo "== unit tests: telemetry =="
cargo test -q --lib telemetry:: -- --test-threads=4

echo "== compile smoke (lib) =="
cargo check -q --lib

if [ "$fail" -eq 0 ]; then
  echo "OK tracing-check"
  exit 0
fi
echo "tracing-check FAILED" >&2
exit 65
