#!/usr/bin/env bash
# Local gate: en/pt-BR FTL parity + i18n unit surface (no GitHub Actions).
set -euo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting (`--smart-case` widens matches, `--max-columns`
# truncates them away). Clearing the variable neutralizes the whole file; `-s`
# would close only one of those doors.
export RIPGREP_CONFIG_PATH=
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

# `mapfile` is a bash 4 builtin and macOS ships bash 3.2, so every array read
# in this file uses the portable read loop below instead (2026-09-04).
EN_KEYS=()
while IFS= read -r __line; do EN_KEYS+=("$__line"); done < <(extract_keys "$EN")
PT_KEYS=()
while IFS= read -r __line; do PT_KEYS+=("$__line"); done < <(extract_keys "$PT")

if [[ "${#EN_KEYS[@]}" -eq 0 ]]; then
  echo "FAIL: en.ftl has no keys" >&2
  exit 1
fi

# PT_KEYS can legitimately be empty and bash 3.2 aborts on "${arr[@]}" of an
# empty array under `set -u`, so it is expanded defensively.
DIFF="$(comm -3 <(printf '%s\n' "${EN_KEYS[@]}") <(printf '%s\n' "${PT_KEYS[@]+"${PT_KEYS[@]}"}") || true)"
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

# Pass K: headers point at UiMessage (not legacy mensagem.rs)
if grep -q 'mensagem\.rs' "$EN" "$PT" 2>/dev/null; then
  echo "FAIL: FTL still references mensagem.rs (use ui_message.rs)" >&2
  exit 1
fi
if ! grep -q 'ui_message.rs' "$EN" || ! grep -q 'ui_message.rs' "$PT"; then
  echo "FAIL: FTL headers must reference ui_message.rs" >&2
  exit 1
fi
echo "PASS  FTL headers reference ui_message.rs"

# GAP-047: every emitted suggestion key must exist in BOTH catalogs.
# Source of truth: suggestion_key("<k>") call sites and UiMessage::from_suggestion_key arms.
MISSING_KEYS=0
while IFS= read -r sk; do
  [[ -z "$sk" ]] && continue
  # Catalog id is the kebab-case form of the snake_case suggestion key.
  ftl_id="${sk//_/-}"
  if ! rg -q "^${ftl_id}[[:space:]]*=" "$EN"; then
    echo "FAIL: suggestion_key(\"$sk\") has no '$ftl_id' key in locales/en.ftl" >&2
    MISSING_KEYS=1
  fi
  if ! rg -q "^${ftl_id}[[:space:]]*=" "$PT"; then
    echo "FAIL: suggestion_key(\"$sk\") has no '$ftl_id' key in locales/pt-BR.ftl" >&2
    MISSING_KEYS=1
  fi
  # Anchored on the symbol across src/i18n, not on one file: the catalog
  # was a single ui_message.rs and is now a module, and a path-pinned
  # check would fail for the wrong reason after any such move.
  if ! rg -q "\"$sk\" => UiMessage::" src/i18n/; then
    echo "FAIL: suggestion_key(\"$sk\") has no UiMessage variant in src/i18n/" >&2
    MISSING_KEYS=1
  fi
done < <(rg -o -r '$1' 'suggestion_key\("([a-z0-9_]+)"' src/ --no-filename | sort -u)
if [[ "$MISSING_KEYS" -ne 0 ]]; then
  echo "FAIL: emitted suggestions without catalog coverage" >&2
  exit 1
fi
echo "PASS  every emitted suggestion_key has en/pt-BR catalog coverage"

# GAP-047: every ftl_id() declared in UiMessage must exist in BOTH catalogs.
MISSING_IDS=0
while IFS= read -r id; do
  [[ -z "$id" ]] && continue
  if ! rg -q "^${id}[[:space:]]*=" "$EN" || ! rg -q "^${id}[[:space:]]*=" "$PT"; then
    echo "FAIL: UiMessage ftl_id '$id' missing from a catalog" >&2
    MISSING_IDS=1
  fi
done < <(rg -o -r '$1' 'UiMessage::\w+ => "([a-z0-9-]+)",' src/i18n/ --no-filename | sort -u)
if [[ "$MISSING_IDS" -ne 0 ]]; then
  exit 1
fi
echo "PASS  every UiMessage ftl_id has en/pt-BR catalog coverage"

# Pass K: config set validates lang; bare pt rejected
if ! rg -n 'validate_lang_token' src/xdg/config_ops/set.rs src/i18n/mod.rs >/dev/null; then
  echo "FAIL: missing validate_lang_token wiring" >&2
  exit 1
fi
echo "PASS  validate_lang_token present"

if ! rg -n 'parse_token\("pt"\), None' src/i18n/ui_locale.rs >/dev/null; then
  echo "FAIL: bare pt must be rejected in parse_token tests" >&2
  exit 1
fi
echo "PASS  bare pt rejection covered"

# Pass K: feature gates route through the catalog, never EN literals.
# Anchored on the whole tree, not one file: the gate logic moved from
# dispatch/gates.rs to capability/ and a path-pinned check went stale silently.
if rg -n 'Pass --experimental-screencast|Pass --category-memory|Pass --category-extensions|Pass --category-third-party|Pass --category-webmcp' \
  src/ --glob '!src/i18n/**' --glob '!src/**/tests.rs' >/dev/null; then
  echo "FAIL: a feature gate still carries hard-coded EN catalog text outside src/i18n" >&2
  rg -n 'Pass --experimental-screencast|Pass --category-memory|Pass --category-extensions|Pass --category-third-party|Pass --category-webmcp' \
    src/ --glob '!src/i18n/**' --glob '!src/**/tests.rs' >&2
  exit 1
fi
for gate_key in screencast_flag category_memory category_extensions third_party_flag webmcp_flag; do
  if ! rg -q "\"${gate_key}\"" src/ --glob '!src/i18n/**'; then
    echo "FAIL: feature gate key '${gate_key}' is not referenced outside src/i18n" >&2
    exit 1
  fi
done
echo "PASS  feature gates route through the suggestion catalog"

# Pass K: early locale on clap error path
if ! rg -n 'scan_lang_flag_from_argv' src/lib.rs src/i18n/mod.rs >/dev/null; then
  echo "FAIL: missing early lang scan for clap errors" >&2
  exit 1
fi
echo "PASS  early locale scan for clap errors"

# GAP-047: source audit of with_suggestion call sites.
# Catches a hand-written English literal, which the key-coverage checks above
# cannot see. Parsing lives in src/i18n/catalog_audit.rs so it is Rust-aware.
if ! cargo test --lib i18n::catalog_audit --quiet; then
  echo "FAIL: with_suggestion source audit (raw literal added, or catalog text hand-copied)" >&2
  exit 1
fi
echo "PASS  with_suggestion source audit (no new raw literals, no hand-copied catalog text)"

# Compile-time + unit surface
cargo test --lib i18n:: --quiet
cargo test --test golden_i18n --quiet

echo "i18n-check: PASS"
