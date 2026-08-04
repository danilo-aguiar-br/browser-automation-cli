#!/usr/bin/env bash
# DEPRECATED SHIM — the real gate is `scripts/inventory-flat-check.sh`.
#
# WHY THE RENAME
#   `scripts/ci-check.sh` auto-discovers verifiers with the glob
#   `scripts/*-check.sh`. This filename does NOT match it, so for the whole
#   video/audio cycle the gate existed, passed when run by hand, and never ran
#   in the bundle. Eighteen markdown files drifted to a stale count of 67 while
#   the runner reported green.
#
#   The file is kept (not deleted) because TESTING.md EN/PT and several audit
#   entries point at this path by name. It delegates so there is exactly one
#   implementation and no second source of truth.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

exec "$ROOT/scripts/inventory-flat-check.sh" "$@"

# ── Everything below is unreachable legacy, preserved verbatim ───────────────
# `exec` above replaces this process, so none of it runs. It stays as the
# historical record of what the gate checked before the coverage widened.

BIN="${BIN:-}"
if [[ -z "$BIN" ]]; then
  if [[ -x "$ROOT/target/debug/browser-automation-cli" ]]; then
    BIN="$ROOT/target/debug/browser-automation-cli"
  elif [[ -x "$ROOT/target/release/browser-automation-cli" ]]; then
    BIN="$ROOT/target/release/browser-automation-cli"
  else
    echo "verify-inventory-flat: FAIL (bin missing; build debug or release)" >&2
    echo "verify-inventory-flat: FAIL"
    exit 1
  fi
fi

# Bump EXPECTED when agent inventory grows (live SSOT: commands --json length).
EXPECTED=68
fail=0

if ! command -v jaq >/dev/null 2>&1 && ! command -v python3 >/dev/null 2>&1; then
  echo "verify-inventory-flat: FAIL (need jaq or python3)" >&2
  echo "verify-inventory-flat: FAIL"
  exit 1
fi

json="$("$BIN" --json commands 2>/dev/null || true)"
if [[ -z "$json" ]]; then
  echo "verify-inventory-flat: FAIL (commands --json empty)" >&2
  echo "verify-inventory-flat: FAIL"
  exit 1
fi

if command -v jaq >/dev/null 2>&1; then
  count="$(printf '%s' "$json" | jaq -r '.data.commands | length' 2>/dev/null || echo 0)"
  has_image="$(printf '%s' "$json" | jaq -r '[.data.commands[] | (if type=="object" then .name else . end)] | any(.=="image")' 2>/dev/null || echo false)"
  has_video="$(printf '%s' "$json" | jaq -r '[.data.commands[] | (if type=="object" then .name else . end)] | any(.=="video")' 2>/dev/null || echo false)"
  has_audio="$(printf '%s' "$json" | jaq -r '[.data.commands[] | (if type=="object" then .name else . end)] | any(.=="audio")' 2>/dev/null || echo false)"
else
  eval "$(printf '%s' "$json" | python3 -c 'import sys,json; d=json.load(sys.stdin); cmds=d.get("data",{}).get("commands",[]); names=[c.get("name") if isinstance(c,dict) else c for c in cmds]; print(f"count={len(names)}"); print(f"has_image={\"true\" if \"image\" in names else \"false\"}"); print(f"has_video={\"true\" if \"video\" in names else \"false\"}"); print(f"has_audio={\"true\" if \"audio\" in names else \"false\"}")')"
fi

if [[ "${count}" != "$EXPECTED" ]]; then
  echo "verify-inventory-flat: FAIL (commands count=${count} expected=${EXPECTED})" >&2
  fail=1
fi
if [[ "${has_image}" != "true" ]]; then
  echo "verify-inventory-flat: FAIL (missing image in commands)" >&2
  fail=1
fi
if [[ "${has_video}" != "true" ]]; then
  echo "verify-inventory-flat: FAIL (missing video in commands)" >&2
  fail=1
fi
if [[ "${has_audio}" != "true" ]]; then
  echo "verify-inventory-flat: FAIL (missing audio in commands)" >&2
  fail=1
fi

# Flat blocks: "hover image keys" without video is a regression
if command -v rg >/dev/null 2>&1; then
  if rg -n --glob '*.md' 'hover image keys' docs/ 2>/dev/null | rg -v 'hover image video keys' >/dev/null 2>&1; then
    echo "verify-inventory-flat: FAIL (docs flat 'hover image keys' without video)" >&2
    rg -n --glob '*.md' 'hover image keys' docs/ 2>/dev/null | rg -v 'hover image video keys' >&2 || true
    fail=1
  fi
fi

for readme in README.md README.pt-BR.md; do
  if [[ -f "$readme" ]]; then
    if ! rg -q '`video`' "$readme" 2>/dev/null && ! grep -q '`video`' "$readme" 2>/dev/null; then
      echo "verify-inventory-flat: FAIL (${readme} missing \`video\`)" >&2
      fail=1
    fi
    if ! rg -q '`audio`' "$readme" 2>/dev/null && ! grep -q '`audio`' "$readme" 2>/dev/null; then
      echo "verify-inventory-flat: FAIL (${readme} missing \`audio\`)" >&2
      fail=1
    fi
  fi
done

# Agent playbook honesty (residual-audio-03): AGENTS must list audio + live 68
for agents in docs/AGENTS.md docs/AGENTS.pt-BR.md; do
  if [[ -f "$agents" ]]; then
    if ! rg -q '`audio`' "$agents" 2>/dev/null && ! grep -q '`audio`' "$agents" 2>/dev/null; then
      echo "verify-inventory-flat: FAIL (${agents} missing \`audio\`)" >&2
      fail=1
    fi
    if ! rg -q '\*\*68\*\*' "$agents" 2>/dev/null && ! grep -q '**68**' "$agents" 2>/dev/null; then
      echo "verify-inventory-flat: FAIL (${agents} missing inventory **68**)" >&2
      fail=1
    fi
    if rg -n 'inventory \*\*67\*\*|inventário \*\*67\*\*|\(\*\*67\*\* agent|\(\*\*67 nomes|Inventory \(67\)|Inventário Completo de Comandos \(67\)' "$agents" 2>/dev/null | head -1 >/dev/null; then
      echo "verify-inventory-flat: FAIL (${agents} still has stale inventory **67** claim)" >&2
      rg -n 'inventory \*\*67\*\*|inventário \*\*67\*\*|\(\*\*67\*\* agent|\(\*\*67 nomes|Inventory \(67\)|Inventário Completo de Comandos \(67\)' "$agents" 2>/dev/null | head -5 >&2 || true
      fail=1
    fi
  fi
done

if [[ "$fail" -ne 0 ]]; then
  echo "verify-inventory-flat: FAIL"
  exit 1
fi
echo "verify-inventory-flat: OK (commands=${EXPECTED} image+video+audio; docs flat honest; AGENTS 68)"
exit 0
