#!/usr/bin/env bash
# Local gate: rules_rust external process execution (no GitHub Actions).
# Pass M — timeout capture helper, BatBadBut defense, no shell spawn in prod.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

source "$ROOT/scripts/lib/module_paths.sh"
module_paths_self_test || exit 65

# A module is `x.rs` OR `x/`; this gate asserts behaviour, not file layout.
LIGHTHOUSE="$(mod_path src/commands/ops/lighthouse)"
NATIVE_BROWSER="$(mod_path src/native/browser)"

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; fail=1; }

echo "== process-check (external process rules / Pass M) =="

# 1) No shell hosts in production src (allow cfg(test) fixtures).
shell_prod=$(rg -n 'Command::new\("(sh|cmd|cmd\.exe|powershell|pwsh|/bin/sh|/bin/bash)"\)' src/ --glob '*.rs' \
  | rg -v 'tests?\.rs:|#\[cfg\(test\)\]|/tests/' || true)
if [ -z "$shell_prod" ]; then
  pass "no shell Command::new in production src"
else
  # /bin/sh only under lightpanda tests is OK
  if echo "$shell_prod" | rg -qv 'lightpanda/tests\.rs'; then
    bad "shell Command::new in production"
    echo "$shell_prod"
  else
    pass "shell Command only in lightpanda tests"
  fi
fi

# 2) Helper + safe binary present.
if rg -n 'fn run_capture_with_timeout|fn is_spawn_safe_binary|fn wait_child_or_kill' src/platform/process_util.rs >/dev/null; then
  pass "process_util helpers present"
else
  bad "process_util helpers missing"
fi

# 3) lighthouse + screencast use timed capture.
if rg -n 'run_capture_with_timeout' "$LIGHTHOUSE" >/dev/null \
  && rg -n 'run_capture_with_timeout' src/browser/session/media/screencast.rs >/dev/null; then
  pass "lighthouse + ffmpeg use run_capture_with_timeout"
else
  bad "lighthouse/ffmpeg missing run_capture_with_timeout"
fi

# 4) wait_or_kill must poll (not kill-only).
if rg -n 'wait_or_kill|wait_child_or_kill|try_wait' "$NATIVE_BROWSER" src/native/cdp/lightpanda/process.rs src/platform/process_util.rs >/dev/null; then
  pass "wait_or_kill / wait_child_or_kill poll path present"
else
  bad "wait_or_kill poll path missing"
fi

# 5) Named timeouts + screencast ffmpeg constants.
if rg -n 'DEFAULT_LIGHTHOUSE_TIMEOUT_SECS|DEFAULT_FFMPEG_ENCODE_TIMEOUT_SECS|SCREENCAST_FFMPEG_FRAMERATE|LIGHTHOUSE_CHROME_FLAGS|LOOPBACK_HOST' src/constants/ >/dev/null; then
  pass "process timeout / ffmpeg / loopback constants named"
else
  bad "process constants missing"
fi

# 6) XDG timeout keys.
if rg -n 'lighthouse_timeout_secs|ffmpeg_timeout_secs' src/xdg/config_ops/ src/xdg/resolve.rs >/dev/null; then
  pass "XDG lighthouse_timeout_secs + ffmpeg_timeout_secs"
else
  bad "XDG process timeout keys missing"
fi

# 7) Toolchain ≥ 1.77.2 (BatBadBut).
if rg -n 'rust-version\s*=\s*"1\.(7[7-9]|[8-9][0-9]|[0-9]{3,})' Cargo.toml >/dev/null \
  || rg -n 'channel\s*=\s*"1\.(7[7-9]|[8-9][0-9])' rust-toolchain.toml >/dev/null; then
  pass "rust-version / toolchain ≥ 1.77 (BatBadBut)"
else
  bad "rust-version may be below 1.77.2"
fi

# 8) No process::exit in src.
if rg -n 'std::process::exit|process::exit\(' src/ --glob '*.rs' | rg -v '^\s*//' >/dev/null 2>&1; then
  bad "process::exit in src"
else
  pass "no process::exit in src"
fi

# 9) Unit tests for process_util + lighthouse mock + platform.
echo "== unit tests (process_util / platform / lighthouse) =="
if cargo test -q --lib platform::process_util:: 2>&1 | tail -8 \
  && cargo test -q --lib platform:: 2>&1 | tail -5 \
  && cargo test -q --lib lighthouse_tests 2>&1 | tail -5; then
  pass "process-related lib tests"
else
  bad "process-related lib tests failed"
fi

if [ "$fail" -ne 0 ]; then
  echo "== process-check FAILED =="
  exit 1
fi
echo "== process-check OK =="
exit 0
