#!/usr/bin/env bash
# Local hygiene gate for GAP-051 (file size). No GitHub Actions.
#
# WHY THIS EXISTS
#   GAP-051 brought every production file under 300 lines in Wave 0, and Wave 3
#   silently undid it: 34 files crossed the limit again and nobody noticed,
#   because the criterion never had a verifier. A rule without a gate is a
#   suggestion.
#
# WHY THE UNIT IS CODE LINES AND NOT PHYSICAL LINES
#   GAP-051 caps file length; GAP-046 mandates doc comments on every public
#   item. Measured in physical lines those two pull in opposite directions:
#   satisfying 046 breaks 051, and the cheapest way to pass this gate becomes
#   DELETING documentation. That is a perverse incentive, and the plan text is
#   explicit that the limit is on lines of CODE. Prose is not code.
#   Measured example: src/cli/commands.rs is 722 physical lines and 463 code
#   lines; the 259-line difference is entirely rustdoc.
#
# TWO REAL TRAPS IN `tokei`, both of which produce a false green
#   1. There is NO `stats.lines` key. The per-language keys are `blanks`,
#      `blobs`, `code`, `comments`. A check written as
#          tokei ... | jaq '.. | select(.stats.lines > 300)'
#      compares `null > 300`, false for EVERY file, so the gate reports zero
#      offenders and exits 0. That is an AFFIRMATIVE false green: it does not
#      merely miss violations, it asserts there are none.
#   2. Rust doc comments are NOT counted in the parent `comments`. tokei parses
#      them as an embedded Markdown blob under `stats.blobs.Markdown`. So
#      `code + comments + blanks` silently omits them and does not reconstruct
#      the file. Only `.stats.code` is used below, so this does not affect the
#      criterion — it is recorded because summing those three looks correct.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

LIMIT="${FILESIZE_LIMIT:-300}"
TARGET_DIR="${1:-src}"

# ── Declared exceptions ────────────────────────────────────────────────
# Every entry needs a justification here. An undocumented entry is how an
# exception list becomes a place to hide debt.
#
#   src/cli/commands.rs
#     Single clap `Commands` enum. It is a public type, so flattening it into
#     modules is a semver break, and splitting the enum would fan out into 61
#     match arms across the dispatcher. Indivisible by construction, not by
#     convenience.
#   src/commands/dispatch/mod.rs
#     A single EXHAUSTIVE `match` over every `Commands` variant, and nothing
#     else: the context type and exit mapping already moved to `ctx.rs`.
#     Splitting the match means each family returns the command back when it is
#     not its own, which DELETES the compiler guarantee that a new variant fails
#     to build until it is routed. That guarantee has caught unrouted commands;
#     the line count has not. Indivisible without trading a real check for a
#     cosmetic one.
EXCEPTIONS=(
  "src/cli/commands.rs"
  "src/commands/dispatch/mod.rs"
)

is_exception() {
  local candidate="$1"
  local allowed
  for allowed in "${EXCEPTIONS[@]}"; do
    [[ "$candidate" == "$allowed" ]] && return 0
  done
  return 1
}

# Test files carry fixtures and table-driven cases; length there is not the
# readability problem the limit targets.
is_test_file() {
  local candidate="$1"
  [[ "$candidate" =~ (^|/)tests?\.rs$ ]] && return 0
  [[ "$candidate" =~ (^|/)test_utils\.rs$ ]] && return 0
  [[ "$candidate" =~ (^|/)tests/ ]] && return 0
  return 1
}

# INLINE `#[cfg(test)] mod tests` IS NOT PRODUCTION CODE
#   `is_test_file` exempts test PATHS, but says nothing about a test module
#   living inside a production file. Measured case: src/scrape_local/project.rs
#   reported 352 code lines, of which ~50 are an inline `mod tests` starting at
#   line 360. The gate was demanding that production code be split to make room
#   for table-driven tests — the exact perverse incentive the header warns about
#   for doc comments, one layer down.
#
#   Same unit as the rest of the gate: the tail is measured with tokei, not with
#   a physical line count, so blanks and comments inside the test module are
#   treated identically to everywhere else.
inline_test_code_lines() {
  local file="$1"
  local mod_line start
  # Anchor on the module declaration, not on `#[cfg(test)]`: a cfg-gated `fn`
  # or `use` is production surface and must keep counting. The attribute sits on
  # the line directly above, so the tail starts one line earlier.
  mod_line="$(rg -n '^\s*(pub )?mod tests \{' "$file" 2>/dev/null | head -1 | choose -f ':' 0 || true)"
  [[ -z "$mod_line" ]] && { echo 0; return 0; }
  start=$((mod_line - 1))
  # Guard: only discount when the module really is cfg-gated.
  rg -q '^\s*#\[cfg\(test\)\]' <(bat -pP -r "${start}:${start}" "$file" 2>/dev/null) 2>/dev/null || {
    echo 0
    return 0
  }

  local tmp
  tmp="$(mktemp -t filesize-check-XXXXXX.rs)"
  bat -pP -r "${start}:" "$file" >"$tmp" 2>/dev/null || {
    rm -f "$tmp"
    echo 0
    return 0
  }
  local tail_code
  tail_code="$(tokei "$tmp" -o json 2>/dev/null | jaq -r '.Rust.reports[0].stats.code // 0' 2>/dev/null || echo 0)"
  rm -f "$tmp"
  echo "${tail_code:-0}"
}

echo "== filesize-check (limit ${LIMIT} code lines, target ${TARGET_DIR}) =="

# One tokei pass for the whole tree. Per-file invocation would be correct but
# pays process startup once per file; the map is built once and read below.
declare -A CODE_LINES
while IFS=$'\t' read -r path count; do
  CODE_LINES["$path"]="$count"
done < <(tokei "$TARGET_DIR" -o json | jaq -r '.Rust.reports[] | "\(.name)\t\(.stats.code)"' | sd '^\./' '')

offenders=0
checked=0
exempted=0

while IFS= read -r file; do
  is_test_file "$file" && continue
  checked=$((checked + 1))
  lines="${CODE_LINES[$file]:-}"
  if [[ -z "$lines" ]]; then
    # A Rust file fd found that tokei did not report is a measurement hole, not
    # a passing file. Fail loudly rather than skip silently.
    printf 'FAIL      ?  %s  (no tokei report — cannot measure)\n' "$file"
    offenders=$((offenders + 1))
    continue
  fi
  [[ "$lines" -le "$LIMIT" ]] && continue
  # Only pay the per-file tokei pass for files that would otherwise FAIL.
  inline_tests="$(inline_test_code_lines "$file")"
  if [[ "$inline_tests" -gt 0 ]]; then
    prod_lines=$((lines - inline_tests))
    if [[ "$prod_lines" -le "$LIMIT" ]]; then
      printf 'INFO  %5d  %s  (%d production + %d inline tests)\n' \
        "$lines" "$file" "$prod_lines" "$inline_tests"
      continue
    fi
    lines="$prod_lines"
  fi
  if is_exception "$file"; then
    printf 'INFO  %5d  %s  (declared exception)\n' "$lines" "$file"
    exempted=$((exempted + 1))
    continue
  fi
  printf 'FAIL  %5d  %s\n' "$lines" "$file"
  offenders=$((offenders + 1))
done < <(fd -e rs . "$TARGET_DIR" | sort)

echo "----"
printf 'checked=%d  exceptions=%d  over_limit=%d\n' "$checked" "$exempted" "$offenders"

if [[ "$offenders" -ne 0 ]]; then
  echo "== filesize-check FAILED: ${offenders} file(s) over ${LIMIT} lines =="
  echo "Split the file, or add it to EXCEPTIONS in this script WITH a justification."
  exit 1
fi
echo "== filesize-check PASS =="
exit 0
