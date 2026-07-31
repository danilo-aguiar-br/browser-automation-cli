#!/usr/bin/env bash
# Local hygiene gate for rules_rust_encerramento_graceful_shutdown (one-shot CLI).
# No GitHub Actions — run manually or from scripts/ci-check.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

source "$ROOT/scripts/lib/module_paths.sh"
module_paths_self_test || exit 65

# The entry surface (SIGPIPE handler, dual flush) moved out of lib.rs during
# GAP-051. These checks are about `run_from_args`, so anchor on the symbol that
# names it instead of the file that used to hold it.
ENTRY_FILES="$(files_defining 'pub fn run_from_args' src/)"

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
if rg -n 'pub async fn shutdown_signal' src/browser/shutdown.rs >/dev/null \
  || rg -n 'pub async fn shutdown_signal' src/browser/ >/dev/null; then
  pass "shutdown_signal central detector present"
else
  bad "missing pub async fn shutdown_signal"
fi

# Pass F: lifecycle is a directory (CancellationToken in ledger.rs).
if rg -n 'CancellationToken' src/lifecycle/ >/dev/null; then
  pass "CancellationToken in lifecycle"
else
  bad "CancellationToken missing from lifecycle"
fi

# 2b) Shared cancel-aware block_on (browser + I/O) with cancel-first bias
if rg -n 'pub fn block_on_with_shutdown' src/runtime_util.rs >/dev/null \
  && rg -n 'block_on_with_shutdown' src/browser/commands.rs >/dev/null \
  && rg -n 'block_on_with_shutdown' src/runtime_util.rs >/dev/null; then
  pass "block_on_with_shutdown shared by browser + block_on_io"
else
  bad "block_on_with_shutdown missing (I/O path must race cancel/signals)"
fi

if rg -n 'biased;' src/runtime_util.rs >/dev/null \
  && rg -A6 'biased;' src/runtime_util.rs | rg -n 'cancel\.cancelled' >/dev/null; then
  pass "biased select prioritizes cancel"
else
  bad "cancel-first biased select missing in block_on_with_shutdown"
fi

# 3) SIGTERM grace before SIGKILL (must not be back-to-back kills without wait)
if rg -n 'kill_unix_graceful|FINALIZE_CHILD_GRACE' src/lifecycle/ >/dev/null; then
  pass "SIGTERM→grace→SIGKILL residual path"
else
  bad "missing graceful residual kill helper"
fi

# Immediate dual kill antipattern in finalize body (two consecutive libc::kill without grace)
if rg -n 'SIGTERM' src/lifecycle/ >/dev/null \
  && rg -n 'SIGKILL' src/lifecycle/ >/dev/null \
  && ! rg -n 'kill_unix_graceful' src/lifecycle/ >/dev/null; then
  bad "SIGTERM/SIGKILL without kill_unix_graceful"
else
  pass "no immediate SIGTERM+SIGKILL without grace helper"
fi

# 4) Broken pipe / SIGPIPE contract
# shellcheck disable=SC2086  # ENTRY_FILES is a newline list of paths
if rg -n 'SIGPIPE|SIG_DFL' $ENTRY_FILES >/dev/null \
  && rg -n 'BrokenPipe' src/error.rs src/output.rs >/dev/null; then
  pass "SIGPIPE → BrokenPipe → 141 path present"
else
  bad "BrokenPipe/SIGPIPE contract incomplete"
fi

# 5) Dual flush before DIE
# shellcheck disable=SC2086
if rg -n 'flush_stdout' $ENTRY_FILES >/dev/null && rg -n 'flush_stderr' $ENTRY_FILES >/dev/null; then
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
if rg -n 'second shutdown signal' src/runtime_util.rs src/browser/ >/dev/null; then
  pass "double-signal force finalize documented in code"
else
  warn "double-signal force path string not found (check block_on_with_shutdown)"
fi

# 7b) Windows ctrl_close (console close) captured for one-shot
if rg -n 'ctrl_close|CtrlClose' src/browser/shutdown.rs >/dev/null; then
  pass "Windows ctrl_close trigger present"
else
  warn "Windows ctrl_close not registered (console close may skip cooperative cancel)"
fi

# 7c) Browser close wait is a named constant (not magic 5)
if rg -n 'BROWSER_CLOSE_WAIT_SECS' src/constants/ src/native/cdp/oxide.rs >/dev/null; then
  pass "BROWSER_CLOSE_WAIT_SECS named finalize wait"
else
  bad "BROWSER_CLOSE_WAIT_SECS missing from finalize path"
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
  # cargo test takes a single filter positional; two names are rejected as an
  # unexpected argument, so run one invocation per filter.
  cargo test --lib pre_cancelled_token_returns_exit_130 -- --nocapture
  cargo test --lib shutdown_trigger_labels -- --nocapture
  cargo test --lib runtime_util:: -- --nocapture
fi

if [[ "$fail" -ne 0 ]]; then
  echo "shutdown-check: FAILED"
  exit 1
fi
echo "shutdown-check: PASS"
exit 0
