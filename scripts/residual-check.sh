#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Local residual hygiene gate for browser-automation-cli (PRD §5N).
# No GitHub Actions / CD — run by humans or agents on the workstation.
#
# The verdict comes from the product's own residual report, not from an ad-hoc
# temp-dir glob. Two reasons (GAP-002 / GAP-003):
#   - profiles live under the XDG cache root, not only under /tmp, so a glob over
#     /tmp alone reports a clean host while residue accumulates elsewhere;
#   - a live *sibling* invocation is healthy, so counting live marker processes
#     as failures made this gate fail whenever two runs overlapped.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN="${BROWSER_AUTOMATION_CLI_BIN:-}"
if [[ -z "$BIN" ]]; then
  if [[ -x "$ROOT/target/release/browser-automation-cli" ]]; then
    BIN="$ROOT/target/release/browser-automation-cli"
  elif [[ -x "$ROOT/target/debug/browser-automation-cli" ]]; then
    BIN="$ROOT/target/debug/browser-automation-cli"
  elif command -v browser-automation-cli >/dev/null 2>&1; then
    BIN="$(command -v browser-automation-cli)"
  else
    echo "FAIL: browser-automation-cli binary not found" >&2
    exit 1
  fi
fi

WORKDIR="${TMPDIR:-/tmp}/browser-automation-cli-residual-check-$$"
mkdir -p "$WORKDIR"
trap 'rm -rf "$WORKDIR"' EXIT

# Raw stdout to disk before analysis; exit code in its own variable; jaq only.
source "$ROOT/scripts/lib/e2e_common.sh"

echo "== residual-check: bin=$BIN =="

residual_line() {
  e2e_jaq_raw '.data.residual | "markers=\(.cli_marker_dirs) orphans=\(.orphan_marker_dirs) singleton=\(.chromium_tmp_singleton_orphans) siblings=\(.sibling_live_processes)"'
}

# Path-light: BORN GC only.
e2e_run "doctor_born" --json doctor --quick --offline
if ! e2e_expect_rc 0 "doctor_born"; then
  echo "FAIL: BORN doctor exited non-zero; $(e2e_fail_detail)" >&2
  exit 1
fi
if ! e2e_expect_jaq '.data.residual != null' "doctor_born"; then
  echo "FAIL: doctor JSON missing residual report; $(e2e_fail_detail)" >&2
  exit 1
fi
echo "after BORN doctor: $(residual_line)"

# One-shot browser work.
PDF="$WORKDIR/residual-check.pdf"
e2e_run "print_pdf" --json print-pdf --url about:blank --path "$PDF"
if ! e2e_expect_rc 0 "print_pdf"; then
  echo "FAIL: print-pdf exited non-zero; $(e2e_fail_detail)" >&2
  exit 1
fi

# Final verdict: zero orphan markers and zero Chromium Singleton orphans.
if ! e2e_assert_residual_zero "doctor_final"; then
  echo "FAIL: residual gate; $(e2e_fail_detail)" >&2
  exit 1
fi
echo "after print-pdf: $(residual_line)"

echo "PASS residual-check"
exit 0
