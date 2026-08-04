#!/usr/bin/env bash
# Shared helper: locate the inline `#[cfg(test)]` module inside a Rust file.
#
# WHY THIS EXISTS
#   Several verifiers assert properties about PRODUCTION code and were failing
#   on test code instead. The failure mode is perverse: the more faithfully a
#   test reproduces the bad input the product must reject, the more certainly it
#   trips the gate. Measured cases in this repo:
#
#     - `network-check.sh` flagged `--remote-debugging-address=0.0.0.0` in
#       `pin_debugging_port_forces_loopback_bind`, the test whose entire purpose
#       is to prove that wildcard bind is rewritten to loopback.
#     - `json-ndjson-check.sh` flagged `serde_json::from_str` in a lighthouse
#       test parsing a checked-in fixture, where neither BOM stripping nor a
#       size ceiling can apply.
#     - `filesize-check.sh` charged ~50 lines of table-driven tests against a
#       300-line production budget.
#
# WHY NOT "FROM #[cfg(test)] TO EOF"
#   That shortcut is wrong twice over, and `verifier-controls-check.sh` caught
#   both by mutation:
#
#     1. `mod tests;` (no body) declares the tests in ANOTHER file. Nothing in
#        this file is test code, so discarding the tail hides real production
#        code — which is exactly how a control that appends a raw parser to
#        `src/native/cdp/discovery/mod.rs` went undetected.
#     2. Rust allows items AFTER the test module. Anything below it is
#        production and must keep counting.
#
#   So the span is the module BLOCK, delimited by its own braces, and only when
#   the block actually exists.
#
# CONTRACT
#   `inline_test_span <file>` prints "<open> <close>" (1-indexed, inclusive).
#   Prints "0 0" when the file has no inline test module.
# shellcheck shell=bash

inline_test_span() {
  local file="$1"
  local open close
  # Column-0 `mod tests {` only: a nested or indented match is not the file's
  # test module and its closing brace would not be at column 0 either.
  open="$(rg -n '^(pub )?mod tests \{' "$file" 2>/dev/null | head -1 | choose -f ':' 0 || true)"
  if [[ -z "$open" ]]; then
    echo "0 0"
    return 0
  fi
  # The attribute sits on the line above; include it so a gate matching on
  # `#[cfg(test)]` itself does not see a stray production line.
  local attr_line=$((open - 1))
  if [[ "$attr_line" -ge 1 ]] &&
    rg -q '^\s*#\[cfg\(test\)\]' <(bat -pP -r "${attr_line}:${attr_line}" "$file" 2>/dev/null) 2>/dev/null; then
    open="$attr_line"
  fi
  # First column-0 `}` at or after the module opener closes it. rustfmt keeps
  # the closing brace of a top-level item at column 0, and every gate in this
  # repo runs after `cargo fmt --check`.
  close="$(rg -n '^\}' "$file" 2>/dev/null | choose -f ':' 0 | while read -r n; do
    [[ "$n" -gt "$open" ]] && {
      echo "$n"
      break
    }
  done)"
  # An unterminated block means the file does not parse; let the gate see
  # everything rather than silently exempting the tail.
  [[ -z "$close" ]] && close="$open"
  echo "$open $close"
}
