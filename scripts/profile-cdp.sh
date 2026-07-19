#!/usr/bin/env bash
# D-12 / D-16 / D-21: local CDP path profiling (no CI, no committed flamegraphs).
# Usage:
#   ./scripts/profile-cdp.sh
#   ./scripts/profile-cdp.sh --samply
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CMD=(cargo run --release --quiet -- --json goto about:blank)

echo "== profile-cdp (one-shot goto about:blank) =="
echo "Command: ${CMD[*]}"
echo
echo "Options:"
echo "  1) wall time only (default)"
echo "  2) cargo flamegraph  (cargo install flamegraph)"
echo "  3) samply            (cargo install samply)"
echo

if [[ "${1:-}" == "--flamegraph" ]]; then
  if ! command -v cargo-flamegraph >/dev/null 2>&1 && ! cargo flamegraph -h >/dev/null 2>&1; then
    echo "install: cargo install flamegraph"
    exit 1
  fi
  cargo flamegraph --bin browser-automation-cli -- --json goto about:blank
  echo "wrote flamegraph.svg (local only — do not commit)"
  exit 0
fi

if [[ "${1:-}" == "--samply" ]]; then
  if ! command -v samply >/dev/null 2>&1; then
    echo "install: cargo install samply"
    exit 1
  fi
  samply record -- cargo run --release --quiet -- --json goto about:blank
  exit 0
fi

/usr/bin/time -f 'elapsed_sec=%e max_rss_kb=%M' "${CMD[@]}" || true
echo
echo "Tip: ./scripts/profile-cdp.sh --flamegraph | --samply"
