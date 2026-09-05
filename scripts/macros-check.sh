#!/usr/bin/env bash
# Local hygiene gate for rules_rust_macros (one-shot CLI, not a macro library).
# No GitHub Actions / CD — run manually or from scripts/ci-check.sh.
#
# Usage:
#   ./scripts/macros-check.sh
set -euo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting (`--smart-case` widens matches, `--max-columns`
# truncates them away). Clearing the variable neutralizes the whole file; `-s`
# would close only one of those doors.
export RIPGREP_CONFIG_PATH=
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; fail=1; }

echo "== macros-check (rules_rust_macros / declarative + built-in hygiene) =="

# 1) No production placeholders left behind
todo_hits=$(rg -n 'todo!\(|unimplemented!\(|dbg!\(' src/ --glob '*.rs' || true)
if [ -z "$todo_hits" ]; then
  pass "no todo!/unimplemented!/dbg! in src/"
else
  bad "todo!/unimplemented!/dbg! present in src/"
  echo "$todo_hits"
fi

# 2) This crate must not export public macros (not a macro library)
export_hits=$(rg -n '#\[macro_export\]|macro_export' src/ --glob '*.rs' || true)
if [ -z "$export_hits" ]; then
  pass "no #[macro_export] (crate is not a macro library)"
else
  bad "public macro_export found — document justification or remove"
  echo "$export_hits"
fi

# 3) Prefer generics: no macro_rules! *definitions* in src after Pass 16
# (CDP forwarders are generic fns). Comments may still mention the ban.
#
# DECLARED EXCEPTIONS. Each entry states why generics/traits cannot express the
# construct. An undocumented entry is how an exception list becomes a hiding
# place, so the justification lives here and not in a commit message.
#
#   src/xdg/policy/knobs/expand.rs — `policy_knobs!`
#     Generates, from ONE table row, six things that must stay in lockstep: a
#     serde struct field, a `key` constant, the default reader, the `config get`
#     arm, the `config set` arm, and the `config list-keys` entry. Generics
#     cannot declare struct fields or match arms, and a trait cannot mint a
#     `const` name per row. The alternative is 86 rows x 6 hand-written sites,
#     which is exactly the drift this macro exists to prevent.
#
#   src/commands/meta/schema/derive.rs — `is_type!`
#     Compares a clap `ValueParser` type id against a probe built for a Rust
#     TYPE. The type is the argument, so a generic fn cannot take it without
#     naming `AnyValueId`, which clap does not re-export. Function-local and
#     three lines long.
MACRO_EXCEPTIONS='src/xdg/policy/knobs/expand\.rs|src/commands/meta/schema/derive\.rs'
rules_hits=$(rg -n '^\s*macro_rules!\s+\w+' src/ --glob '*.rs' || true)
rules_unexcused=$(echo "$rules_hits" | rg -v "$MACRO_EXCEPTIONS" | rg . || true)
if [ -z "$rules_unexcused" ]; then
  if [ -n "$rules_hits" ]; then
    pass "macro_rules! only in declared exceptions (see justifications above)"
    echo "$rules_hits" | sed 's/^/      /'
  else
    pass "no macro_rules! definitions in src/ (generics/build.rs preferred)"
  fi
else
  bad "macro_rules! definition present — exhaust generics/traits first or document justification"
  echo "$rules_unexcused"
fi

# 4) No proc-macro crate declaration (would be a different product surface)
if rg -n '\[lib\]' Cargo.toml >/dev/null 2>&1 && rg -n 'proc-macro\s*=\s*true' Cargo.toml >/dev/null 2>&1; then
  bad "proc-macro = true in this application crate"
else
  pass "no proc-macro = true (application crate)"
fi

# 5) CDP generation path: build.rs + include!(concat!(env!(OUT_DIR)))
# Pass G: types live under cdp/types/ (include! in types/mod.rs).
if rg -n 'include!\(\s*concat!\(\s*env!\("OUT_DIR"\)' src/native/cdp/types/ >/dev/null 2>&1; then
  pass "CDP types via include!(concat!(env!(OUT_DIR)))"
else
  bad "missing include! of OUT_DIR cdp_generated.rs"
fi

if rg -n 'cdp_generated|OUT_DIR' build.rs >/dev/null; then
  pass "build.rs emits cdp_generated into OUT_DIR"
else
  bad "build.rs missing CDP generation"
fi

# 6) Built-in compile-time env! for package identity (not runtime getenv for version)
if rg -n 'env!\("CARGO_PKG_VERSION"\)|option_env!\("GIT_SHA"\)' src/ --glob '*.rs' >/dev/null; then
  pass "env!/option_env! for package/build identity"
else
  bad "missing env! package identity usage"
fi

# 7) Forbidden dual-alloc format!+println! antipattern
if rg -n 'println!\(\s*&?format!|eprintln!\(\s*&?format!' src/ --glob '*.rs' >/dev/null 2>&1; then
  bad "format! nested inside println!/eprintln! (double allocation)"
  rg -n 'println!\(\s*&?format!|eprintln!\(\s*&?format!' src/ --glob '*.rs' || true
else
  pass "no format! nested in println!/eprintln!"
fi

# 8) Generic CDP forwarder present (replacement for macro_rules! fwd)
# Pass G: forwarder lives in cdp/client/forwarders.rs.
if rg -n 'fn spawn_cdp_event_forwarder' src/native/cdp/client/ >/dev/null; then
  pass "spawn_cdp_event_forwarder generic helper present"
else
  bad "missing generic CDP event forwarder"
fi

# 9) panic! only allowed in tests / human_panic setup
#
# WHY THIS TRACKS BLOCKS AND NOT LINES
#   The previous shape excluded any line matching `cfg(test)`, and the only line
#   that ever matches is the ATTRIBUTE itself. A `panic!` three lines into an
#   inline `#[cfg(test)] mod tests` was therefore reported as production code.
#   The answer at the time was to tolerate whole paths -- src/cache/,
#   src/lifecycle/, src/concurrency/, src/sync_util.rs -- and a path tolerance
#   hides a real production panic exactly as well as it hides a false one.
#
#   Tracking the block removes the class instead of the instance. Measured while
#   making this change: every panic! in those tolerated paths was a test panic,
#   so the tolerance list is DELETED rather than extended. The gate is strictly
#   stronger than the version it replaces, not merely quieter.
#
# WHY MODULE-LEVEL cfg(test) IS ALSO A BLOCK
#   Block tracking reads the attribute where it is WRITTEN, and for a whole
#   test-only module it is written at the declaration in lib.rs, not in the
#   file. `#[cfg(test)] pub mod test_utils;` keeps every line of
#   src/test_utils.rs out of a release binary, yet the file itself contains no
#   `#[cfg(test)]` for the awk above to see, so its `panic!` was reported as
#   production code.
#
#   The list is DERIVED from lib.rs rather than spelled here, for the same
#   reason the path tolerance was deleted: a literal name would keep excusing
#   the file after someone drops the attribute. Remove the attribute and the
#   file is scanned again on the next run, with no edit to this script.
test_only_mods=$(
  rg -A1 '^#\[cfg\(test\)\]$' src/lib.rs 2>/dev/null |
    rg -o '^\s*(?:pub )?mod ([a-z0-9_]+);' -r '$1' || true
)
test_only_paths='^$'
if [ -n "$test_only_mods" ]; then
  test_only_paths=$(
    printf '%s\n' "$test_only_mods" |
      sed 's#^#^src/#; s#$#(\\.rs$|/)#' |
      tr '\n' '|' | sed 's/|$//'
  )
fi
panic_prod=$(
  rg -l 'panic!\(' src/ --glob '*.rs' 2>/dev/null | rg -v 'tests?\.rs$' |
    rg -v "$test_only_paths" |
    while IFS= read -r f; do
      awk -v F="$f" '
        BEGIN { intest = 0; pending = 0; depth = 0 }
        {
          if (intest) {
            o = gsub(/\{/, "{"); c = gsub(/\}/, "}")
            depth += o - c
            if (depth <= 0) intest = 0
            next
          }
          if ($0 ~ /#\[cfg\(test\)\]/) { pending = 1; next }
          if (pending) {
            o = gsub(/\{/, "{"); c = gsub(/\}/, "}")
            if (o > 0) { depth = o - c; pending = 0; if (depth > 0) intest = 1; next }
            if ($0 ~ /;[[:space:]]*$/) pending = 0
            next
          }
          if ($0 ~ /panic!\(/ && $0 !~ /^[[:space:]]*\/\// && $0 !~ /human_panic/) {
            print F ":" FNR ":" $0
          }
        }
      ' "$f"
    done
)
if [ -z "$panic_prod" ]; then
  pass "panic! confined to tests and human_panic (block-aware)"
else
  bad "unexpected panic! in production code"
  echo "$panic_prod"
fi

# 10) Pass J: single-source HTTP_USER_AGENT via compile-time CARGO_PKG_* (not product env)
if rg -n 'pub const HTTP_USER_AGENT' src/constants/ >/dev/null \
  && rg -n 'env!\("CARGO_PKG_NAME"\)' src/constants/ >/dev/null \
  && rg -n 'env!\("CARGO_PKG_VERSION"\)' src/constants/ >/dev/null \
  && rg -n 'env!\("CARGO_PKG_HOMEPAGE"\)' src/constants/ >/dev/null; then
  pass "HTTP_USER_AGENT in constants uses CARGO_PKG_NAME/VERSION/HOMEPAGE"
else
  bad "HTTP_USER_AGENT missing or not built from CARGO_PKG_* in constants.rs"
fi

# 11) Pass J: no hard-coded package name inside concat! UA fragments
ua_hard=$(rg -n 'concat!\(\s*"browser-automation-cli/' src/ --glob '*.rs' || true)
if [ -z "$ua_hard" ]; then
  pass "no hard-coded package name in concat! UA fragments"
else
  bad "hard-coded package name in concat! (use env!(CARGO_PKG_NAME))"
  echo "$ua_hard"
fi

# 12) Pass J: XDG APPLICATION + sheet temp prefix track package name
if rg -n 'APPLICATION:\s*&str\s*=\s*env!\("CARGO_PKG_NAME"\)' src/xdg/paths.rs >/dev/null; then
  pass "XDG APPLICATION = env!(CARGO_PKG_NAME)"
else
  bad "XDG APPLICATION must be env!(CARGO_PKG_NAME)"
fi

if rg -n 'XLSX_TMP_NAME_PREFIX|env!\("CARGO_PKG_NAME"\)' src/constants/ >/dev/null \
  && rg -n 'XLSX_TMP_NAME_PREFIX' src/sheet_local.rs >/dev/null; then
  pass "xlsx temp prefix uses XLSX_TMP_NAME_PREFIX / CARGO_PKG_NAME"
else
  bad "sheet_local temp prefix not wired to XLSX_TMP_NAME_PREFIX"
fi

# 13) Pass J: consumers share constants UA (no DEFAULT_HTTP_UA local)
if rg -n 'DEFAULT_HTTP_UA' src/ --glob '*.rs' >/dev/null; then
  bad "DEFAULT_HTTP_UA still present — use constants::HTTP_USER_AGENT"
  rg -n 'DEFAULT_HTTP_UA' src/ --glob '*.rs' || true
else
  pass "no DEFAULT_HTTP_UA local (shared HTTP_USER_AGENT)"
fi

if [ "$fail" -ne 0 ]; then
  echo "== macros-check FAILED =="
  exit 1
fi
echo "== macros-check PASS =="
