#!/usr/bin/env bash
# Local hygiene gate for rules_rust_encerramento_graceful_shutdown (one-shot CLI).
# No GitHub Actions — run manually or from scripts/ci-check.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
warn() { printf 'WARN  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; fail=1; }

echo "== shutdown-check (one-shot graceful shutdown hygiene) =="

# 1) No std::process::exit (must use ExitCode + finalize)
if rg -n 'std::process::exit|process::exit\(' src/ --glob '*.rs' | rg -v '^\s*//' >/dev/null 2>&1; then
  bad "found process::exit in src (prefer ExitCode + Lifecycle::finalize)"
  rg -n 'std::process::exit|process::exit\(' src/ --glob '*.rs' || true
else
  pass "no process::exit in src"
fi

# 2) Central detector + cancel token
if rg -n 'pub async fn shutdown_signal' src/browser/mod.rs >/dev/null; then
  pass "shutdown_signal central detector present"
else
  bad "missing pub async fn shutdown_signal"
fi

if rg -n 'CancellationToken' src/lifecycle.rs >/dev/null; then
  pass "CancellationToken in lifecycle"
else
  bad "CancellationToken missing from lifecycle"
fi

# 3) SIGTERM grace before SIGKILL (must not be back-to-back kills without wait)
if rg -n 'kill_unix_graceful|FINALIZE_CHILD_GRACE' src/lifecycle.rs >/dev/null; then
  pass "SIGTERM→grace→SIGKILL residual path"
else
  bad "missing graceful residual kill helper"
fi

# Immediate dual kill antipattern in finalize body (two consecutive libc::kill without grace)
if rg -n 'SIGTERM' src/lifecycle.rs >/dev/null \
  && rg -n 'SIGKILL' src/lifecycle.rs >/dev/null \
  && ! rg -n 'kill_unix_graceful' src/lifecycle.rs >/dev/null; then
  bad "SIGTERM/SIGKILL without kill_unix_graceful"
else
  pass "no immediate SIGTERM+SIGKILL without grace helper"
fi

# 4) Broken pipe / SIGPIPE contract
if rg -n 'SIGPIPE|SIG_DFL' src/lib.rs >/dev/null \
  && rg -n 'BrokenPipe' src/error.rs src/output.rs >/dev/null; then
  pass "SIGPIPE → BrokenPipe → 141 path present"
else
  bad "BrokenPipe/SIGPIPE contract incomplete"
fi

# 5) Dual flush before DIE
if rg -n 'flush_stdout' src/lib.rs >/dev/null && rg -n 'flush_stderr' src/lib.rs >/dev/null; then
  pass "dual flush in run_from_args"
else
  bad "missing dual flush before exit"
fi

# 6) Exit codes 130 / 141
if rg -n 'Cancelled => 130|Cancelled => "cancelled"' src/error.rs >/dev/null \
  || rg -n 'ErrorKind::Cancelled => 130' src/error.rs >/dev/null; then
  pass "exit 130 Cancelled"
else
  # broader
  if rg -n 'Cancelled => 130' src/error.rs >/dev/null; then
    pass "exit 130 Cancelled"
  else
    bad "Cancelled exit code not 130"
  fi
fi

if rg -n 'BrokenPipe => 141' src/error.rs >/dev/null; then
  pass "exit 141 BrokenPipe"
else
  bad "BrokenPipe exit code not 141"
fi

# 7) Second-signal force finalize
if rg -n 'second shutdown signal' src/browser/mod.rs >/dev/null; then
  pass "double-signal force finalize documented in code"
else
  warn "double-signal force path string not found (check block_on_browser_timeout)"
fi

# 8) No daemon crates forced for one-shot
if rg -n 'tokio-graceful-shutdown|tokio-graceful"' Cargo.toml >/dev/null 2>&1; then
  warn "daemon shutdown crate in Cargo.toml (may be overkill for one-shot)"
else
  pass "no tokio-graceful* daemon crate (one-shot OK)"
fi

# 9) Inventory-only unit tests
if [[ "${1:-}" == "--inventory-only" ]]; then
  echo "== inventory-only: skip cargo test =="
else
  echo "== unit tests (lifecycle + cancel 130) =="
  cargo test --lib lifecycle:: -- --nocapture
  cargo test --lib pre_cancelled_token_returns_exit_130 shutdown_trigger_labels -- --nocapture
fi

if [[ "$fail" -ne 0 ]]; then
  echo "shutdown-check: FAILED"
  exit 1
fi
echo "shutdown-check: PASS"
exit 0
