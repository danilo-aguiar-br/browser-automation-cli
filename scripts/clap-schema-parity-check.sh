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
#   the published schema for `storage` listed only `action`. An agent reading
#   the contract had no way to learn about a mandatory argument.
#
# THIS SCRIPT IS A THIN ADAPTER. The comparison lives in
# `tests/clap_schema_parity.rs`, which `cargo test --tests` already runs inside
# `ci-check.sh`. It moved there on 2026-08-18 because it was a Python script,
# and the product ships no interpreter: the gate failed on any host without
# the interpreter, under `set -euo pipefail`, with no guard. Coverage is unchanged --
# same clap walk, same recursive schema walk, same two failure classes -- and
# the port resolves the binary through `CARGO_BIN_EXE_` instead of guessing
# between debug and release, which also removes the staleness guard this script
# used to need.
#
# CLEAN STDOUT: one status line on stdout; diagnostics on stderr.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v cargo >/dev/null 2>&1; then
  echo "clap-schema-parity-check: FAIL (cargo missing)" >&2
  echo "clap-schema-parity-check: FAIL"
  exit 1
fi

if cargo test --test clap_schema_parity -- --nocapture >&2; then
  echo "clap-schema-parity-check: OK (every clap flag appears in schema)"
  exit 0
fi

echo "clap-schema-parity-check: FAIL"
exit 1
