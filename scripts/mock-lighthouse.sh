#!/usr/bin/env bash
# Minimal mock lighthouse binary for CI (browser-automation-cli tests).
# Usage: --lighthouse-path scripts/mock-lighthouse.sh
set -euo pipefail
url="${1:-}"
out_path=""
for arg in "$@"; do
  case "$arg" in
    --output-path=*) out_path="${arg#--output-path=}" ;;
  esac
done
if [[ -z "$out_path" ]]; then
  out_path="./report"
fi
# Lighthouse CLI writes base path + extension for multi-output
html="${out_path}.html"
json="${out_path}.json"
# Also support when base is .../report without extension and flags request report.html
base_dir="$(dirname "$out_path")"
mkdir -p "$base_dir"
cat >"$html" <<'HTML'
<!doctype html><title>mock lighthouse</title><p>ok</p>
HTML
cat >"$json" <<'JSON'
{
  "categories": {
    "accessibility": { "id": "accessibility", "title": "Accessibility", "score": 1 },
    "seo": { "id": "seo", "title": "SEO", "score": 0.9 },
    "best-practices": { "id": "best-practices", "title": "Best Practices", "score": 0.95 }
  },
  "audits": {
    "a": { "score": 1 },
    "b": { "score": 0.5 }
  }
}
JSON
# Also write report.html / report.json aliases when path ends with report
if [[ "$out_path" == *report ]]; then
  cp "$html" "${base_dir}/report.html" 2>/dev/null || true
  cp "$json" "${base_dir}/report.json" 2>/dev/null || true
fi
exit 0
