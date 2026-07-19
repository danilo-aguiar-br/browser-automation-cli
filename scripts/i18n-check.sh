#!/usr/bin/env bash
# Local gate: en/pt-BR FTL parity + i18n unit surface (no GitHub Actions).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

EN="${ROOT}/locales/en.ftl"
PT="${ROOT}/locales/pt-BR.ftl"

if [[ ! -f "$EN" || ! -f "$PT" ]]; then
  echo "FAIL: missing locales/en.ftl or locales/pt-BR.ftl" >&2
  exit 1
fi

# Extract bare message ids (key = value), ignore comments.
extract_keys() {
  grep -E '^[a-zA-Z0-9_-]+[[:space:]]*=' "$1" | sed -E 's/[[:space:]]*=.*//' | sort -u
}

mapfile -t EN_KEYS < <(extract_keys "$EN")
mapfile -t PT_KEYS < <(extract_keys "$PT")

if [[ "${#EN_KEYS[@]}" -eq 0 ]]; then
  echo "FAIL: en.ftl has no keys" >&2
  exit 1
fi

DIFF="$(comm -3 <(printf '%s\n' "${EN_KEYS[@]}") <(printf '%s\n' "${PT_KEYS[@]}") || true)"
if [[ -n "${DIFF}" ]]; then
  echo "FAIL: FTL key parity en vs pt-BR:" >&2
  echo "$DIFF" >&2
  exit 1
fi

# No empty values
while IFS= read -r line; do
  if [[ "$line" =~ ^[a-zA-Z0-9_-]+[[:space:]]*=[[:space:]]*$ ]]; then
    echo "FAIL: empty FTL value: $line" >&2
    exit 1
  fi
done < <(grep -E '^[a-zA-Z0-9_-]+[[:space:]]*=' "$EN" "$PT")

# pt-BR must keep accents on known tokens (NFC human review cue)
if ! grep -q 'invocação' "$PT"; then
  echo "FAIL: pt-BR.ftl missing accented 'invocação'" >&2
  exit 1
fi

echo "FTL parity: ${#EN_KEYS[@]} keys OK"

# Compile-time + unit surface
cargo test --lib i18n:: --quiet
cargo test --test golden_i18n --quiet

echo "i18n-check: PASS"
