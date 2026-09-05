#!/usr/bin/env bash
# Local gate: rules_rust network best practices for one-shot agent CLI (Pass N).
# No GitHub Actions. Product law: CLI+XDG only; no_proxy; SSRF; body caps.
set -euo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting (`--smart-case` widens matches, `--max-columns`
# truncates them away). Clearing the variable neutralizes the whole file; `-s`
# would close only one of those doors.
export RIPGREP_CONFIG_PATH=
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
# shellcheck source=scripts/lib/rust-regions.sh
source "$ROOT/scripts/lib/rust-regions.sh"

source "$ROOT/scripts/lib/module_paths.sh"
module_paths_self_test || exit 65

# A module is `x.rs` OR `x/`; this gate asserts behaviour, not file layout.
ROBOTS="$(mod_path src/robots)"
CDP_DISCOVERY="$(mod_path src/native/cdp/discovery)"

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; fail=1; }

echo "== network-check (Pass N network / SSRF / HTTP client) =="

# 1) net module present
if [[ -f src/net/ssrf.rs && -f src/net/body.rs && -f src/net/mod.rs ]]; then
  pass "src/net/{ssrf,body,mod}.rs present"
else
  bad "src/net module missing"
fi

# 2) no_proxy on product HTTP clients (never honor HTTP_PROXY env)
if rg -n '\.no_proxy\(\)' "$ROBOTS" >/dev/null \
  && rg -n '\.no_proxy\(\)' src/llm_local.rs >/dev/null; then
  pass "async + blocking clients call no_proxy()"
else
  bad "missing no_proxy() on product HTTP clients"
fi

# 3) connect_timeout on shared clients
if rg -n 'connect_timeout' "$ROBOTS" >/dev/null \
  && rg -n 'connect_timeout' src/llm_local.rs >/dev/null; then
  pass "connect_timeout set on product clients"
else
  bad "missing connect_timeout on product clients"
fi

# 4) body limited helper used by scrape (no naked resp.bytes() in scrape_local/http)
if rg -n 'resp\.bytes\(\)' src/scrape_local/http.rs >/dev/null; then
  bad "scrape_local/http.rs still uses resp.bytes() (use read_body_limited)"
else
  pass "scrape uses read_body_limited (no naked bytes())"
fi

# One `rg` over three paths matches when ANY of them is wired, which passed
# with two of the three unwired. The message promises all three, so assert all
# three. Found by scripts/verifier-controls-check.sh, which removed the helper from
# robots/ only and watched this gate stay green.
missing_body_limit=""
for target in src/scrape_local/http.rs "$ROBOTS" "$CDP_DISCOVERY"; do
  rg -q 'read_body_limited' "$target" || missing_body_limit="$missing_body_limit $target"
done
if [ -z "$missing_body_limit" ]; then
  pass "read_body_limited wired (scrape/robots/discovery)"
else
  bad "read_body_limited not wired:$missing_body_limit"
fi

# 5) SSRF assert on scrape + webhook
if rg -n 'assert_safe_http_url' src/scrape_local/http.rs >/dev/null \
  && rg -n 'assert_safe_http_url' src/commands/nav/session.rs >/dev/null; then
  pass "SSRF assert on scrape + webhook"
else
  bad "SSRF assert missing on scrape/webhook"
fi

# 6) no 0.0.0.0 bind in production src (comments and tests allowed)
#
# TESTS MUST BE ABLE TO NAME THE BAD INPUT
#   A test that proves the product REJECTS a wildcard bind has to write the
#   wildcard down. Filtering on the word "test" only catches lines that happen to
#   contain it, so the fixture line itself slips through and the gate fails on
#   the very evidence that the invariant holds.
#
#   Measured case: `src/native/cdp/chrome/spawn.rs` has
#   `pin_debugging_port_forces_loopback_bind`, whose whole point is to feed
#   `--remote-debugging-address=0.0.0.0` in and assert that loopback comes out.
#   The better the test, the more certainly it tripped this gate.
#
#   `awk`-free by house rule: strip each file's `#[cfg(test)]` tail with `bat`
#   before matching, so only production lines can ever reach the filter.
bind_bad=""
while IFS=: read -r rs lineno rest; do
  [[ -z "$rs" || -z "$lineno" ]] && continue
  read -r test_open test_close < <(inline_test_span "$rs")
  # Inside the inline `#[cfg(test)]` BLOCK: not production, not this gate's
  # business. Items below the block still count — see scripts/lib/rust-regions.sh.
  [[ "$test_open" -gt 0 && "$lineno" -ge "$test_open" && "$lineno" -le "$test_close" ]] && continue
  bind_bad="${bind_bad}${rs}:${lineno}:${rest}"$'\n'
done < <(rg -n '0\.0\.0\.0' src/ --glob '*.rs' 2>/dev/null || true)
bind_bad=$(printf '%s' "$bind_bad" | rg -v '^$|never|Never|//|product law|comment|docs|note|assert' || true)
if [ -z "$bind_bad" ]; then
  pass "no production 0.0.0.0 bind"
else
  # Allow explicit product-law documentation lines
  if echo "$bind_bad" | rg -qv 'never|Never|product law|MITM|LOOPBACK|comment'; then
    bad "unexpected 0.0.0.0 in src"
    echo "$bind_bad"
  else
    pass "0.0.0.0 only in docs/product-law comments"
  fi
fi

# 7) socket2 not a direct dependency
if rg -n '^socket2\s*=' Cargo.toml >/dev/null; then
  bad "socket2 is still a direct dependency (Pass N: remove)"
else
  pass "socket2 not a direct dependency"
fi

# 8) XDG network keys allowlisted
for k in http_ssrf_mode http_timeout_secs http_connect_timeout_secs scrape_max_body_bytes \
         llm_http_timeout_secs redis_allow_remote redis_connect_timeout_secs; do
  if rg -n "\"$k\"" src/xdg/config_ops/ >/dev/null; then
    :
  else
    bad "XDG key missing from config_ops: $k"
  fi
done
pass "XDG network keys in config_ops allowlist"

# 9) cache async spawn_blocking path
if rg -n 'spawn_blocking' src/cache/async_ops.rs >/dev/null \
  && rg -n 'get_async|put_async' src/scrape_local/http.rs >/dev/null; then
  pass "cache get/put async via spawn_blocking"
else
  bad "cache async_ops / scrape wiring missing"
fi

# 10) Redis host policy + connect_timeout + nodelay
if rg -n 'assert_redis_host_allowed|connect_timeout|set_nodelay' src/cache/redis.rs >/dev/null; then
  pass "redis host policy + connect_timeout + nodelay"
else
  bad "redis Pass N hardening missing"
fi

# 11) MITM uses loopback_socket_addr (no hardcode [127,0,0,1])
if rg -n '\[127,\s*0,\s*0,\s*1\]' src/mitm_local/proxy.rs >/dev/null; then
  bad "MITM still hardcodes [127,0,0,1]"
else
  pass "MITM uses loopback helper (no hardcode octets)"
fi

# 12) named network constants present
if rg -n 'HTTP_REDIRECT_MAX|HTTP_POOL_MAX_IDLE_PER_HOST|DEFAULT_SCRAPE_MAX_BODY_BYTES|DEFAULT_HTTP_CONNECT_TIMEOUT_SECS' src/constants/ >/dev/null; then
  pass "named network constants present"
else
  bad "named network constants missing from constants.rs"
fi

if [[ "$fail" -ne 0 ]]; then
  echo "network-check FAILED"
  exit 1
fi
echo "network-check PASS"
exit 0
