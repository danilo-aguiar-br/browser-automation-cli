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

CONFIG_EN="docs/CONFIGURATION.md"
CONFIG_PT="docs/CONFIGURATION.pt-BR.md"

# ── 1. Every live XDG key must be documented, in both languages ──────────
# Read from the binary, never from a transcribed list.
mapfile -t LIVE_KEYS < <("$BIN" --json config list-keys 2>/dev/null | jaq -r '.data.keys[].key')
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
mapfile -t LIVE_CMDS < <("$BIN" --json commands 2>/dev/null | jaq -r '.data.commands[]')
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
  if printf '%s' "$text" | rg -q -i -- \
    'not a global|not global|are not global|is not global|do not pass|never pass|não é flag global|não são flag|não são global|nao e flag global|não passe|nunca passe|already taken|já eram|ja eram|per-command|por comando|would have collided|colidiria|LOCAL flag|flag LOCAL|exit `?2'; then
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
done < <(rg -n -i --glob '*.md' --glob '*.txt' \
  -g '!gaps.md' -g '!CLAUDE.md' -g '!AGENTS.md' -g '!MEMORY.md' -g '!base_firecrawl/**' \
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

if [[ "$fail" -ne 0 ]]; then
  echo "doc-coverage-check: FAIL"
  exit 1
fi
echo "doc-coverage-check: OK"
