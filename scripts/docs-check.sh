#!/usr/bin/env bash
# Local docs.rs validation pipeline (rules_rust_docsrs_documentacao_automatica).
#
# Phases (sequential; abort on failure):
#
#   ABORT ON FAILURE MEANS EVERY PHASE BELOW THE FAILURE DOES NOT RUN.
#   That is not a footnote, it is how this gate misreports. Measured
#   2026-08-31: one broken rustdoc intra-doc link failed phase 2, so phase 5 —
#   the aquamarine / feature-gating audit — never executed at all. The
#   meta-gate `verifier-controls-check.sh` then removed the aquamarine gating
#   and reported that THIS script "stayed silent", which reads as a defect in
#   the meta-gate and was in fact this abort. A failure here is therefore TWO
#   facts: the phase that failed, and every later phase that silently stopped
#   protecting anything. Fix the first before believing anything about the rest.
#
#   1. cargo check
#   2. cargo doc --no-deps --features docs-mermaid (stable; Mermaid opt-in)
#   3. optional: cargo +nightly doc --no-deps --features docs-mermaid (doc_cfg)
#   4. optional: rustdoc JSON via nightly unstable options
#   5. coverage audit: docs-mermaid feature + optional aquamarine + metadata
#   6. emit NDJSON progress on stdout; human logs on stderr
#
# Explicitly OUT OF SCOPE (user / product law):
#   - GitHub Actions / .github workflows
#   - CD / publish to crates.io (manual release only)
#   - Replacing docs.rs with a proprietary host
#
# Exit codes (sysexits-style where applicable):
#   0   — all requested phases passed
#   65  — documentation / metadata audit failed (EX_DATAERR)
#   70  — software / build failure (EX_SOFTWARE)
#   124 — external timeout wrapper (if caller uses `timeout`)
set -euo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting (`--smart-case` widens matches, `--max-columns`
# truncates them away). Clearing the variable neutralizes the whole file; `-s`
# would close only one of those doors.
export RIPGREP_CONFIG_PATH=

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

source "$ROOT/scripts/lib/module_paths.sh"
module_paths_self_test || exit 65
# Crate docs = lib.rs plus whatever it splices in with include_str!.
CRATE_DOC_FILES="$(crate_doc_files)"

TIMEOUT_SECS="${DOCS_CHECK_TIMEOUT:-600}"
RUN_NIGHTLY="${DOCS_CHECK_NIGHTLY:-1}"
RUN_JSON="${DOCS_CHECK_JSON:-1}"
DOCS_OUT="${DOCS_CHECK_OUT:-target/doc}"

json_escape() {
  # Minimal JSON string escape for NDJSON progress lines.
  #
  # `jaq -R` reads the line as a raw string and `.` re-emits it as JSON, which is
  # the same escape table the previous `json.dumps` applied. The swap is not
  # cosmetic: this gate runs inside `ci-check`, so an interpreter dependency here
  # made the product's own closure criterion fail on any host that lacks the
  # interpreter — under `set -euo pipefail`, with no guard. The tool must be Rust
  # end to end, and `jaq` already is.
  #
  # Newlines are folded to spaces first because `-R` is line-oriented and a
  # multi-line argument would otherwise emit several JSON strings, corrupting the
  # NDJSON line this function is embedded in.
  printf '%s\n' "${1//$'\n'/ }" | jaq -R -c .
}

progress() {
  # Machine-readable progress for agent consumers (stdout only).
  local phase="$1" status="$2" msg="${3:-}"
  printf '{"schema_version":1,"event":"docs_check","phase":%s,"status":%s,"message":%s}\n' \
    "$(json_escape "$phase")" \
    "$(json_escape "$status")" \
    "$(json_escape "$msg")"
}

log() {
  printf '[docs-check] %s\n' "$*" >&2
}

die() {
  local code="$1"; shift
  log "ERROR: $*"
  progress "abort" "error" "$*"
  exit "$code"
}

run_with_soft_timeout() {
  # Prefer `timeout` when available. Supports both GNU coreutils and
  # cargo-installed `timeout SECONDS CMD` (this environment).
  local label="$1"; shift
  if command -v timeout >/dev/null 2>&1; then
    set +e
    if timeout --help 2>&1 | grep -q -- '--signal'; then
      timeout --signal=TERM "${TIMEOUT_SECS}s" "$@"
    else
      # Simple timeout: first arg is whole seconds.
      timeout "${TIMEOUT_SECS}" "$@"
    fi
    local ec=$?
    set -e
    if [[ $ec -eq 124 ]]; then
      die 124 "phase timed out after ${TIMEOUT_SECS}s: ${label}"
    fi
    return $ec
  else
    "$@"
  fi
}

# --- Phase 1: validate sources ---
log "Phase 1 — cargo check"
progress "1_check" "start" "cargo check --lib"
run_with_soft_timeout "cargo check" cargo check --lib
progress "1_check" "ok" "cargo check clean"

# --- Phase 2: HTML via cargo doc (stable / active toolchain) ---
# Do NOT pass --cfg docsrs on stable: `#![feature(doc_cfg)]` is nightly-only.
# docs.rs always builds with nightly + rustdoc-args from Cargo.toml.
# Enable docs-mermaid so aquamarine processes Mermaid (default build omits it).
#
# `-D warnings`, NOT `env -u RUSTDOCFLAGS`. Unsetting the variable made this
# phase the one place in the project where rustdoc diagnostics were advisory:
# `scripts/ci-check.sh` already builds docs with `-D warnings`, so a broken
# intra-doc link failed there while passing here. A docs gate that cannot fail
# on a documentation defect is not a gate.
log "Phase 2 — cargo doc --no-deps --features docs-mermaid (stable/public API, no docsrs cfg)"
progress "2_html" "start" "cargo doc --no-deps --features docs-mermaid"
run_with_soft_timeout "cargo doc" env RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features docs-mermaid
progress "2_html" "ok" "HTML under ${DOCS_OUT}"

# --- Phase 3: nightly docs.rs simulation (doc_cfg gate) ---
if [[ "${RUN_NIGHTLY}" == "1" ]] && command -v rustup >/dev/null 2>&1 && rustup toolchain list | grep -q '^nightly'; then
  log "Phase 3 — cargo +nightly doc --no-deps --features docs-mermaid (docsrs + doc_cfg)"
  progress "3_nightly" "start" "cargo +nightly doc --cfg docsrs"
  # Mirror package.metadata.docs.rs rustdoc-args on nightly.
  if run_with_soft_timeout "nightly doc" \
      env RUSTDOCFLAGS="--cfg docsrs --generate-link-to-definition -Z unstable-options" \
      cargo +nightly doc --no-deps --features docs-mermaid; then
    progress "3_nightly" "ok" "nightly rustdoc with docsrs + doc_cfg"
  else
    die 70 "nightly cargo doc failed (doc_cfg / rustdoc)"
  fi
else
  log "Phase 3 — skipped (set DOCS_CHECK_NIGHTLY=1 and install nightly to enable)"
  progress "3_nightly" "skip" "nightly toolchain unavailable or disabled"
fi

# --- Phase 4: rustdoc JSON (optional, nightly unstable) ---
# Use a separate CARGO_TARGET_DIR so JSON mode does not wipe HTML under target/doc.
JSON_DIR="target/rustdoc-json"
JSON_TARGET="target/rustdoc-json-build"
if [[ "${RUN_JSON}" == "1" ]] && command -v rustup >/dev/null 2>&1 && rustup toolchain list | grep -q '^nightly'; then
  log "Phase 4 — rustdoc JSON (nightly unstable, isolated target dir)"
  progress "4_json" "start" "rustdoc --output-format json"
  mkdir -p "${JSON_DIR}" "${JSON_TARGET}"
  set +e
  run_with_soft_timeout "rustdoc json" \
    env CARGO_TARGET_DIR="${JSON_TARGET}" \
        RUSTDOCFLAGS="--cfg docsrs -Z unstable-options --output-format json" \
    cargo +nightly rustdoc --lib --features docs-mermaid
  json_ec=$?
  set -e
  if [[ $json_ec -eq 0 ]] && find "${JSON_TARGET}" -name '*.json' 2>/dev/null | head -1 | grep -q .; then
    find "${JSON_TARGET}" -name '*.json' -exec cp -f {} "${JSON_DIR}/" \; 2>/dev/null || true
    progress "4_json" "ok" "JSON under ${JSON_DIR}"
  else
    log "Phase 4 — rustdoc JSON unavailable or empty (non-fatal; HTML phase is canonical)"
    progress "4_json" "warn" "json generation skipped or empty"
  fi
else
  log "Phase 4 — skipped"
  progress "4_json" "skip" "json disabled or no nightly"
fi

# --- Phase 5: embed / deps (optional docs-mermaid + aquamarine) ---
log "Phase 5 — docs-mermaid feature + aquamarine + docs.rs metadata audit"
progress "5_mermaid" "start" "Cargo.toml + aquamarine optional"
if ! grep -q 'docs-mermaid' Cargo.toml; then
  die 65 "docs-mermaid feature missing from Cargo.toml"
fi
if ! grep -q 'aquamarine' Cargo.toml; then
  die 65 "aquamarine missing from Cargo.toml (optional Mermaid dep required)"
fi
if ! grep -qE 'aquamarine = \{ version = "0\.6", optional = true \}' Cargo.toml; then
  die 65 "aquamarine must be optional so default builds skip proc-macro-error2"
fi
if ! grep -qE 'docs-mermaid\s*=\s*\["dep:aquamarine"\]' Cargo.toml; then
  die 65 "docs-mermaid must enable dep:aquamarine"
fi
# This pair is about CODE, not prose: the `aquamarine` attribute must exist and
# must sit behind the feature. It decorates whichever item carries the diagram,
# so it travels when that item is moved — search the tree, not a file.
if ! rg -q 'aquamarine' src/ --glob '*.rs'; then
  die 65 "aquamarine attribute absent from src/"
fi
if ! rg -q 'feature = "docs-mermaid".*aquamarine|aquamarine.*feature = "docs-mermaid"' src/ --glob '*.rs'; then
  die 65 "aquamarine must be gated behind feature docs-mermaid in the same attribute"
fi
progress "5_mermaid" "ok" "docs-mermaid optional aquamarine wired"

# --- Phase 6: canonical section / metadata coverage ---
log "Phase 6 — crate-level docs + Cargo metadata"
progress "6_coverage" "start" "audit lib.rs and Cargo.toml"

need_in_lib=(
  "## Overview"
  "## Quick Start"
  "## Features"
  "## Targets"
  "## MSRV"
  "## Safety"
  "## Error handling"
  "## Examples"
  "## See also"
  "feature(doc_cfg)"
)
for needle in "${need_in_lib[@]}"; do
  # shellcheck disable=SC2086
  if ! rg -qF "${needle}" $CRATE_DOC_FILES; then
    die 65 "crate-level docs missing section/token: ${needle}"
  fi
done

# Must NOT reintroduce removed feature gate.
# shellcheck disable=SC2086
if rg -q 'feature\(doc_auto_cfg\)' $CRATE_DOC_FILES; then
  die 65 "doc_auto_cfg still present — migrate to doc_cfg only (Oct 2025)"
fi

need_in_toml=(
  'documentation = "https://docs.rs/browser-automation-cli"'
  'rust-version'
  '[package.metadata.docs.rs]'
  'default-target'
  'targets'
  'rustdoc-args'
  'all-features'
)
for needle in "${need_in_toml[@]}"; do
  if ! grep -qF "${needle}" Cargo.toml; then
    die 65 "Cargo.toml missing required docs.rs metadata: ${needle}"
  fi
done

# README badges order: docs.rs before crates.io (canonical positions 1–2).
if [[ -f README.md ]]; then
  docs_line="$(grep -n 'img.shields.io/docsrs/' README.md | head -1 | cut -d: -f1 || true)"
  crates_line="$(grep -n 'img.shields.io/crates/v/' README.md | head -1 | cut -d: -f1 || true)"
  if [[ -z "${docs_line}" ]]; then
    die 65 "README.md missing docs.rs badge"
  fi
  if [[ -n "${crates_line}" && "${docs_line}" -gt "${crates_line}" ]]; then
    die 65 "README.md badge order: docs.rs must precede crates.io"
  fi
fi

# Hand-maintained agent surfaces (not a docs.rs replacement).
for f in llms.txt llms-full.txt; do
  if [[ ! -f "$f" ]]; then
    die 65 "missing ${f} (agent llms.txt surface)"
  fi
done

progress "6_coverage" "ok" "canonical sections present"

# --- Phase 7: broken link gate already in crate deny; smoke open index ---
log "Phase 7 — HTML smoke"
progress "7_links" "start" "index.html exists"
INDEX="$(find target/doc -path '*/browser_automation_cli/index.html' 2>/dev/null | head -1 || true)"
if [[ -z "${INDEX}" ]]; then
  # cargo doc may use hyphen→underscore crate dir
  INDEX="$(find target/doc -name 'index.html' 2>/dev/null | head -1 || true)"
fi
if [[ -z "${INDEX}" || ! -f "${INDEX}" ]]; then
  die 65 "cargo doc produced no index.html under target/doc"
fi
progress "7_links" "ok" "${INDEX}"

# Phase 8 publish intentionally omitted (no CD / crates.io from this script).
progress "8_publish" "skip" "publish is manual; no GitHub Actions / CD"

log "docs-check OK"
progress "done" "ok" "all local docs phases passed"
exit 0
