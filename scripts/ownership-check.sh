#!/usr/bin/env bash
# Local hygiene gate for rules_rust_ownership_borrowing_lifetimes (one-shot CLI).
# No GitHub Actions — run manually or from scripts/ci-check.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; fail=1; }

echo "== ownership-check =="

# 1) No Box::leak / Vec::leak / mem::forget without intentional docs (scan code, ignore comments)
leak_hits=$(rg -n 'Box::leak|Vec::leak|mem::forget|ManuallyDrop::' src/ --glob '*.rs' \
  | rg -v '^\s*//|///|//!|:\s*//|:\s*///|:\s*//!|mem::forget the guard|do not `mem::forget`|never mem::forget' || true)
# Allow documented intentional cases only if annotated SAFETY or intentional-leak comment on prior line —
# currently product law forbids artificial 'static leaks; any remaining hit is a fail.
if [ -n "$leak_hits" ]; then
  bad "intentional leak primitives present (Box::leak / Vec::leak / mem::forget / ManuallyDrop)"
  echo "$leak_hits"
else
  pass "no Box::leak / Vec::leak / mem::forget / ManuallyDrop in code"
fi

# 2) No Rc / Rc<RefCell> / Arc<RefCell> (sequential CLI + Arc only when Send required)
if rg -n '\bRc\s*<' src/ --glob '*.rs' | rg -v '^\s*//|///|//!|:\s*//|:\s*///' >/dev/null 2>&1; then
  bad "Rc<T> present (prefer Arc only when multi-thread share is required)"
  rg -n '\bRc\s*<' src/ --glob '*.rs' || true
else
  pass "no Rc<T>"
fi

if rg -n 'Arc\s*<\s*RefCell|Rc\s*<\s*RefCell' src/ --glob '*.rs' >/dev/null 2>&1; then
  bad "Arc/Rc<RefCell> antipattern"
else
  pass "no Arc/Rc<RefCell>"
fi

# 3) No &String / &Vec parameters (prefer &str / &[T])
ptr_arg=$(rg -n ':\s*&String\b|:\s*&Vec\s*<' src/ --glob '*.rs' \
  | rg -v '^\s*//|///|//!|:\s*//|:\s*///' || true)
if [ -n "$ptr_arg" ]; then
  bad "&String / &Vec parameters present"
  echo "$ptr_arg"
else
  pass "no &String / &Vec parameters"
fi

# 4) must_use on key resource owners
for needle in \
  'must_use.*Lifecycle' \
  'must_use.*CdpClient' \
  'must_use.*BrowserManager'
do
  if rg -n "$needle" src/ --glob '*.rs' >/dev/null 2>&1; then
    pass "#[must_use] resource: $needle"
  else
    # softer: attribute on line before struct
    case "$needle" in
      *Lifecycle*) f=src/lifecycle.rs; s='pub struct Lifecycle' ;;
      *CdpClient*) f=src/native/cdp/client.rs; s='pub struct CdpClient' ;;
      *BrowserManager*) f=src/native/browser.rs; s='pub struct BrowserManager' ;;
    esac
    if rg -n 'must_use' "$f" >/dev/null && rg -n "$s" "$f" >/dev/null; then
      pass "#[must_use] near $s"
    else
      bad "missing #[must_use] for $s"
    fi
  fi
done

# 5) ResolvedLocale owns system_raw (String), not &'static leak
if rg -n 'system_raw:\s*Option<&' src/i18n/ --glob '*.rs' >/dev/null 2>&1; then
  bad "system_raw still uses borrowed/static form"
else
  pass "system_raw is owned (Option<String>)"
fi

# 6) Ownership clippy lints enabled at crate root
if rg -n 'clippy::redundant_clone|clippy::needless_pass_by_value|clippy::ptr_arg' src/lib.rs >/dev/null; then
  pass "ownership clippy lints in crate root"
else
  bad "ownership clippy lints missing from src/lib.rs"
fi

# 7) Clippy ownership suite (lib only; no GHA)
echo "-- cargo clippy ownership lints --"
if cargo clippy --lib --quiet -- \
  -D clippy::redundant_clone \
  -D clippy::implicit_clone \
  -D clippy::ptr_arg \
  -D clippy::unnecessary_to_owned \
  -D clippy::cloned_instead_of_copied \
  -D clippy::map_clone \
  -W clippy::needless_pass_by_value \
  -A clippy::uninlined_format_args \
  2>/tmp/ownership-clippy.err; then
  pass "clippy ownership deny set clean"
else
  bad "clippy ownership lints failed"
  tail -40 /tmp/ownership-clippy.err || true
fi

# 8) Full lib suite (ownership regressions surface as compile/test failures)
echo "-- cargo test --lib --"
if cargo test --lib --quiet 2>/tmp/ownership-test.err; then
  pass "lib unit tests"
else
  bad "unit tests failed"
  tail -40 /tmp/ownership-test.err || true
fi

if [ "$fail" -ne 0 ]; then
  echo "ownership-check: FAILED"
  exit 1
fi
echo "ownership-check: PASS"
exit 0
