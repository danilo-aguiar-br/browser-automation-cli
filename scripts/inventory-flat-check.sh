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
#   STALE_COUNT    = EXPECTED - 1 (previous tip size the docs must not claim)
EXPECTED=69
EXPECTED_CLAP=67
STALE_COUNT=$((EXPECTED - 1))
fail=0

if ! command -v jaq >/dev/null 2>&1 && ! command -v python3 >/dev/null 2>&1; then
  echo "inventory-flat-check: FAIL (need jaq or python3)" >&2
  echo "inventory-flat-check: FAIL"
  exit 1
fi

json="$("$BIN" --json commands 2>/dev/null || true)"
if [[ -z "$json" ]]; then
  echo "inventory-flat-check: FAIL (commands --json empty)" >&2
  echo "inventory-flat-check: FAIL"
  exit 1
fi

if command -v jaq >/dev/null 2>&1; then
  count="$(printf '%s' "$json" | jaq -r '.data.commands | length' 2>/dev/null || echo 0)"
  has_image="$(printf '%s' "$json" | jaq -r '[.data.commands[] | (if type=="object" then .name else . end)] | any(.=="image")' 2>/dev/null || echo false)"
  has_video="$(printf '%s' "$json" | jaq -r '[.data.commands[] | (if type=="object" then .name else . end)] | any(.=="video")' 2>/dev/null || echo false)"
  has_audio="$(printf '%s' "$json" | jaq -r '[.data.commands[] | (if type=="object" then .name else . end)] | any(.=="audio")' 2>/dev/null || echo false)"
  has_record="$(printf '%s' "$json" | jaq -r '[.data.commands[] | (if type=="object" then .name else . end)] | any(.=="record")' 2>/dev/null || echo false)"
else
  eval "$(printf '%s' "$json" | python3 -c 'import sys,json; d=json.load(sys.stdin); cmds=d.get("data",{}).get("commands",[]); names=[c.get("name") if isinstance(c,dict) else c for c in cmds]; print(f"count={len(names)}"); print(f"has_image={\"true\" if \"image\" in names else \"false\"}"); print(f"has_video={\"true\" if \"video\" in names else \"false\"}"); print(f"has_audio={\"true\" if \"audio\" in names else \"false\"}"); print(f"has_record={\"true\" if \"record\" in names else \"false\"}")')"
fi

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

# Clap product surface must not lag the inventory either (HOW_TO + ARCHITECTURE).
for doc in docs/HOW_TO_USE.md docs/HOW_TO_USE.pt-BR.md docs/ARCHITECTURE.md docs/ARCHITECTURE.pt-BR.md; do
  [[ -f "$doc" ]] || continue
  if rg -qn "[Cc]lap product surface is \\*\\*(6[0-6]|[1-5][0-9])\\*\\*|superfície clap.*\\*\\*(6[0-6]|[1-5][0-9])\\*\\*|clap product surface is ${EXPECTED_CLAP}|subcommand count is 66|subcomandos clap de produto é \\*\\*66\\*\\*|product subcommand count is 66" "$doc" 2>/dev/null; then
    echo "inventory-flat-check: FAIL (${doc} clap surface below ${EXPECTED_CLAP} or stale 66)" >&2
    rg -n "[Cc]lap product surface is \\*\\*(6[0-6]|[1-5][0-9])\\*\\*|superfície clap.*\\*\\*(6[0-6]|[1-5][0-9])\\*\\*|subcommand count is 66|subcomandos clap de produto é \\*\\*66\\*\\*|product subcommand count is 66" "$doc" 2>/dev/null >&2 || true
    fail=1
  fi
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
if command -v python3 >/dev/null 2>&1; then
  set +e
  skill_eq_out="$(
    EXPECTED_N="$EXPECTED" BIN_PATH="$BIN" python3 - <<'PY'
import json, os, re, subprocess, sys
expected = int(os.environ["EXPECTED_N"])
bin_path = os.environ["BIN_PATH"]
raw = subprocess.check_output([bin_path, "--json", "commands"], text=True)
data = json.loads(raw)
cmds = data.get("data", {}).get("commands", [])
live = []
for c in cmds:
    if isinstance(c, dict):
        live.append(c.get("name") or "")
    else:
        live.append(str(c))
live = [n for n in live if n]
live_set = set(live)
if len(live) != expected or len(live_set) != expected:
    print(f"live_invalid n={len(live)} uniq={len(live_set)} expected={expected}")
    sys.exit(2)
skills = [
    "skills/browser-automation-cli-en/SKILL.md",
    "skills/browser-automation-cli-pt/SKILL.md",
]
pat = re.compile(
    r"(?:MUST recognize all|DEVE conhecer estes)\s+(\d+)\s*[—\-]\s*(.+)$",
    re.M,
)
fail = 0
for path in skills:
    try:
        text = open(path, encoding="utf-8", errors="replace").read()
    except OSError as e:
        print(f"{path}: read_error={e}")
        fail = 1
        continue
    m = pat.search(text)
    if not m:
        print(f"{path}: NO_SKILL_LINE")
        fail = 1
        continue
    claim = int(m.group(1))
    names = [t.strip() for t in m.group(2).split(",") if t.strip()]
    sset = set(names)
    missing = sorted(live_set - sset)
    extra = sorted(sset - live_set)
    ok = (
        claim == expected
        and len(names) == expected
        and len(sset) == expected
        and not missing
        and not extra
        and "record" in sset
    )
    print(
        f"{path}: claim={claim} n={len(names)} uniq={len(sset)} "
        f"missing={missing} extra={extra} record={'record' in sset}"
    )
    if not ok:
        fail = 1
sys.exit(2 if fail else 0)
PY
  )"
  skill_eq_ec=$?
  set -e
  if [[ "$skill_eq_ec" -ne 0 ]]; then
    echo "inventory-flat-check: FAIL (skills set-eq vs live):" >&2
    printf '%s\n' "$skill_eq_out" >&2
    fail=1
  fi
fi

# ── llms* flat list: cardinality + uniqueness (blocks record,record false-green) ──
if command -v python3 >/dev/null 2>&1; then
  for llms in llms.txt llms-full.txt llms.pt-BR.txt llms-full.pt-BR.txt; do
    [[ -f "$llms" ]] || continue
    set +e
    py_out="$(
      EXPECTED_N="$EXPECTED" LLMS_PATH="$llms" python3 - <<'PY'
import os, re, sys
from collections import Counter
path = os.environ["LLMS_PATH"]
expected = int(os.environ["EXPECTED_N"])
text = open(path, encoding="utf-8", errors="replace").read()
m = re.search(
    r"(?:Full inventory|Inventário completo)[^\n]*?:\s*(.+)",
    text,
    re.I,
)
if not m:
    print("NO_INVENTORY_LINE")
    sys.exit(0)
body = m.group(1).strip()
tokens = [t.strip() for t in body.split(",") if t.strip()]
if tokens:
    tokens[-1] = tokens[-1].rstrip(".")
c = Counter(tokens)
dups = sorted([k for k, v in c.items() if v > 1])
n = len(tokens)
uniq = len(c)
has_record = "record" in c
print(f"n={n};uniq={uniq};dups={dups!r};record={has_record}")
if n != expected or uniq != expected or not has_record or dups:
    sys.exit(2)
sys.exit(0)
PY
    )"
    py_ec=$?
    set -e
    if [[ "$py_ec" -ne 0 ]]; then
      echo "inventory-flat-check: FAIL (${llms} flat list invalid: ${py_out})" >&2
      fail=1
    fi
  done
fi


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
mapfile -t doc_targets < <(
  fd -e md -e txt --max-depth 2 . README.md docs/ skills/ 2>/dev/null
  fd -e md -e txt --max-depth 1 . . 2>/dev/null
)
for doc in "${doc_targets[@]}"; do
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
