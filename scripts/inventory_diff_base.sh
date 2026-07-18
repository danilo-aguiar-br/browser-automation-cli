#!/usr/bin/env bash
# Local inventory gate (GAP-015/025): base knowledge tool names ⊆ CLI map + fixture.
# No GitHub Actions — run manually or from scripts/gates_v0.1.3.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASE="$ROOT/base_conhecimento_chrome-devtools-mcp-main/src/tools"
FIXTURE="$ROOT/tests/fixtures/tool-reference.md"
BIN="${BIN:-$ROOT/target/release/browser-automation-cli}"

if [[ ! -d "$BASE" ]]; then
  echo "SKIP: base knowledge dir missing"
  exit 0
fi

mapfile -t BASE_NAMES < <(rg -o "name: '[a-zA-Z0-9_]+'" "$BASE" --no-filename | sed "s/name: '//;s/'//" | sort -u)
mapfile -t FIXTURE_NAMES < <(rg -o '^### `[^`]+`' "$FIXTURE" | sed 's/^### `//;s/`$//' | sort -u)

# Aliases that are not separate CLI tools
declare -A ALIASES=(
  [evaluate]=evaluate_script
  [navigate]=navigate_page
  [screenshot]=take_screenshot
)

fail=0
for n in "${BASE_NAMES[@]}"; do
  canon="${ALIASES[$n]:-$n}"
  if printf '%s\n' "${FIXTURE_NAMES[@]}" | rg -qx -- "$canon" || printf '%s\n' "${FIXTURE_NAMES[@]}" | rg -qx -- "$n"; then
    continue
  fi
  echo "MISSING_IN_FIXTURE: $n (canonical=$canon)"
  fail=1
done

if [[ -x "$BIN" ]]; then
  mapfile -t CLI_TOOLS < <("$BIN" commands --json 2>/dev/null | jaq -r '.data.devtools_tool_map[]?.tool // empty' 2>/dev/null | sort -u || true)
  if [[ ${#CLI_TOOLS[@]} -gt 0 ]]; then
    for n in "${BASE_NAMES[@]}"; do
      canon="${ALIASES[$n]:-$n}"
      if printf '%s\n' "${CLI_TOOLS[@]}" | rg -qx -- "$canon" || printf '%s\n' "${CLI_TOOLS[@]}" | rg -qx -- "$n"; then
        continue
      fi
      # get_tab_id maps to page tab-id
      if [[ "$n" == "get_tab_id" ]] && "$BIN" page tab-id --help >/dev/null 2>&1; then
        continue
      fi
      echo "MISSING_IN_CLI: $n"
      fail=1
    done
  fi
fi

if [[ "$fail" -ne 0 ]]; then
  echo "inventory_diff_base: FAIL"
  exit 1
fi
echo "inventory_diff_base: OK (${#BASE_NAMES[@]} base names)"
