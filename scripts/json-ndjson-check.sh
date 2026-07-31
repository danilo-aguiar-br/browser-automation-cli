#!/usr/bin/env bash
# Local gate: JSON / NDJSON rules (Pass 13 / rules_rust_json_e_ndjson).
# No CI/GHA — operator runs this script.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

source "$ROOT/scripts/lib/module_paths.sh"
module_paths_self_test || exit 65
fail=0
ok() { printf 'PASS  %s\n' "$*"; }
bad() { printf 'FAIL  %s\n' "$*"; fail=1; }

echo "=== json-ndjson-check (Pass 13) ==="

# 1) serde_json is the sole production JSON parser (no simd-json / sonic-rs / json crate)
if rg -n '^\s*(simd-json|sonic-rs)\s*=' Cargo.toml >/dev/null 2>&1; then
  bad "unexpected SIMD JSON crate in Cargo.toml (prefer serde_json without benchmark)"
else
  ok "Cargo.toml: no simd-json/sonic-rs"
fi

if ! rg -n 'serde_json\s*=' Cargo.toml >/dev/null; then
  bad "serde_json missing from Cargo.toml"
else
  ok "serde_json declared"
fi

if rg -n 'serde\s*=\s*\{[^}]*derive' Cargo.toml >/dev/null; then
  ok "serde with derive"
else
  bad "serde derive feature not found"
fi

# 2) Shared helpers module exists
if [[ -f src/json_util.rs ]]; then
  ok "src/json_util.rs present"
else
  bad "src/json_util.rs missing"
fi

# 3) BOM strip + size ceilings documented in json_util
for needle in strip_utf8_bom MAX_JSON_FILE_BYTES MAX_NDJSON_LINE_BYTES to_compact_string write_json_file_atomic; do
  if rg -n "$needle" src/json_util.rs >/dev/null; then
    ok "json_util has $needle"
  else
    bad "json_util missing $needle"
  fi
done

# 4) No pretty print on agent stdout path
if rg -n 'to_string_pretty' src/output.rs src/envelope.rs >/dev/null 2>&1; then
  bad "pretty print in output/envelope (agent machine path must be compact)"
else
  ok "output/envelope: no to_string_pretty"
fi

# 5) NDJSON run path uses line limit + BOM-aware parse
if rg -n 'check_ndjson_line_len|json_util' src/commands/run/ >/dev/null; then
  ok "run script path uses json_util (BOM / line limits)"
else
  bad "run.rs not wired to json_util"
fi

# 6) json_steps must not swallow encode errors
# Binding name is free: the violation is discarding the Err arm, not the word
# chosen for the Ok binding (an identifier-anchored pattern would go green on a
# rename while the bug stayed).
if rg -n 'if let Ok\((?:mut )?[A-Za-z_][A-Za-z0-9_]*\) = serde_json::to_string' src/commands/run/ >/dev/null; then
  bad "run json_steps still swallows encode errors"
else
  ok "run json_steps propagates encode errors"
fi

# 7) Typed envelope structs
if rg -n 'struct SuccessEnvelope|struct ErrorEnvelope' src/envelope.rs >/dev/null; then
  ok "typed SuccessEnvelope / ErrorEnvelope"
else
  bad "envelope still untyped-only"
fi

# 8) No json5 runtime in src
if rg -n 'use json5|json5::' src 2>/dev/null; then
  bad "json5 crate used in src (machine interop must be RFC 8259)"
else
  ok "no json5 runtime dependency in src"
fi

# 9) Module registered in lib.rs
if rg -n 'pub mod json_util' src/lib.rs >/dev/null; then
  ok "json_util exported from lib"
else
  bad "json_util not in lib.rs"
fi

# 10) Untrusted product parsers must use json_util (BOM / limits), not raw serde_json::from_str
# `rg` on a path that does not exist also returns non-zero, so a plain
# else-branch here reported "no raw serde_json::from_str" for modules that had
# been split into directories and no longer existed under those names. The gate
# was green while asserting a property about nothing. Resolve the module first
# and fail loudly when it is gone.
for base in src/native/perf_insight src/native/cdp/discovery src/commands/ops/lighthouse src/llm_local; do
  if ! f="$(mod_path "$base")"; then
    bad "$base: module not found as .rs or directory (gate cannot assert on it)"
    continue
  fi
  if rg -q 'serde_json::from_str' "$f"; then
    bad "$f still calls serde_json::from_str directly (use json_util)"
  else
    ok "$f: no raw serde_json::from_str"
  fi
done

# 11) XDG keys for JSON size ceilings (anti-hardcode product law)
for key in max_json_file_bytes max_ndjson_line_bytes max_cli_json_payload_bytes; do
  if rg -n "\"$key\"" src/xdg >/dev/null; then
    ok "xdg config key $key"
  else
    bad "xdg missing config key $key"
  fi
  if rg -n "resolve_$key|resolve_max_" src/xdg >/dev/null; then
    :
  fi
done
for fn in resolve_max_json_file_bytes resolve_max_ndjson_line_bytes resolve_max_cli_json_payload_bytes; do
  if rg -n "fn $fn" src/xdg >/dev/null; then
    ok "xdg has $fn"
  else
    bad "xdg missing $fn"
  fi
done

# 12) Named defaults live in constants (not magic literals only in json_util)
for c in DEFAULT_MAX_JSON_FILE_BYTES DEFAULT_MAX_NDJSON_LINE_BYTES DEFAULT_MAX_CLI_JSON_PAYLOAD_BYTES ENVELOPE_SCHEMA_VERSION; do
  if rg -n "$c" src/constants/ >/dev/null; then
    ok "constants has $c"
  else
    bad "constants missing $c"
  fi
done

# 13) Unit tests (one FILTERNAME per cargo test invocation)
echo "--- cargo test (json_util / envelope / parse_script) ---"
ut_ok=1
for filter in json_util envelope parse_script; do
  if ! cargo test --lib "$filter" -- --test-threads=4; then
    ut_ok=0
  fi
done
if [[ "$ut_ok" -eq 1 ]]; then
  ok "unit tests green"
else
  bad "unit tests failed"
fi

if [[ "$fail" -ne 0 ]]; then
  echo "=== RESULT: FAIL ==="
  exit 1
fi
echo "=== RESULT: PASS ==="
exit 0
