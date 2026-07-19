#!/usr/bin/env bash
# Local hygiene gate for rules_rust_macros (one-shot CLI, not a macro library).
# No GitHub Actions / CD — run manually or from scripts/ci-check.sh.
#
# Usage:
#   ./scripts/macros-check.sh
set -euo pipefail
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
rules_hits=$(rg -n '^\s*macro_rules!\s+\w+' src/ --glob '*.rs' || true)
if [ -z "$rules_hits" ]; then
  pass "no macro_rules! definitions in src/ (generics/build.rs preferred)"
else
  bad "macro_rules! definition present — exhaust generics/traits first or document justification"
  echo "$rules_hits"
fi

# 4) No proc-macro crate declaration (would be a different product surface)
if rg -n '\[lib\]' Cargo.toml >/dev/null 2>&1 && rg -n 'proc-macro\s*=\s*true' Cargo.toml >/dev/null 2>&1; then
  bad "proc-macro = true in this application crate"
else
  pass "no proc-macro = true (application crate)"
fi

# 5) CDP generation path: build.rs + include!(concat!(env!(OUT_DIR)))
if rg -n 'include!\(\s*concat!\(\s*env!\("OUT_DIR"\)' src/native/cdp/types.rs >/dev/null; then
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
if rg -n 'fn spawn_cdp_event_forwarder' src/native/cdp/client.rs >/dev/null; then
  pass "spawn_cdp_event_forwarder generic helper present"
else
  bad "missing generic CDP event forwarder"
fi

# 9) panic! only allowed in tests / human_panic setup / intentional test helpers
# Flag non-test panic! outside cfg(test) blocks is soft: list for review
panic_prod=$(rg -n 'panic!\(' src/ --glob '*.rs' | rg -v 'tests?\.rs|cfg\(test\)|human_panic|// ' || true)
# Always pass if only main.rs human_panic / known test modules; hard-fail only on unexpected modules
unexpected=$(echo "$panic_prod" | rg -v 'src/main\.rs|src/cache\.rs|src/lifecycle\.rs' || true)
if [ -z "$unexpected" ]; then
  pass "panic! surface limited (main human_panic / test helpers)"
else
  bad "unexpected panic! in production modules"
  echo "$unexpected"
fi

if [ "$fail" -ne 0 ]; then
  echo "== macros-check FAILED =="
  exit 1
fi
echo "== macros-check PASS =="
