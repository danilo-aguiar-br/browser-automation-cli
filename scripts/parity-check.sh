#!/usr/bin/env bash
# Local gate: three-layer DevTools parity (GAP-021/023/024/043/044).
#
# Layer 1 name and layer 2 parameter are enumerable by scanning. Layer 3
# semantics is not: precondition and effect live in the reference handler
# declaration, which is why a name-only scoreboard read green while GAP-041 to
# GAP-043 stayed open.
#
# The reference tree and docs_prd/ are gitignored. When they are absent this
# gate SKIPS LOUDLY. A silent pass here would rebuild the exact blind spot the
# gate exists to remove.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

REF="$ROOT/base_conhecimento_chrome-devtools-mcp-main/src/tools"
MATRIX="$ROOT/docs_prd/parity_devtools_matrix.md"
REGISTRY="$ROOT/docs_prd/parity_intentional_divergences.json"
PRD="$ROOT/docs_prd/prd_browser-automation-cli.md"

missing=()
[[ -d "$REF" ]]       || missing+=("$REF")
[[ -f "$MATRIX" ]]    || missing+=("$MATRIX")
[[ -f "$REGISTRY" ]]  || missing+=("$REGISTRY")
[[ -f "$PRD" ]]       || missing+=("$PRD")

if [[ ${#missing[@]} -gt 0 ]]; then
  printf 'SKIP parity-check: absent inputs (gitignored tree):\n' >&2
  printf '  %s\n' "${missing[@]}" >&2
  printf 'This is NOT a pass. Run on a tree that has the reference checkout.\n' >&2
  exit 0
fi

BIN="$ROOT/target/debug/browser-automation-cli"
if [[ ! -x "$BIN" ]]; then
  printf 'SKIP parity-check: %s absent; run `cargo build` first. NOT a pass.\n' "$BIN" >&2
  exit 0
fi

fail=0

if ! python3 "$ROOT/scripts/gen-parity-matrix.py" --check; then
  printf 'FAIL: three-layer parity matrix stale or divergence untriaged\n' >&2
  fail=1
fi

if ! python3 "$ROOT/scripts/gen-flag-reconciliation.py" --check; then
  printf 'FAIL: PRD flag reconciliation stale\n' >&2
  fail=1
fi

# A frozen tool count in PRD prose is the defect that made the coverage claim
# false: it said 51 while the reference carried 53.
if rg -n '\b(51|52|53)[[:space:]]+tools\b|\b51/51\b' "$PRD" >/dev/null 2>&1; then
  printf 'FAIL: PRD freezes a reference tool count in prose:\n' >&2
  rg -n '\b(51|52|53)[[:space:]]+tools\b|\b51/51\b' "$PRD" >&2
  fail=1
fi

# `--verbose` on `view` is the global log flag, not the detail flag. Following
# the PRD mapping yields a reduced tree with exit zero and no error to notice.
if rg -n 'take_snapshot.*--verbose' "$PRD" >/dev/null 2>&1; then
  printf 'FAIL: PRD maps take_snapshot onto --verbose; the detail flag is --detailed\n' >&2
  fail=1
fi

if [[ $fail -eq 0 ]]; then
  printf 'PASS parity-check: three layers verified against the reference handlers\n'
fi
exit "$fail"
