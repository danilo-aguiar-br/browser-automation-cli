#!/usr/bin/env bash
# D-03: generate shell completions into target/completions/ (packaging helper).
# NOT a ci-check verifier: a packaging helper. It produces a release artefact
# and asserts nothing, so it has no pass/fail result for a bundle to record.
# Completions remain available at runtime via `browser-automation-cli completions <shell>`.
# This script freezes artefacts for distro packaging without build.rs network I/O.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
OUT="${1:-target/completions}"
# The single argument is an output DIRECTORY, not a flag. Without this guard a
# flag-shaped argument becomes a directory name and the failure surfaces as
# `mkdir: illegal option -- -` from BSD mkdir, which names neither this script
# nor the mistake. Measured 2026-09-04, invoking it with `--check` on the
# assumption that it was a verifier: it is not, and says so at the top.
case "$OUT" in
  -*)
    echo "usage: ${BASH_SOURCE[0]##*/} [OUTPUT_DIR]" >&2
    echo "  This is a packaging helper and takes no flags; it asserts nothing" >&2
    echo "  and has no --check mode. Got: $OUT" >&2
    exit 2
    ;;
esac
mkdir -p "$OUT"
BIN=(cargo run --quiet --)
for sh in bash zsh fish elvish powershell; do
  echo "generating $sh → $OUT/browser-automation-cli.$sh"
  "${BIN[@]}" completions "$sh" >"$OUT/browser-automation-cli.$sh"
done
echo "ok: completions in $OUT"
