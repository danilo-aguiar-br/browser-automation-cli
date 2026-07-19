#!/usr/bin/env bash
# Local residual hygiene gate for browser-automation-cli (PRD §5N).
# No GitHub Actions / CD — run by humans or agents on the workstation.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

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

echo "== residual-check: bin=$BIN =="

count_markers() {
  # shellcheck disable=SC2012
  (shopt -s nullglob; set -- /tmp/browser-automation-cli-chrome-*; echo "$#")
}

count_chromium_tmp() {
  # shellcheck disable=SC2012
  (shopt -s nullglob; set -- /tmp/org.chromium.Chromium.* /tmp/.org.chromium.Chromium.*; echo "$#")
}

before_m="$(count_markers)"
before_c="$(count_chromium_tmp)"
echo "before markers=$before_m chromium_tmp=$before_c"

# Path-light: BORN GC only
"$BIN" --json doctor --quick --offline >/tmp/browser-automation-cli-doctor-residual.json

after_born_m="$(count_markers)"
after_born_c="$(count_chromium_tmp)"
echo "after BORN doctor markers=$after_born_m chromium_tmp=$after_born_c"

# One-shot browser work
PDF="/tmp/browser-automation-cli-residual-check.pdf"
"$BIN" --json print-pdf --url about:blank --path "$PDF" >/tmp/browser-automation-cli-print-pdf-residual.json

after_m="$(count_markers)"
after_c="$(count_chromium_tmp)"
echo "after print-pdf markers=$after_m chromium_tmp=$after_c"

if [[ "$after_m" != "0" ]]; then
  echo "FAIL: CLI chrome markers remain: $after_m" >&2
  exit 1
fi

# Live CLI automation processes must be zero
if pgrep -af 'browser-automation-cli-chrome' 2>/dev/null | grep -v pgrep | grep -q .; then
  echo "FAIL: live browser-automation-cli-chrome process" >&2
  pgrep -af 'browser-automation-cli-chrome' || true
  exit 1
fi

if ! grep -q residual /tmp/browser-automation-cli-doctor-residual.json; then
  echo "FAIL: doctor JSON missing residual" >&2
  exit 1
fi

echo "PASS residual-check"
exit 0
