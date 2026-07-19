#!/usr/bin/env bash
# Local performance hygiene checks (rules_rust_eficiencia_e_performance).
# No GitHub Actions / CD — intentional product policy.
#
# Usage:
#   ./scripts/perf-check.sh                  # profile inventory + release build smoke
#   ./scripts/perf-check.sh --inventory-only # gates only (no release rebuild)
#   ./scripts/perf-check.sh --rss            # also run scripts/rss-baseline.sh (needs release bin)
#   ./scripts/perf-check.sh --bench          # cargo bench --bench cli_parse (slow)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DO_RSS=0
DO_BENCH=0
INVENTORY_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --rss) DO_RSS=1 ;;
    --bench) DO_BENCH=1 ;;
    --inventory-only) INVENTORY_ONLY=1 ;;
    -h|--help)
      sed -n '2,14p' "$0"
      exit 0
      ;;
  esac
done

echo "==> Cargo profiles (release must be speed-first)"
if ! rg -q 'lto\s*=\s*"fat"' Cargo.toml; then
  echo "FAIL: [profile.release] missing lto = \"fat\"" >&2
  exit 65
fi
if ! rg -q 'codegen-units\s*=\s*1' Cargo.toml; then
  echo "FAIL: [profile.release] missing codegen-units = 1" >&2
  exit 65
fi
if ! rg -q 'panic\s*=\s*"abort"' Cargo.toml; then
  echo "FAIL: [profile.release] missing panic = \"abort\"" >&2
  exit 65
fi
if ! rg -q 'strip\s*=\s*("symbols"|true)' Cargo.toml; then
  echo "FAIL: [profile.release] missing strip" >&2
  exit 65
fi
if ! rg -q '\[profile\.release-size\]' Cargo.toml; then
  echo "FAIL: missing [profile.release-size] (opt-level z)" >&2
  exit 65
fi
if ! rg -q '\[profile\.bench\]' Cargo.toml; then
  echo "FAIL: missing [profile.bench]" >&2
  exit 65
fi
echo "    release / release-size / bench profiles OK"

echo "==> Global allocator (mimalloc)"
if ! rg -q 'mimalloc::MiMalloc' src/main.rs; then
  echo "FAIL: binary missing #[global_allocator] mimalloc" >&2
  exit 65
fi
echo "    mimalloc OK"

echo "==> No target-cpu=native in committed cargo config (portability)"
# Comments may document the anti-pattern; only non-comment lines fail.
if [[ -f .cargo/config.toml ]] && rg -q '^\s*[^#].*target-cpu=native' .cargo/config.toml; then
  echo "FAIL: .cargo/config.toml pins target-cpu=native (breaks multi-machine dist)" >&2
  exit 65
fi
echo "    portable flags OK"

if [[ "$INVENTORY_ONLY" -eq 1 ]]; then
  echo "==> cargo check (inventory-only)"
  cargo check -q
  echo "==> PASS perf-check (inventory-only)"
  exit 0
fi

echo "==> cargo check"
cargo check -q

echo "==> cargo build --release (measures real opt path; may take a while)"
cargo build --release -q
BIN="target/release/browser-automation-cli"
if [[ ! -x "$BIN" ]]; then
  echo "FAIL: missing $BIN" >&2
  exit 70
fi
SIZE=$(wc -c <"$BIN" | tr -d ' ')
echo "    release binary bytes=$SIZE"

if [[ "$DO_RSS" -eq 1 ]]; then
  echo "==> RSS baseline (doctor offline quick)"
  ./scripts/rss-baseline.sh doctor --offline --quick --json || true
fi

if [[ "$DO_BENCH" -eq 1 ]]; then
  echo "==> criterion cli_parse (release-like bench profile)"
  cargo bench --bench cli_parse -- --sample-size 20
fi

echo "==> PASS perf-check"
echo "Hints (manual, not automated here):"
echo "  cargo build --profile release-prof   # debug=1, no strip (flamegraph)"
echo "  cargo flamegraph --profile release-prof --bin browser-automation-cli -- doctor --offline --quick"
echo "  RUSTFLAGS='-C target-cpu=native' cargo build --release   # local turbo only"
echo "  cargo build --profile release-size"
echo "  cargo bloat --release -n 20"
echo "  ./scripts/latency-check.sh && ./scripts/latency-baseline.sh --build"
exit 0
