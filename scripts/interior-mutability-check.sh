#!/usr/bin/env bash
# Local hygiene gate for rules_rust_interior_mutability (one-shot CLI).
# No GitHub Actions — run manually or from scripts/ci-check.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; fail=1; }

echo "== interior-mutability-check =="

# 1) No static mut
if rg -n 'static mut ' src/ --glob '*.rs' >/dev/null 2>&1; then
  bad "static mut present"
  rg -n 'static mut ' src/ --glob '*.rs' || true
else
  pass "no static mut"
fi

# 2) No Arc<RefCell> / Rc<RefCell> (invalid or OOP antipattern)
if rg -n 'Arc\s*<\s*RefCell|Rc\s*<\s*RefCell|Arc<RefCell|Rc<RefCell' src/ --glob '*.rs' >/dev/null 2>&1; then
  bad "Arc/Rc<RefCell> present"
else
  pass "no Arc/Rc<RefCell>"
fi

# 3) No lazy_static in new code
if rg -n 'lazy_static!' src/ --glob '*.rs' >/dev/null 2>&1; then
  bad "lazy_static! present (prefer OnceLock/LazyLock)"
else
  pass "no lazy_static"
fi

# 4) No Mutex<bool> (prefer AtomicBool) — ignore comments/docs
mutex_bool=$(rg -n 'Mutex\s*<\s*bool\s*>|Mutex::new\(false\)|Mutex::new\(true\)' src/ --glob '*.rs' \
  | rg -v '^\s*//|///|//!|\*\s' || true)
if [ -n "$mutex_bool" ]; then
  bad "Mutex<bool> present (prefer AtomicBool)"
  echo "$mutex_bool"
else
  pass "no Mutex<bool>"
fi

# 5) RefCell only in documented TLS lifecycle (single-thread) — ignore comments
refcell_hits=$(rg -n 'RefCell' src/ --glob '*.rs' | rg -v '^\s*//|///|//!|\*\s' || true)
# Also drop pure doc lines that still match path:line:content with /// after colon
refcell_code=$(echo "$refcell_hits" | rg -v ':\s*//|:\s*///|:\s*//!|:\s*\*' || true)
if [ -z "$refcell_code" ]; then
  pass "no RefCell code (none required)"
# Pass F: lifecycle is a directory (TLS RefCell in tls.rs / ledger).
elif echo "$refcell_code" | rg -v 'src/lifecycle\.rs|src/lifecycle/' | rg -q .; then
  bad "RefCell outside lifecycle TLS"
  echo "$refcell_code"
else
  pass "RefCell only in lifecycle TLS"
fi

# 6) Poison recovery helper for residual ledger
if rg -n 'fn with_ledger_mut|into_inner' src/lifecycle/ >/dev/null; then
  pass "lifecycle ledger poison recovery helper"
else
  bad "missing lifecycle with_ledger_mut / into_inner"
fi

# 7) MITM capture poison recovery
if rg -n 'fn lock_capture' src/mitm_local >/dev/null; then
  pass "mitm lock_capture poison recovery"
else
  bad "missing mitm lock_capture"
fi

# 8) No silent `if let Ok(..) = <anything>.lock()` ANYWHERE in src/
#
# This check used to pin the receiver to `...ledger.lock()`. That is the
# "verified subset" failure mode: the property was asserted only where it was
# already known to hold, so every OTHER mutex could swallow poisoning freely
# and the gate stayed green. The receiver is now unconstrained.
#
# Why the pattern is a defect and not a style choice: `Mutex::lock()` returns
# `Err` only when a previous holder panicked. `if let Ok(..)` turns that into a
# silently skipped block — the write never happens, no error surfaces, and the
# next reader sees stale state. Recover explicitly with
# `unwrap_or_else(|e| e.into_inner())` (or a named helper) so the poisoned case
# is a decision instead of an accident.
SILENT_LOCK_RE='if let Ok\((mut )?[A-Za-z_][A-Za-z0-9_]*\) = [^;]*\.lock\(\)'
silent_locks=$(rg -n "$SILENT_LOCK_RE" src/ --glob '*.rs' | rg -v '^\s*//|///|//!' || true)
if [ -n "$silent_locks" ]; then
  bad "silent if-let Ok on a Mutex lock (poisoning swallowed)"
  echo "$silent_locks"
  echo "      fix: let guard = X.lock().unwrap_or_else(|e| e.into_inner());"
else
  pass "no silent if-let Ok on any Mutex lock"
fi

# 8b) Same defect spelled with `.lock().ok()`
SILENT_LOCK_OK_RE='\.lock\(\)\s*\.ok\(\)'
silent_lock_ok=$(rg -n "$SILENT_LOCK_OK_RE" src/ --glob '*.rs' | rg -v '^\s*//|///|//!' || true)
if [ -n "$silent_lock_ok" ]; then
  bad "silent .lock().ok() (poisoning swallowed)"
  echo "$silent_lock_ok"
else
  pass "no silent .lock().ok()"
fi

# 9) tokio Mutex justification on CdpClient (Pass G: client/types.rs)
if rg -n 'tokio::sync::Mutex|held across' src/native/cdp/client/ >/dev/null; then
  pass "CdpClient tokio Mutex documented"
else
  bad "CdpClient missing tokio Mutex docs"
fi

# 10) AtomicBool PLAIN_OVERRIDE Ordering documented
if rg -n 'Ordering::Relaxed' src/color.rs >/dev/null \
   && rg -n 'PLAIN_OVERRIDE' src/color.rs >/dev/null; then
  pass "color AtomicBool Ordering documented"
else
  bad "color atomic ordering docs missing"
fi

# 11) Inventory (optional detail)
echo "INFO  interior mutability inventory:"
rg -n 'RefCell|OnceLock|LazyLock|AtomicBool|AtomicU|std::sync::Mutex|tokio::sync::Mutex|RwLock|UnsafeCell|static mut' \
  src/ --glob '*.rs' | sed 's/^/  /' | head -80 || true

if [ "$fail" -ne 0 ]; then
  echo "== interior-mutability-check FAILED =="
  exit 1
fi
echo "== interior-mutability-check PASS =="
exit 0
