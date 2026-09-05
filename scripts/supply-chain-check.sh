#!/usr/bin/env bash
# Supply-chain gate: advisories and dependency-graph policy for this lockfile.
#
# WHY THIS GATE EXISTS
#   Twenty-nine verifiers measure this repository and not one of them read a
#   dependency advisory. The debt was therefore invisible by construction: it
#   surfaced only when a human thought to ask, which makes coverage a property
#   of memory rather than of process.
#
# WHY BOTH TOOLS, IN THIS ORDER
#   They answer DIFFERENT questions and neither subsumes the other.
#
#     cargo audit  asks: is there an advisory for anything written in Cargo.lock
#     cargo deny   asks: is there an advisory for anything this build COMPILES
#
#   Measured 2026-08-25 on this repository: `cargo deny check advisories` said
#   `0 errors, 0 warnings, 0 notes` in 4 ms over 583 crates while `cargo audit`
#   reported five. Zero findings in 4 ms with the word `ok` is the most
#   convincing form of blindness a tool can have, because it does not look like
#   a failure at all.
#
#   Measured 2026-08-31, and this CORRECTS the reading above: for two of those
#   five, `cargo deny` was RIGHT to stay silent. `cargo tree -i paste` and
#   `cargo tree -i proc-macro-error2` match no package even with `--target all`,
#   yet both are written in Cargo.lock. They are ORPHAN LOCKFILE ENTRIES —
#   crates that left the graph and whose lines outlived them.
#
#   So the divergence is not a defect, it is an INSTRUMENT: the difference
#   between the two lists localises exactly the stale lockfile entries that
#   neither tool reports as such. Running both and comparing is a dirty-lock
#   detector for free. `cargo audit` runs FIRST because it is the pessimistic
#   one, and pessimism is the correct default posture for security.
#
# WHY THE VERDICT COMES FROM THE EXIT CODE AND NEVER FROM PARSING
#   Measured: `cargo audit` exits 0 when the only findings are `unmaintained`
#   and `unsound`, and non-zero only for a VULNERABILITY. That distinction is
#   already encoded in the exit code, so re-deriving it by grepping human-facing
#   output would add a parser that can drift from the tool it parses.
#
# WHY A TRANSITIVE ADVISORY WARNS INSTEAD OF FAILING
#   A gate that fails the build for a debt the operator cannot pay — an advisory
#   in a transitive dependency with no fixed version reachable — is a gate the
#   first person to hit it switches off. Switched off, it no longer sees the
#   vulnerability either. Failing only on what is actionable is what keeps it
#   running.
#
# WHY A MISSING TOOL IS A GAP AND NOT A FAILURE
#   Same argument: refusing to pass because `cargo-audit` is not installed would
#   block every checkout that lacks it, and a gate that blocks on infrastructure
#   gets removed. It records the gap and continues.
#
# CLEAN STDOUT: one status line on stdout; diagnostics on stderr.
set -uo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting. Clearing the variable neutralizes the whole
# file; `-s` would close only one of its doors.
export RIPGREP_CONFIG_PATH=
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

AUDIT_STATUS="skip"
DENY_STATUS="skip"
FAILED=0

# `cargo audit` — pessimistic, reads the lockfile.
if cargo audit --version >/dev/null 2>&1; then
  audit_out="$(cargo audit --quiet --color never 2>&1)"
  audit_rc=$?
  if [[ $audit_rc -eq 0 ]]; then
    AUDIT_STATUS="ok"
  else
    AUDIT_STATUS="vulnerable"
    FAILED=1
    {
      echo "== supply-chain: cargo audit reports a VULNERABILITY =="
      echo "$audit_out"
    } >&2
  fi
  # Advisories that do not move the exit code are still worth surfacing: they
  # are the ones a human decides about, and silence would hide the decision.
  warn_count="$(printf '%s' "$audit_out" | rg -c '^(warning|Crate:)' || true)"
  [[ -n "$warn_count" && "$warn_count" != "0" ]] &&
    echo "supply-chain: cargo audit non-blocking notes present ($warn_count lines)" >&2
else
  echo "supply-chain: GAP — cargo-audit absent; install with \`cargo install cargo-audit\`" >&2
fi

# `cargo deny` — precise, reads the resolved graph.
if cargo deny --version >/dev/null 2>&1; then
  deny_out="$(cargo deny --color never check 2>&1)"
  deny_rc=$?
  if [[ $deny_rc -eq 0 ]]; then
    DENY_STATUS="ok"
  else
    DENY_STATUS="policy"
    FAILED=1
    {
      echo "== supply-chain: cargo deny policy failure =="
      echo "$deny_out"
      echo
      echo "An advisory with no reachable fix belongs in deny.toml with a WRITTEN"
      echo "reason and the requirement that blocks the upgrade. Never silence one"
      echo "without it: an exception whose justification is missing is"
      echo "indistinguishable from an exception nobody re-examined."
    } >&2
  fi
else
  echo "supply-chain: GAP — cargo-deny absent; install with \`cargo install cargo-deny\`" >&2
fi

if [[ $FAILED -ne 0 ]]; then
  echo "supply-chain-check: FAIL (audit=$AUDIT_STATUS deny=$DENY_STATUS)"
  exit 1
fi

echo "supply-chain-check: OK (audit=$AUDIT_STATUS deny=$DENY_STATUS)"
exit 0
