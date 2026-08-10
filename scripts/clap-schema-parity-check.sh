#!/usr/bin/env bash
# Clap-vs-schema parity gate. No GitHub Actions.
#
# WHY A SEPARATE GATE FROM schema-drift-check.sh
#   That gate compares `docs/schemas/*.json` against the live binary, and is
#   correct on its axis. It structurally cannot see this failure: both sides of
#   that comparison derive from the same hand-written schema module, so they
#   agree perfectly about a surface neither describes. This gate compares the
#   PARSER against the schema, which is the axis nothing was checking.
#
#   Measured 2026-08-06, 29 flags accepted by clap and absent from `schema`.
#   `storage export --path` is REQUIRED -- omitting it is a usage error -- and
#   the published schema for `storage` lists only `action`. An agent reading the
#   contract has no way to learn about a mandatory argument.
#
# THIS SCRIPT IS A THIN ADAPTER. The comparison lives in the Python next to it,
# for the same reason schema-drift-check.sh delegates to the generator: one
# algorithm, one place.
#
# CLEAN STDOUT: one status line on stdout; diagnostics on stderr.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="${BAC_BIN:-$ROOT/target/release/browser-automation-cli}"
PARITY="$ROOT/scripts/clap_schema_parity.py"

if [[ ! -f "$PARITY" ]]; then
  echo "clap-schema-parity-check: FAIL (comparator missing)" >&2
  echo "clap-schema-parity-check: FAIL"
  exit 1
fi

if [[ ! -x "$BIN" ]]; then
  echo "clap-schema-parity-check: FAIL (binary missing: $BIN)" >&2
  echo "  build first: cargo build --release --locked" >&2
  echo "clap-schema-parity-check: FAIL"
  exit 1
fi

# A stale binary answers for a parser that no longer exists, which is a false
# green rather than a missing check. Same guard the schema generator uses.
newest_src="$(fd -e rs . src --exec-batch ls -t 2>/dev/null | head -1 || true)"
if [[ -n "$newest_src" && "$newest_src" -nt "$BIN" ]]; then
  echo "clap-schema-parity-check: FAIL (binary older than $newest_src)" >&2
  echo "  rebuild first: cargo build --release --locked" >&2
  echo "clap-schema-parity-check: FAIL"
  exit 1
fi

if python3 "$PARITY" "$BIN" >&2; then
  echo "clap-schema-parity-check: OK (every clap flag appears in schema)"
  exit 0
fi

echo "clap-schema-parity-check: FAIL"
exit 1
