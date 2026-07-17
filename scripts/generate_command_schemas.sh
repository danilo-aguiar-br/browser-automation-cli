#!/usr/bin/env bash
# Generate static JSON Schema files for every CLI command from live meta.rs surface.
# Source of truth: `browser-automation-cli schema --cmd <name> --json`
# Usage:
#   bash scripts/generate_command_schemas.sh           # write docs/schemas/<cmd>.schema.json
#   bash scripts/generate_command_schemas.sh --check    # exit 1 if any file would change
# Env:
#   BIN=/path/to/browser-automation-cli
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="${BIN:-$ROOT/target/release/browser-automation-cli}"
if [[ ! -x "$BIN" ]]; then
  BIN="$ROOT/target/debug/browser-automation-cli"
fi
if [[ ! -x "$BIN" ]]; then
  echo "error: binary not found; build with cargo build --release --locked or set BIN=" >&2
  exit 2
fi

CHECK=0
if [[ "${1:-}" == "--check" ]]; then
  CHECK=1
fi

OUT_DIR="$ROOT/docs/schemas"
mkdir -p "$OUT_DIR"

REPO_ID_BASE="https://github.com/danilo-aguiar-br/browser-automation-cli/docs/schemas"

# Inventory from live CLI (must match meta::COMMANDS)
mapfile -t COMMANDS < <(
  "$BIN" --json commands 2>/dev/null | python3 -c '
import json, sys
raw = sys.stdin.read()
data = json.loads(raw)
cmds = data.get("data", data).get("commands")
if not isinstance(cmds, list) or not cmds:
    sys.exit("commands --json missing data.commands")
for c in cmds:
    print(c)
'
)

if [[ ${#COMMANDS[@]} -lt 1 ]]; then
  echo "error: empty command inventory from $BIN" >&2
  exit 2
fi

changed=0
wrote=0
checked_ok=0

for cmd in "${COMMANDS[@]}"; do
  # file name: command name is already kebab-case
  outfile="$OUT_DIR/${cmd}.schema.json"
  live_json="$("$BIN" --json schema --cmd "$cmd" 2>/dev/null || true)"
  if [[ -z "$live_json" ]]; then
    echo "error: schema --cmd $cmd returned empty" >&2
    exit 2
  fi

  rendered="$(
    CMD_NAME="$cmd" REPO_ID_BASE="$REPO_ID_BASE" python3 -c '
import json, os, sys

cmd = os.environ["CMD_NAME"]
base = os.environ["REPO_ID_BASE"]
raw = sys.stdin.read()
try:
    envelope = json.loads(raw)
except json.JSONDecodeError as e:
    sys.stderr.write(f"invalid json for {cmd}: {e}\n")
    sys.exit(2)

data = envelope.get("data", envelope)
# Prefer nested schema object from meta schema_for_cmd
schema = data.get("schema")
if not isinstance(schema, dict):
    schema = {
        "type": data.get("type", "object"),
        "description": data.get("description", f"{cmd} command input"),
        "properties": data.get("properties") or {},
        "required": data.get("required") or [],
        "additionalProperties": False,
    }

props = schema.get("properties")
if props is None:
    props = {}
required = schema.get("required")
if required is None:
    required = []
description = schema.get("description") or f"{cmd} command input"
stype = schema.get("type") or "object"
additional = schema.get("additionalProperties", False)

doc = {
    "$schema": "http://json-schema.org/draft-07/schema#",
    "$id": f"{base}/{cmd}.schema.json",
    "title": f"{cmd} command input",
    "type": stype,
    "description": description,
    "properties": props,
    "required": required,
    "additionalProperties": additional,
}

print(json.dumps(doc, indent=2, ensure_ascii=False, sort_keys=False))
print()  # trailing newline
' <<<"$live_json"
  )"

  if [[ "$CHECK" -eq 1 ]]; then
    if [[ ! -f "$outfile" ]]; then
      echo "MISSING $outfile"
      changed=$((changed + 1))
      continue
    fi
    if ! printf '%s' "$rendered" | diff -u "$outfile" - >/dev/null; then
      echo "DRIFT  $outfile"
      printf '%s' "$rendered" | diff -u "$outfile" - | head -40 || true
      changed=$((changed + 1))
    else
      checked_ok=$((checked_ok + 1))
    fi
  else
    # write only if different (stable mtime when unchanged)
    if [[ -f "$outfile" ]] && printf '%s' "$rendered" | diff -q "$outfile" - >/dev/null 2>&1; then
      :
    else
      printf '%s' "$rendered" >"$outfile"
      wrote=$((wrote + 1))
      echo "WROTE  $outfile"
    fi
  fi
done

if [[ "$CHECK" -eq 1 ]]; then
  echo "check: ok=$checked_ok drift_or_missing=$changed total_commands=${#COMMANDS[@]}"
  if [[ "$changed" -ne 0 ]]; then
    exit 1
  fi
  exit 0
fi

echo "generate: wrote_or_updated=$wrote total_commands=${#COMMANDS[@]} out=$OUT_DIR"
echo "preserved envelopes: envelope-success, envelope-error, run-script-step (not overwritten)"
