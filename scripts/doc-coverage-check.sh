#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Public documentation coverage gate — the docs must describe the LIVE surface.
#
# # Why this file exists
#
# `scripts/docs-check.sh` validates rustdoc: it builds the crate docs and audits
# `Cargo.toml` metadata. It never opens README, never opens `docs/`, and has no
# opinion about whether the prose still matches the binary.
#
# `scripts/inventory-flat-check.sh` pins the COMMAND count in prose. It counts.
# It does not check that each command is actually named anywhere, and it says
# nothing at all about configuration keys.
#
# So the largest documentation surface in the product had no gate: 176 XDG keys,
# of which 132 appeared in no public document. An agent told to "discover keys
# with `config list-keys --json`" can do that, but a human comparing the product
# against alternatives reads the docs, and the docs described 44 keys.
#
# This gate reads the COMPILED BINARY for the live surface, never a hardcoded
# list, so it cannot drift the way a transcribed count drifts.
#
# CLEAN STDOUT: one status line per assertion on stdout; diagnostics on stderr.
set -uo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting (`--smart-case` widens matches, `--max-columns`
# truncates them away). Clearing the variable neutralizes the whole file; `-s`
# would close only one of those doors.
export RIPGREP_CONFIG_PATH=

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Binary resolution, in order. The PATH fallback is not convenience: the
# verifier-controls harness copies the tree WITHOUT `target/` and runs this gate
# there, so a build-dir-only lookup would abort before the first assertion — and
# a gate that reaches no assertion is indistinguishable from a gate that passes.
BIN="${BIN:-}"
for candidate in \
  "$BIN" \
  "$ROOT/target/release/browser-automation-cli" \
  "$ROOT/target/debug/browser-automation-cli" \
  "$(command -v browser-automation-cli 2>/dev/null || true)"; do
  if [[ -n "$candidate" && -x "$candidate" ]]; then
    BIN="$candidate"
    break
  fi
done
if [[ ! -x "$BIN" ]]; then
  echo "doc-coverage-check: FAIL (no binary; run cargo build --release)" >&2
  echo "doc-coverage-check: FAIL"
  exit 1
fi

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad() {
  printf 'FAIL  %s\n' "$1"
  fail=1
}

echo "== doc coverage (live binary surface vs public prose) =="

# NAME the binary this run measured, and say when it is not the one this tree
# builds.
#
# Measured 2026-08-29: with no `target/release` present, resolution fell through
# to `~/.cargo/bin/browser-automation-cli` and the gate reported "the live
# command count is 69" against a tree whose binary carries 71 — `feed` and
# `sitemap` were missing from the installed build. Eleven documents were then
# "corrected" away from the truth, because the gate had measured a DIFFERENT
# product and said so nowhere.
#
# A version check would not have caught it: both binaries print 0.1.9. Inside a
# development cycle the version is a name, not a fingerprint, so the only honest
# signal is the PATH the gate actually opened. Printing it costs one line and
# turns a silent substitution into something a reader can see.
# Normalise before comparing. `BIN=./target/release/...` is a path INSIDE this
# tree that a literal prefix match against "$ROOT/target/" rejects, so the first
# shape of this check warned about the correct binary — a false alarm on the
# very gate written to catch a false measurement.
BIN_ABS="$(cd "$(dirname "$BIN")" 2>/dev/null && pwd)/$(basename "$BIN")"
echo "   binary: $BIN_ABS"
case "$BIN_ABS" in
  "$ROOT/target/"*) ;;
  *)
    # This compares a PATH, so say that and not more. A binary built with
    # `cargo build --target-dir <elsewhere>` is current and still lands here,
    # and claiming it "is not built from this tree" would be false. What the
    # check actually knows is that it cannot vouch for the binary's age, which
    # is the part worth telling.
    echo "   NOTE: that binary is outside this tree's target/, so this check" >&2
    echo "         cannot tell whether it was built from the sources being" >&2
    echo "         audited. Confirm it is current, or set BIN= to one under" >&2
    echo "         $ROOT/target/." >&2
    ;;
esac

CONFIG_EN="docs/CONFIGURATION.md"
CONFIG_PT="docs/CONFIGURATION.pt-BR.md"

# ── 1. Every live XDG key must be documented, in both languages ──────────
# Read from the binary, never from a transcribed list.
# `mapfile` is a bash 4 builtin and macOS ships bash 3.2, so every array read
# in this file uses the portable read loop below instead (2026-09-04).
LIVE_KEYS=()
while IFS= read -r __line; do LIVE_KEYS+=("$__line"); done < <("$BIN" --json config list-keys 2>/dev/null | jaq -r '.data.keys[].key')
if [[ "${#LIVE_KEYS[@]}" -lt 2 ]]; then
  bad "could not read the live XDG key list from the binary"
else
  for doc in "$CONFIG_EN" "$CONFIG_PT"; do
    if [[ ! -f "$doc" ]]; then
      bad "$doc is missing (the XDG reference has no home)"
      continue
    fi
    missing=0
    first_missing=""
    for key in "${LIVE_KEYS[@]}"; do
      if ! rg -q --fixed-strings -- "$key" "$doc"; then
        missing=$((missing + 1))
        [[ -z "$first_missing" ]] && first_missing="$key"
      fi
    done
    if [[ "$missing" -ne 0 ]]; then
      bad "$doc omits $missing of ${#LIVE_KEYS[@]} live XDG keys (first: $first_missing)"
    else
      pass "$doc documents all ${#LIVE_KEYS[@]} live XDG keys"
    fi
  done
fi

# A key documented but no longer live is the other half of the same drift, and
# the more misleading half: it teaches argv the binary rejects.
if [[ -f "$CONFIG_EN" && "${#LIVE_KEYS[@]}" -gt 1 ]]; then
  stale=0
  first_stale=""
  while IFS= read -r documented; do
    hit=0
    for key in "${LIVE_KEYS[@]}"; do
      [[ "$documented" == "$key" ]] && hit=1 && break
    done
    if [[ "$hit" -eq 0 ]]; then
      stale=$((stale + 1))
      [[ -z "$first_stale" ]] && first_stale="$documented"
    fi
  done < <(rg -o -r '$1' '^- `([a-z][a-z0-9_]{3,})` —' "$CONFIG_EN" | sort -u)
  if [[ "$stale" -ne 0 ]]; then
    bad "$CONFIG_EN documents $stale key(s) the binary no longer exposes (first: $first_stale)"
  else
    pass "$CONFIG_EN documents no retired key"
  fi
fi

# ── 2. Every live command must be named in the entry-point documents ─────
LIVE_CMDS=()
while IFS= read -r __line; do LIVE_CMDS+=("$__line"); done < <("$BIN" --json commands 2>/dev/null | jaq -r '.data.commands[]')
if [[ "${#LIVE_CMDS[@]}" -lt 2 ]]; then
  bad "could not read the live command list from the binary"
else
  for doc in README.md README.pt-BR.md docs/HOW_TO_USE.md docs/HOW_TO_USE.pt-BR.md; do
    [[ -f "$doc" ]] || { bad "$doc is missing"; continue; }
    missing=0
    first_missing=""
    for cmd in "${LIVE_CMDS[@]}"; do
      if ! rg -q --fixed-strings -- "\`$cmd" "$doc"; then
        missing=$((missing + 1))
        [[ -z "$first_missing" ]] && first_missing="$cmd"
      fi
    done
    if [[ "$missing" -ne 0 ]]; then
      bad "$doc never names $missing of ${#LIVE_CMDS[@]} live commands (first: $first_missing)"
    else
      pass "$doc names all ${#LIVE_CMDS[@]} live commands"
    fi
  done
fi

# ── 3. Bilingual pairing is a rule, so it gets an assertion ──────────────
# `gaps.md` and `CLAUDE.md` are internal and excluded from the tarball.
orphans=0
while IFS= read -r doc; do
  case "$doc" in
    ./gaps.md | ./CLAUDE.md | ./AGENTS.md | ./MEMORY.md) continue ;;
  esac
  base="${doc%.*}"
  ext="${doc##*.}"
  if [[ ! -f "${base}.pt-BR.${ext}" ]]; then
    printf '  orphan: %s has no .pt-BR pair\n' "$doc" >&2
    orphans=$((orphans + 1))
  fi
done < <(fd -d 1 -e md -e txt . . 2>/dev/null | rg -v 'pt-BR'; fd -e md . docs/ 2>/dev/null | rg -v 'pt-BR|schemas/README')
if [[ "$orphans" -ne 0 ]]; then
  bad "$orphans public document(s) have no .pt-BR mirror"
else
  pass "every public document has a .pt-BR mirror"
fi

# ── 4. No document may present a per-command flag as global ─────────────
# `--select`, `--filter`, `--limit` and `--sort` are LOCAL flags on scrape,
# crawl, map, search, batch-scrape and the media `info` verbs. The universal
# envelope flags are `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`.
# Calling the first set global sends an agent to `unexpected argument`.
#
# The assertion is scope-aware on purpose: a naive "does this flag appear"
# check would fire on the legitimate scrape documentation.
global_help="$("$BIN" --help 2>&1)"
scope_violations=0
while IFS= read -r hit; do
  [[ -z "$hit" ]] && continue
  file="${hit%%:*}"
  rest="${hit#*:}"
  line="${rest%%:*}"
  text="${rest#*:}"
  # Known limitation, stated rather than hidden: this is proximity matching, so
  # a sentence that names the flag in order to say it is NOT global reads the
  # same as one that offers it. Prose explaining the `-rows` suffix does exactly
  # that, so lines carrying an explicit disclaimer are skipped. A line that
  # merely LISTS the flag under a "these are global" heading has no disclaimer
  # and is still caught — which is the regression this assertion exists for.
  # The marker list is deliberately explicit rather than clever. Every entry was
  # added because a real, correct sentence tripped the proximity match: prose
  # that names the flag precisely in order to warn against it reads identically
  # to prose that offers it. Extend this list when a new phrasing appears; do
  # NOT weaken the proximity match itself, or the assertion stops firing on the
  # regression it exists for.
  #
  # `flags LOCAIS` is listed beside `flag LOCAL` because Portuguese inflects the
  # adjective and English does not. `LOCAL flags` still contains `LOCAL flag` as
  # a substring, so the English disclaimer matched for free while its faithful
  # translation did not — the assertion fired on the pt-BR mirror of a sentence
  # it had just approved in English. A marker list written against one language
  # will keep producing that asymmetry; add both forms when adding either.
  if printf '%s' "$text" | rg -q -i -- \
    'not a global|not global|are not global|is not global|do not pass|never pass|não é flag global|não são flag|não são global|nao e flag global|não passe|nunca passe|already taken|já eram|ja eram|per-command|por comando|would have collided|colidiria|LOCAL flag|flag LOCAL|flags LOCAIS|flags locais|nunca estas globais|nunca estas flags globais|local `--|` local|exit `?2'; then
    continue
  fi
  for flag in --select --filter --limit --sort; do
    # Word-boundary match so `--filter-rows` never reads as `--filter`.
    if printf '%s' "$text" | rg -q -- "$flag(\`|\s|,|\.|$)"; then
      if ! printf '%s' "$global_help" | rg -q -- "$flag"; then
        printf '  scope: %s:%s presents %s as global\n' "$file" "$line" "$flag" >&2
        scope_violations=$((scope_violations + 1))
      fi
    fi
  done
# `CLAUDE.md` and `AGENTS.md` at the root are agent instruction files for OTHER
# tools and are excluded from the tarball; they legitimately document a
# `--select` belonging to a different binary.
# `base_*/` holds gitignored reference material that `Cargo.toml` already
# excludes from the package. The glob is generic on purpose: naming a single
# vendor directory both left the sibling ones scanned and wrote a product name
# this repository must not carry.
done < <(rg -n -i --glob '*.md' --glob '*.txt' \
  -g '!gaps.md' -g '!CLAUDE.md' -g '!AGENTS.md' -g '!MEMORY.md' -g '!base_*/**' \
  'global|GLOBAIS|globais|every command|todos os .. comandos|all .. commands' . 2>/dev/null || true)
if [[ "$scope_violations" -ne 0 ]]; then
  bad "$scope_violations documentation line(s) present a per-command flag as global"
else
  pass "no document presents a per-command flag as global"
fi

# ── 5. The eight real envelope flags must be documented ─────────────────
# A local counter, not the global `fail`: gating the PASS line on `fail` would
# silence this assertion whenever an EARLIER one failed, which reads as "the
# check did not run" — the exact ambiguity this gate exists to remove.
undocumented_flags=0
for flag in --fields --filter-rows --limit-rows --sort-rows --dedupe-by \
            --count-only --truncate-content --max-output-bytes; do
  if ! rg -q --fixed-strings -- "$flag" README.md docs/AGENTS.md 2>/dev/null; then
    bad "the universal envelope flag $flag is documented nowhere in README/AGENTS"
    undocumented_flags=$((undocumented_flags + 1))
  fi
done
if [[ "$undocumented_flags" -eq 0 ]]; then
  pass "the eight universal envelope flags are documented"
fi

# ── 6. Product configuration is XDG, never a product environment variable ─
# The product reads no product env var. A document that teaches one is teaching
# a knob that silently does nothing.
env_teaching="$(rg -n --glob '*.md' --glob '*.txt' -g '!gaps.md' -g '!docs/TESTING*' \
  'export BROWSER_AUTOMATION|BAC_[A-Z_]+=|BROWSER_AUTOMATION_CLI_[A-Z_]+=' . 2>/dev/null || true)"
if [[ -n "$env_teaching" ]]; then
  printf '%s\n' "$env_teaching" >&2
  bad "a document teaches a product environment variable as configuration"
else
  pass "no document teaches a product environment variable"
fi

# ── 7. Every target linked from llms.txt must exist ──────────────────────
broken=0
for src in llms.txt llms.pt-BR.txt llms-full.txt llms-full.pt-BR.txt; do
  [[ -f "$src" ]] || { bad "$src is missing"; continue; }
  while IFS= read -r target; do
    [[ -z "$target" ]] && continue
    case "$target" in http*) continue ;; esac
    if [[ ! -e "${target%%#*}" ]]; then
      printf '  broken: %s -> %s\n' "$src" "$target" >&2
      broken=$((broken + 1))
    fi
  done < <(rg -o -r '$1' '\]\(([^)]+)\)' "$src" 2>/dev/null | sort -u)
done
if [[ "$broken" -ne 0 ]]; then
  bad "$broken llms.txt link(s) point at a file that does not exist"
else
  pass "every llms.txt link resolves to a real file"
fi

# ── 8. Every GLOBAL flag must appear in a usage document ────────────────
# Section 4 above audits one direction: a document must not present a local
# flag as global. That check has a blind spot exactly the size of this one — it
# says nothing about a global flag that appears in NO document at all.
#
# The blind spot was measured, not theorised. `--input-profile` and
# `--input-seed` shipped as globals and reached zero usage documents, and the
# round that found them added `--no-stealth`, `--proxy`, `--warmup`,
# `--no-xvfb` and `--stealth-profile` to the same hole. Seven flags an agent
# could only discover by reading `--help`, in a product whose whole contract is
# that the documents are the interface.
#
# The universe is the LIVE binary's global help, never a list kept by hand: a
# hand list stops matching the product on the first flag added after it.
undocumented=0
usage_docs=(AGENTS.md HOW_TO_USE.md docs/AGENTS.md docs/HOW_TO_USE.md)
present_docs=()
for doc in "${usage_docs[@]}"; do
  [[ -f "$doc" ]] && present_docs+=("$doc")
done
if [[ "${#present_docs[@]}" -eq 0 ]]; then
  bad "no usage document found; cannot audit global flag coverage"
else
  while IFS= read -r flag; do
    [[ -z "$flag" ]] && continue
    # `--help` and `--version` are clap built-ins every CLI has; documenting
    # them teaches nothing and their absence is not a coverage gap.
    case "$flag" in --help | --version) continue ;; esac
    if ! rg -q -F -- "$flag" "${present_docs[@]}" 2>/dev/null; then
      printf '  undocumented global: %s\n' "$flag" >&2
      undocumented=$((undocumented + 1))
    fi
  done < <("$BIN" --help 2>&1 | rg -o -r '$1' '^\s+(?:-\w,\s+)?(--[a-z][a-z0-9-]*)' | sort -u)
  if [[ "$undocumented" -ne 0 ]]; then
    bad "$undocumented global flag(s) appear in no usage document"
  else
    pass "every global flag appears in a usage document"
  fi
fi

# ── 9. A count TRANSCRIBED into prose must equal the live count ─────────
# Assertions 1 and 2 above are presence tests, and the header of this file says
# a presence test "cannot drift the way a transcribed count drifts". That is
# true and it is only half the story: presence and arithmetic are independent
# properties, and the gate was blind to the second one.
#
# Measured on 2026-08-10: the live surface carried 204 keys, assertion 1
# confirmed `docs/CONFIGURATION.md` documented all 204 — and the prose still
# transcribed the number 176 in 21 places, including inside the very documents
# this gate had just approved. Every key was present; the sentence counting them
# described the previous release.
#
# The match is deliberately narrow. `176` is also a legitimate default value, a
# byte count and a historical CHANGELOG entry, so the pattern fires only on a
# number standing next to a word that QUALIFIES it as a count of keys or of
# commands. The two CHANGELOG files are excluded for the same reason: an entry
# recording "176 XDG keys" under 0.1.7 was correct for 0.1.7, and a version
# history that had to be rewritten on every release would stop being a history.
MAX_NAMES_IN_SUMMARY=8

LIVE_KEY_COUNT="${#LIVE_KEYS[@]}"
LIVE_CMD_COUNT="${#LIVE_CMDS[@]}"
count_drift=0
if [[ "$LIVE_KEY_COUNT" -lt 2 || "$LIVE_CMD_COUNT" -lt 2 ]]; then
  bad "cannot audit transcribed counts without the live surface"
else
  count_docs=()
  while IFS= read -r doc; do
    [[ -n "$doc" ]] && count_docs+=("$doc")
  done < <(fd -d 1 -e md . . 2>/dev/null | rg -v 'CHANGELOG|gaps\.md|CLAUDE\.md|AGENTS\.md|MEMORY\.md'
    fd -e md . docs/ skills/ 2>/dev/null)
  if [[ "${#count_docs[@]}" -eq 0 ]]; then
    bad "no public document found; cannot audit transcribed counts"
  else
    COUNT_PHRASE='[0-9]{2,4}\s+(?:live\s+|XDG\s+|configuration\s+)*(?:keys|commands|chaves|comandos)\b'
    while IFS= read -r hit; do
      [[ -z "$hit" ]] && continue
      file="${hit%%:*}"
      rest="${hit#*:}"
      line="${rest%%:*}"
      text="${rest#*:}"
      # Same class of exemption as assertion 4, for the same reason: a sentence
      # that quotes a stale number precisely in order to forbid hard-coding it
      # reads identically to one that asserts it. Extend this list when a new
      # phrasing appears; do NOT widen the count pattern itself.
      if printf '%s' "$text" | rg -q -i -- \
        'do not hard-?code|hard-?code|hard-?coded|do not claim|não fixe|nao fixe|não hardcode|nao hardcode|discover with|descubra'; then
        continue
      fi
      # A line that binds its number to a NAMED RELEASE is history, not a claim
      # about the product in front of you, and rewriting it on every release
      # would destroy the record it exists to keep. This is the same exemption
      # the CHANGELOG gets, applied per line instead of per file, because
      # `INTEGRATIONS` and `MIGRATION` carry per-version entries inside
      # documents that also describe the current surface.
      #
      # The exemption is bounded on purpose: it needs the version token on the
      # SAME line. `Live inventory is 59 commands`, sitting under a version
      # heading but claiming the present tense on its own line, is still caught.
      if printf '%s' "$text" | rg -q -- '\bv?[0-9]+\.[0-9]+\.[0-9]+\b'; then
        continue
      fi
      while IFS= read -r phrase; do
        [[ -z "$phrase" ]] && continue
        number="${phrase%%[![:digit:]]*}"
        # `${var,,}` is bash 4 case expansion and macOS ships bash 3.2, where it
        # is a `bad substitution` at RUNTIME — measured 2026-09-04, printing the
        # error and then falling through with `lowered` unset, so every phrase
        # took the `*)` arm and was compared against the COMMAND count. The
        # check reported nothing and looked green. `tr` is POSIX.
        lowered="$(printf '%s' "$phrase" | tr '[:upper:]' '[:lower:]')"
        case "$lowered" in
          *keys | *chaves) expected="$LIVE_KEY_COUNT" noun="XDG key" ;;
          *) expected="$LIVE_CMD_COUNT" noun="command" ;;
        esac
        if [[ "$number" != "$expected" ]]; then
          printf '  count drift: %s:%s says "%s" but the live %s count is %s\n' \
            "$file" "$line" "$phrase" "$noun" "$expected" >&2
          count_drift=$((count_drift + 1))
        fi
      done < <(printf '%s' "$text" | rg -o -i -- "$COUNT_PHRASE")
    done < <(rg -n -H -i -- "$COUNT_PHRASE" "${count_docs[@]}" 2>/dev/null || true)
    if [[ "$count_drift" -ne 0 ]]; then
      bad "$count_drift documentation line(s) transcribe a count the live surface contradicts"
    else
      pass "every transcribed key/command count matches the live surface"
    fi
  fi
fi

# ── 10. The EMBEDDED skills must cover the live surface ─────────────────
# Assertions 1 and 2 read `docs/` and the READMEs. They never open `skills/`,
# and `skills/` is the one documentation surface that ships INSIDE the crate and
# is loaded verbatim by an agent. Measured on 2026-08-10: both embedded
# `references/xdg-keys.md` listed 176 keys, missing exactly the 28 that 0.1.8
# added — so an agent carrying the packaged skill operated the previous release.
#
# The failure names the absent keys, never only how many. A count tells the
# reader that something is wrong; a name tells them what to write.
SKILL_DIRS=(skills/browser-automation-cli-en skills/browser-automation-cli-pt)

if [[ "$LIVE_KEY_COUNT" -lt 2 ]]; then
  bad "cannot audit the embedded skills without the live XDG key list"
else
  for dir in "${SKILL_DIRS[@]}"; do
    doc="$dir/references/xdg-keys.md"
    if [[ ! -f "$doc" ]]; then
      bad "$doc is missing (the embedded skill has no XDG reference)"
      continue
    fi
    missing_keys=()
    for key in "${LIVE_KEYS[@]}"; do
      rg -q --fixed-strings -- "$key" "$doc" || missing_keys+=("$key")
    done
    if [[ "${#missing_keys[@]}" -ne 0 ]]; then
      for key in "${missing_keys[@]}"; do
        printf '  skill key gap: %s omits %s\n' "$doc" "$key" >&2
      done
      bad "$doc omits live XDG key(s); missing: ${missing_keys[*]:0:MAX_NAMES_IN_SUMMARY}"
    else
      pass "$doc lists every live XDG key"
    fi
  done
fi

if [[ "$LIVE_CMD_COUNT" -lt 2 ]]; then
  bad "cannot audit the embedded skills without the live command list"
else
  for dir in "${SKILL_DIRS[@]}"; do
    doc="$dir/SKILL.md"
    if [[ ! -f "$doc" ]]; then
      bad "$doc is missing (the embedded skill has no entry point)"
      continue
    fi
    missing_cmds=()
    for cmd in "${LIVE_CMDS[@]}"; do
      # Word-boundary rather than the backticked form assertion 2 uses: the
      # requirement is that the command is NAMED, and the English skill lists
      # its inventory unquoted. The known cost is stated rather than hidden — a
      # command whose name is also an ordinary word can be satisfied by prose,
      # which fails OPEN, never closed.
      rg -q -- "\b${cmd}\b" "$doc" || missing_cmds+=("$cmd")
    done
    if [[ "${#missing_cmds[@]}" -ne 0 ]]; then
      for cmd in "${missing_cmds[@]}"; do
        printf '  skill command gap: %s never names %s\n' "$doc" "$cmd" >&2
      done
      bad "$doc never names live command(s); missing: ${missing_cmds[*]:0:MAX_NAMES_IN_SUMMARY}"
    else
      pass "$doc names every live command"
    fi
  done
fi

# ── 11. Every GLOBAL flag must appear in the EMBEDDED skills too ─────────
# Assertion 8 audits the usage documents a human reads. It says nothing about
# the skills the crate ships, and those are what an agent actually loads: a flag
# absent there is a flag the agent will never reach for, however well the README
# describes it. Measured on 2026-08-10: twelve globals shipped in 0.1.8 with no
# mention in either embedded skill.
skill_flag_gaps=0
GLOBAL_FLAGS=()
while IFS= read -r __line; do GLOBAL_FLAGS+=("$__line"); done < <("$BIN" --help 2>&1 |
  rg -o -r '$1' '^\s+(?:-\w,\s+)?(--[a-z][a-z0-9-]*)' | sort -u)
if [[ "${#GLOBAL_FLAGS[@]}" -lt 2 ]]; then
  bad "could not read the live global flag list from the binary"
else
  for dir in "${SKILL_DIRS[@]}"; do
    skill_docs=("$dir/SKILL.md")
    while IFS= read -r ref; do
      [[ -n "$ref" ]] && skill_docs+=("$ref")
    done < <(fd -e md . "$dir/references" 2>/dev/null || true)
    for flag in "${GLOBAL_FLAGS[@]}"; do
      case "$flag" in --help | --version) continue ;; esac
      if ! rg -q -F -- "$flag" "${skill_docs[@]}" 2>/dev/null; then
        printf '  skill flag gap: %s never names %s\n' "$dir" "$flag" >&2
        skill_flag_gaps=$((skill_flag_gaps + 1))
      fi
    done
  done
  if [[ "$skill_flag_gaps" -ne 0 ]]; then
    bad "$skill_flag_gaps global flag(s) appear in no embedded skill document"
  else
    pass "every global flag appears in both embedded skills"
  fi
fi

# ── 12. Every GLOBAL flag must appear in BOTH agent contract documents ──
# Assertion 8 searches its four usage documents in a SINGLE ripgrep call, so it
# is an OR: one document naming the flag satisfies the whole set. That is a
# real property and it is weaker than its own PASS line suggests — it answers
# "is this flag written down anywhere" when the question an agent needs
# answered is "is it written where I look".
#
# Measured on 2026-08-10: `--mitm-no-redact-secrets` shipped in 0.1.8 as the
# only way to turn secret masking OFF, lived in `docs/HOW_TO_USE.md` alone, and
# assertion 8 passed green while both agent contract documents omitted it.
#
# The pair is checked SEPARATELY rather than together, because the bilingual
# rule of this repository is that neither language may carry less technical
# content than the other. Searching both at once would rebuild the same OR one
# level down and let a Portuguese-only gap pass as English coverage.
#
# The universe stays the live binary's help. `docs/AGENTS.md` is the human
# mirror of the contract the embedded skill states, so assertion 11 and this one
# together mean a global flag reaches the agent by both routes or neither.
contract_docs=(docs/AGENTS.md docs/AGENTS.pt-BR.md)
contract_flag_gaps=0
if [[ "${#GLOBAL_FLAGS[@]}" -lt 2 ]]; then
  bad "cannot audit the agent contract without the live global flag list"
else
  for doc in "${contract_docs[@]}"; do
    if [[ ! -f "$doc" ]]; then
      bad "$doc is missing (the agent contract has no $doc side)"
      continue
    fi
    missing_flags=()
    for flag in "${GLOBAL_FLAGS[@]}"; do
      case "$flag" in --help | --version) continue ;; esac
      rg -q -F -- "$flag" "$doc" 2>/dev/null || missing_flags+=("$flag")
    done
    if [[ "${#missing_flags[@]}" -ne 0 ]]; then
      for flag in "${missing_flags[@]}"; do
        printf '  contract flag gap: %s never names %s\n' "$doc" "$flag" >&2
      done
      contract_flag_gaps=$((contract_flag_gaps + ${#missing_flags[@]}))
      bad "$doc omits live global flag(s); missing: ${missing_flags[*]:0:MAX_NAMES_IN_SUMMARY}"
    fi
  done
  if [[ "$contract_flag_gaps" -eq 0 ]]; then
    pass "every global flag appears in both agent contract documents"
  fi
fi

# ── 13. Each bilingual pair must carry the same STRUCTURE ───────────────
# `scripts/audit_bilingual_docs.sh` already compares the two languages, and it
# compares the COMMAND INVOCATIONS inside them. That catches a recipe that
# drifted and is blind to everything around it: measured on 2026-08-10,
# `docs/COOKBOOK.md` carried 189 headings against 165 in the Portuguese file
# with every invocation matching, so twenty-four sections of explanation existed
# in one language only while the bilingual audit stayed green.
#
# Heading count is a proxy, not a proof of translation quality — it cannot see a
# section that was translated badly. It is chosen because it is the cheapest
# signal that survives translation: titles change words, structure does not.
#
# The pair list is DISCOVERED, never hand-kept, so a document added tomorrow is
# audited without editing this gate. Files with no `.pt-BR.md` sibling are
# skipped rather than failed: `docs/schemas/README.md` is deliberately a single
# bilingual file and says so in its own header.
heading_drift=0
while IFS= read -r en_doc; do
  [[ -z "$en_doc" ]] && continue
  case "$en_doc" in *.pt-BR.md) continue ;; esac
  pt_doc="${en_doc%.md}.pt-BR.md"
  [[ -f "$pt_doc" ]] || continue
  en_h="$(rg -c '^#{1,3} ' "$en_doc" 2>/dev/null || echo 0)"
  pt_h="$(rg -c '^#{1,3} ' "$pt_doc" 2>/dev/null || echo 0)"
  if [[ "$en_h" != "$pt_h" ]]; then
    printf '  bilingual drift: %s has %s headings, %s has %s\n' \
      "$en_doc" "$en_h" "$pt_doc" "$pt_h" >&2
    heading_drift=$((heading_drift + 1))
  fi
done < <({
  fd -e md . . -d 1 2>/dev/null
  fd -e md . docs/ 2>/dev/null
} | sort -u)
if [[ "$heading_drift" -ne 0 ]]; then
  bad "$heading_drift bilingual pair(s) diverge in section structure"
else
  pass "every bilingual pair carries the same section structure"
fi

# ── 14. The embedded skill PAIR must carry the same structure ───────────
# Assertion 13 discovers its universe through the `X.md` / `X.pt-BR.md` sibling
# convention. The embedded skills express the same bilingual invariant through a
# different convention entirely: two sibling DIRECTORIES, `-en` and `-pt`, whose
# files share a relative path. Assertion 13 cannot see them, and no other
# assertion compares the two skills to each other.
#
# Measured on 2026-08-10: the English SKILL.md carried 54 headings against 53 in
# the Portuguese one. The section `Critical step one-liners`, eight ready-made
# `run` step payloads, existed in English only, while assertions 11 and 13 both
# reported green — 11 because it audits each skill against the binary and never
# against its sibling, 13 because the directory convention is invisible to it.
#
# A missing FILE is failed rather than skipped here, unlike assertion 13: a
# reference that exists in one language only is a whole document of instruction
# the other language never receives.
skill_pair_drift=0
en_skill="skills/browser-automation-cli-en"
pt_skill="skills/browser-automation-cli-pt"
if [[ ! -d "$en_skill" || ! -d "$pt_skill" ]]; then
  bad "the embedded skill pair is incomplete ($en_skill / $pt_skill)"
else
  while IFS= read -r en_file; do
    [[ -z "$en_file" ]] && continue
    rel="${en_file#"$en_skill"/}"
    pt_file="$pt_skill/$rel"
    if [[ ! -f "$pt_file" ]]; then
      printf '  skill pair gap: %s has no counterpart at %s\n' "$en_file" "$pt_file" >&2
      skill_pair_drift=$((skill_pair_drift + 1))
      continue
    fi
    en_h="$(rg -c '^#{1,3} ' "$en_file" 2>/dev/null || echo 0)"
    pt_h="$(rg -c '^#{1,3} ' "$pt_file" 2>/dev/null || echo 0)"
    if [[ "$en_h" != "$pt_h" ]]; then
      printf '  skill pair drift: %s has %s headings, %s has %s\n' \
        "$en_file" "$en_h" "$pt_file" "$pt_h" >&2
      skill_pair_drift=$((skill_pair_drift + 1))
    fi
  done < <(fd -e md . "$en_skill" 2>/dev/null | sort)
  if [[ "$skill_pair_drift" -ne 0 ]]; then
    bad "$skill_pair_drift skill file(s) diverge between the two languages"
  else
    pass "both embedded skills carry the same file and section structure"
  fi
fi

# ── 15. Each SKILL.md must honour the packaging contract ────────────────
# The assertions above prove the skills describe the binary correctly. None of
# them proves the skills are still LOADABLE as skills. Four properties decide
# that, and every one of them fails silently when it breaks:
#
#   * the word ceiling — a skill past it is truncated, and the truncation takes
#     the end of the file, which is where the formulas live
#   * the description length — the field that decides auto-activation, so a skill
#     that overflows it may simply never be selected
#   * a colon inside the description VALUE — YAML then reads the field as a map
#     and the frontmatter stops parsing as a string
#   * a fenced code block — this repository's rule is that a skill carries
#     instruction and inline references, never code
#
# Version identifiers are failed in the same pass. A skill is a consolidated
# instruction set, so a gap number or release label dates content that must read
# as the permanent contract of the tool.
SKILL_MAX_WORDS=5000
SKILL_MAX_DESC_CHARS=1024
skill_contract_gaps=0
for dir in "${SKILL_DIRS[@]}"; do
  main="$dir/SKILL.md"
  if [[ ! -f "$main" ]]; then
    bad "$main is missing"
    skill_contract_gaps=$((skill_contract_gaps + 1))
    continue
  fi
  words="$(wc -w <"$main")"
  if [[ "$words" -gt "$SKILL_MAX_WORDS" ]]; then
    printf '  skill contract: %s has %s words, ceiling is %s\n' \
      "$main" "$words" "$SKILL_MAX_WORDS" >&2
    skill_contract_gaps=$((skill_contract_gaps + 1))
  fi
  desc="$(rg -N --max-count 1 '^description: ' "$main" 2>/dev/null | sd '^description: ' '')"
  if [[ -z "$desc" ]]; then
    printf '  skill contract: %s declares no description\n' "$main" >&2
    skill_contract_gaps=$((skill_contract_gaps + 1))
  else
    if [[ "${#desc}" -gt "$SKILL_MAX_DESC_CHARS" ]]; then
      printf '  skill contract: %s description is %s chars, ceiling is %s\n' \
        "$main" "${#desc}" "$SKILL_MAX_DESC_CHARS" >&2
      skill_contract_gaps=$((skill_contract_gaps + 1))
    fi
    if [[ "$desc" == *:* ]]; then
      printf '  skill contract: %s description value contains a colon\n' "$main" >&2
      skill_contract_gaps=$((skill_contract_gaps + 1))
    fi
  fi
  while IFS= read -r sfile; do
    [[ -z "$sfile" ]] && continue
    if rg -q '^```' "$sfile" 2>/dev/null; then
      printf '  skill contract: %s carries a fenced code block\n' "$sfile" >&2
      skill_contract_gaps=$((skill_contract_gaps + 1))
    fi
    if rg -q 'GAP-[0-9]' "$sfile" 2>/dev/null; then
      printf '  skill contract: %s cites a version-specific gap identifier\n' "$sfile" >&2
      skill_contract_gaps=$((skill_contract_gaps + 1))
    fi
  done < <(fd -e md . "$dir" 2>/dev/null)
done
if [[ "$skill_contract_gaps" -ne 0 ]]; then
  bad "$skill_contract_gaps skill packaging contract violation(s)"
else
  pass "both embedded skills honour the packaging contract"
fi

if [[ "$fail" -ne 0 ]]; then
  echo "doc-coverage-check: FAIL"
  exit 1
fi
echo "doc-coverage-check: OK"
