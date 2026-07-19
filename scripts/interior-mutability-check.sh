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
elif echo "$refcell_code" | rg -v 'src/lifecycle.rs' | rg -q .; then
  bad "RefCell outside lifecycle TLS"
  echo "$refcell_code"
else
  pass "RefCell only in lifecycle TLS"
fi

# 6) Poison recovery helper for residual ledger
if rg -n 'fn with_ledger_mut|into_inner' src/lifecycle.rs >/dev/null; then
  pass "lifecycle ledger poison recovery helper"
else
  bad "missing lifecycle with_ledger_mut / into_inner"
fi

# 7) MITM capture poison recovery
if rg -n 'fn lock_capture' src/mitm_local.rs >/dev/null; then
  pass "mitm lock_capture poison recovery"
else
  bad "missing mitm lock_capture"
fi

# 8) No silent if-let Ok on life.ledger.lock() (must use helpers)
if rg -n 'if let Ok\(mut ledger\) = life\.ledger\.lock\(\)' src/ --glob '*.rs' >/dev/null 2>&1; then
  bad "silent if-let Ok ledger.lock still present"
  rg -n 'if let Ok\(mut ledger\) = life\.ledger\.lock\(\)' src/ --glob '*.rs' || true
else
  pass "ledger access via with_ledger_mut / record_chrome / clear_chrome"
fi

# 9) tokio Mutex justification on CdpClient
if rg -n 'tokio::sync::Mutex|held across' src/native/cdp/client.rs >/dev/null; then
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
