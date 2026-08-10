#!/usr/bin/env bash
# Shim: run one property of `tests/phantom_flag_gate.rs` and surface its message.
#
# WHY THIS FILE EXISTS
#   The phantom-flag scan lives in a Rust integration test, where it belongs: the
#   product is Rust end to end, and `ci-check` already runs it under `cargo test
#   --tests`. But `scripts/verifier-controls-check.sh` drives every control with
#   `bash $script`, so a control needs something bash can invoke. This is that
#   adapter and nothing more — it holds no assertion of its own.
#
# WHY NOT NAMED `*-check.sh`
#   `ci-check.sh` auto-discovers every executable `scripts/*-check.sh`. A gate
#   named that way here would run the same three properties a second time, once
#   through the test suite and once through this shim, and a second green adds no
#   information while costing a full test-binary link.
#
# USAGE
#   scripts/phantom-flag-gate.sh [test-name-filter]
#
# The filter is passed straight to libtest, so the caller picks one property.
# Output is forwarded verbatim: the control greps the assertion message, which is
# written by the test and not by this file.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

filter="${1:-}"

# `--nocapture` matters: the assertion text the control looks for is inside the
# panic message, and without it libtest buffers the output of a failing test in a
# form the grep would still see — but nocapture makes the ordering deterministic
# when several properties run.
if [[ -n "$filter" ]]; then
  cargo test --quiet --test phantom_flag_gate "$filter" -- --nocapture 2>&1
else
  cargo test --quiet --test phantom_flag_gate -- --nocapture 2>&1
fi
