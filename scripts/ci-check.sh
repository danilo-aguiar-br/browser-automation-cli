#!/usr/bin/env bash
# Local CI gate (project may gitignore .github/). Rules: clap audit + tests + optional cargo-audit.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== cargo fmt (check) =="
if command -v rustfmt >/dev/null 2>&1; then
  cargo fmt --all -- --check || true
fi

echo "== cargo test (clap + unit + integration smoke) =="
cargo test --tests --lib \
  --test clap_command_debug_assert \
  --test clap_global_flag_collision \
  --test clap_arg_coverage \
  --test manpage_cli \
  --test doctor_cli \
  --test envelope_schema \
  -- --quiet

echo "== clap debug_assert via binary help =="
cargo run --quiet -- version --json
cargo run --quiet -- man >/dev/null
cargo run --quiet -- completions bash >/dev/null

if command -v cargo-audit >/dev/null 2>&1; then
  echo "== cargo audit =="
  cargo audit || true
else
  echo "== cargo audit skipped (install: cargo install cargo-audit) =="
fi

if command -v cargo-deny >/dev/null 2>&1 && [[ -f deny.toml ]]; then
  echo "== cargo deny =="
  cargo deny check || true
else
  echo "== cargo deny skipped =="
fi

if [[ -x scripts/ownership-check.sh ]]; then
  echo "== ownership-check (local gate) =="
  scripts/ownership-check.sh
fi

if [[ -x scripts/parallelism-check.sh ]]; then
  echo "== parallelism-check (local gate) =="
  scripts/parallelism-check.sh
fi

echo "== ci-check OK =="
