#!/usr/bin/env bash
# Static schema drift gate.
#
# WHY THIS FILE EXISTS AS A SEPARATE SCRIPT
#   `scripts/generate_command_schemas.sh --check` already detects drift and has
#   for a long time. It simply never ran: `scripts/ci-check.sh` auto-discovers
#   verifiers with the glob `scripts/*-check.sh`, and `generate_command_schemas.sh`
#   does not match it. The capability existed; the wiring did not. Seven of 68
#   schemas drifted with every audit reporting green.
#
#   This is a thin adapter, not a second implementation. There is exactly one
#   drift algorithm, and it lives in the generator.
#
# WHY THE RUNTIME IS THE SOURCE OF TRUTH
#   `docs/schemas/*.json` is a DERIVED artifact, not a document. Agents consume
#   it as a contract, so when the file and the binary disagree the binary wins
#   and the file is regenerated — never the reverse. `docs/schemas/audio.schema.json`
#   had been hand-copied from `src/commands/meta/schema/scrape_tools.rs`, which
#   is how a derived artifact silently became a second source of truth.
#
# CLEAN STDOUT: one status line on stdout; diagnostics on stderr.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ ! -x "$ROOT/scripts/generate_command_schemas.sh" ]] &&
  [[ ! -f "$ROOT/scripts/generate_command_schemas.sh" ]]; then
  echo "schema-drift-check: FAIL (generator missing)" >&2
  echo "schema-drift-check: FAIL"
  exit 1
fi

# The generator separates its two failure kinds and this gate used to collapse
# them: exit 1 means the schemas DRIFTED, exit 2 means a PRECONDITION failed
# (no binary to interrogate, bad arguments). Both printed "regenerate with ...",
# which sends the operator to fix the wrong problem — and regenerating without a
# binary cannot work at all, so the advice was not merely useless.
#
# Measured 2026-09-01: a release build interrupted by cargo lock contention left
# no release binary, the generator answered exit 2 with "binary not found", and
# this gate still advised regeneration on the very next line.
set +e
bash "$ROOT/scripts/generate_command_schemas.sh" --check >&2
generator_rc=$?
set -e

if [[ "$generator_rc" -eq 0 ]]; then
  echo "schema-drift-check: OK (docs/schemas matches the live binary)"
  exit 0
fi

if [[ "$generator_rc" -eq 2 ]]; then
  echo "schema-drift-check: FAIL (precondition, NOT drift: read the generator error above)" >&2
  echo "schema-drift-check: FAIL"
  exit 2
fi

echo "schema-drift-check: FAIL (regenerate with: bash scripts/generate_command_schemas.sh)" >&2
echo "schema-drift-check: FAIL"
exit 1
