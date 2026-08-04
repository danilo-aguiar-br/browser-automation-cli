#!/usr/bin/env bash
# Reachable-but-never-reached gate: `pub use` with no call site in `src/`.
#
# WHY THIS GATE EXISTS
#   `cargo clippy` cannot catch this. The `dead_code` lint stops at the crate
#   boundary: once an item is re-exported with `pub use`, it is part of the
#   public API and reachable *in principle*, so the compiler stays silent even
#   when nothing inside the crate ever calls it.
#
#   That silence hid a real defect. `run_ocr_rs()` was written to refuse
#   `--engine ocrs` and name the real blocker. It was correct, it was public,
#   and no dispatch arm ever called it, so the product answered `unknown ocr
#   engine: ocrs` — indistinguishable from a typo — while the build was green
#   and the changelog claimed the message shipped.
#
#   The lesson generalises past OCR: refusal paths, error constructors and
#   diagnostic helpers have no consumer that exercises them by accident. For
#   ordinary code a missing call site shows up as a broken feature. For code
#   whose whole job is to say "no", a missing call site shows up as nothing at
#   all. Existence of the code was taken as evidence that it ran. It is not.
#
# WHAT THIS CHECKS
#   For every name exported by a `pub use` under `src/`, count references in
#   `src/`, `tests/`, `benches/` and `examples/` that are neither the re-export
#   itself nor the item's own definition. Zero references means the crate
#   exports something nothing ever reaches.
#
#   Integration tests count as call sites on purpose: a `tests/` file is a real
#   external consumer that executes the item, which is precisely the evidence
#   this gate is asking for. An item exercised only by a test is documented and
#   exercised; an item exercised by nothing is neither.
#
# WHAT THIS DOES NOT CHECK
#   It does not prove a call is *reachable at runtime* — only that one exists in
#   the source. A symbol referenced solely from dead branches still passes. This
#   gate raises the floor; it is not a reachability prover.
#
# EXCEPTIONS
#   Some exports are legitimately library-only surface. Each one is listed in
#   ALLOWLIST below WITH A REASON. An undocumented entry is how an exception
#   list turns into a place to hide debt — the same failure this gate exists to
#   prevent, one level up.
#
# CLEAN STDOUT: one status line on stdout; every diagnostic on stderr.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# ── Documented exceptions ───────────────────────────────────────────────────
# Format: "symbol # reason it is exported without an internal call site".
ALLOWLIST=(
  # (empty — add entries only with a reason, never to silence a finding)
)

allowed() {
  local needle="$1" entry
  for entry in "${ALLOWLIST[@]}"; do
    [[ "${entry%% #*}" == "$needle" ]] && return 0
  done
  return 1
}

if ! command -v rg >/dev/null 2>&1; then
  echo "reachability-check: FAIL (ripgrep required)" >&2
  echo "reachability-check: FAIL"
  exit 1
fi

# ── Collect exported names from every `pub use` statement ───────────────────
# Statements may span lines, so match through to the terminating semicolon.
mapfile -t statements < <(
  rg --multiline --multiline-dotall --no-filename --no-line-number \
     -o 'pub(?:\([^)]*\))? use [^;]*;' \
     --glob '!*.bak.*' --glob '*.rs' src/ 2>/dev/null || true
)

declare -A exported=()
for stmt in "${statements[@]}"; do
  # Normalise whitespace and strip the leading `pub use` / trailing `;`.
  body="${stmt//$'\n'/ }"
  body="${body#*use }"
  body="${body%;}"
  # Keep only the brace list when there is one; a nested path list is flattened
  # by the same comma split because we always take the segment after the last
  # `::` of each item.
  body="${body//\{/,}"
  body="${body//\}/,}"
  IFS=',' read -ra items <<<"$body"
  for item in "${items[@]}"; do
    name="${item// /}"
    [[ -z "$name" ]] && continue
    # `x as y` exports `y`.
    if [[ "$item" == *" as "* ]]; then
      name="${item##* as }"
      name="${name// /}"
    else
      name="${name##*::}"
    fi
    # Skip glob re-exports, `self`, and anything that is not an identifier.
    [[ "$name" == "*" || "$name" == "self" || "$name" == "crate" ]] && continue
    [[ "$name" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || continue
    exported["$name"]=1
  done
done

if [[ ${#exported[@]} -eq 0 ]]; then
  echo "reachability-check: FAIL (no pub use statements parsed; extractor broken)" >&2
  echo "reachability-check: FAIL"
  exit 1
fi

# ── For each exported name, look for a reference that is neither the
#    re-export line nor the definition line ──────────────────────────────────
SEARCH_ROOTS=(src)
for extra in tests benches examples; do
  [[ -d "$extra" ]] && SEARCH_ROOTS+=("$extra")
done

orphans=()
for name in "${!exported[@]}"; do
  allowed "$name" && continue
  hits="$(
    rg -w --no-heading --line-number --glob '!*.bak.*' --glob '*.rs' \
       -- "$name" "${SEARCH_ROOTS[@]}" 2>/dev/null |
      rg -v 'pub(\([^)]*\))? use ' |
      rg -v "\b(fn|struct|enum|trait|type|const|static|union|mod|macro_rules!)\s+${name}\b" |
      rg -c '' || true
  )"
  hits="${hits:-0}"
  if [[ "$hits" -eq 0 ]]; then
    orphans+=("$name")
  fi
done

if [[ ${#orphans[@]} -gt 0 ]]; then
  echo "reachability-check: FAIL (${#orphans[@]} re-exported symbol(s) reached by nothing)" >&2
  for name in "${orphans[@]}"; do
    echo "  $name" >&2
    rg -n --glob '*.rs' --glob '!*.bak.*' -w -- "$name" "${SEARCH_ROOTS[@]}" 2>/dev/null |
      rg 'pub(\([^)]*\))? use ' | head -n 2 >&2 || true
  done
  echo "reachability-check: allowlist an entry WITH A REASON, wire a call site, or delete the item" >&2
  echo "reachability-check: FAIL"
  exit 1
fi

echo "reachability-check: PASS (exported=${#exported[@]} allowlisted=${#ALLOWLIST[@]} orphans=0)"
