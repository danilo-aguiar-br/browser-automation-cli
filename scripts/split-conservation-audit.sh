#!/usr/bin/env bash
# NOT a ci-check verifier, on purpose. Do NOT add `# ci-check: verifier` here.
#   This audit takes PAIRS of arguments. Invoked with none, as a gate would, it
#   prints `total ausente: 0` and exits 0 having audited NOTHING. Measured
#   2026-08-26: it was marked, and the gate scored that empty run as PASS. It is
#   a tool you point at a split you just made, not a standing property.
# Conservation audit: every significant line of a pre-split file must still
# exist somewhere under the directory that replaced it.
#
# Usage:
#   bash scripts/split-conservation-audit.sh <original.rs> <new_dir> [<original.rs> <new_dir> ...]
# Exit:
#   0 nothing lost
#   1 at least one significant line vanished
#
# WHY COMPILING IS NOT ENOUGH
#   Splitting `commands/ops/lighthouse.rs` silently dropped the doc comment of
#   `enum LighthouseSource`. Every gate stayed green: the crate compiled, 487
#   tests passed, `clippy -D warnings` passed, and `cargo doc -D warnings`
#   passed.
#
#   `missing_docs` cannot catch that class: it fires on a PUBLIC item that has
#   no doc, and the enum is `pub(crate)`. A gate that measures the PRESENCE of
#   documentation is blind to documentation that was DELETED from an item it
#   does not cover. The same reasoning applies to any content whose absence is
#   still valid Rust — a dropped match arm with a catch-all sibling, a dropped
#   comment, a dropped test helper that nothing else calls.
#
#   So the invariant here is conservation, not compilation: run this BEFORE
#   removing the original, and treat a non-zero exit as a lost hunk, not a nit.
#
# TRIVIAL LINES
#   Excluded because a split legitimately rewrites them: imports, lone
#   delimiters, blank lines and module attributes. Keep this list tight — every
#   prefix added here is a place a real loss could hide.
#
# WHY BASH AND NOT AN INTERPRETER
#   Ported from Python on 2026-08-18. The product is Rust end to end and ships
#   no interpreter, so a repository tool that needs one is a tool that some
#   hosts simply do not have. Same rules, same report, same exit codes.
set -euo pipefail

# A line that a split may legitimately rewrite carries no evidence of loss.
is_significant() {
  local s="$1"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  [[ -n "$s" ]] || return 1
  case "$s" in
    '}' | '{' | '};' | ')' | '),' | '()' | '],' | ']') return 1 ;;
    'use '* | '#!['* | 'mod '* | 'pub use '* | 'pub(crate) use '* | 'pub(super) use '*) return 1 ;;
  esac
  return 0
}

audit() {
  local original="$1" new_root="$2"
  local haystack="" f line trimmed
  local -a old_lines=() missing=()

  while IFS= read -r f; do
    haystack+="$(<"$f")"$'\n'
  done < <(fd -e rs . "$new_root" --type f | LC_ALL=C sort)

  while IFS= read -r line || [[ -n "$line" ]]; do
    if is_significant "$line"; then
      trimmed="${line#"${line%%[![:space:]]*}"}"
      trimmed="${trimmed%"${trimmed##*[![:space:]]}"}"
      old_lines+=("$trimmed")
      [[ "$haystack" == *"$trimmed"* ]] || missing+=("$trimmed")
    fi
  done <"$original"

  local name="${original##*/}"
  if [[ "${#missing[@]}" -gt 0 ]]; then
    echo "FAIL  ${name}: ${#missing[@]}/${#old_lines[@]} linhas ausentes"
    local i
    for ((i = 0; i < ${#missing[@]} && i < 5; i++)); do
      echo "        ${missing[$i]:0:78}"
    done
  else
    echo "ok    ${name}: ${#old_lines[@]} linhas significativas preservadas"
  fi
  # Reported through a variable, never through the exit status: a count above
  # 255 would wrap and a large loss would read as a small one.
  LOST="${#missing[@]}"
}

total=0
audited=0
while [[ $# -gt 1 ]]; do
  orig="$1"
  root_dir="$2"
  shift 2
  if [[ ! -e "$orig" || ! -e "$root_dir" ]]; then
    echo "skip  ${orig##*/}: sem referencia"
    continue
  fi
  LOST=0
  audit "$orig" "$root_dir"
  total=$((total + LOST))
  audited=$((audited + 1))
done

echo
# ZERO PAIRS IS NOT ZERO LOSSES (measured 2026-08-26)
#   Invoked with no arguments this printed `total ausente: 0` and exited 0,
#   having compared nothing. A runner that scores exit codes read that as a
#   PASS, and a green that measured nothing is worse than no green: it occupies
#   the slot where a real verdict would have been noticed missing.
#
#   Exit 3 is the project's "declined to run" code, which `ci-check.sh` records
#   as SKIP and counts as a FAILURE precisely so an unrun check cannot pass.
if [[ "$audited" -eq 0 ]]; then
  echo "split-conservation-audit: no pair given, so NOTHING was audited" >&2
  echo "usage: bash scripts/split-conservation-audit.sh <original.rs> <new_dir> [...]" >&2
  exit 3
fi
echo "total ausente: ${total} (pares auditados: ${audited})"
[[ "$total" -eq 0 ]] || exit 1
exit 0
