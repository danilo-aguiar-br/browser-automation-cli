#!/usr/bin/env bash
# ci-check: verifier
#   Opts this script into the ci-check gate. Discovery used to be the glob
#   `scripts/*-check.sh` alone, so a blocking verifier whose name ended in
#   `-audit.sh`, `-gate.sh` or `_docs.sh` was invisible to the gate no matter
#   how loudly it failed. Measured 2026-08-26: FOUR such verifiers existed and
#   none of them had ever run inside ci-check. Name was an accident; this line
#   is the intent.
# Audit bilingual public docs: CLI invocations inside code fences must match EN vs PT.
# Usage:
#   bash scripts/audit_bilingual_docs.sh
# Exit:
#   0 all pairs match
#   1 invocation drift
#   2 missing pair file or fatal error
#
# WHY THIS IS BASH AND NOT AN INTERPRETER
#   This gate used to embed a Python heredoc. The product is Rust end to end and
#   ships no interpreter, so a gate that reaches for one fails on any host that
#   lacks it — under `set -euo pipefail`, with no guard. Ported 2026-08-18 with
#   the same extraction rules, the same multiset comparison and the same report
#   shape: fenced blocks, backslash continuations, quote-aware comment removal,
#   pipeline segments, leading env/sudo/command/time prefixes, whitespace
#   normalisation, then a multiset diff of the surviving invocations.

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting (`--smart-case` widens matches, `--max-columns`
# truncates them away). Clearing the variable neutralizes the whole file; `-s`
# would close only one of those doors.
export RIPGREP_CONFIG_PATH=
set -euo pipefail

# `sort` and `comm` must agree on collation or `comm` rejects its own input as
# unsorted. Byte order is also the stable one across hosts.
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CLI_TOKEN="browser-automation-cli"
# Literal backslash by code point, so no pattern below needs one inline; see
# the note beside the line-continuation test for what happened when it did.
BACKSLASH=$'\x5c'
# Same treatment for the backtick: it is both a quoting character and the
# character being searched for, and writing it inline is what left this file
# unparseable.
BACKTICK=$'\x60'
# Single and double quote by code point, for the same reason: this function
# scans for quote characters, so writing them inline makes the scanner's own
# source ambiguous to the parser reading it.
SINGLE_QUOTE=$'\x27'
DOUBLE_QUOTE=$'\x22'

PAIRS=(
  "README.md|README.pt-BR.md"
  "CHANGELOG.md|CHANGELOG.pt-BR.md"
  "CONTRIBUTING.md|CONTRIBUTING.pt-BR.md"
  "SECURITY.md|SECURITY.pt-BR.md"
  "INTEGRATIONS.md|INTEGRATIONS.pt-BR.md"
  "CODE_OF_CONDUCT.md|CODE_OF_CONDUCT.pt-BR.md"
  "docs/HOW_TO_USE.md|docs/HOW_TO_USE.pt-BR.md"
  "docs/AGENTS.md|docs/AGENTS.pt-BR.md"
  "docs/COOKBOOK.md|docs/COOKBOOK.pt-BR.md"
  "docs/CROSS_PLATFORM.md|docs/CROSS_PLATFORM.pt-BR.md"
  "docs/MIGRATION.md|docs/MIGRATION.pt-BR.md"
  "docs/TESTING.md|docs/TESTING.pt-BR.md"
  "llms.txt|llms.pt-BR.txt"
  # ADDED 2026-08-28 — the list was five pairs short of the disk.
  #   `fd -e md -e txt -tf pt-BR` finds EIGHTEEN EN/pt-BR pairs outside
  #   `docs_prd/` and `skills/`; this array carried thirteen, so
  #   `llms-full`, `PRIVACY`, `CONFIGURATION`, `ARCHITECTURE` and `ROADMAP`
  #   could diverge freely while the gate reported `ok=13 fail=0`. All five
  #   PASS on the first run, so this closes a latent exposure and not an
  #   active defect — which is exactly why nothing had ever surfaced it.
  #   A hand-written list is the defect; it is kept here rather than
  #   globbed because the auditor must FAIL on a missing counterpart, and
  #   discovery from the pt-BR side can only see pairs that already exist.
  #   Adding a document therefore means adding a line here, on purpose.
  "llms-full.txt|llms-full.pt-BR.txt"
  "PRIVACY.md|PRIVACY.pt-BR.md"
  "docs/CONFIGURATION.md|docs/CONFIGURATION.pt-BR.md"
  "docs/ARCHITECTURE.md|docs/ARCHITECTURE.pt-BR.md"
  "docs/ROADMAP.md|docs/ROADMAP.pt-BR.md"
  # `skills/*/SKILL.md` is EXCLUDED, and this is the reason the note above owed
  # the reader. Added to PAIRS on 2026-08-29 and removed the same day, because
  # measuring it is what produced the justification.
  #
  # The pair reports 43 prefixed invocations in EN against 13 in PT, which reads
  # as a 30-example gap and is not one. The two files carry the SAME 57 sections
  # and differ only in how a playbook is laid out: EN opens `#### A. Diagnostics`
  # and repeats the binary on each line, PT writes `- DEVE executar` and names
  # the binary once per bullet, continuing with bare subcommands after `;`. Both
  # spell the binary once and then economise; EN simply breaks into more lines.
  #
  # Compared on what the documents actually COVER — distinct subcommands named
  # in any code span — EN has 106 and PT has 113, and the only differences are
  # generic words (`format`, `formats`, `usage` against `apis`, `domains`,
  # `export`, `graphql`, `ws`). PT covers marginally MORE.
  #
  # So this auditor would be measuring layout on this pair, and satisfying it
  # would mean adding 36 prefixes to inflate a counter without teaching the
  # reader anything. That is optimising the metric instead of the goal. Include
  # the pair here only after the two files agree on layout.
)

# Remove an unquoted `#` comment. Quote state is tracked so that a `#` inside
# '...' or "..." survives, which is exactly what the previous implementation did.
strip_shell_comment() {
  local line="$1"
  if [[ "$line" != *"#"* ]]; then
    printf '%s' "${line%"${line##*[![:space:]]}"}"
    return
  fi
  local i c in_single=0 in_double=0 out=""
  for ((i = 0; i < ${#line}; i++)); do
    c="${line:i:1}"
    if [[ "$c" == "$SINGLE_QUOTE" && "$in_double" -eq 0 ]]; then
      in_single=$((1 - in_single))
    elif [[ "$c" == "$DOUBLE_QUOTE" && "$in_single" -eq 0 ]]; then
      in_double=$((1 - in_double))
    elif [[ "$c" == "#" && "$in_single" -eq 0 && "$in_double" -eq 0 ]]; then
      break
    fi
    out+="$c"
  done
  printf '%s' "${out%"${out##*[![:space:]]}"}"
}

# Print every CLI invocation found inside fenced code blocks of $1, one per line.
extract_invocations() {
  local path="$1"
  [[ -f "$path" ]] || return 0

  local line stripped buf="" in_fence=0 i
  local -a logical=() parts=()

  # Pass 1: keep only fenced bodies, joining backslash continuations.
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" == "$BACKTICK$BACKTICK$BACKTICK"* ]]; then
      in_fence=$((1 - in_fence))
      if [[ -n "$buf" ]]; then
        logical+=("$buf")
        buf=""
      fi
      continue
    fi
    # Outside a fence, harvest INLINE backticks. This project writes its
    # commands in inline code spans at least as often as in fenced blocks —
    # `browser-automation-cli --json config init` in docs/CONFIGURATION.md is
    # the canonical shape — and pass 1 used to discard every one of them.
    # Measured 2026-08-29, the cost of that blindness: docs/CONFIGURATION.md
    # contributed 0 invocations while holding 9, and the SKILL.md pair
    # contributed 0 while holding 42 in EN against 12 in PT. Six pairs were
    # compared as empty-against-empty and reported OK, which is agreement with
    # anything rather than agreement with each other.
    #
    # Splitting on the backtick makes the ODD indices the span bodies, which
    # avoids a regex loop that has to consume its own match to terminate.
    if [[ "$in_fence" -ne 1 ]]; then
      if [[ "$line" == *"$BACKTICK"*"$CLI_TOKEN"* ]]; then
        IFS="$BACKTICK" read -r -a parts <<<"$line"
        for ((i = 1; i < ${#parts[@]}; i += 2)); do
          [[ "${parts[i]}" == *"$CLI_TOKEN"* ]] && logical+=("${parts[i]}")
        done
      fi
      continue
    fi
    stripped="${line%"${line##*[![:space:]]}"}"
    # The backslash lives in a variable rather than inline. Written as a literal
    # here — `*'\'` in the test and `${stripped%'\'}` in the strip — this file
    # did not PARSE: `bash -n` reported "syntax error near unexpected token
    # `then'" and the script exited 2 before running a single check.
    #
    # That is the failure mode worth recording. A gate that cannot parse does
    # not report a wrong answer, it reports nothing, and a caller reading only
    # the summary cannot tell "no findings" from "never ran". Measured
    # 2026-08-31: it had been exiting 2 the whole time.
    if [[ "$stripped" == *"$BACKSLASH" ]]; then
      stripped="${stripped%"$BACKSLASH"}"
      stripped="${stripped%"${stripped##*[![:space:]]}"}"
      buf+="${stripped} "
      continue
    fi
    if [[ -n "$buf" ]]; then
      buf+="${stripped#"${stripped%%[![:space:]]*}"}"
      logical+=("$buf")
      buf=""
    else
      logical+=("$line")
    fi
  done <"$path"
  [[ -n "$buf" ]] && logical+=("$buf")

  # Pass 2: one invocation per pipeline segment that names the binary.
  local seg inv
  local -a segs words
  for line in ${logical[@]+"${logical[@]}"}; do
    [[ "$line" == *"$CLI_TOKEN"* ]] || continue
    line="$(strip_shell_comment "$line")"
    line="${line#"${line%%[![:space:]]*}"}"
    [[ -n "$line" && "$line" == *"$CLI_TOKEN"* ]] || continue
    IFS='|' read -r -a segs <<<"$line"
    for seg in "${segs[@]}"; do
      [[ "$seg" == *"$CLI_TOKEN"* ]] || continue
      seg="${seg#"${seg%%[![:space:]]*}"}"
      seg="${seg%"${seg##*[![:space:]]}"}"
      # leading `VAR=value ` assignments
      while [[ "$seg" =~ ^[A-Za-z_][A-Za-z0-9_]*=[^[:space:]]+[[:space:]]+(.*)$ ]]; do
        seg="${BASH_REMATCH[1]}"
      done
      # leading sudo / command / time
      if [[ "$seg" =~ ^(sudo|command|time)[[:space:]]+(.*)$ ]]; then
        seg="${BASH_REMATCH[2]}"
      fi
      [[ "$seg" == *"$CLI_TOKEN"* ]] || continue
      inv="${CLI_TOKEN}${seg#*"$CLI_TOKEN"}"
      # collapse runs of whitespace, trim both ends
      read -r -a words <<<"$inv"
      inv="${words[*]}"
      [[ -n "$inv" ]] && printf '%s\n' "$inv"
    done
  done
}

ok_pairs=0
# FACTUAL ANCHORS: a date and a bolded number do not translate.
#
# Measured 2026-09-01: this auditor answered `ok=18 fail=0` while
# `docs/TESTING.pt-BR.md` attributed the counts 44 and 80 to the measurement
# date 2026-08-28, which never produced them, and had lost the whole sentence
# naming the test that re-measures both. Every invocation matched, because the
# divergence lived in PROSE and this file only ever compared commands. A gate
# that is green on a page stating a false measured date is worse than no gate:
# the reader trusts the pair BECAUSE it passed.
#
# The invariant is language-independent. An ISO date and a `**42**` are the
# same bytes in both files, so a translation may rewrite every word around them
# and must never change them. Measured across the 13 real pairs on 2026-09-01:
# both multisets already agree, so this is a regression guard and not a wish.
#
# Deliberately NOT every integer. Prose counts things the other language
# phrases differently ("three files" against "3 arquivos"), and a gate that
# fires on that teaches the reader to ignore it. The bold marker is the
# project convention for a number the docs FREEZE, which is exactly the set
# that must survive translation intact.
extract_anchors() {
  rg --no-filename -o '20[0-9][0-9]-[0-9][0-9]-[0-9][0-9]' "$1" 2>/dev/null || true
  rg --no-filename -o '[*][*][0-9]+[*][*]' "$1" 2>/dev/null || true
}

# SELF-TEST: a mangled pattern must never read as "this pair has no anchors".
#
# Measured 2026-09-01, inside the very change that added the check above: an
# editing accident stripped the quotes and the backslashes, leaving the literal
# `b20[0-9]{2}-...`, which matches nothing. Both sides then extracted ZERO
# anchors, the two empty multisets agreed, and the pair reported OK while the
# pt-BR page carried a measurement date that never produced its numbers.
#
# That is the same fail-open this section exists to close, reproduced inside
# the closing itself, and it was invisible until the check was PROVEN to fail
# on an injected defect. An extractor that answers "nothing found" is
# indistinguishable from an extractor that is broken, so it must be asked a
# question whose answer is known before it is trusted with one that is not.
#
# Bracket expressions rather than `\b` and `\*` on purpose: the patterns now
# survive a shell layer that eats backslashes, so the probe below is a second
# lock and not the only one.
anchor_self_test() {
  local probe="$tmp/anchor_probe" got
  printf 'ver 2026-09-01 e **44** aqui\n' >"$probe"
  got="$(extract_anchors "$probe" | sort | tr '\n' ' ')"
  if [[ "$got" != '**44** 2026-09-01 ' ]]; then
    echo "audit_bilingual_docs: anchor extractor is broken" >&2
    echo "  probe expected '**44** 2026-09-01 ' and got '${got}'" >&2
    echo "  every pair would compare empty-against-empty and pass" >&2
    exit 2
  fi
}

fail_pairs=0
missing_files=0

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
anchor_self_test

for pair in "${PAIRS[@]}"; do
  en_rel="${pair%%|*}"
  pt_rel="${pair##*|}"

  if [[ ! -f "$en_rel" && ! -f "$pt_rel" ]]; then
    echo "SKIP  ${en_rel} ↔ ${pt_rel}  (both missing)"
    continue
  fi
  if [[ ! -f "$en_rel" ]]; then
    echo "FAIL  ${en_rel} ↔ ${pt_rel}"
    echo "  - missing file: ${en_rel}"
    missing_files=$((missing_files + 1))
    fail_pairs=$((fail_pairs + 1))
    continue
  fi
  if [[ ! -f "$pt_rel" ]]; then
    echo "FAIL  ${en_rel} ↔ ${pt_rel}"
    echo "  - missing file: ${pt_rel}"
    missing_files=$((missing_files + 1))
    fail_pairs=$((fail_pairs + 1))
    continue
  fi

  extract_invocations "$en_rel" >"$tmp/en"
  extract_invocations "$pt_rel" >"$tmp/pt"
  en_count="$(rg -c '^' "$tmp/en" || echo 0)"
  pt_count="$(rg -c '^' "$tmp/pt" || echo 0)"

  sort "$tmp/en" >"$tmp/en.sorted"
  sort "$tmp/pt" >"$tmp/pt.sorted"
  # comm on sorted input with duplicates IS the multiset difference.
  comm -23 "$tmp/en.sorted" "$tmp/pt.sorted" >"$tmp/miss_pt"
  comm -13 "$tmp/en.sorted" "$tmp/pt.sorted" >"$tmp/miss_en"
  miss_pt_n="$(rg -c '^' "$tmp/miss_pt" || echo 0)"
  miss_en_n="$(rg -c '^' "$tmp/miss_en" || echo 0)"

  # A pair that yields ZERO invocations on BOTH sides is not "in agreement":
  # it is UNAUDITED, and the multiset comparison below would be an empty set
  # against an empty set, which agrees with everything. Measured 2026-08-29:
  # three pairs sat in that state while the summary read `ok=18 fail=0` —
  # `docs/CONFIGURATION.md` names the binary 9 times, `llms-full.txt` 3 times
  # and the `SKILL.md` pair 43 times in EN against 13 in PT, a 30-occurrence
  # divergence the auditor was reporting as OK. The cause is mechanical: pass 1
  # keeps only fenced bodies, so a document that writes its commands in prose
  # or in an indented block contributes nothing and still passes.
  #
  # The distinction that makes this safe is `$CLI_TOKEN` in the RAW file. A
  # document with no commands at all (CODE_OF_CONDUCT, PRIVACY) legitimately
  # extracts zero and is left alone; a document that demonstrably talks about
  # the binary and still extracts zero is one the extractor cannot see.
  if [[ "$en_count" -eq 0 && "$pt_count" -eq 0 ]]; then
    en_raw="$(rg -c -F "$CLI_TOKEN" "$en_rel" 2>/dev/null || echo 0)"
    pt_raw="$(rg -c -F "$CLI_TOKEN" "$pt_rel" 2>/dev/null || echo 0)"
    if [[ "$en_raw" -gt 0 || "$pt_raw" -gt 0 ]]; then
      echo "FAIL  ${en_rel} ↔ ${pt_rel}  (unaudited: zero invocations extracted)"
      echo "  the files name ${CLI_TOKEN} ${en_raw} time(s) in EN and ${pt_raw} in PT,"
      echo "  but none of it sits inside a fenced code block, so this pair is"
      echo "  compared as empty-against-empty and agrees with anything."
      echo "  fix: fence the commands, or drop the pair from PAIRS on purpose."
      fail_pairs=$((fail_pairs + 1))
      continue
    fi
  fi

  extract_anchors "$en_rel" | sort >"$tmp/en.anchors"
  extract_anchors "$pt_rel" | sort >"$tmp/pt.anchors"
  comm -23 "$tmp/en.anchors" "$tmp/pt.anchors" >"$tmp/anchor_pt"
  comm -13 "$tmp/en.anchors" "$tmp/pt.anchors" >"$tmp/anchor_en"
  anchor_pt_n="$(rg -c '^' "$tmp/anchor_pt" || echo 0)"
  anchor_en_n="$(rg -c '^' "$tmp/anchor_en" || echo 0)"
  anchor_bad=$((anchor_pt_n + anchor_en_n))

  if [[ "$miss_pt_n" -eq 0 && "$miss_en_n" -eq 0 && "$anchor_bad" -eq 0 ]]; then
    echo "OK    ${en_rel} ↔ ${pt_rel}  (${en_count} invocations)"
    ok_pairs=$((ok_pairs + 1))
    if ! cmp -s "$tmp/en" "$tmp/pt"; then
      echo "  warn: same multiset but different order"
    fi
  else
    echo "FAIL  ${en_rel} ↔ ${pt_rel}"
    echo "  en_count=${en_count} pt_count=${pt_count}"
    while IFS= read -r anchor; do
      echo "  - anchor_missing_in_pt: ${anchor}"
    done <"$tmp/anchor_pt"
    while IFS= read -r anchor; do
      echo "  - anchor_missing_in_en: ${anchor}"
    done <"$tmp/anchor_en"
    while IFS= read -r inv; do
      echo "  - missing_in_pt: ${inv}"
    done < <(head -n 50 "$tmp/miss_pt")
    if [[ "$miss_pt_n" -gt 50 ]]; then
      echo "  - missing_in_pt: ... +$((miss_pt_n - 50)) more"
    fi
    while IFS= read -r inv; do
      echo "  - missing_in_en: ${inv}"
    done < <(head -n 50 "$tmp/miss_en")
    if [[ "$miss_en_n" -gt 50 ]]; then
      echo "  - missing_in_en: ... +$((miss_en_n - 50)) more"
    fi
    fail_pairs=$((fail_pairs + 1))
  fi
done

echo "Summary: ok=${ok_pairs} fail=${fail_pairs} missing_files=${missing_files}"
# Missing FILES are not the same result as a failed comparison: if every
  # failure was a file that does not exist, nothing was actually audited, and
  # exit 2 says "could not run" rather than "ran and disagreed".
  if [[ "$missing_files" -ne 0 && "$fail_pairs" -eq "$missing_files" && "$ok_pairs" -eq 0 ]]; then
  exit 2
fi
if [[ "$fail_pairs" -ne 0 ]]; then
  exit 1
fi
exit 0
