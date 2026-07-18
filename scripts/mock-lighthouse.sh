#!/usr/bin/env bash
# Minimal mock lighthouse binary for browser-automation-cli tests (GAP-006/014).
# Echoes argv so e2e can assert --gather-mode=snapshot vs navigation.
set -euo pipefail
url="${1:-}"
out_path=""
mode_arg=""
for arg in "$@"; do
  case "$arg" in
    --output-path=*) out_path="${arg#--output-path=}" ;;
    --gather-mode=*) mode_arg="${arg#--gather-mode=}" ;;
  esac
done
if [[ -z "$out_path" ]]; then
  out_path="./report"
fi
html="${out_path}.html"
json="${out_path}.json"
base_dir="$(dirname "$out_path")"
mkdir -p "$base_dir"
printf '%s\n' "$@" > "${base_dir}/mock-lighthouse.argv"
mode_json="${mode_arg:-navigation}"
cat >"$html" <<HTML
<!doctype html><title>mock lighthouse</title><p>mode=${mode_json}</p>
HTML
cat >"$json" <<JSON
{
  "categories": {
    "accessibility": { "id": "accessibility", "title": "Accessibility", "score": 1 },
    "seo": { "id": "seo", "title": "SEO", "score": 0.9 },
    "best-practices": { "id": "best-practices", "title": "Best Practices", "score": 0.95 }
  },
  "audits": {
    "a": { "score": 1 },
    "b": { "score": 0.5 }
  },
  "configSettings": { "gatherMode": "${mode_json}" }
}
JSON
if [[ "$out_path" == *report ]]; then
  cp "$html" "${base_dir}/report.html" 2>/dev/null || true
  cp "$json" "${base_dir}/report.json" 2>/dev/null || true
fi
exit 0
