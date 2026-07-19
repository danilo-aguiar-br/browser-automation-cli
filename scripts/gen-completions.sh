#!/usr/bin/env bash
# D-03: generate shell completions into target/completions/ (packaging helper).
# Completions remain available at runtime via `browser-automation-cli completions <shell>`.
# This script freezes artefacts for distro packaging without build.rs network I/O.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
OUT="${1:-target/completions}"
mkdir -p "$OUT"
BIN=(cargo run --quiet --)
for sh in bash zsh fish elvish powershell; do
  echo "generating $sh → $OUT/browser-automation-cli.$sh"
  "${BIN[@]}" completions "$sh" >"$OUT/browser-automation-cli.$sh"
done
echo "ok: completions in $OUT"
