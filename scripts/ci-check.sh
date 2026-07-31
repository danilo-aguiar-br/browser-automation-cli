#!/usr/bin/env bash
# Local CI gate (project may gitignore .github/). Blocking by design.
#
# GAP-027: every step is mandatory. There is no `|| true` in this file and no
# silent skip — a missing tool is a FAILURE with install instructions, because a
# gate that quietly drops a check is worse than no gate.
#
# Steps run to completion and failures are collected, so one broken step does
# not hide the rest. Exit status is non-zero when any step failed.
#
# Verifier discovery: every executable `scripts/*-check.sh` (except this file)
# runs automatically. Drop a new verifier in scripts/ and the gate picks it up —
# no edit here.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Deterministic glob order regardless of operator locale.
export LC_ALL=C

failed=()

step() {
  local name="$1"
  shift
  printf '\n== %s ==\n' "$name"
  if "$@"; then
    printf 'PASS  %s\n' "$name"
  else
    printf 'FAIL  %s\n' "$name"
    failed+=("$name")
  fi
}

require_tool() {
  local bin="$1" install="$2"
  if ! command -v "$bin" >/dev/null 2>&1; then
    printf 'missing required tool: %s\ninstall with: %s\n' "$bin" "$install" >&2
    return 1
  fi
  return 0
}

# ── Formatting ──────────────────────────────────────────────────────────────
fmt_check() {
  require_tool rustfmt "rustup component add rustfmt" || return 1
  cargo fmt --all -- --check
}
step "cargo fmt (check)" fmt_check

# ── Lint ────────────────────────────────────────────────────────────────────
clippy_check() {
  require_tool cargo-clippy "rustup component add clippy" || return 1
  cargo clippy --all-targets --all-features -- -D warnings
}
step "cargo clippy (-D warnings)" clippy_check

# ── Docs ────────────────────────────────────────────────────────────────────
# Broken intra-doc links are contract rot for a crate published on docs.rs.
doc_check() {
  RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --quiet
}
step "cargo doc (-D warnings)" doc_check

# ── Tests ───────────────────────────────────────────────────────────────────
# Two steps, because compiling everything and RUNNING everything need different
# flags.
#
# 1. `--all-targets --no-run` keeps benches and examples in the compile set, so
#    a bench that stopped compiling still cannot pass unnoticed (GAP-005).
#
# 2. `--tests -- --test-threads=1` runs the suite SERIALLY. Measured cause: the
#    libtest harness runs the tests INSIDE each binary on parallel threads, and
#    this crate has ten gates that each launch Chrome. Concurrent launches
#    produced three distinct failures — `SingletonLock: No such file or
#    directory`, `No chromiumoxide Page for session_id`, and `Page.navigate:
#    Request timed out` — all above load 45 and all absent when serialized.
#    Measured 2/2 clean serial runs (exit 0, zero failed binaries) against
#    intermittent failures in parallel.
#
#    Serial is also FASTER here: 101s serial vs 148s parallel. Thirty-two
#    Chromes contending cost more than they save, so this buys reliability at
#    negative time cost.
#
#    NOT `--all-targets` in step 2: criterion benches reject `--test-threads`
#    and the whole invocation dies with `unexpected argument`.
step "cargo test (compile all targets)" \
  cargo test --all-targets --all-features --no-run --quiet
step "cargo test --tests (serial)" \
  cargo test --tests --all-features --quiet -- --test-threads=1

# ── Binary smoke (clap debug_assert on the real argv tree) ───────────────────
binary_smoke() {
  cargo run --quiet -- version --json >/dev/null &&
    cargo run --quiet -- man >/dev/null &&
    cargo run --quiet -- completions bash >/dev/null
}
step "binary smoke (version/man/completions)" binary_smoke

# ── Supply chain ────────────────────────────────────────────────────────────
audit_check() {
  require_tool cargo-audit "cargo install cargo-audit" || return 1
  cargo audit
}
step "cargo audit" audit_check

deny_check() {
  require_tool cargo-deny "cargo install cargo-deny" || return 1
  if [[ ! -f deny.toml ]]; then
    printf 'deny.toml missing at repo root\n' >&2
    return 1
  fi
  cargo deny check
}
step "cargo deny" deny_check

# ── Local rule verifiers (auto-discovered) ──────────────────────────────────
self="$(basename "$0")"
shopt -s nullglob
found_verifier=0
for verifier in scripts/*-check.sh; do
  name="$(basename "$verifier")"
  [[ "$name" == "$self" ]] && continue
  found_verifier=1
  if [[ ! -x "$verifier" ]]; then
    printf '\n== %s ==\nFAIL  %s (not executable: chmod +x %s)\n' "$name" "$name" "$verifier"
    failed+=("$name (not executable)")
    continue
  fi
  step "$name" "$verifier"
done
shopt -u nullglob

if [[ "$found_verifier" -eq 0 ]]; then
  printf '\nFAIL  no scripts/*-check.sh verifier discovered\n'
  failed+=("verifier discovery")
fi

# ── Summary ─────────────────────────────────────────────────────────────────
printf '\n== summary ==\n'
if [[ ${#failed[@]} -eq 0 ]]; then
  printf 'ci-check OK (all steps passed)\n'
  exit 0
fi

printf 'ci-check FAILED (%d step(s)):\n' "${#failed[@]}"
for name in "${failed[@]}"; do
  printf '  - %s\n' "$name"
done
exit 1
