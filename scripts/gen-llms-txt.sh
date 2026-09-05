#!/usr/bin/env bash
# D-10: regenerate inventory section of llms.txt from live `commands --json`.
# NOT a ci-check verifier by glob, and that is deliberate: this is a GENERATOR.
# The bundle invokes it BY NAME as a fixed step in `--check` mode, so discovery
# would run it twice and the second run would assert nothing new.
# Does not replace full prose; appends a machine section agents can trust.
#
# Usage:
#   bash scripts/gen-llms-txt.sh            # write llms.txt
#   bash scripts/gen-llms-txt.sh --check    # exit 1 if the file would change
#   bash scripts/gen-llms-txt.sh PATH       # write some other file
#
# ONE MOVE, NOT TWO (measured 2026-08-26)
#   The previous shape cut the old block, `mv`-ed the truncated file over $OUT,
#   and only THEN appended the new one. Anything stopping between those two
#   steps left $OUT permanently missing the block, carrying the exit code of the
#   half that did succeed. Measured on this date: all four `llms*.txt` carried
#   ZERO `GENERATED_COMMANDS_JSON` markers, and regenerating llms.txt took it
#   from 68 lines to 74 with the first 68 identical byte for byte. The block
#   agents are told to trust was simply absent, and no gate could see it.
#   The whole file is now assembled in a sibling temporary and moved ONCE, so
#   $OUT is either the old file or the complete new one, never a torn middle.
#
# IDEMPOTENT (measured 2026-08-26)
#   The cut kept the blank line that preceded the marker and the append added
#   another, so every run grew the file by one blank line: 74 became 75.
#   `$(...)` strips ALL trailing newlines and the `printf` puts exactly one
#   back, so running this twice is a no-op. This is a precondition for
#   `--check`, not a nicety: a generator that is not idempotent fails its own
#   output and turns the gate red on a clean tree.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CHECK=0
if [[ "${1:-}" == "--check" ]]; then
  CHECK=1
  shift
fi

# BOTH LANGUAGES, BECAUSE THE BLOCK IS NOT PROSE (measured 2026-08-26)
#   This generator only ever wrote `llms.txt`. Restoring the block there made
#   `scripts/audit_bilingual_docs.sh` fail with `en_count=1 pt_count=0`: the
#   bilingual auditor requires every CLI invocation to appear in both files,
#   and the inventory is a list of invocations.
#   The block is machine-readable JSON emitted by `commands --json`, so it is
#   IDENTICAL in both languages by construction — there is nothing to
#   translate, and leaving pt-BR without it is a hole, not a language choice.
#   The surrounding prose stays whatever each file already says.
if [[ $# -gt 0 ]]; then
  TARGETS=("$@")
else
  TARGETS=(llms.txt llms.pt-BR.txt)
fi

TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

cargo run --quiet -- commands --json >"$TMP"

stale=0
for OUT in "${TARGETS[@]}"; do
  # Sibling of $OUT so the final `mv` stays inside one filesystem and is atomic.
  NEW="$(mktemp "${OUT}.gen.XXXXXX")"

  if [[ -f "$OUT" ]]; then
    # Prose = everything before the marker, trailing blanks normalised away.
    prose="$(awk 'BEGIN{p=1} /^<!-- GENERATED_COMMANDS_JSON/{p=0} p' "$OUT")"
    printf '%s\n' "$prose" >"$NEW"
  else
    printf '# browser-automation-cli\n' >"$NEW"
  fi

  {
    echo
    echo '<!-- GENERATED_COMMANDS_JSON: do not edit by hand; run scripts/gen-llms-txt.sh -->'
    echo '```json'
    cat "$TMP"
    echo '```'
    echo '<!-- END_GENERATED_COMMANDS_JSON -->'
  } >>"$NEW"

  if [[ "$CHECK" -eq 1 ]]; then
    if cmp -s "$OUT" "$NEW"; then
      echo "ok: $OUT matches the live command inventory"
    else
      echo "$OUT is stale; run scripts/gen-llms-txt.sh" >&2
      stale=1
    fi
    rm -f "$NEW"
    continue
  fi

  mv "$NEW" "$OUT"
  echo "ok: appended commands inventory to $OUT"
done

# Every target is reported before the exit, so one stale file does not hide the
# verdict on the others.
exit "$stale"
