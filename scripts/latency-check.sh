#!/usr/bin/env bash
# Local latency hygiene gates (rules_rust_latencia_reduzir).
# No GitHub Actions / CD — intentional product policy.
#
# Usage:
#   ./scripts/latency-check.sh                 # inventory + unit tests
#   ./scripts/latency-check.sh --baseline      # also wall-clock P50/P99 (needs release bin)
#   ./scripts/latency-check.sh --inventory-only
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DO_BASELINE=0
INVENTORY_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --baseline) DO_BASELINE=1 ;;
    --inventory-only) INVENTORY_ONLY=1 ;;
    -h|--help)
      sed -n '2,12p' "$0"
      exit 0
      ;;
  esac
done

fail() { echo "FAIL: $*" >&2; exit 65; }

echo "==> Release profile (latency build: LTO fat + CGU 1 + abort)"
rg -q 'lto\s*=\s*"fat"' Cargo.toml || fail '[profile.release] missing lto = "fat"'
rg -q 'codegen-units\s*=\s*1' Cargo.toml || fail 'codegen-units = 1 required'
rg -q 'panic\s*=\s*"abort"' Cargo.toml || fail 'panic = "abort" required'
rg -q 'opt-level\s*=\s*3' Cargo.toml || fail 'opt-level = 3 required'
echo "    release OK"

echo "==> release-prof profile (debug=1 for flamegraph / perf)"
rg -q '\[profile\.release-prof\]' Cargo.toml || fail 'missing [profile.release-prof]'
rg -q 'debug\s*=\s*1' Cargo.toml || fail 'release-prof should set debug = 1'
echo "    release-prof OK"

echo "==> Global allocator mimalloc"
rg -q 'mimalloc::MiMalloc' src/main.rs || fail 'missing #[global_allocator] mimalloc'
echo "    mimalloc OK"

echo "==> Bounded browser runtime (not num_cpus multi_thread)"
rg -q 'BROWSER_WORKER_THREADS' src/runtime_util.rs || fail 'missing BROWSER_WORKER_THREADS'
rg -q 'max_blocking_threads' src/runtime_util.rs || fail 'missing max_blocking_threads cap'
rg -q 'build_browser_runtime' src/browser/mod.rs || fail 'block_on_browser_timeout must use build_browser_runtime'
# Forbid ad-hoc multi_thread without worker_threads in product sources (tests OK).
if rg -n 'new_multi_thread\(\)' src --glob '*.rs' | rg -v 'runtime_util\.rs' | rg -q .; then
  # Only runtime_util (or browser via helper) may construct multi_thread.
  bad=$(rg -n 'new_multi_thread\(\)' src --glob '*.rs' | rg -v 'runtime_util\.rs' || true)
  if [[ -n "$bad" ]]; then
    echo "$bad" >&2
    fail 'ad-hoc new_multi_thread outside runtime_util.rs'
  fi
fi
echo "    browser runtime bounded OK"

echo "==> I/O paths use current_thread block_on_io"
rg -q 'fn block_on_io' src/runtime_util.rs || fail 'missing block_on_io'
rg -q 'runtime_util::block_on_io' src/commands_prd/mod.rs || fail 'extract_llm must use block_on_io'
rg -q 'runtime_util::block_on_io' src/workflow_local.rs || fail 'workflow offline must use block_on_io'
# No remaining ad-hoc current_thread builders for scrape
if rg -n 'new_current_thread\(\)' src --glob '*.rs' | rg -v 'runtime_util\.rs' | rg -q .; then
  bad=$(rg -n 'new_current_thread\(\)' src --glob '*.rs' | rg -v 'runtime_util\.rs' || true)
  if [[ -n "$bad" ]]; then
    echo "$bad" >&2
    fail 'ad-hoc new_current_thread outside runtime_util.rs'
  fi
fi
echo "    I/O runtime OK"

echo "==> HTTP TCP_NODELAY on shared client"
rg -q 'tcp_nodelay\(true\)' src/robots.rs || fail 'shared_http_client missing tcp_nodelay(true)'
echo "    tcp_nodelay OK"

echo "==> No target-cpu=native in committed cargo config"
if [[ -f .cargo/config.toml ]] && rg -q '^\s*[^#].*target-cpu=native' .cargo/config.toml; then
  fail '.cargo/config.toml pins target-cpu=native'
fi
echo "    portable flags OK"

echo "==> Latency budget docs present"
rg -q 'Latency budgets' src/runtime_util.rs || fail 'runtime_util missing latency budget table'
echo "    budgets documented OK"

if [[ "$INVENTORY_ONLY" -eq 1 ]]; then
  echo "==> cargo check (inventory-only)"
  cargo check -q
  echo "==> PASS latency-check (inventory-only)"
  exit 0
fi

echo "==> cargo test runtime_util"
cargo test -q --lib runtime_util

echo "==> cargo check"
cargo check -q

if [[ "$DO_BASELINE" -eq 1 ]]; then
  echo "==> wall-clock baseline (P50/P99)"
  if [[ ! -x target/release/browser-automation-cli ]]; then
    cargo build --release -q
  fi
  ./scripts/latency-baseline.sh --samples 20
fi

echo "==> PASS latency-check"
echo "Hints:"
echo "  ./scripts/latency-baseline.sh --build"
echo "  cargo build --profile release-prof"
echo "  cargo flamegraph --profile release-prof --bin browser-automation-cli -- doctor --offline --quick"
echo "  # N/A product law: PGO/BOLT, isolcpus, mlockall, kernel bypass"
exit 0
