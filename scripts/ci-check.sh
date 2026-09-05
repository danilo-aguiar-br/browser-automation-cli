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

# ── Verifier discovery, and why it is not the glob alone ────────────────────
# Discovery was `scripts/*-check.sh` and nothing else, so whether a blocking
# verifier ran depended on how its FILE was named. Measured 2026-08-26: four
# verifiers — `audit_bilingual_docs.sh`, `phantom-flag-gate.sh`,
# `split-conservation-audit.sh`, `verify-inventory-flat.sh` — had never once
# run inside this gate, purely because their names end in `_docs.sh`,
# `-gate.sh`, `-audit.sh` and `-flat.sh`. All four were green when finally run,
# so nothing was hidden this time; the point is that nothing WOULD have been
# shown either.
#
# A script now opts in by declaring `# ci-check: verifier` in its header. The
# glob stays so the existing 28 keep working with no edit, and intent replaces
# accident for everything else.
self="$(basename "$0")"
shopt -s nullglob
verifiers=()
for candidate in scripts/*.sh; do
  name="$(basename "$candidate")"
  [[ "$name" == "$self" ]] && continue
  if [[ "$name" == *-check.sh ]] || grep -q '^# ci-check: verifier' "$candidate"; then
    verifiers+=("$candidate")
  fi
done
shopt -u nullglob

# ── Progress, and the exit 124 it exists to prevent ─────────────────────────
# This gate builds release, runs the suite serially and then runs every
# verifier, each of which may build again. Measured 2026-08-26: a 900-second
# ceiling killed it during `cargo build --release`, and exit 124 reads as
# "the gate is broken" rather than "the auditor was in a hurry".
#
# The denominator is DERIVED from the literal `step` calls in this file plus
# the discovered list. A hand-written total drifts on the first insertion, and
# a denominator that lies is worse than no counter at all. Each line is printed
# BEFORE the work, so a kill is attributable to a named step.
STEP_FIXED="$(grep -c '^step "' "$0")"
STEP_TOTAL=$(( STEP_FIXED + ${#verifiers[@]} ))
STEP_N=0
printf 'ci-check: %d steps (%d fixed + %d verifiers)\n' \
  "$STEP_TOTAL" "$STEP_FIXED" "${#verifiers[@]}"
printf 'ci-check: budget 30-90 min on a warm target/ (MEASURED 2026-08-26:\n'
printf 'ci-check: 56.6 min at 44 steps, 30.2 min at 41); a lower ceiling\n'
printf 'ci-check: returns 124 and that is the ceiling, not a failure.\n'

# ── Citable artifact (OPP-GATE-BUNDLE) ──────────────────────────────────────
# Six audit waves wrote "filesize PASS (over_limit=0)" into gaps.md while the
# script reported 5 offenders. Nothing forced the CLAIM to come from a RUN, so
# the VERIFY list was copied from the previous wave instead of re-executed.
#
# Every step now appends its real verdict here. A close that cites this file
# cites an execution; a close that cites prose is visibly not citing this file.
# NOT under `target/`, and that is the whole point.
#
# This transcript is the citable evidence that the bundle ran and what it said.
# It used to live in `target/gates/`, which `cargo clean` removes and which one
# of the bundle's own steps rebuilds — so the proof of a run was stored in the
# most volatile directory in the repository. Measured 2026-08-28: the artefact
# for the 2026-08-26 green run no longer existed, so a closure that had been
# recorded as evidence could not be produced on request.
ARTIFACT_DIR="$ROOT/.gates"
ARTIFACT="$ARTIFACT_DIR/ci-check.txt"
mkdir -p "$ARTIFACT_DIR"
: >"$ARTIFACT"
printf 'ci-check run\nrepo=%s\n\n' "$ROOT" >>"$ARTIFACT"

# Exit 3 means "this verifier declined to run". It is recorded as SKIP and it
# FAILS the gate, because GAP-027 says a check that does not run is not a check
# that passed. A verifier that printed "this is NOT a pass" and returned 0 was
# tallied as PASS here, so `ci-check` exit 0 meant less than it claimed.
readonly SKIP_EXIT=3

step() {
  local name="$1"
  shift
  STEP_N=$(( STEP_N + 1 ))
  printf '\n== [%d/%d] %s ==\n' "$STEP_N" "$STEP_TOTAL" "$name"
  local rc=0
  "$@" || rc=$?
  if [[ $rc -eq 0 ]]; then
    printf 'PASS  %s\n' "$name"
    printf 'PASS  %s\n' "$name" >>"$ARTIFACT"
  elif [[ $rc -eq $SKIP_EXIT ]]; then
    printf 'SKIP  %s (declined to run; this is NOT a pass)\n' "$name"
    printf 'SKIP  %s\n' "$name" >>"$ARTIFACT"
    failed+=("$name (skipped)")
  else
    printf 'FAIL  %s\n' "$name"
    printf 'FAIL  %s\n' "$name" >>"$ARTIFACT"
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

# ── Lockfile freshness ──────────────────────────────────────────────────────
# Every step above resolves dependencies WITHOUT `--locked`, so each one is free
# to rewrite `Cargo.lock` and then succeed against the file it just wrote. That
# makes a stale lockfile structurally invisible here.
#
# Measured 2026-08-06: the manifest was bumped to 0.1.8 while `Cargo.lock` still
# carried 0.1.7. Ten local gates passed; `cargo build --release --locked` failed
# with "the lock file needs to be updated but --locked was passed". The first
# reader of that failure was the publish path, which is the worst place to learn
# it.
#
# `--locked` is the whole point of this step: it forbids the implicit rewrite, so
# the check fails on drift instead of quietly repairing it.
lockfile_check() {
  cargo build --locked --quiet
}
step "cargo build --locked (lockfile freshness)" lockfile_check

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

# ── Release artefact the downstream gates measure ───────────────────────────
# Ten of the auto-discovered verifiers resolve `target/release/browser-automation-cli`
# and this script never built it. The loop below walks `scripts/*-check.sh` in
# glob order, so `clap-schema-parity-check.sh` (c) runs long before
# `perf-check.sh` (p), which is the only step that ever ran `cargo build
# --release`. The result: a clean tree FAILED and a tree still holding the
# artefact from a previous run PASSED, with no change to any source file.
#
# Measured 2026-08-18: exit 1 with `clap-schema-parity-check.sh` FAIL on a tree
# whose `target/release` had been removed; the same gate returned exit 0 the
# moment the artefact existed. A verdict that depends on residue from an earlier
# invocation does not measure the tree, it measures the machine's history.
#
# Building here also pins WHICH binary the gates read: the resolution order in
# those scripts prefers `target/release`, then `target/debug`, then `PATH`. With
# no release artefact, a gate could certify this source tree using whatever
# version happens to be installed in `~/.cargo/bin`.
release_build() {
  cargo build --release --locked --quiet
}
step "cargo build --release (artefact for downstream gates)" release_build

# ── Provenance of the artefact every downstream gate reads ──────────────────
# The comment above pins WHICH binary the gates read; the ARTEFACT never said
# which one that was. A green verdict named the gates that passed and stayed
# silent on the thing they measured, and that silence is not neutral here: two
# resolvers in this repo (`generate_command_schemas.sh`, `gen-flag-reconciliation.sh`)
# break the release/debug tie with `-nt`, and an explicit `BIN=` beats both. So
# "PASS" alone cannot distinguish a gate that read THIS build from one that read
# an older artefact, a debug build, or a copy installed in `~/.cargo/bin`.
#
# Recording it costs one exec and one stat, and turns the artefact from a
# scoreboard into evidence: the reader can tell whether the verdict applies to
# the tree in front of them. `--version` and the byte count are used rather than
# a checksum because `sha256sum` is not portable to macOS, and this file must
# behave the same on all three supported hosts.
record_binary_provenance() {
  local bin="$ROOT/target/release/browser-automation-cli"
  {
    printf '\n== binary under test ==\n'
    printf 'path=%s\n' "$bin"
    if [[ -x "$bin" ]]; then
      printf 'version=%s\n' "$("$bin" --version 2>/dev/null | head -n1)"
      printf 'size_bytes=%s\n' "$(wc -c <"$bin" | tr -d ' ')"
    else
      printf 'status=MISSING (downstream gates may have read target/debug or PATH)\n'
    fi
    printf '\n'
  } >>"$ARTIFACT"
}
record_binary_provenance

# ── Generators, checked rather than run ─────────────────────────────────────
# A generated file that nobody regenerates is a stale file with a header that
# claims otherwise. These three write into the repo, so the gate runs them in
# `--check` mode: they report drift and touch nothing.
#
# Measured 2026-08-26, all three invisible to this gate because a generator's
# name never matched the verifier glob:
#   - `docs_prd/flag_reconciliation.md` still told the reader to run
#     `python3 scripts/gen-flag-reconciliation.py`, a file deleted in the
#     migration to bash. A generated document instructing you to run a script
#     that no longer exists.
#   - all four `llms*.txt` carried ZERO `GENERATED_COMMANDS_JSON` markers. The
#     machine-readable inventory agents are told to trust was simply absent.
#   - the schema check declined with exit 2 against a stale binary, which is
#     the script protecting its own comparison and is a real verdict too.
step "gen-flag-reconciliation --check" \
  bash scripts/gen-flag-reconciliation.sh --check
step "gen-llms-txt --check" \
  bash scripts/gen-llms-txt.sh --check
step "generate_command_schemas --check" \
  bash scripts/generate_command_schemas.sh --check

# ── Local rule verifiers (discovered above) ─────────────────────────────────
if [[ ${#verifiers[@]} -eq 0 ]]; then
  printf '\nFAIL  no verifier discovered (glob nor `# ci-check: verifier` mark)\n'
  failed+=("verifier discovery")
fi
for verifier in "${verifiers[@]}"; do
  name="$(basename "$verifier")"
  if [[ ! -x "$verifier" ]]; then
    STEP_N=$(( STEP_N + 1 ))
    printf '\n== [%d/%d] %s ==\nFAIL  %s (not executable: chmod +x %s)\n' \
      "$STEP_N" "$STEP_TOTAL" "$name" "$name" "$verifier"
    failed+=("$name (not executable)")
    continue
  fi
  step "$name" "$verifier"
done

# ── Summary ─────────────────────────────────────────────────────────────────
printf '\n== summary ==\n'
if [[ ${#failed[@]} -eq 0 ]]; then
  printf 'ci-check OK (all steps passed)\n' | tee -a "$ARTIFACT"
  printf 'artifact: %s\n' "$ARTIFACT"
  exit 0
fi

{
  printf 'ci-check FAILED (%d step(s)):\n' "${#failed[@]}"
  for name in "${failed[@]}"; do
    printf '  - %s\n' "$name"
  done
} | tee -a "$ARTIFACT"
printf 'artifact: %s\n' "$ARTIFACT"
exit 1
