#!/usr/bin/env bash
# Measure RSS baseline for release binary (rules_rust_economia_de_recursos).
# Ground truth: "Maximum resident set size" from /usr/bin/time -v (or GNU time).
# No GitHub Actions / CD — local / operator script only.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="${BIN:-./target/release/browser-automation-cli}"
CMD_ARGS=("${@:-doctor --offline --quick --json}")

if [[ ! -x "$BIN" ]]; then
  echo "Building release binary…" >&2
  cargo build --release --locked 2>/dev/null || cargo build --release
  BIN="./target/release/browser-automation-cli"
fi

TIME_BIN=""
if [[ -x /usr/bin/time ]]; then
  TIME_BIN=/usr/bin/time
elif command -v gtime >/dev/null 2>&1; then
  TIME_BIN="$(command -v gtime)"
fi

echo "{\"event\":\"rss_baseline_start\",\"bin\":\"$BIN\",\"args\":$(printf '%s\n' "${CMD_ARGS[@]}" | jq -R . | jq -s . 2>/dev/null || echo '[]')}"

if [[ -n "$TIME_BIN" ]]; then
  # shellcheck disable=SC2086
  set +e
  OUT="$("$TIME_BIN" -v "$BIN" "${CMD_ARGS[@]}" 2>&1 >/dev/null)"
  EC=$?
  set -e
  RSS="$(printf '%s\n' "$OUT" | sed -n 's/.*Maximum resident set size[^:]*: *//p' | head -1)"
  echo "{\"event\":\"rss_baseline\",\"exit\":$EC,\"max_rss_kb\":${RSS:-null},\"unit\":\"kilobytes\"}"
  printf '%s\n' "$OUT" | grep -E 'Maximum resident|User time|System time|Percent of CPU|File system' >&2 || true
else
  # Fallback: /proc self sample (less accurate than peak RSS).
  set +e
  "$BIN" "${CMD_ARGS[@]}" >/dev/null
  EC=$?
  set -e
  echo "{\"event\":\"rss_baseline\",\"exit\":$EC,\"max_rss_kb\":null,\"note\":\"install GNU time for peak RSS\"}" >&2
fi

exit 0
