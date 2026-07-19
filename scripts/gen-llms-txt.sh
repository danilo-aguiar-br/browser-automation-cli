#!/usr/bin/env bash
# D-10: regenerate inventory section of llms.txt from live `commands --json`.
# Does not replace full prose; appends a machine section agents can trust.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
OUT="${1:-llms.txt}"
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

cargo run --quiet -- commands --json >"$TMP"

if [[ -f "$OUT" ]]; then
  # Drop previous generated block if present.
  if grep -q '^<!-- GENERATED_COMMANDS_JSON' "$OUT"; then
    awk 'BEGIN{p=1} /^<!-- GENERATED_COMMANDS_JSON/{p=0} p' "$OUT" >"${TMP}.keep"
    mv "${TMP}.keep" "$OUT"
  fi
else
  printf '# browser-automation-cli\n\n' >"$OUT"
fi

{
  echo
  echo '<!-- GENERATED_COMMANDS_JSON: do not edit by hand; run scripts/gen-llms-txt.sh -->'
  echo '```json'
  cat "$TMP"
  echo '```'
  echo '<!-- END_GENERATED_COMMANDS_JSON -->'
} >>"$OUT"

echo "ok: appended commands inventory to $OUT"
