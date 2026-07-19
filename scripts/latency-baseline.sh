#!/usr/bin/env bash
# Wall-clock latency baseline with tail percentiles (rules_rust_latencia_reduzir).
#
# Measures **agent meta paths** (no Chrome) so Rust/CLI regressions are visible
# without conflating Chrome boot (external WCET, seconds).
#
# Reports P50, P99, P999 (and max) — never mean-only. Outliers are kept.
#
# Usage:
#   ./scripts/latency-baseline.sh              # release bin; N=40 samples
#   ./scripts/latency-baseline.sh --samples 80
#   ./scripts/latency-baseline.sh --bin path/to/browser-automation-cli
#   ./scripts/latency-baseline.sh --build      # cargo build --release first
#
# Exit 0 always after printing (measurement tool). Gate hygiene is latency-check.sh.
set -euo pipefail
# Force C locale so printf/awk accept '.' as decimal separator (pt_BR uses ',').
export LC_ALL=C
export LANG=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SAMPLES=40
DO_BUILD=0
BIN="${ROOT}/target/release/browser-automation-cli"
WARMUP=3

while [[ $# -gt 0 ]]; do
  case "$1" in
    --samples) SAMPLES="${2:?}"; shift 2 ;;
    --bin) BIN="${2:?}"; shift 2 ;;
    --build) DO_BUILD=1; shift ;;
    --warmup) WARMUP="${2:?}"; shift 2 ;;
    -h|--help)
      sed -n '2,16p' "$0"
      exit 0
      ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

if [[ "$DO_BUILD" -eq 1 ]]; then
  echo "==> cargo build --release"
  cargo build --release -q
fi

if [[ ! -x "$BIN" ]]; then
  echo "FAIL: missing executable $BIN (run with --build or cargo build --release)" >&2
  exit 70
fi

# Run one command, print elapsed seconds (stdout/stderr discarded).
elapsed_s() {
  python3 -c '
import subprocess, sys, time
cmd = sys.argv[1:]
t0 = time.perf_counter()
subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
print(f"{time.perf_counter() - t0:.9f}")
' "$@"
}

# stdin: one float per line → print percentile at fraction p (nearest-rank).
percentile() {
  local p="$1"
  python3 -c '
import sys
p = float(sys.argv[1])
vals = sorted(float(x) for x in sys.stdin if x.strip())
n = len(vals)
if n == 0:
    print("nan")
    raise SystemExit(1)
k = min(n - 1, max(0, int(round(p * (n - 1)))))
print(f"{vals[k]:.6f}")
' "$p"
}

report_stats() {
  local label="$1"
  local n="$2"
  shift 2
  # remaining args: list of times as separate words — pass via stdin instead
  python3 -c '
import sys, json
label = sys.argv[1]
n = int(sys.argv[2])
vals = sorted(float(x) for x in sys.stdin if x.strip())
assert len(vals) == n, (len(vals), n)

def pct(p):
    k = min(n - 1, max(0, int(round(p * (n - 1)))))
    return vals[k]

mn, mx = vals[0], vals[-1]
p50, p99, p999 = pct(0.50), pct(0.99), pct(0.999)
mean = sum(vals) / n
print(
    f"    n={n} min={mn:.6f}s p50={p50:.6f}s "
    f"p99={p99:.6f}s p999={p999:.6f}s max={mx:.6f}s "
    f"(mean={mean:.6f}s diagnostic only)"
)
print(json.dumps({
    "path": label,
    "n": n,
    "min_s": round(mn, 6),
    "p50_s": round(p50, 6),
    "p99_s": round(p99, 6),
    "p999_s": round(p999, 6),
    "max_s": round(mx, 6),
    "mean_s": round(mean, 6),
}, separators=(",", ":")))
' "$label" "$n"
}

measure_cmd() {
  local label="$1"
  shift
  local i t
  local tmp
  tmp=$(mktemp)
  echo "==> $label (warmup=$WARMUP samples=$SAMPLES)"
  for ((i = 0; i < WARMUP; i++)); do
    "$@" >/dev/null 2>&1 || true
  done
  for ((i = 0; i < SAMPLES; i++)); do
    t=$(elapsed_s "$@")
    printf '%s\n' "$t" >>"$tmp"
  done
  report_stats "$label" "$SAMPLES" <"$tmp"
  rm -f "$tmp"
}

echo "bin=$BIN"
echo "note: Chrome CDP paths are I/O-bound external WCET — not sampled here"
echo "budgets (order-of-magnitude, release, host-local): doctor offline p99 <= 0.050s; --help p99 <= 0.080s"
echo

measure_cmd "help" "$BIN" --help
measure_cmd "doctor_offline_quick_json" "$BIN" --json doctor --offline --quick
measure_cmd "version_json" "$BIN" --json version

echo
echo "==> PASS latency-baseline (see p50/p99/p999 above; mean is diagnostic only)"
echo "Hints:"
echo "  cargo bench --bench cli_parse"
echo "  cargo build --profile release-prof && cargo flamegraph --profile release-prof -- doctor --offline --quick"
echo "  ./scripts/latency-check.sh"
exit 0
