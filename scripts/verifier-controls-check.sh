#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# POSITIVE CONTROLS for the verifiers re-anchored after GAP-051.
#
# # Why this file exists
#
# Seven verifiers were anchored on FILE PATHS (`rg 'fn init_tracing_local'
# src/tracing_local.rs`). Splitting a module into a directory made every one of
# them fail with `No such file or directory` while printing a message that
# blamed a missing function that was present and working. Red for the wrong
# reason.
#
# Re-anchoring them on symbols fixes that — and opens a worse failure mode. A
# widened search can match something unrelated, or an anchor can be deleted
# outright, and the verifier then passes forever. GREEN for the wrong reason is
# worse than red for the wrong reason, because nobody investigates green.
#
# So each re-anchored assertion gets a control here: copy the tree, DELETE the
# property, run the verifier, and require it to complain. A control that does
# not fail is a verifier that does not verify.
#
# This is mutation testing scoped to the gates. It never touches the real tree.
#
# Usage: ./scripts/verifier-controls-check.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad() { printf 'FAIL  %s\n' "$1"; fail=1; }

echo "== verifier-controls (each gate must DETECT the property's absence) =="

# run_control <label> <script+args> <expected-substring> <mutation-command...>
#
# The mutation runs inside a throwaway copy with $WORK as cwd. The verifier is
# then run there and must print <expected-substring>. Matching on the MESSAGE
# rather than the exit code keeps the control honest for gates that continue
# past a failed assertion to run cargo, which would muddy the exit status.
run_control() {
  local label="$1" script="$2" expected="$3"
  shift 3
  local work
  work="$(mktemp -d "${TMPDIR:-/tmp}/bac-verifier-control-XXXXXX")"
  # Copy the whole working tree minus the heavy, irrelevant parts.
  #
  # A partial copy is a trap: a gate that aborts early because the sandbox lacks
  # `benches/` or `llms.txt` never reaches the assertion under control, and the
  # control then reports the gate as blind when it is merely unreachable. That
  # false alarm cost two rounds here — the sandbox must be complete enough that
  # the gate fails for the reason the mutation created.
  fd -H -d 1 . "$ROOT" \
    -E target -E .git -E '*.sqlite*' -E '*.7z' \
    -E 'base_conhecimento*' -E 'docs_rules*' -E graphrag -E node_modules \
    -x cp -r {} "$work/" \; 2>/dev/null

  (cd "$work" && "$@") || {
    bad "$label: mutation command failed, control is inconclusive"
    rm -rf "$work"
    return
  }

  local out
  # Share the real target dir so a gate that runs cargo reuses the existing
  # build instead of compiling the dependency graph from scratch per control.
  # shellcheck disable=SC2086  # $script carries its own flags on purpose
  out="$(cd "$work" && CARGO_TARGET_DIR="$ROOT/target/verifier-controls" \
    timeout 900 bash $script 2>&1 || true)"
  if printf '%s' "$out" | rg -qF "$expected"; then
    pass "$label"
  else
    bad "$label: gate stayed silent after the property was removed"
    printf '      expected to see: %s\n' "$expected"
  fi
  rm -rf "$work"
}

# 1) tracing-check must notice init_tracing_local disappearing.
run_control "tracing-check detects missing init_tracing_local" \
  "scripts/tracing-check.sh --inventory-only" \
  "missing tracing_local module / init_tracing_local" \
  bash -c "sd 'fn init_tracing_local' 'fn renamed_away' \$(rg -l 'fn init_tracing_local' src/)"

# 2) tracing-check must notice the correlation span leaving the entry surface.
run_control "tracing-check detects missing cli_run span" \
  "scripts/tracing-check.sh --inventory-only" \
  "missing cli_run correlation span" \
  bash -c "sd 'cli_run' 'span_renamed' \$(rg -l 'cli_run' src/)"

# 3) shutdown-check must notice the dual flush going away.
run_control "shutdown-check detects missing dual flush" \
  "scripts/shutdown-check.sh --inventory-only" \
  "missing dual flush before exit" \
  bash -c "sd 'flush_stdout' 'flush_renamed' \$(rg -l 'flush_stdout' src/)"

# 4) process-check must notice lighthouse dropping the timed capture helper.
run_control "process-check detects lighthouse without run_capture_with_timeout" \
  "scripts/process-check.sh" \
  "lighthouse/ffmpeg missing run_capture_with_timeout" \
  bash -c "sd 'run_capture_with_timeout' 'capture_renamed' \$(rg -l 'run_capture_with_timeout' src/commands/ops/lighthouse)"

# 5) network-check must notice read_body_limited losing its wiring.
run_control "network-check detects unwired read_body_limited" \
  "scripts/network-check.sh" \
  "read_body_limited not wired" \
  bash -c "sd 'read_body_limited' 'body_renamed' \$(rg -l 'read_body_limited' src/robots)"

# 6) ownership-check must notice the pages() slice API vanishing.
run_control "ownership-check detects missing pages() slice API" \
  "scripts/ownership-check.sh" \
  "missing pages() slice API" \
  bash -c "sd 'fn pages' 'fn pages_renamed' \$(rg -l 'fn pages' src/native/browser/tabs)"

# 7) natives-check must notice its lighthouse property being removed.
run_control "natives-check detects missing lighthouse process helper" \
  "scripts/natives-check.sh" \
  "Pass M process helper missing or not wired" \
  bash -c "sd 'run_capture_with_timeout' 'capture_renamed' \$(rg -l 'run_capture_with_timeout' src/commands/ops/lighthouse)"

# 8) json-ndjson-check must notice a raw parser reappearing in a SPLIT module.
#
# This is the control that matters most here: before the fix the gate reported
# "no raw serde_json::from_str" for `discovery` and `lighthouse` because it read
# paths that no longer existed. It was green while asserting nothing.
run_control "json-ndjson-check detects raw serde_json in a split module" \
  "scripts/json-ndjson-check.sh" \
  "still calls serde_json::from_str directly" \
  bash -c "printf '\nfn __control() { let _ = serde_json::from_str::<serde_json::Value>(\"{}\"); }\n' >> src/native/cdp/discovery/mod.rs"

# 9) docs-check must notice the aquamarine feature gate being dropped.
run_control "docs-check detects ungated aquamarine" \
  "scripts/docs-check.sh" \
  "aquamarine must be gated behind feature docs-mermaid" \
  bash -c "sd 'feature = \"docs-mermaid\"' 'feature = \"other\"' \$(rg -l 'docs-mermaid' src/ --glob '*.rs')"

# 10) natives-check Pass N must notice a NEW native dependency arriving.
#
# The allowlist exists because the previous claim ("cc/cmake only via TLS") was
# folklore that nobody re-measured — SQLite, mimalloc and zstd compile C too. An
# allowlist that cannot detect an addition would be the same folklore in list form.
run_control "natives-check detects a new *-sys dependency" \
  "scripts/natives-check.sh" \
  "new native dependency not in the documented allowlist" \
  bash -c "printf '\n[[package]]\nname = \"totally-new-sys\"\nversion = \"1.0.0\"\n' >> Cargo.lock"

# 11) natives-check must notice openssl reaching the graph.
run_control "natives-check detects openssl in the graph" \
  "scripts/natives-check.sh" \
  "openssl reached the graph" \
  bash -c "printf '\n[[package]]\nname = \"openssl\"\nversion = \"0.10.0\"\n' >> Cargo.lock"

# 12) natives-check must notice aws-lc-sys LEAVING, which retires a prerequisite.
#
# The only control here that fires on good news. Without it, cmake would stay in
# the documented prerequisites forever after upstream stopped requiring it.
run_control "natives-check detects aws-lc-sys disappearing" \
  "scripts/natives-check.sh" \
  "aws-lc-sys is GONE" \
  bash -c "sd 'name = \"aws-lc-sys\"' 'name = \"aws-lc-sys-renamed\"' Cargo.lock"

# The agent-ops gate is the newest verifier and it exists precisely because a
# green unit suite coexisted with a broken binary. Giving it no control here
# would repeat the mistake one level up: a gate nobody proved can detect the
# absence of the property it claims to check.
#
# Deleting the FTL suggestion's `--fields` and putting `--select` back is the
# exact regression the gate was written for — `--select` is a real flag on
# scrape/crawl/map/search, so a naive "flag exists somewhere" check passes and
# only the scope-aware one fires.
run_control "agent-ops-check detects a suggestion citing a non-global flag" \
  "scripts/agent-ops-check.sh" \
  "absent from the global help" \
  bash -c "sd -- '--fields' '--select' locales/en.ftl"

# doc-coverage-check: the documentation surface is the largest thing in the
# product with no gate before this wave — 132 of 176 XDG keys appeared in no
# public document. Three controls, one per class of drift, because the three
# assertions fail for different reasons and a single mutation would only prove
# one of them alive.
# `heap_dominator_max_states` is chosen because it appears EXACTLY ONCE in the
# reference. A key such as `dialog_settle_ms` also appears inside an example
# command, so deleting its definition would leave the gate finding the other
# occurrence and passing — an inconclusive control that reads like a healthy one.
# The replacement must not begin with a hyphen either: `sd` parses that as a
# flag and exits 2, which the harness reports as "mutation failed" rather than
# as a gate defect. Both traps fired here before this comment existed.
run_control "doc-coverage-check detects an undocumented XDG key" \
  "scripts/doc-coverage-check.sh" \
  "omits 1 of 176 live XDG keys" \
  bash -c "sd 'heap_dominator_max_states' 'KEY_REMOVED_FOR_CONTROL' docs/CONFIGURATION.md"

run_control "doc-coverage-check detects a command that no entry-point document names" \
  "scripts/doc-coverage-check.sh" \
  "never names 1 of 69 live commands" \
  bash -c "sd -- '\`screencast' 'SCREENCAST_REMOVED_FOR_CONTROL' README.md"

# The scope assertion is the one most likely to rot into a false-green, because
# the naive version of it passes: `--select` really does exist on `scrape`.
run_control "doc-coverage-check detects a per-command flag presented as global" \
  "scripts/doc-coverage-check.sh" \
  "present a per-command flag as global" \
  bash -c "printf '%s\n' '- These flags are GLOBAL on every one of the 69 commands: \`--select\`' >> docs/ROADMAP.md"

if [ "$fail" -eq 0 ]; then
  echo "== verifier-controls OK =="
  exit 0
fi
echo "== verifier-controls FAILED ==" >&2
exit 65
