#!/usr/bin/env bash
# Generate static JSON Schema files for every CLI command from live meta.rs surface.
# NOT a ci-check verifier by glob, and that is deliberate: this is a GENERATOR.
# The bundle invokes it BY NAME as a fixed step in `--check` mode, so discovery
# would run it twice and the second run would assert nothing new.
# Source of truth: `browser-automation-cli schema --cmd <name> --json`
# Usage:
#   bash scripts/generate_command_schemas.sh           # write docs/schemas/<cmd>.schema.json
#   bash scripts/generate_command_schemas.sh --check    # exit 1 if any file would change
# Env:
#   BIN=/path/to/browser-automation-cli
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# PICK THE NEWER BINARY, NOT THE PREFERRED ONE
#   The old order was release-then-debug, which silently compares `docs/schemas`
#   against a STALE artifact: any workflow that iterates with `cargo build`
#   (debug) while an older `target/release` sits on disk gets a green check for a
#   binary that no longer reflects the source. That is the same false-green this
#   gate exists to prevent, one level down — the drift moves from the schema file
#   to the thing the schema file is compared against.
#
#   An explicit `BIN=` still wins, because a caller naming a binary means it.
if [[ -z "${BIN:-}" ]]; then
  REL="$ROOT/target/release/browser-automation-cli"
  DBG="$ROOT/target/debug/browser-automation-cli"
  if [[ -x "$REL" && -x "$DBG" ]]; then
    if [[ "$DBG" -nt "$REL" ]]; then BIN="$DBG"; else BIN="$REL"; fi
  elif [[ -x "$REL" ]]; then
    BIN="$REL"
  else
    BIN="$DBG"
  fi
fi
if [[ ! -x "$BIN" ]]; then
  echo "error: binary not found; build with cargo build --release --locked or set BIN=" >&2
  exit 2
fi

# STALENESS IS A FAILURE, NOT A WARNING
#   Even the newer of the two binaries can predate the sources. Comparing a
#   derived artifact against something older than the code it derives from
#   answers a question nobody asked, and answers it green.
newest_src="$(fd -e rs . "$ROOT/src" --exec-batch ls -t 2>/dev/null | head -1 || true)"
if [[ -n "$newest_src" && "$newest_src" -nt "$BIN" ]]; then
  echo "error: $BIN is older than $newest_src" >&2
  echo "       rebuild before checking schemas, or the comparison is meaningless" >&2
  echo "       cargo build --release --locked   # then re-run" >&2
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
# `mapfile` is a bash 4 builtin and macOS ships bash 3.2, so the read loop
# below is the portable equivalent (2026-09-04).
COMMANDS=()
while IFS= read -r __line; do COMMANDS+=("$__line"); done < <(
  "$BIN" --json commands 2>/dev/null | jaq -r '
    ((.data // .).commands) as $cmds
    | if ($cmds | type) != "array" or ($cmds | length) == 0
      then error("commands --json missing data.commands")
      else $cmds[] | (if type == "object" then .name else . end)
      end
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

  # RENDERED BY jaq, NOT BY AN INTERPRETER
  #   `jaq` preserves object key order and its two-space pretty form is
  #   byte-identical to what this step used to emit, so `docs/schemas/*.json`
  #   does not move. The swap removes a Python dependency from a generator that
  #   `schema-drift-check.sh` calls, which on a host without the interpreter
  #   turned a drift gate into an unconditional error.
  rendered="$(
    printf '%s' "$live_json" | jaq --indent 2 --arg cmd "$cmd" --arg base "$REPO_ID_BASE" '
      (.data // .) as $data
      | (if ($data.schema | type) == "object"
         then $data.schema
         else {
           type: ($data.type // "object"),
           description: ($data.description // ($cmd + " command input")),
           properties: (if ($data.properties // {}) == null then {} else ($data.properties // {}) end),
           required: ($data.required // []),
           additionalProperties: false
         }
         end) as $schema
      | {
          "$schema": "http://json-schema.org/draft-07/schema#",
          "$id": ($base + "/" + $cmd + ".schema.json"),
          "title": ($cmd + " command input"),
          "type": (if ($schema.type // "") == "" then "object" else $schema.type end),
          "description": (if ($schema.description // "") == "" then ($cmd + " command input") else $schema.description end),
          "properties": ($schema.properties // {}),
          "required": ($schema.required // []),
          "additionalProperties": (if ($schema | has("additionalProperties")) then $schema.additionalProperties else false end)
        }
    '
  )"
  if [[ -z "$rendered" ]]; then
    echo "invalid json for $cmd: schema --cmd produced no renderable document" >&2
    exit 2
  fi

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
