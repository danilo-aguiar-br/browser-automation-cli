#!/usr/bin/env bash
# Local inventory gate (agent-native): live command count + docs honesty.
#
# WHY THIS FILE IS NAMED `*-check.sh`
#   `scripts/ci-check.sh` auto-discovers verifiers with the glob
#   `scripts/*-check.sh`. The previous name, `verify-inventory-flat.sh`, does
#   NOT match that glob, so this gate never ran automatically and 18 markdown
#   files drifted to a stale count of 67 with the runner reporting green.
#   `verify-inventory-flat.sh` is kept as a thin shim for existing docs and
#   muscle memory; it delegates here.
#
# WHY THE COVERAGE IS WIDER THAN THE OLD SCRIPT
#   The old anti-stale regex only visited `docs/AGENTS.md*`. Meanwhile
#   `docs/HOW_TO_USE.pt-BR.md` literally contained `Inventário Completo de
#   Comandos (67)` — a string the regex WOULD have matched, in a file the loop
#   never opened. A gate that checks two files and claims to protect the
#   inventory is a false green by construction.
#
# WHY STALE_COUNT TRACKS EXPECTED-1 (not a frozen 67)
#   After `record` landed, live inventory became 69. Docs still claiming 68
#   passed because STALE_COUNT stayed at 67. That is the same false-green
#   family: gate scope narrower than the tip claim. STALE_COUNT must be the
#   immediately previous inventory size.
#
# CLEAN STDOUT: one status line on stdout; diagnostics on stderr.
set -euo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting (`--smart-case` widens matches, `--max-columns`
# truncates them away). Clearing the variable neutralizes the whole file; `-s`
# would close only one of those doors.
export RIPGREP_CONFIG_PATH=
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="${BIN:-}"
if [[ -z "$BIN" ]]; then
  if [[ -x "$ROOT/target/debug/browser-automation-cli" ]]; then
    BIN="$ROOT/target/debug/browser-automation-cli"
  elif [[ -x "$ROOT/target/release/browser-automation-cli" ]]; then
    BIN="$ROOT/target/release/browser-automation-cli"
  else
    echo "inventory-flat-check: FAIL (bin missing; build debug or release)" >&2
    echo "inventory-flat-check: FAIL"
    exit 1
  fi
fi

# Bump both when the agent inventory grows.
#   EXPECTED       = live SSOT, `commands --json | .data.commands | length`
#   EXPECTED_CLAP  = EXPECTED minus the surfaces reachable only through `exec`
#                    (`select-option`, `pick`), which is what docs call the
#                    "clap product surface".
#   STALE_COUNTS   = every previous tip size the docs must not claim
#
# WHY STALE_COUNTS IS A LIST AND NOT `EXPECTED - 1`
#   That formula assumed the inventory grows one name at a time. On 2026-08-28
#   it grew by TWO (`sitemap`, `feed`), so `EXPECTED - 1` became 70 — a size the
#   product never had — while ~90 doc sites still claimed 69 and would have
#   passed in silence. That is the exact false-green family this header already
#   describes twice; the formula reproduced it the first time it was tested by
#   a jump larger than one.
EXPECTED=71
EXPECTED_CLAP=69
STALE_COUNTS=(69 70)
# Interpolated ~35 times inside `stale_re`; a regex alternation keeps every one
# of those call sites unchanged.
STALE_COUNT="($(IFS='|'; printf '%s' "${STALE_COUNTS[*]}"))"
fail=0

if ! command -v jaq >/dev/null 2>&1; then
  echo "inventory-flat-check: FAIL (need jaq)" >&2
  echo "inventory-flat-check: FAIL"
  exit 1
fi

json="$("$BIN" --json commands 2>/dev/null || true)"
if [[ -z "$json" ]]; then
  echo "inventory-flat-check: FAIL (commands --json empty)" >&2
  echo "inventory-flat-check: FAIL"
  exit 1
fi

# `jaq` is the only JSON reader in this gate. The second branch that used to sit
# here parsed the same envelope in another language; it went away with the rest
# of the Python surface, because a gate that reaches for an interpreter the
# product does not ship fails on any host without it.
count="$(printf '%s' "$json" | jaq -r '.data.commands | length' 2>/dev/null || echo 0)"
has_image="$(printf '%s' "$json" | jaq -r '[.data.commands[] | (if type=="object" then .name else . end)] | any(.=="image")' 2>/dev/null || echo false)"
has_video="$(printf '%s' "$json" | jaq -r '[.data.commands[] | (if type=="object" then .name else . end)] | any(.=="video")' 2>/dev/null || echo false)"
has_audio="$(printf '%s' "$json" | jaq -r '[.data.commands[] | (if type=="object" then .name else . end)] | any(.=="audio")' 2>/dev/null || echo false)"
has_record="$(printf '%s' "$json" | jaq -r '[.data.commands[] | (if type=="object" then .name else . end)] | any(.=="record")' 2>/dev/null || echo false)"

if [[ "${count}" != "$EXPECTED" ]]; then
  echo "inventory-flat-check: FAIL (commands count=${count} expected=${EXPECTED})" >&2
  fail=1
fi
for pair in "image:${has_image}" "video:${has_video}" "audio:${has_audio}" "record:${has_record}"; do
  name="${pair%%:*}"
  present="${pair##*:}"
  if [[ "${present}" != "true" ]]; then
    echo "inventory-flat-check: FAIL (missing ${name} in commands)" >&2
    fail=1
  fi
done

# Flat blocks: "hover image keys" without video is a regression.
if command -v rg >/dev/null 2>&1; then
  if rg -n --glob '*.md' 'hover image keys' docs/ 2>/dev/null | rg -v 'hover image video keys' >/dev/null 2>&1; then
    echo "inventory-flat-check: FAIL (docs flat 'hover image keys' without video)" >&2
    rg -n --glob '*.md' 'hover image keys' docs/ 2>/dev/null | rg -v 'hover image video keys' >&2 || true
    fail=1
  fi
fi

# README must name record + live inventory count (not only video/audio).
for readme in README.md README.pt-BR.md; do
  [[ -f "$readme" ]] || continue
  for name in video audio record; do
    if ! rg -q "\`${name}\`" "$readme" 2>/dev/null; then
      echo "inventory-flat-check: FAIL (${readme} missing \`${name}\`)" >&2
      fail=1
    fi
  done
  if ! rg -q "\\*\\*${EXPECTED}\\*\\*" "$readme" 2>/dev/null; then
    echo "inventory-flat-check: FAIL (${readme} missing inventory **${EXPECTED}**)" >&2
    fail=1
  fi
done

# Agent playbooks must name media + record and carry the live count.
for agents in docs/AGENTS.md docs/AGENTS.pt-BR.md; do
  [[ -f "$agents" ]] || continue
  for name in audio record; do
    if ! rg -q "\`${name}\`" "$agents" 2>/dev/null; then
      echo "inventory-flat-check: FAIL (${agents} missing \`${name}\`)" >&2
      fail=1
    fi
  done
  if ! rg -q "\\*\\*${EXPECTED}\\*\\*" "$agents" 2>/dev/null; then
    echo "inventory-flat-check: FAIL (${agents} missing inventory **${EXPECTED}**)" >&2
    fail=1
  fi
done

# ── Anti-stale sweep across EVERY doc that states an inventory count ─────────
# Widened after auditoria-e2e-10 and again after NC-DOCS-TIP-68-POS-RECORD:
# README, ARCHITECTURE, and llms* claimed 68 while the gate never opened them.
STALE_TARGETS=(
  README.md
  README.pt-BR.md
  docs/AGENTS.md
  docs/AGENTS.pt-BR.md
  docs/ARCHITECTURE.md
  docs/ARCHITECTURE.pt-BR.md
  docs/HOW_TO_USE.md
  docs/HOW_TO_USE.pt-BR.md
  docs/schemas/README.md
  docs/TESTING.md
  docs/TESTING.pt-BR.md
  docs/ROADMAP.md
  docs/ROADMAP.pt-BR.md
  docs/MIGRATION.md
  docs/MIGRATION.pt-BR.md
  docs/CROSS_PLATFORM.md
  docs/CROSS_PLATFORM.pt-BR.md
  docs/COOKBOOK.md
  docs/COOKBOOK.pt-BR.md
  llms.txt
  llms-full.txt
  llms.pt-BR.txt
  llms-full.pt-BR.txt
  CONTRIBUTING.md
  CONTRIBUTING.pt-BR.md
  INTEGRATIONS.md
  INTEGRATIONS.pt-BR.md
  skills/browser-automation-cli-en/SKILL.md
  skills/browser-automation-cli-pt/SKILL.md
)
stale_re="inventory \\*\\*${STALE_COUNT}\\*\\*|inventário \\*\\*${STALE_COUNT}\\*\\*|Full inventory \\(\\*\\*${STALE_COUNT}|Inventário completo \\(\\*\\*${STALE_COUNT}|\\(\\*\\*${STALE_COUNT}\\*\\* agent|\\(\\*\\*${STALE_COUNT} nomes|\\(\\*\\*${STALE_COUNT} nomes de agente|inventário vivo \\(\\*\\*${STALE_COUNT}|live inventory \\(\\*\\*${STALE_COUNT}|Inventory \\(${STALE_COUNT}\\)|Full Command Inventory \\(${STALE_COUNT}\\)|Full agent inventory \\(${STALE_COUNT}\\)|Inventário Completo de Comandos \\(${STALE_COUNT}\\)|Inventário completo de agente \\(${STALE_COUNT}\\)|Command Inventory \\(${STALE_COUNT}\\)|Command input schemas \\(${STALE_COUNT}|schemas \\(${STALE_COUNT}\\)|${STALE_COUNT} command schemas|${STALE_COUNT} schemas|lists \\*\\*${STALE_COUNT}\\*\\*|lista \\*\\*${STALE_COUNT}\\*\\*|inventory tip \\(Unreleased\\): \\*\\*${STALE_COUNT}\\*\\*|Inventory tip: \\*\\*${STALE_COUNT}\\*\\*|Inventário tip: \\*\\*${STALE_COUNT}\\*\\*|inventário \\(tip Unreleased \\*\\*${STALE_COUNT}\\*\\*|tip Unreleased \\*\\*${STALE_COUNT}\\*\\*|inventory tip Unreleased \\*\\*${STALE_COUNT}\\*\\*|inventário tip Unreleased \\*\\*${STALE_COUNT}\\*\\*|MUST recognize all ${STALE_COUNT}|DEVE conhecer estes ${STALE_COUNT}|\\(inventory ${STALE_COUNT}\\)|\\(inventário ${STALE_COUNT}\\)|inventory ${STALE_COUNT}\\)|inventário ${STALE_COUNT}\\)|commands --json\` \\(${STALE_COUNT}\\)|commands --json \\(${STALE_COUNT}\\)|length'  # ${STALE_COUNT}"
for doc in "${STALE_TARGETS[@]}"; do
  [[ -f "$doc" ]] || continue
  if rg -qn "$stale_re" "$doc" 2>/dev/null; then
    echo "inventory-flat-check: FAIL (${doc} still claims inventory ${STALE_COUNT})" >&2
    rg -n "$stale_re" "$doc" 2>/dev/null >&2 || true
    fail=1
  fi
done

# ── Structural sweep on the four tip-claim files ────────────────────────────
#
# WHY THIS EXISTS ALONGSIDE `stale_re`
#   `stale_re` enumerates roughly thirty-five PHRASES, so it only catches a
#   stale count written the way someone already wrote it once. On 2026-08-28
#   `CONTRIBUTING.pt-BR.md` carried a bold **69** inside a parenthetical about
#   0.1.7 history, in a sentence no phrase in that alternation describes, and
#   the gate passed it while its English twin wrote the same 69 unbolded.
#
# WHY IT IS SCOPED TO FOUR FILES AND TO THE INVENTORY LINE
#   The first cut swept every STALE_TARGET and produced fifteen false
#   positives, because 69 is BOTH a stale inventory size and the CURRENT clap
#   surface: `clap product surface is **69**` is correct and must stay. The
#   number alone cannot say which quantity it counts, so the check is limited
#   to the line that names the inventory, in the four files that already carry
#   the tip-claim assertions above. Everything wider needs the two quantities
#   to stop sharing a value, which is not something a gate can arrange.
for doc in CONTRIBUTING.md CONTRIBUTING.pt-BR.md INTEGRATIONS.md INTEGRATIONS.pt-BR.md; do
  [[ -f "$doc" ]] || continue
  if bold_stale="$(rg -n "^.*[Ii]nvent(ory|ário|ario).*\*\*${STALE_COUNT}\*\*.*$" "$doc" 2>/dev/null)"; then
    echo "inventory-flat-check: FAIL (${doc} bolds a stale inventory size ${STALE_COUNT} on an inventory line)" >&2
    printf '%s\n' "$bold_stale" | head -n 3 >&2
    fail=1
  fi
done

# ── Clap product surface must equal the live value, in every document ──────────
#
# REWRITTEN 2026-08-28. The rule used to ENUMERATE stale values; it now COMPARES
# against `EXPECTED_CLAP`. Three defects drove that change, and each was
# invisible for its own reason.
#
#   SCOPE — the loop visited four files, and the comment that stood here said so:
#   "(HOW_TO + ARCHITECTURE)". `docs/COOKBOOK.md:1469` carried `Clap product
#   surface is **66** names`, 66 being the very value the old alternation spelled
#   out as stale in three places, and the gate reported OK because it never
#   opened the file. `doc-coverage-check.sh` missed it too, so the line sat
#   outside BOTH counting gates rather than only one.
#
#   RANGE BELOW — `6[0-6]` stops at 66, so **67** and **68** fell through even in
#   a file that WAS in scope. `docs/MIGRATION.md:258` said `Tip clap product
#   surface is **67** names` against a live 69: not a stale label but a FALSE
#   number, in the only gap between the known-obsolete value and the right one.
#
#   RANGE ABOVE — an enumeration of values BELOW the target can never catch a
#   claim above it. `clap product surface is **71**` is just as false as **66**,
#   because 71 is the AGENT inventory and 69 is the clap surface, and the two
#   sharing a document is exactly how they get swapped. Equality closes both
#   directions at once and needs no maintenance when the number moves.
#
# THERE IS DELIBERATELY NO HISTORICAL EXEMPTION. An earlier cut skipped any line
# carrying a `0.1.` marker, which would have skipped `Tip 0.1.9 clap product
# surface is **69**` — the single most important line to check. Present tense is
# the signal instead: a sentence that says the surface IS a number is claiming
# the tip and must match it, while a historical note says "listed" or "was" and
# never matches this phrase. Measured across twenty-five documents: twenty lines
# match, all twenty say 69, zero exemptions needed.
CLAP_PHRASE_RE="([Cc]lap product surface is"
CLAP_PHRASE_RE+="|[Cc]lap product subcommand count is"
CLAP_PHRASE_RE+="|[Ss]uperfície clap de produto[^*]{0,28}"
CLAP_PHRASE_RE+="|subcomandos clap de produto é) ?\\*\\*([0-9]+)\\*\\*"
for doc in "${STALE_TARGETS[@]}"; do
  [[ -f "$doc" ]] || continue
  while IFS= read -r hit; do
    [[ -z "$hit" ]] && continue
    claimed="$(printf '%s' "$hit" | rg -o "$CLAP_PHRASE_RE" -r '$2' | head -n 1)"
    if [[ -n "$claimed" && "$claimed" != "$EXPECTED_CLAP" ]]; then
      echo "inventory-flat-check: FAIL (${doc} claims clap surface ${claimed}, live value is ${EXPECTED_CLAP})" >&2
      printf '%s\n' "$hit" >&2
      fail=1
    fi
  done < <(rg -n "$CLAP_PHRASE_RE" "$doc" 2>/dev/null || true)
done
# ── Agent-facing surfaces must name record + live count (contrib/skills/integrations) ──
# Phrase-family membership: gate scope must cover every tip claim surface, not only README.
for doc in CONTRIBUTING.md CONTRIBUTING.pt-BR.md INTEGRATIONS.md INTEGRATIONS.pt-BR.md; do
  [[ -f "$doc" ]] || continue
  if ! rg -q "\brecord\b" "$doc" 2>/dev/null; then
    echo "inventory-flat-check: FAIL (${doc} missing record in inventory tip path)" >&2
    fail=1
  fi
  if ! rg -q "\*\*${EXPECTED}\*\*" "$doc" 2>/dev/null; then
    echo "inventory-flat-check: FAIL (${doc} missing inventory **${EXPECTED}**)" >&2
    fail=1
  fi
done
for skill in skills/browser-automation-cli-en/SKILL.md skills/browser-automation-cli-pt/SKILL.md; do
  [[ -f "$skill" ]] || continue
  if ! rg -q "\brecord\b" "$skill" 2>/dev/null; then
    echo "inventory-flat-check: FAIL (${skill} missing record in command list)" >&2
    fail=1
  fi
  if ! rg -q "all ${EXPECTED}|estes ${EXPECTED}|\*\*${EXPECTED}\*\*" "$skill" 2>/dev/null; then
    echo "inventory-flat-check: FAIL (${skill} missing live count ${EXPECTED})" >&2
    fail=1
  fi
done

# ── Skills set-equality vs live commands (blocks CSV missing a name while count phrase stays) ──
# Ported from Python to jaq + rg + bash on 2026-08-18. Same assertions, same
# report shape: live inventory must be EXPECTED distinct names, each SKILL.md
# must carry the claim line, and its CSV must be set-equal to the live names
# with `record` present. The port removes the interpreter, not a single check.
render_list() {
  # Render a name list the way the old report did: [] or ['a', 'b'].
  local out="" item
  for item in "$@"; do
    [[ -n "$out" ]] && out+=", "
    out+="'${item}'"
  done
  printf '[%s]' "$out"
}

# `mapfile` is a bash 4 builtin and macOS ships bash 3.2, so every array read
# in this file uses the portable read loop below instead. bash 3.2 also aborts
# on "${arr[@]}" of an empty array under `set -u`, hence the `[@]+` guards on
# every expansion that can legitimately see zero elements (2026-09-04).
live_names=()
while IFS= read -r __line; do live_names+=("$__line"); done < <(
  printf '%s' "$json" |
    jaq -r '.data.commands[] | (if type=="object" then (.name // "") else tostring end)' 2>/dev/null |
    rg -v '^\s*$' || true
)
live_uniq_n="$(printf '%s\n' "${live_names[@]+"${live_names[@]}"}" | LC_ALL=C sort -u | rg -c '^' || echo 0)"

if [[ "${#live_names[@]}" -ne "$EXPECTED" || "$live_uniq_n" -ne "$EXPECTED" ]]; then
  echo "inventory-flat-check: FAIL (skills set-eq vs live):" >&2
  echo "live_invalid n=${#live_names[@]} uniq=${live_uniq_n} expected=${EXPECTED}" >&2
  fail=1
else
  live_sorted="$(printf '%s\n' "${live_names[@]}" | LC_ALL=C sort -u)"
  skill_eq_report=""
  skill_eq_fail=0
  for path in skills/browser-automation-cli-en/SKILL.md skills/browser-automation-cli-pt/SKILL.md; do
    if [[ ! -f "$path" ]]; then
      skill_eq_report+="${path}: read_error=no such file"$'\n'
      skill_eq_fail=1
      continue
    fi
    claim_line="$(
      rg -N -o -m1 '(?:MUST recognize all|DEVE conhecer estes)\s+\d+\s*[—\-]\s*.+$' "$path" 2>/dev/null || true
    )"
    if [[ -z "$claim_line" ]]; then
      skill_eq_report+="${path}: NO_SKILL_LINE"$'\n'
      skill_eq_fail=1
      continue
    fi
    # ALTERNATION, NOT A BRACKET (measured 2026-08-26)
    #   This was `[—-]`, and a bracket expression matches BYTES. The em-dash is
    #   three bytes (E2 80 94), so under the `LC_ALL=C` that ci-check.sh exports
    #   the class consumed only the FIRST byte and the other two fell into the
    #   capture: the pt-BR list came back as `\x80\x94 doctor, commands` and the
    #   gate reported `missing=['doctor'] extra=['?? doctor']` against a
    #   perfectly valid UTF-8 document.
    #
    #   The en file separates with an ASCII hyphen and the pt file with an
    #   em-dash, so ONLY pt failed — and only inside ci-check. Run on its own,
    #   under a UTF-8 locale, this verifier passed. A verdict that flips with
    #   the caller's locale measures the caller, not the tree.
    #
    #   `(—|-)` matches the character as a unit in both locales; it costs one
    #   capture group, so the CSV moved from index 3 to index 4.
    if [[ ! "$claim_line" =~ (MUST\ recognize\ all|DEVE\ conhecer\ estes)[[:space:]]+([0-9]+)[[:space:]]*(—|-)[[:space:]]*(.+)$ ]]; then
      skill_eq_report+="${path}: NO_SKILL_LINE"$'\n'
      skill_eq_fail=1
      continue
    fi
    claim="${BASH_REMATCH[2]}"
    csv="${BASH_REMATCH[4]}"
    names=()
    IFS=',' read -r -a raw_names <<<"$csv"
    for tok in "${raw_names[@]}"; do
      tok="${tok#"${tok%%[![:space:]]*}"}"
      tok="${tok%"${tok##*[![:space:]]}"}"
      [[ -n "$tok" ]] && names+=("$tok")
    done
    skill_sorted="$(printf '%s\n' "${names[@]}" | LC_ALL=C sort -u)"
    uniq_n="$(printf '%s\n' "$skill_sorted" | rg -c '^' || echo 0)"
    missing=()
    while IFS= read -r __line; do missing+=("$__line"); done < <(comm -23 <(printf '%s\n' "$live_sorted") <(printf '%s\n' "$skill_sorted"))
    extra=()
    while IFS= read -r __line; do extra+=("$__line"); done < <(comm -13 <(printf '%s\n' "$live_sorted") <(printf '%s\n' "$skill_sorted"))
    has_rec=False
    if printf '%s\n' "${names[@]}" | rg -qx 'record'; then has_rec=True; fi
    skill_eq_report+="${path}: claim=${claim} n=${#names[@]} uniq=${uniq_n} missing=$(render_list "${missing[@]+"${missing[@]}"}") extra=$(render_list "${extra[@]+"${extra[@]}"}") record=${has_rec}"$'\n'
    if [[ "$claim" -ne "$EXPECTED" || "${#names[@]}" -ne "$EXPECTED" || "$uniq_n" -ne "$EXPECTED" ||
      "${#missing[@]}" -ne 0 || "${#extra[@]}" -ne 0 || "$has_rec" != "True" ]]; then
      skill_eq_fail=1
    fi
  done
  if [[ "$skill_eq_fail" -ne 0 ]]; then
    echo "inventory-flat-check: FAIL (skills set-eq vs live):" >&2
    printf '%s' "$skill_eq_report" >&2
    fail=1
  fi
fi

# ── llms* flat list: cardinality + uniqueness (blocks record,record false-green) ──
# Ported from Python to rg + bash on 2026-08-18. Same four assertions on the
# flat inventory line: EXPECTED tokens, EXPECTED distinct tokens, no duplicate,
# `record` present. A file without the line still SKIPS, as before.
for llms in llms.txt llms-full.txt llms.pt-BR.txt llms-full.pt-BR.txt; do
  [[ -f "$llms" ]] || continue
  body="$(
    rg -N -o -i -m1 -r '$1' '(?:Full inventory|Inventário completo)[^\n]*?:\s*(.+)' "$llms" 2>/dev/null || true
  )"
  if [[ -z "$body" ]]; then
    # NO_INVENTORY_LINE — the old comparator exited 0 here, and so does this one.
    continue
  fi
  tokens=()
  IFS=',' read -r -a raw_tokens <<<"$body"
  for tok in "${raw_tokens[@]}"; do
    tok="${tok#"${tok%%[![:space:]]*}"}"
    tok="${tok%"${tok##*[![:space:]]}"}"
    [[ -n "$tok" ]] && tokens+=("$tok")
  done
  if [[ "${#tokens[@]}" -gt 0 ]]; then
    last_idx=$((${#tokens[@]} - 1))
    last_tok="${tokens[$last_idx]}"
    while [[ "$last_tok" == *. ]]; do last_tok="${last_tok%.}"; done
    tokens[$last_idx]="$last_tok"
  fi
  n="${#tokens[@]}"
  uniq="$(printf '%s\n' "${tokens[@]}" | LC_ALL=C sort -u | rg -c '^' || echo 0)"
  dups=()
  while IFS= read -r __line; do dups+=("$__line"); done < <(printf '%s\n' "${tokens[@]}" | LC_ALL=C sort | uniq -d)
  has_record=False
  if printf '%s\n' "${tokens[@]}" | rg -qx 'record'; then has_record=True; fi
  if [[ "$n" -ne "$EXPECTED" || "$uniq" -ne "$EXPECTED" || "$has_record" != "True" || "${#dups[@]}" -ne 0 ]]; then
    echo "inventory-flat-check: FAIL (${llms} flat list invalid: n=${n};uniq=${uniq};dups=$(render_list "${dups[@]+"${dups[@]}"}");record=${has_record})" >&2
    fail=1
  fi
done


# ── Capability parity: what AGENTS teaches, HOW_TO_USE must also teach ──────
PARITY_MARKERS=(
  'run --script -'
  'video[^`]*\|manifest|video manifest'
  'record'
)
for marker in "${PARITY_MARKERS[@]}"; do
  for doc in docs/AGENTS.md docs/AGENTS.pt-BR.md docs/HOW_TO_USE.md docs/HOW_TO_USE.pt-BR.md; do
    [[ -f "$doc" ]] || continue
    if ! rg -q -- "$marker" "$doc" 2>/dev/null; then
      echo "inventory-flat-check: FAIL (${doc} never documents /${marker}/)" >&2
      fail=1
    fi
  done
done

# ── Negative sweep: the excised OCR surface must not be TAUGHT again ─────────
EXCISED_RE='(image ocr|--engine +ocrs|--ocr-lang|config set +(ocr_engine|ocr_lang|tesseract_path)|`(ocr_engine|ocr_lang|tesseract_path)`)'
doc_targets=()
while IFS= read -r __line; do doc_targets+=("$__line"); done < <(
  fd -e md -e txt --max-depth 2 . README.md docs/ skills/ 2>/dev/null
  fd -e md -e txt --max-depth 1 . . 2>/dev/null
)
for doc in "${doc_targets[@]+"${doc_targets[@]}"}"; do
  [[ -f "$doc" ]] || continue
  case "$doc" in
    */CHANGELOG*.md | CHANGELOG*.md | ./CHANGELOG*.md) continue ;;
    *gaps.md) continue ;;
  esac
  offenders="$(
    rg -ni "$EXCISED_RE" "$doc" 2>/dev/null |
      rg -vi 'remov|excis|deleted|dropped|no longer|deixou de|foi remov|sem ação|no ocr|not exist' || true
  )"
  if [[ -n "$offenders" ]]; then
    echo "inventory-flat-check: FAIL (${doc} still teaches the excised OCR surface)" >&2
    printf '%s\n' "$offenders" | head -n 3 >&2
    fail=1
  fi
done

if [[ "$fail" -ne 0 ]]; then
  echo "inventory-flat-check: FAIL"
  exit 1
fi
echo "inventory-flat-check: OK (commands=${EXPECTED} clap=${EXPECTED_CLAP} image+video+audio+record; parity+excision honest)"
exit 0
