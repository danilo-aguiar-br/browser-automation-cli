#!/usr/bin/env bash
# Local residual stress (PRD §5N stress of N one-shot invocations).
# Default N=20 for fast local gate; set N=100 for full PRD proof.
# PROIBIDO wire this into GitHub Actions.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
N="${N:-20}"

BIN="${BROWSER_AUTOMATION_CLI_BIN:-}"
if [[ -z "$BIN" ]]; then
  if [[ -x "$ROOT/target/release/browser-automation-cli" ]]; then
    BIN="$ROOT/target/release/browser-automation-cli"
  elif [[ -x "$ROOT/target/debug/browser-automation-cli" ]]; then
    BIN="$ROOT/target/debug/browser-automation-cli"
  elif command -v browser-automation-cli >/dev/null 2>&1; then
    BIN="$(command -v browser-automation-cli)"
  else
    echo "FAIL: browser-automation-cli binary not found" >&2
    exit 1
  fi
fi

count_markers() {
  (shopt -s nullglob; set -- /tmp/browser-automation-cli-chrome-*; echo "$#")
}

count_chromium_tmp() {
  (shopt -s nullglob; set -- /tmp/org.chromium.Chromium.* /tmp/.org.chromium.Chromium.*; echo "$#")
}

echo "== residual-stress N=$N bin=$BIN =="
start_c="$(count_chromium_tmp)"
start_m="$(count_markers)"
echo "start markers=$start_m chromium_tmp=$start_c"

for i in $(seq 1 "$N"); do
  PDF="/tmp/browser-automation-cli-stress-$i.pdf"
  "$BIN" --json print-pdf --url about:blank --path "$PDF" >/dev/null
  m="$(count_markers)"
  if [[ "$m" != "0" ]]; then
    echo "FAIL: markers non-zero after iteration $i: $m" >&2
    exit 1
  fi
done

end_c="$(count_chromium_tmp)"
end_m="$(count_markers)"
echo "end markers=$end_m chromium_tmp=$end_c"

if [[ "$end_m" != "0" ]]; then
  echo "FAIL: markers remain" >&2
  exit 1
fi

# Chromium tmp may fluctuate during run; end must not explode vs start.
# Allow small absolute growth of 2 for races with other host tools.
if (( end_c > start_c + 2 )); then
  echo "FAIL: chromium tmp grew excessively: start=$start_c end=$end_c" >&2
  exit 1
fi

echo "PASS residual-stress N=$N"
exit 0
