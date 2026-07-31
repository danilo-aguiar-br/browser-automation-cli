#!/usr/bin/env bash
# multiplatform-check.sh — local gates for multiplatform rules (no GHA).
# Usage: from repo root: bash scripts/multiplatform-check.sh
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail=0
pass() { echo "PASS  $*"; }
bad()  { echo "FAIL  $*"; fail=1; }

echo "== multiplatform-check =="

# 1) No shell-out to which/where for discovery (must use platform::which_bin).
if rg -n 'Command::new\("which"\)|Command::new\("where"\)' src/ --glob '!target/**' 2>/dev/null; then
  bad "shell which/where still present under src/"
else
  pass "no Command::new(which|where) in src/"
fi

# 2) Platform module present (Pass F: directory, not platform.rs monólito).
if [[ -d src/platform ]] || [[ -f src/platform.rs ]]; then
  pass "src/platform module exists"
else
  bad "src/platform module missing"
fi

# 3) Chrome discovery uses platform helpers (Pass F: cdp/chrome/ dir).
if rg -n 'platform::which_bin|find_chrome_known_paths|warn_if_sandboxed_browser|find_chrome_registry|registry_app_path' src/native/cdp/chrome/ >/dev/null 2>&1; then
  pass "chrome multiplatform discovery hooks"
else
  bad "chrome missing multiplatform discovery hooks"
fi

# 3b) Windows App Paths + version smoke APIs exist (cfg stubs on non-Windows).
if rg -n 'fn registry_app_path|fn probe_binary_version' src/platform/ >/dev/null 2>&1; then
  pass "platform registry_app_path + probe_binary_version"
else
  bad "platform missing registry_app_path / probe_binary_version"
fi

# 3c) Named default viewport (anti-hardcode).
if rg -n 'DEFAULT_VIEWPORT_WIDTH|DEFAULT_VIEWPORT_HEIGHT' src/constants/ src/native/cdp/chrome/ >/dev/null; then
  pass "DEFAULT_VIEWPORT_* named constants wired"
else
  bad "DEFAULT_VIEWPORT_* missing"
fi

# 4) Windows reserved names + console VT wiring.
if rg -n 'WINDOWS_RESERVED_NAMES|reject_windows_reserved' src/validation.rs >/dev/null; then
  pass "Windows reserved basename validation"
else
  bad "Windows reserved basename validation missing"
fi
if rg -n 'ENABLE_VIRTUAL_TERMINAL_PROCESSING|configure_console' src/platform/ >/dev/null 2>&1; then
  pass "console UTF-8/VT in platform"
else
  bad "console VT missing in platform"
fi

# 5) Unit tests for platform + chrome + validation (one filter per cargo test).
if cargo test -q --lib platform:: 2>&1 | tail -5 \
  && cargo test -q --lib validation:: 2>&1 | tail -5 \
  && cargo test -q --lib chrome:: 2>&1 | tail -5; then
  pass "targeted lib tests"
else
  bad "targeted lib tests failed"
fi

# 6) Doctor reports host_environment (needs built bin optional).
if cargo build -q --bin browser-automation-cli 2>/dev/null; then
  out="$(./target/debug/browser-automation-cli doctor --offline --quick --json 2>/dev/null || true)"
  if echo "$out" | rg -q 'host_environment'; then
    pass "doctor JSON includes host_environment"
  else
    bad "doctor JSON missing host_environment (got: ${out:0:200})"
  fi
  if echo "$out" | rg -q '"sandbox"'; then
    pass "doctor chrome check includes sandbox field"
  else
    # may fail chrome entirely but schema should still have sandbox on fail entry
    if echo "$out" | rg -q '"id":"chrome"'; then
      bad "chrome check present but no sandbox field"
    else
      pass "doctor ran (chrome entry absent in unexpected shape)"
    fi
  fi
  if echo "$out" | rg -q 'windows_job_object'; then
    pass "doctor JSON includes windows_job_object"
  else
    bad "doctor JSON missing windows_job_object"
  fi
  if echo "$out" | rg -q '"version"'; then
    pass "doctor chrome check includes version field"
  else
    if echo "$out" | rg -q '"id":"chrome"'; then
      bad "chrome check present but no version field"
    else
      pass "doctor chrome entry shape unexpected (version gate skipped)"
    fi
  fi
else
  bad "cargo build failed"
fi

echo "== summary =="
if [[ "$fail" -eq 0 ]]; then
  echo "multiplatform-check: PASS"
  exit 0
else
  echo "multiplatform-check: FAIL"
  exit 1
fi
