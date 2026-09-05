#!/usr/bin/env bash
# Reconcile the PRD global-flag list against the implemented surface (GAP-023).
#
# NOT a ci-check verifier by glob, and that is deliberate: this is a GENERATOR.
# The bundle invokes it BY NAME as a fixed step in `--check` mode, so discovery
# would run it twice and the second run would assert nothing new.
#
# For every flag the PRD section 7 declares global, decide where the capability
# actually lives today:
#
#   global  present in `GlobalOpts`
#   local   present as a flag of one or more subcommands
#   xdg     present as an XDG config key (per-host, not per-invocation)
#   absent  nowhere
#
# `xdg` is not a synonym for `global`: an XDG key cannot be varied per
# invocation, which is the whole reason an agent would reach for a flag. That
# difference is the point of the table.
#
# Usage:
#   bash scripts/gen-flag-reconciliation.sh            # write the table
#   bash scripts/gen-flag-reconciliation.sh --check    # exit 1 if stale
# Exit: 0 write, 65 unusable input, 1 divergence with --check.
#
# WHY BASH AND NOT AN INTERPRETER
#   Ported from Python on 2026-08-18. The product is Rust end to end and ships
#   no interpreter; a repository tool that needs one is a tool some hosts do not
#   have. Same four buckets, same parsing rules, same output bytes.
set -euo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting (`--smart-case` widens matches, `--max-columns`
# truncates them away). Clearing the variable neutralizes the whole file; `-s`
# would close only one of those doors.
export RIPGREP_CONFIG_PATH=
export LC_ALL=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PRD="$ROOT/docs_prd/prd_browser-automation-cli.md"
OUT="$ROOT/docs_prd/flag_reconciliation.md"
GLOBAL_RS="$ROOT/src/cli/global.rs"

# RESOLVE THE BINARY LIKE THE NEIGHBOURING STEP DOES
#   This was pinned to `target/debug` with no fallback. The gate builds only
#   `--release` (`ci-check.sh`, "artefact for downstream gates"), so on a clean
#   tree the file this script needs does not exist at the moment it runs, and
#   `debug` appears only as a side effect of `cargo test` running earlier. That
#   is an undeclared ordering coupling, and it was measured failing.
#
#   `scripts/generate_command_schemas.sh` — the adjacent fixed step in the same
#   gate — already solved this: take the NEWER of the two so an iteration with
#   `cargo build` is never compared against a stale `target/release`, and let an
#   explicit `BIN=` win, because a caller naming a binary means it.
if [[ -z "${BIN:-}" ]]; then
  REL="$ROOT/target/release/browser-automation-cli"
  DBG="$ROOT/target/debug/browser-automation-cli"
  if [[ -x "$REL" && -x "$DBG" ]]; then
    if [[ "$DBG" -nt "$REL" ]]; then BIN="$DBG"; else BIN="$REL"; fi
  elif [[ -x "$REL" ]]; then
    BIN="$REL"
  else
    BIN="$DBG"
  fi
fi

die() {
  echo "$1" >&2
  exit "${2:-65}"
}

# `declare -A` needs bash 4 and macOS ships bash 3.2 (2026-09-04). Every map in
# this script is therefore either a `case` or a newline-delimited set: no key is
# ever mangled into a variable name, so a flag with `-` stays exact.

# True when the newline-delimited set $1 contains the exact line $2.
set_has() {
  printf '%s' "$1" | rg -q -x -F -e "$2"
}

# Number of non-empty lines in the newline-delimited set $1.
set_size() {
  if [[ -z "$1" ]]; then
    printf '0'
    return 0
  fi
  printf '%s\n' "$1" | rg -c '' || true
}

# PRD flag -> XDG key, where the capability moved to per-host configuration.
xdg_equivalent() {
  case "$1" in
  --chrome-path) printf 'chrome_path' ;;
  --ffmpeg-path) printf 'ffmpeg_path' ;;
  --lighthouse-path) printf 'lighthouse_path' ;;
  --artifacts-dir) printf 'artifacts_dir' ;;
  --no-color) printf 'color' ;;
  --max-body-bytes) printf 'scrape_max_body_bytes' ;;
  --concurrency) printf 'http_pool_max_idle_per_host' ;;
  --viewport) printf 'default_viewport_width' ;;
  *) printf '' ;;
  esac
}

# ── PRD side ────────────────────────────────────────────────────────────────
# The flag may carry a value placeholder inside the same backticks
# (`--timeout <secs>`, `--robots-policy honor|ignore`). Anchoring on the closing
# backtick silently dropped 26 of the 50 entries.
prd_global_flags() {
  [[ -f "$PRD" ]] || die "PRD not found: $PRD"
  local line in_section=0
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$in_section" -eq 0 ]]; then
      [[ "${line#"${line%%[![:space:]]*}"}" == "### Flags Globais" ]] && in_section=1
      continue
    fi
    [[ "$line" == '###'* || "$line" == '##'* ]] && break
    if [[ "$line" =~ ^-[[:space:]]+\`(--?[a-z0-9-]+)([\ =][^\`]*)?\` ]]; then
      printf '%s\n' "${BASH_REMATCH[1]}"
    fi
  done <"$PRD"
}

# ── Implementation side ─────────────────────────────────────────────────────
# Collect `short = 'q'` as `-q`. The PRD lists some flags only in short form;
# matching long names alone made every one of them read as `absent`, a fictional
# debt that made the whole scoreboard untrustworthy.
short_flags() {
  rg -o -r '-$1' "short\s*=\s*'([A-Za-z0-9])'" "$1" 2>/dev/null || true
}

# `#[command(flatten)]` FIELDS ARE CONTAINERS, NOT FLAGS
#
#   Measured 2026-09-01 against `--help`, which is the arbiter here: this
#   function read `pub agent_ops:` and `pub mitm_args:` as the flags
#   `--agent-ops` and `--mitm-args`, and NEITHER exists on the binary. Both are
#   struct fields carrying `#[command(flatten)]`, so clap splices their members
#   in and never names the container.
#
#   The same blindness cost more in the other direction. The nine real
#   `--mitm-*` globals live in `src/cli/mitm_args.rs` as bare `pub field:` with
#   the long name DERIVED, so `real_globals` never saw them and `local_flags`
#   could not either: its two patterns want `long = "..."` or an `#[arg(...)]`
#   glued to a field with no `pub`. Regenerating therefore moved all nine from
#   `global` to `absent` and invented eight units of fictional debt.
#
#   The split into `mitm_args.rs` and `agent_ops_args.rs` happened to satisfy
#   `scripts/filesize-check.sh`, and both modules say so in their headers. A
#   move inside Rust must not change what this table reports, so the fix
#   FOLLOWS the flatten instead of pinning the old file layout.
flatten_containers() {
  rg -U -o -r '$1' '#\[command\(flatten\)\]\s*\n\s+pub ([a-z_0-9]+):' "$GLOBAL_RS" 2>/dev/null || true
}

flatten_modules() {
  rg -U -o -r '$1' '#\[command\(flatten\)\]\s*\n\s+pub [a-z_0-9]+: super::([a-z_0-9]+)::' "$GLOBAL_RS" 2>/dev/null || true
}

harvest_arg_struct() {
  local src="$1"
  rg -o -r '--$1' '^\s+pub ([a-z_0-9]+):' "$src" 2>/dev/null | sd -- '_' '-' || true
  rg -o -r '--$1' 'long\s*=\s*"([a-z0-9-]+)"' "$src" 2>/dev/null || true
  short_flags "$src"
}

real_globals() {
  [[ -f "$GLOBAL_RS" ]] || die "GlobalOpts source not found: $GLOBAL_RS"
  local module src
  {
    harvest_arg_struct "$GLOBAL_RS"
    while IFS= read -r module; do
      [[ -z "$module" ]] && continue
      src="$ROOT/src/cli/$module.rs"
      # Fail loudly. A flatten whose module moved would otherwise drop every
      # flag it carries and read as honest absence.
      [[ -f "$src" ]] || die "flattened arg module not found: $src"
      harvest_arg_struct "$src"
    done < <(flatten_modules)
  } | rg -v '^\s*$' | sort -u |
    rg -v -x -F -f <(flatten_containers | sd -- '^(.+)$' '--$1' | sd -- '_' '-'; echo '--__never__')
}

local_flags() {
  local path
  while IFS= read -r path; do
    [[ "$(basename "$path")" == "global.rs" ]] && continue
    rg -o -r '--$1' 'long\s*=\s*"([a-z0-9-]+)"' "$path" 2>/dev/null || true
    short_flags "$path"
    # `#[arg(long)]` derives the flag name from the field below it.
    rg -U -o -r '--$1' '#\[arg\((?:[^)]*\blong\b)[^)]*\)\]\s*\n\s+([a-z_0-9]+):' "$path" 2>/dev/null |
      sd -- '_' '-' || true
  done < <(fd -d 1 -e rs . "$ROOT/src/cli" | sort)
}

# `die` is useless in here: see the guard below the `--check` parse.
xdg_keys() {
  local raw
  raw="$("$BIN" --json config list-keys 2>/dev/null)" || return 1
  printf '%s' "$raw" | jaq -r '.data.keys[].key' || return 1
}

CHECK=0
[[ "${1:-}" == "--check" ]] && CHECK=1

# CHECK THE BINARY IN THE PARENT, WHERE `die` CAN ACTUALLY ABORT
#   `xdg_keys` used to `die` on a missing binary, and it is consumed below as
#   `< <(xdg_keys)`. Process substitution runs it in a SUBSHELL, so that
#   `exit 65` killed the child and never this script: the parent carried on with
#   an EMPTY key list, built the table with no `xdg` column, and reported the
#   document as `stale`. The TRUE cause was printed one line above the false
#   one, and obeying the false one regenerates the document with zero XDG keys
#   and exit 0 — after which `--check` passes and certifies the corruption for
#   good. Bash has no `set -e` that crosses process substitution, so this guard
#   has to live out here, where an abort actually reaches the script.
[[ -x "$BIN" ]] || die "binary not built: $BIN (build it, or pass BIN=)"

# `mapfile` is a bash 4 builtin and macOS ships bash 3.2, so the read loop
# below is the portable equivalent (2026-09-04).
prd=()
while IFS= read -r __line; do prd+=("$__line"); done < <(prd_global_flags)
[[ "${#prd[@]}" -gt 0 ]] || die "PRD global flag list parsed to zero entries"

G_LIST=""
L_LIST=""
X_LIST=""
while IFS= read -r f; do [[ -z "$f" ]] || G_LIST="${G_LIST}${f}"$'\n'; done < <(real_globals)
[[ "$(set_size "$G_LIST")" -gt 0 ]] || die "GlobalOpts parsed to zero fields"
while IFS= read -r f; do [[ -z "$f" ]] || L_LIST="${L_LIST}${f}"$'\n'; done < <(local_flags | sort -u)
[[ "$(set_size "$L_LIST")" -gt 0 ]] || die "subcommand flag list parsed to zero entries"
while IFS= read -r k; do [[ -z "$k" ]] || X_LIST="${X_LIST}${k}"$'\n'; done < <(xdg_keys)
# EVERY SOURCE ANSWERS BEFORE THE DOCUMENT IS REWRITTEN
#   An empty bucket is never a real measurement here: it means the source failed
#   inside a subshell that could not abort this one. Refusing to continue is the
#   whole difference between a loud failure and a silently corrupted artifact.
[[ "$(set_size "$X_LIST")" -gt 0 ]] || die "XDG key list parsed to zero entries"

rows=""
count_global=0
count_local=0
count_xdg=0
count_absent=0
PRD_LIST=""
for flag in "${prd[@]}"; do
  PRD_LIST="${PRD_LIST}${flag}"$'\n'
done
for flag in "${prd[@]}"; do
  key="$(xdg_equivalent "$flag")"
  if set_has "$G_LIST" "$flag"; then
    where="global"
    detail='`GlobalOpts`'
  elif set_has "$L_LIST" "$flag"; then
    where="local"
    detail="flag de subcomando"
  elif [[ -n "$key" ]] && set_has "$X_LIST" "$key"; then
    where="xdg"
    detail="\`config set ${key}\` (por host, NÃO por invocação)"
  else
    where="absent"
    detail="não existe em lugar nenhum"
  fi
  case "$where" in
  global) count_global=$((count_global + 1)) ;;
  local) count_local=$((count_local + 1)) ;;
  xdg) count_xdg=$((count_xdg + 1)) ;;
  *) count_absent=$((count_absent + 1)) ;;
  esac
  rows+="| \`${flag}\` | ${where} | ${detail} |"$'\n'
done

extra=""
while IFS= read -r f; do
  [[ -z "$f" ]] && continue
  set_has "$PRD_LIST" "$f" && continue
  [[ "$f" == "--help" || "$f" == "--version" ]] && continue
  extra+="- \`${f}\`"$'\n'
done < <(printf '%s' "$G_LIST" | sort)
[[ -n "$extra" ]] || extra="- Nenhuma"$'\n'

text="# Reconciliação de flags globais do PRD


## Como este arquivo é produzido
- GERADO por \`scripts/gen-flag-reconciliation.sh\`; NUNCA edite à mão
- Lista do PRD sai da seção \`### Flags Globais\`
- Superfície real sai de \`src/cli/global.rs\`, dos subcomandos e de \`config list-keys\`
- Regenere com \`bash scripts/gen-flag-reconciliation.sh\`


## Por que \`xdg\` não é o mesmo que \`global\`
- Uma chave XDG vale para o host inteiro e NÃO varia por invocação
- Um agente que precise alternar o valor entre duas chamadas não consegue
- Marcar como resolvido o que virou chave XDG esconde essa perda


## Placar
- Flags declaradas globais no PRD: ${#prd[@]}
- Existem como global: ${count_global}
- Existem como flag local de subcomando: ${count_local}
- Existem apenas como chave XDG: ${count_xdg}
- Não existem em lugar nenhum: ${count_absent}


## Tabela
| flag do PRD | onde vive | detalhe |
|---|---|---|
${rows}

## Globais reais que o PRD não lista
${extra}"

if [[ "$CHECK" -eq 1 ]]; then
  current=""
  [[ -f "$OUT" ]] && current="$(<"$OUT")"
  if [[ "$current" != "${text%$'\n'}" ]]; then
    echo "flag reconciliation is stale; run scripts/gen-flag-reconciliation.sh" >&2
    exit 1
  fi
  exit 0
fi

printf '%s' "$text" >"$OUT"
jaq -nc \
  --arg written "$OUT" \
  --argjson global "${count_global}" \
  --argjson local "${count_local}" \
  --argjson xdg "${count_xdg}" \
  --argjson absent "${count_absent}" \
  --argjson prd_flags "${#prd[@]}" \
  '{written:$written,global:$global,local:$local,xdg:$xdg,absent:$absent,prd_flags:$prd_flags}'
