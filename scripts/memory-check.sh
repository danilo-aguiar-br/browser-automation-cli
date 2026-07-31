#!/usr/bin/env bash
# Local hygiene gate for rules_rust_gerenciamento_memoria / RAII (one-shot CLI).
# No GitHub Actions — run manually or from scripts/ci-check.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; fail=1; }

echo "== memory-check (RAII / ownership / allocation hygiene) =="

# 1) No std::process::exit
if rg -n 'std::process::exit|process::exit\(' src/ --glob '*.rs' | rg -v '^\s*//' >/dev/null 2>&1; then
  bad "process::exit in src"
else
  pass "no process::exit"
fi

# 2) mem::forget(...): forbidden after Pass 15 (TracingLocalGuard owns WorkerGuard).
# Comments documenting the ban are allowed.
forget_hits=$(rg -n 'mem::forget\(|std::mem::forget\(' src/ --glob '*.rs' || true)
if [ -z "$forget_hits" ]; then
  pass "no mem::forget(...) (TracingLocalGuard drop flushes WorkerGuard)"
else
  bad "mem::forget(...) present — prefer named TracingLocalGuard until process end"
  echo "$forget_hits"
fi

# 3) No Box::leak / Vec::leak *calls* in src (doc/comment mentions allowed).
# Match only call syntax so prose like "never Box::leak for diagnostics" passes.
if rg -n 'Box::leak\s*\(|Vec::leak\s*\(' src/ --glob '*.rs' \
  | rg -v '^\s*//|^\s*\*|///|//!' >/dev/null 2>&1; then
  bad "Box::leak / Vec::leak present"
  rg -n 'Box::leak\s*\(|Vec::leak\s*\(' src/ --glob '*.rs' || true
else
  pass "no Box::leak / Vec::leak calls"
fi

# 4) RESP / redis bulk budget present
if rg -n 'MAX_RESP_BULK_BYTES|checked_resp_bulk_len|try_reserve_exact' src/cache >/dev/null; then
  pass "redis RESP allocation budget + try_reserve"
else
  bad "missing redis RESP allocation budget"
fi

# 5) Heap snapshot file budget
if rg -n 'MAX_HEAP_SNAPSHOT_BYTES|try_reserve' src/native/heap_snapshot >/dev/null; then
  pass "heap snapshot size budget + try_reserve"
else
  bad "missing heap snapshot budget"
fi

# 6) Lightpanda RAII Option<Child> + kill_and_reap (file or Pass H dir split)
if rg -n 'child: Option<Child>|fn kill_and_reap' src/native/cdp/lightpanda/ >/dev/null 2>&1; then
  pass "Lightpanda Option<Child> RAII + kill_and_reap"
else
  bad "Lightpanda RAII incomplete"
fi

# 7) zeroize on session key material
if rg -n 'zeroize|Zeroize' src/native/state >/dev/null; then
  pass "session key zeroize"
else
  bad "missing zeroize on encrypt/decrypt keys"
fi

# 7b) encryption_key / openrouter returned as Zeroizing (SEC-01)
if rg -n 'Zeroizing' src/xdg >/dev/null && rg -n 'fn encryption_key' src/xdg >/dev/null; then
  pass "XDG encryption_key Zeroizing accessor"
else
  bad "missing Zeroizing encryption_key accessor"
fi

# 8) Drop impls justified (Lifecycle, Lightpanda, EnvGuard)
drops=$(rg -n 'impl Drop for' src/ --glob '*.rs' || true)
echo "$drops" | while read -r line; do
  [ -n "$line" ] && printf 'INFO  Drop: %s\n' "$line"
done
if echo "$drops" | rg -q 'Lifecycle|LightpandaProcess|EnvGuard'; then
  pass "known Drop types present"
else
  bad "expected Drop types missing"
fi

# 9) No Rc (Arc only is fine for this crate)
if rg -n 'use std::rc::Rc|Rc::new' src/ --glob '*.rs' >/dev/null 2>&1; then
  bad "Rc usage found (audit cycles)"
  rg -n 'use std::rc::Rc|Rc::new' src/ --glob '*.rs' || true
else
  pass "no Rc::new / std::rc::Rc"
fi

# 10) Pass 28 — zero-copy count + sg file budget + shared HTTP only
# Pass G: assert_net is a directory (count_cpu in assert_page.rs).
if rg -n 'fn count_cpu' src/concurrency/ >/dev/null \
  && rg -n 'count_cpu' src/browser/session/assert_net/ >/dev/null; then
  pass "count_cpu used for console assert (no full-buffer clone)"
else
  bad "missing count_cpu zero-copy path for console assert"
fi
if rg -n 'MAX_SG_FILE_BYTES|read_source_within_budget' src/sg_local.rs src/constants/ >/dev/null; then
  pass "sg scan/rewrite file size budget"
else
  bad "missing MAX_SG_FILE_BYTES / read_source_within_budget"
fi
if rg -n 'reqwest::get\(' src/ --glob '*.rs' | rg -v '^\s*//|///|//!|test' >/dev/null 2>&1; then
  bad "reqwest::get one-shot client still present (must use shared OnceLock)"
  rg -n 'reqwest::get\(' src/ --glob '*.rs' || true
else
  pass "no throwaway reqwest::get (shared client only)"
fi

# 11) Optional unit tests
if [ "${1:-}" = "--inventory-only" ]; then
  echo "== inventory-only: skip cargo test =="
else
  echo "== cargo test (memory-related) =="
  cargo test --lib resp_bulk_rejects -- --nocapture
  cargo test --lib finalize_is_idempotent -- --nocapture
  cargo test --lib kill_unix_graceful -- --nocapture
  cargo test --lib count_cpu_matches -- --nocapture
fi

if [ "$fail" -ne 0 ]; then
  echo "memory-check FAILED"
  exit 1
fi
echo "memory-check PASS"
exit 0
