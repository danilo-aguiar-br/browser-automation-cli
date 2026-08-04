#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Agent-ops contract gate — runs the COMPILED BINARY, not the functions.
#
# # Why this file exists
#
# `src/agent_ops/tests.rs` has 17 unit tests and they all pass. One of them,
# `a_ceiling_that_cannot_be_met_is_reported_not_mangled`, proves that `apply()`
# builds the over-budget error correctly. It proved that while `doctor` threw
# that exact error away in `Err(_) => {}` and returned exit 0 with an empty
# stdout — an agent asking for a payload the CLI could not deliver was told the
# host was healthy.
#
# The unit test and the defect were in different files, and no gate crossed that
# boundary: `tests/` had ZERO integration coverage of the eight agent-ops flags
# (the single `--fields` match in `tests/clap_arg_coverage.rs` is `--fields-json`
# from `fill-form`). A function that returns the right error is worth nothing if
# the process discards it, so every assertion here goes through argv and reads
# the exit code.
#
# CLEAN STDOUT: one status line per assertion on stdout; diagnostics on stderr.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Binary resolution, in order. The PATH fallback is not convenience: the
# verifier-controls harness copies the tree WITHOUT `target/` and runs this gate
# there, so a build-dir-only lookup made the gate abort before reaching any
# assertion — and a control that reaches no assertion reports the gate as blind
# when it is merely unreachable.
BIN="${BIN:-}"
for candidate in \
  "$BIN" \
  "$ROOT/target/release/browser-automation-cli" \
  "$ROOT/target/debug/browser-automation-cli" \
  "$(command -v browser-automation-cli 2>/dev/null || true)"; do
  if [[ -n "$candidate" && -x "$candidate" ]]; then
    BIN="$candidate"
    break
  fi
done
if [[ ! -x "$BIN" ]]; then
  echo "agent-ops-check: FAIL (no binary; run cargo build --release)" >&2
  echo "agent-ops-check: FAIL"
  exit 1
fi

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad() {
  printf 'FAIL  %s\n' "$1"
  fail=1
}

echo "== agent-ops (binary contract, not unit behaviour) =="

# ── 1. An impossible ceiling must be reported, never swallowed ──────────
# `doctor` is the command agents use to validate residual-zero. It was the one
# command that answered exit 0 with empty stdout here, which reads as success.
for cmd in "doctor --offline --quick" "version" "commands"; do
  out="$("$BIN" -q --json --max-output-bytes 10 $cmd 2>&1)"
  ec=$?
  if [[ "$ec" -ne 2 ]]; then
    bad "impossible ceiling on '$cmd' returned exit $ec (want 2)"
  elif ! printf '%s' "$out" | rg -q '"ok":false'; then
    bad "impossible ceiling on '$cmd' did not emit an error envelope"
  else
    pass "impossible ceiling on '$cmd' reports exit 2 with an envelope"
  fi
done

# A ceiling the payload cannot meet must never produce a silent empty stdout.
out="$("$BIN" -q --json --max-output-bytes 4000 doctor --offline --quick 2>/dev/null)"
if [[ -z "$out" ]]; then
  bad "doctor at a 4000-byte ceiling emitted nothing at all"
else
  pass "doctor at a 4000-byte ceiling emits a payload or an error"
fi

# ── 2. A path that does not resolve must be named, not silently dropped ──
# `--fields typo` returned data:{} with exit 0; `--sort-rows typo` returned the
# rows untouched with matched == total. Both read as success.
probe_unresolved() {
  local label="$1" flag="$2" value="$3"
  local out
  out="$("$BIN" -q --json "$flag" "$value" doctor --offline --quick 2>/dev/null)"
  if printf '%s' "$out" | rg -q '"unresolved_paths"'; then
    pass "$label names the unresolved path"
  else
    bad "$label did not report the unresolved path (silent success)"
  fi
}
probe_unresolved "--fields with a bad path" --fields "nao.existe.mesmo"
probe_unresolved "--sort-rows with a bad path" --sort-rows "nao_existe"
probe_unresolved "--dedupe-by with a bad path" --dedupe-by "nao_existe"

# A path that DOES resolve must stay quiet: the envelope of a clean projection
# has to remain byte-identical to what consumers already parse.
out="$("$BIN" -q --json --fields checks doctor --offline --quick 2>/dev/null)"
if printf '%s' "$out" | rg -q '"unresolved_paths"'; then
  bad "a resolving --fields emitted unresolved_paths (noise on the happy path)"
else
  pass "a resolving --fields keeps the envelope quiet"
fi

# ── 3. No suggestion may cite a flag that does not exist in its scope ────
# The `agent-ops-*` messages are emitted for ANY command, so they may only cite
# GLOBAL flags. They used to say `--select`, which exists only on scrape, crawl,
# map, search, batch-scrape and the media `info` subcommands — so following the
# advice on the other 61 commands produced `unexpected argument '--select'`.
#
# A naive "does this flag exist anywhere" check would have PASSED on `--select`.
# Scope is the whole point of this assertion.
global_help="$("$BIN" --help 2>&1)"
scan_scope_violations() {
  local file="$1" lang="$2"
  local line key flag
  while IFS= read -r line; do
    key="${line%%=*}"
    key="$(printf '%s' "$key" | tr -d '[:space:]')"
    case "$key" in
      agent-ops-*) ;;
      *) continue ;;
    esac
    while IFS= read -r flag; do
      [[ -z "$flag" ]] && continue
      if ! printf '%s' "$global_help" | rg -q -- "$flag"; then
        bad "$lang '$key' cites $flag, absent from the global help"
        return
      fi
    done < <(printf '%s' "$line" | rg -o -- '--[a-z][a-z0-9-]+' | sort -u)
  done <"$file"
  pass "$lang agent-ops suggestions cite only global flags"
}
scan_scope_violations locales/en.ftl "en"
scan_scope_violations locales/pt-BR.ftl "pt-BR"

if [[ "$fail" -ne 0 ]]; then
  echo "agent-ops-check: FAIL"
  exit 1
fi
echo "agent-ops-check: OK"
