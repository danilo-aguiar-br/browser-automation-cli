#!/usr/bin/env bash
# Wall-clock latency baseline with tail percentiles (rules_rust_latencia_reduzir).
# NOT a ci-check verifier: a MEASUREMENT tool with no pass/fail contract. Its
# numbers depend on the host — cores, load, thermal state — so a bundle step
# built on it would report the machine rather than the product, which is the
# one thing a gate must never do.
#
# Measures **agent meta paths** (no Chrome) so Rust/CLI regressions are visible
# without conflating Chrome boot (external WCET, seconds).
#
# Reports P50, P99, P999, P9999 (and max) — never mean-only. Outliers are kept.
# With small N (e.g. 40), nearest-rank P9999 may equal max — still report the tail.
#
# Usage:
#   ./scripts/latency-baseline.sh              # release bin; N=40 samples
#   ./scripts/latency-baseline.sh --samples 80
#   ./scripts/latency-baseline.sh --bin path/to/browser-automation-cli
#   ./scripts/latency-baseline.sh --build      # cargo build --release first
#
# Exit 0 always after printing (measurement tool). Gate hygiene is latency-check.sh.
set -euo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting (`--smart-case` widens matches, `--max-columns`
# truncates them away). Clearing the variable neutralizes the whole file; `-s`
# would close only one of those doors.
export RIPGREP_CONFIG_PATH=
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

# TIMING WITHOUT AN INTERPRETER
#   The three helpers below used to be Python. The product ships no interpreter,
#   so any host without one lost this measurement entirely. Bash 5 exposes
#   `EPOCHREALTIME` with microsecond resolution, which is the resolution this
#   script already reported (`%.6f`), and every statistic here is nearest-rank
#   over sorted samples — integer arithmetic on microseconds, no floats needed.
#   Ported 2026-08-18; same samples, same percentiles, same two output lines.

# WHICH CLOCK THIS HOST HAS, decided once instead of assumed.
#
# `EPOCHREALTIME` is bash 5. macOS ships bash 3.2, where it expands to the
# EMPTY STRING — and empty is the worst possible answer here, because
# `$((10#${now%.*} ...))` on empty does not abort, it yields 0. Measured
# 2026-09-04 on macOS: every sample came back 0 microseconds, every percentile
# printed `0.000000s`, and the script declared `PASS latency-baseline`. A
# benchmark that cannot read a clock must say so; reporting a perfect score is
# strictly worse than crashing, because a crash gets fixed.
#
# `date +%s%N` is not the fallback: BSD `date` has no `%N` and answers the
# literal string `N`. Perl's `Time::HiRes` is core and ships with macOS, so it
# is used when present — as a FALLBACK, never as a requirement, which is the
# distinction the note above about shipping no interpreter was making.
if [[ -n "${EPOCHREALTIME:-}" ]]; then
  CLOCK_SOURCE="bash"
elif command -v perl >/dev/null 2>&1 && perl -MTime::HiRes -e 1 >/dev/null 2>&1; then
  CLOCK_SOURCE="perl"
else
  echo "FAIL: no sub-second clock on this host." >&2
  echo "  This script needs either bash 5 (for \$EPOCHREALTIME) or perl with Time::HiRes." >&2
  echo "  bash here is ${BASH_VERSION}. Refusing to report percentiles it cannot measure." >&2
  exit 70
fi

if [[ "$CLOCK_SOURCE" == "perl" ]]; then
  echo "NOTE: reading the clock through perl (bash ${BASH_VERSION} has no \$EPOCHREALTIME)." >&2
  echo "  Each reading spawns a process, so every sample carries that spawn cost twice." >&2
  echo "  Numbers here are comparable to each other, NOT to numbers taken under bash 5." >&2
fi

# Print integer microseconds from the clock chosen above.
epoch_us() {
  if [[ "$CLOCK_SOURCE" == "perl" ]]; then
    perl -MTime::HiRes=time -e 'printf "%d", time() * 1000000'
    return
  fi
  local now="${EPOCHREALTIME}"
  now="${now/,/.}"
  printf '%s' "$((10#${now%.*} * 1000000 + 10#${now#*.}))"
}

# Format integer microseconds as seconds with six decimals.
fmt_s() {
  printf '%d.%06d' "$(($1 / 1000000))" "$(($1 % 1000000))"
}

# Run one command, print elapsed microseconds (stdout/stderr discarded).
elapsed_us() {
  local t0 t1
  t0="$(epoch_us)"
  "$@" >/dev/null 2>&1 || true
  t1="$(epoch_us)"
  printf '%s\n' "$((t1 - t0))"
}

# stdin: one integer microsecond value per line → print percentile at fraction p
# (nearest-rank), formatted as seconds.
percentile() {
  local p="$1"
  local -a vals
  # `mapfile` is a bash 4 builtin and macOS ships bash 3.2, so every array read
  # in this file uses the portable read loop below instead (2026-09-04).
  local __line
  vals=()
  while IFS= read -r __line; do vals+=("$__line"); done < <(sort -n | rg -v '^\s*$' || true)
  local n="${#vals[@]}"
  if [[ "$n" -eq 0 ]]; then
    echo "nan"
    return 1
  fi
  # k = round(p * (n - 1)) in integer arithmetic, clamped to [0, n-1].
  # `p` is a decimal fraction ("0.5", "0.99", "0.9999"); it is widened to
  # per-10000 units so the rounding stays exact without floats.
  local frac="${p#0}"
  frac="${frac#.}"
  while [[ "${#frac}" -lt 4 ]]; do frac+="0"; done
  frac="${frac:0:4}"
  local k
  k=$(((10#$frac * (n - 1) + 5000) / 10000))
  [[ "$k" -lt 0 ]] && k=0
  [[ "$k" -gt $((n - 1)) ]] && k=$((n - 1))
  fmt_s "${vals[$k]}"
  echo
}

report_stats() {
  local label="$1"
  local n="$2"
  local -a vals
  local __line
  vals=()
  while IFS= read -r __line; do vals+=("$__line"); done < <(sort -n)
  if [[ "${#vals[@]}" -ne "$n" ]]; then
    echo "FAIL: sample count ${#vals[@]} != expected $n" >&2
    exit 70
  fi

  # Nearest-rank index for a percentile expressed in per-10000 units, so that
  # 0.50/0.99/0.999/0.9999 stay exact under integer arithmetic.
  pct_at() {
    local per10k="$1" k
    k=$(((per10k * (n - 1) + 5000) / 10000))
    [[ "$k" -lt 0 ]] && k=0
    [[ "$k" -gt $((n - 1)) ]] && k=$((n - 1))
    printf '%s' "${vals[$k]}"
  }

  local mn mx p50 p99 p999 p9999 sum=0 mean v
  mn="${vals[0]}"
  mx="${vals[$((n - 1))]}"
  p50="$(pct_at 5000)"
  p99="$(pct_at 9900)"
  p999="$(pct_at 9990)"
  p9999="$(pct_at 9999)"
  # bash 3.2 aborts on "${arr[@]}" of an empty array under `set -u`, so the
  # expansion is defensive even though n==0 is not expected here.
  for v in "${vals[@]+"${vals[@]}"}"; do sum=$((sum + v)); done
  mean=$(((sum + n / 2) / n))

  printf '    n=%s min=%ss p50=%ss p99=%ss p999=%ss p9999=%ss max=%ss (mean=%ss diagnostic only)\n' \
    "$n" "$(fmt_s "$mn")" "$(fmt_s "$p50")" "$(fmt_s "$p99")" "$(fmt_s "$p999")" \
    "$(fmt_s "$p9999")" "$(fmt_s "$mx")" "$(fmt_s "$mean")"

  jaq -nc \
    --arg path "$label" \
    --argjson n "$n" \
    --argjson min_s "$(fmt_s "$mn")" \
    --argjson p50_s "$(fmt_s "$p50")" \
    --argjson p99_s "$(fmt_s "$p99")" \
    --argjson p999_s "$(fmt_s "$p999")" \
    --argjson p9999_s "$(fmt_s "$p9999")" \
    --argjson max_s "$(fmt_s "$mx")" \
    --argjson mean_s "$(fmt_s "$mean")" \
    '{path:$path,n:$n,min_s:$min_s,p50_s:$p50_s,p99_s:$p99_s,p999_s:$p999_s,p9999_s:$p9999_s,max_s:$max_s,mean_s:$mean_s}'
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
    t=$(elapsed_us "$@")
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
echo "==> PASS latency-baseline (see p50/p99/p999/p9999 above; mean is diagnostic only)"
echo "Hints:"
echo "  cargo bench --bench cli_parse"
echo "  cargo build --profile release-prof && cargo flamegraph --profile release-prof -- doctor --offline --quick"
echo "  ./scripts/latency-check.sh"
exit 0
