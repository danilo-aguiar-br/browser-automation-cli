#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Shared library for browser e2e acceptance scripts (GAP-037 / GAP-040).
#
# Contract enforced by every helper here:
#   1. RAW stdout is written to a file BEFORE any parsing or assertion runs.
#   2. The exit code is captured into its own variable, never inferred from
#      output shape and never lost to a pipe.
#   3. stdout and stderr are kept in SEPARATE files: stdout is the data
#      contract, stderr is diagnostics. Merging them corrupts the JSON parse.
#   4. Every failure message names the raw stdout file so the operator can read
#      what actually happened instead of re-running the scenario.
#   5. JSON is parsed with `jaq`. `jq` is never used.
#
# Source it from a script that has already defined WORKDIR and BIN:
#   source "$ROOT/scripts/lib/e2e_common.sh"

if [[ -n "${_BAC_E2E_COMMON_SOURCED:-}" ]]; then
  return 0
fi
_BAC_E2E_COMMON_SOURCED=1

# Populated by e2e_run: raw stdout body, its file, stderr file, and exit code.
E2E_OUT=""
E2E_RAW_FILE=""
E2E_ERR_FILE=""
E2E_RC=0
E2E_LAST_LABEL=""

_e2e_seq=0

e2e_log() { printf '%s\n' "$*" >&2; }

# Directory holding raw captures. Defaults under WORKDIR when unset.
e2e_raw_dir() {
  local dir="${E2E_RAW_DIR:-${WORKDIR:?WORKDIR must be set before sourcing e2e_common.sh}/raw}"
  mkdir -p "$dir"
  printf '%s' "$dir"
}

# e2e_run <label> <args...>
#
# Runs "$BIN" with the given args under a timeout. Writes raw stdout and raw
# stderr to separate files BEFORE returning, then exposes:
#   E2E_RC        exit code of the CLI (not of any pipe stage)
#   E2E_RAW_FILE  path to raw stdout
#   E2E_ERR_FILE  path to raw stderr
#   E2E_OUT       raw stdout body, read back from the file
e2e_run() {
  local label="$1"
  shift
  _e2e_seq=$((_e2e_seq + 1))
  local dir slug
  dir="$(e2e_raw_dir)"
  slug="$(printf '%s' "$label" | tr -c '[:alnum:]._-' '_')"
  E2E_LAST_LABEL="$label"
  E2E_RAW_FILE="$dir/$(printf '%03d' "$_e2e_seq")-${slug}.stdout"
  E2E_ERR_FILE="$dir/$(printf '%03d' "$_e2e_seq")-${slug}.stderr"

  # Capture first, analyse later. The exit code lands in its own variable and
  # no pipeline stage can mask it.
  set +e
  timeout "${E2E_TIMEOUT_SECS:-180}" "$BIN" "$@" >"$E2E_RAW_FILE" 2>"$E2E_ERR_FILE"
  E2E_RC=$?
  set -e

  E2E_OUT="$(<"$E2E_RAW_FILE")"
  return 0
}

# Failure message that always names the raw stdout file.
e2e_fail_detail() {
  local label="${1:-$E2E_LAST_LABEL}"
  printf 'label=%s rc=%s raw_stdout=%s raw_stderr=%s' \
    "$label" "$E2E_RC" "$E2E_RAW_FILE" "$E2E_ERR_FILE"
}

# e2e_expect_rc <expected> [label] — assert the captured exit code.
e2e_expect_rc() {
  local want="$1" label="${2:-$E2E_LAST_LABEL}"
  if [[ "$E2E_RC" -ne "$want" ]]; then
    e2e_log "RC_FAIL want=$want $(e2e_fail_detail "$label")"
    return 1
  fi
  return 0
}

# e2e_expect_envelope [label] — strict agent envelope on the RAW stdout file.
e2e_expect_envelope() {
  local label="${1:-$E2E_LAST_LABEL}"
  if ! jaq -e '
    type == "object"
    and .schema_version == 1
    and .ok == true
    and (.data != null)
  ' <"$E2E_RAW_FILE" >/dev/null 2>&1; then
    e2e_log "ENVELOPE_FAIL $(e2e_fail_detail "$label")"
    return 1
  fi
  return 0
}

# e2e_jaq <filter> — run a jaq filter against the RAW stdout file.
e2e_jaq() {
  jaq -e "$1" <"$E2E_RAW_FILE" >/dev/null 2>&1
}

# e2e_jaq_raw <filter> — emit a raw value from the RAW stdout file.
e2e_jaq_raw() {
  jaq -r "$1" <"$E2E_RAW_FILE" 2>/dev/null
}

# e2e_expect_jaq <filter> [label] — assert a jaq filter over raw stdout.
e2e_expect_jaq() {
  local filter="$1" label="${2:-$E2E_LAST_LABEL}"
  if ! e2e_jaq "$filter"; then
    e2e_log "JAQ_FAIL filter=$filter $(e2e_fail_detail "$label")"
    return 1
  fi
  return 0
}

# e2e_expect_steps_ok [label] — every step of a multi-step run must be ok.
e2e_expect_steps_ok() {
  local label="${1:-$E2E_LAST_LABEL}"
  if ! jaq -e '
    ((.data.steps // .steps // null) == null)
    or ((.data.steps // .steps) | type == "array" and all(.ok == true))
  ' <"$E2E_RAW_FILE" >/dev/null 2>&1; then
    e2e_log "STEPS_FAIL $(e2e_fail_detail "$label")"
    return 1
  fi
  return 0
}

# e2e_assert_residual_zero [label]
#
# Residual verification for the end of every browser e2e.
#
# The gate is `orphan_marker_dirs` plus `scavenge_safe_candidates`: residue that
# is past the age floor and that no live process holds, i.e. paths a healthy DIE
# should already have removed. Two counters are deliberately NOT gates:
#   - `sibling_live_processes`: a concurrent invocation is healthy (GAP-002);
#   - `chromium_tmp_singleton_orphans`: includes side-channels still under the
#     age floor, which any process on the host can create, so it is noise rather
#     than proof of a leak.
#
# Returns 0 on a clean host, 1 otherwise. The raw doctor stdout is kept.
e2e_assert_residual_zero() {
  local label="${1:-residual}"
  e2e_run "$label" --json doctor --quick --offline
  if ! e2e_expect_rc 0 "$label"; then
    return 1
  fi

  # Absent fields must FAIL, never default to zero. A stale binary that predates
  # the residual contract would otherwise make this gate pass while residue piles
  # up — the silent-leak failure mode this gate exists to prevent.
  if ! e2e_expect_jaq '
    .data.residual
    | (.orphan_marker_dirs | type) == "number"
      and (.scavenge_safe_candidates | type) == "number"
      and (.process_table_unavailable | type) == "boolean"
  ' "$label"; then
    e2e_log "RESIDUAL_CONTRACT_FAIL doctor emitted no orphan_marker_dirs/scavenge_safe_candidates/process_table_unavailable. The binary predates the residual contract — rebuild it (cargo build --release) or point BROWSER_AUTOMATION_CLI_BIN at a current one. $(e2e_fail_detail "$label")"
    return 1
  fi

  local orphan_markers collectable singleton_orphans unavailable
  orphan_markers="$(e2e_jaq_raw '.data.residual.orphan_marker_dirs')"
  collectable="$(e2e_jaq_raw '.data.residual.scavenge_safe_candidates')"
  singleton_orphans="$(e2e_jaq_raw '.data.residual.chromium_tmp_singleton_orphans')"
  unavailable="$(e2e_jaq_raw '.data.residual.process_table_unavailable')"

  if [[ "$unavailable" == "true" ]]; then
    e2e_log "RESIDUAL_SKIP process table unavailable; GC is fail-closed. $(e2e_fail_detail "$label")"
    return 0
  fi
  if [[ "${orphan_markers:-0}" != "0" || "${collectable:-0}" != "0" ]]; then
    e2e_log "RESIDUAL_FAIL orphan_markers=$orphan_markers collectable=$collectable singleton_noise=$singleton_orphans $(e2e_fail_detail "$label")"
    return 1
  fi
  return 0
}

# e2e_require_bin — abort early when the binary is missing.
e2e_require_bin() {
  if [[ ! -x "${BIN:-}" ]]; then
    e2e_log "ERROR: binary missing: ${BIN:-<unset>} (run cargo build --release)"
    exit 2
  fi
}
