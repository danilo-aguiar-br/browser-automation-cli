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

source "$ROOT/scripts/lib/module_paths.sh"
module_paths_self_test || exit 65

# Resolved once: a module is `x.rs` OR `x/`, and this gate must not care which.
TRACING_LOCAL="$(mod_path src/tracing_local)"
ROBOTS="$(mod_path src/robots)"
# The entry surface moved out of lib.rs during GAP-051. Follow the symbol that
# defines it rather than the file that used to hold it.
ENTRY_FILES="$(files_defining 'pub fn run_from_args' src/)"

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

# 3) Dedicated tracing_local module + init
if rg -q 'fn init_tracing_local' "$TRACING_LOCAL"; then
  pass "init_tracing_local in $TRACING_LOCAL"
else
  bad "missing tracing_local module / init_tracing_local"
fi

# 4) WorkerGuard held (no executable mem::forget)
if rg -n 'mem::forget\(|std::mem::forget\(' src/ --glob '*.rs' >/dev/null 2>&1; then
  bad "mem::forget(...) still present (prefer named TracingLocalGuard drop)"
  rg -n 'mem::forget\(|std::mem::forget\(' src/ --glob '*.rs' || true
else
  pass "no mem::forget(...) (WorkerGuard via TracingLocalGuard)"
fi

# shellcheck disable=SC2086  # ENTRY_FILES is a newline list of paths, intentionally split
if rg -q 'TracingLocalGuard|_tracing_local|WorkerGuard' $ENTRY_FILES "$TRACING_LOCAL"; then
  pass "TracingLocalGuard / WorkerGuard lifecycle wired"
else
  bad "guard lifecycle not wired from run()"
fi

# 5) Rolling builder + max_log_files (XDG-aware)
if rg -q 'max_log_files|clamp_max_log_files|DEFAULT_MAX_LOG_FILES' "$TRACING_LOCAL" \
  && rg -q 'RollingFileAppender::builder|rolling::RollingFileAppender::builder' "$TRACING_LOCAL"; then
  pass "RollingFileAppender builder + max_log_files"
else
  bad "missing rolling Builder / max_log_files"
fi

# 6) ErrorLayer present
if rg -q 'ErrorLayer' "$TRACING_LOCAL"; then
  pass "ErrorLayer (SpanTrace)"
else
  bad "missing ErrorLayer"
fi

# 7) No product RUST_LOG *read* in tracing_local
if rg -n 'env::var(_os)?\([^\)]*RUST_LOG|std::env::var.*"RUST_LOG"' "$TRACING_LOCAL" >/dev/null 2>&1; then
  bad "tracing_local must not read RUST_LOG (XDG + argv only)"
else
  pass "no RUST_LOG env read in tracing_local (product law)"
fi

# 8) No remote OTEL / slog / env_logger *usage*
if rg -n 'use env_logger|env_logger::|use slog|slog::|opentelemetry::|OTEL_EXPORTER' src/ --glob '*.rs' >/dev/null 2>&1; then
  bad "remote/observability anti-pattern import/use in src"
  rg -n 'use env_logger|env_logger::|use slog|slog::|opentelemetry::|OTEL_EXPORTER' src/ --glob '*.rs' || true
else
  pass "no OTEL/env_logger/slog usage in src"
fi

# 9) Production eprintln/dbg for diagnostics (allow tests + build.rs)
if rg -n 'eprintln!|dbg!' "$ROBOTS" src/native/browser/ src/native/snapshot/ "$TRACING_LOCAL" $ENTRY_FILES 2>/dev/null | rg -v '^\s*//' >/dev/null 2>&1; then
  bad "eprintln!/dbg! in production hot modules"
  rg -n 'eprintln!|dbg!' "$ROBOTS" src/native/browser/ src/native/snapshot/ "$TRACING_LOCAL" $ENTRY_FILES || true
else
  pass "no eprintln!/dbg! in tracing/robots/native hot paths"
fi

# 10) Panic bridge after subscriber
if rg -q 'install_panic_tracing_bridge|target: "panic"' "$TRACING_LOCAL"; then
  pass "panic → tracing error bridge"
else
  bad "missing panic tracing bridge"
fi

# 11) Pass I: XDG keys max_log_files + log_rotation (dual-path)
if rg -q 'max_log_files' src/xdg/config_model.rs \
  && rg -q 'log_rotation' src/xdg/config_model.rs \
  && rg -q '"max_log_files"' src/xdg/config_ops/ \
  && rg -q '"log_rotation"' src/xdg/config_ops/ \
  && rg -q 'max_log_files' src/xdg/config_io.rs \
  && rg -q 'log_rotation' src/xdg/config_io.rs; then
  pass "XDG max_log_files + log_rotation (model/ops/io dual-path)"
else
  bad "missing max_log_files/log_rotation XDG dual-path"
fi

# 12) Pass I: validate log_level directive on config set
if rg -q 'validate_log_level_directive' "$TRACING_LOCAL" \
  && rg -q 'validate_log_level_directive' src/xdg/config_ops/; then
  pass "log_level EnvFilter validation on config set"
else
  bad "missing validate_log_level_directive wiring"
fi

# 13) Pass I: log_dir() helper + single load_config in init
if rg -q 'fn log_dir' src/xdg/paths.rs && rg -q 'log_dir()' "$TRACING_LOCAL"; then
  pass "xdg::log_dir used by tracing_local"
else
  bad "missing log_dir helper or usage"
fi

# Count load_config calls in init_tracing_local body (expect one)
load_count=$(rg -n 'load_config\(\)' "$TRACING_LOCAL" | rg -v '^\s*//|//!' | wc -l | tr -d ' ')
if [ "${load_count}" -eq 1 ]; then
  pass "single load_config in tracing_local (memory DRY)"
else
  bad "expected exactly 1 load_config in tracing_local (found ${load_count})"
fi

# 14) Pass I: correlation span in run()
# shellcheck disable=SC2086
if rg -q 'cli_run' $ENTRY_FILES && rg -q 'correlation_id' $ENTRY_FILES; then
  pass "cli_run span with correlation_id in run()"
else
  bad "missing cli_run correlation span"
fi

# 15) Named defaults in constants
if rg -q 'DEFAULT_LOG_LEVEL' src/constants/ \
  && rg -q 'DEFAULT_MAX_LOG_FILES' src/constants/ \
  && rg -q 'DEFAULT_LOG_ROTATION' src/constants/; then
  pass "named log defaults in constants.rs"
else
  bad "missing DEFAULT_LOG_* constants"
fi

if [ "$INVENTORY_ONLY" -eq 1 ]; then
  [ "$fail" -eq 0 ] || exit 65
  echo "OK (inventory only)"
  exit 0
fi

echo "== unit tests: tracing_local + xdg log keys =="
cargo test -q --lib tracing_local:: -- --test-threads=4
cargo test -q --lib xdg:: -- --test-threads=4

echo "== integration schema =="
cargo test -q --test tracing_local_log_schema -- --test-threads=2

echo "== compile smoke (lib) =="
cargo check -q --lib

if [ "$fail" -eq 0 ]; then
  echo "OK tracing-check"
  exit 0
fi
echo "tracing-check FAILED" >&2
exit 65
