#!/usr/bin/env bash
# Local hygiene gate: rules_rust crates nativas — no external CLI when a crate exists.
# No GitHub Actions — run manually or from scripts/ci-check.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

source "$ROOT/scripts/lib/module_paths.sh"
module_paths_self_test || exit 65

# A module is `x.rs` OR `x/`; this gate asserts behaviour, not file layout.
LIGHTHOUSE="$(mod_path src/commands/ops/lighthouse)"

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; fail=1; }

echo "== natives-check (crates nativas / no forbidden shell-outs) =="

# 1) Forbidden human CLIs in production src/ (allowlist only domain binaries).
# Patterns match Command::new("tool") for tools that have pure Rust equivalents.
forbidden_src=$(rg -n 'Command::new\("(fd|find|cat|bat|cp|rm|mv|mkdir|chmod|ls|eza|du|dutree|touch|head|tail|wc|ln|readlink|chown|chgrp|stat|file|realpath|basename|dirname|attr|setfattr|getfattr|xh|curl|wget|xhs|httpie|rg|grep|sd|sed|awk|cut|choose|tr|sort|uniq|grex|jaq|jq|rsv|csvlook|yq|sha256sum|md5sum|sha1sum|b2sum|cksum|ouch|tar|gzip|zip|unzip|date|fend|bc|expr|procs|ps|df|dysk|uname|hostname|whoami|echo|htmlq|env|printenv|export|which|where)"\)' \
  src/ --glob '*.rs' || true)
if [ -z "$forbidden_src" ]; then
  pass "no forbidden Command::new(tool) in src/"
else
  bad "forbidden Command::new(tool) in src/ (use crates / std / platform::which_bin)"
  echo "$forbidden_src"
fi

# 2) build.rs must not shell date or git (pure fs + SystemTime).
# Ignore comments/docs that mention the ban (e.g. "no Command::new(\"git\")").
build_shell=$(rg -n 'Command::new\("(date|git)"\)|ProcessCommand::new\("(date|git)"\)' build.rs \
  | rg -v '^\s*//|///|//!|\*' || true)
if [ -n "$build_shell" ]; then
  bad "build.rs still shells date/git — use pure .git/HEAD + SystemTime"
  echo "$build_shell"
else
  pass "build.rs has no date/git Command"
fi

if rg -n 'fn read_git_head_sha|fn format_unix_secs_utc|fn utc_timestamp_now' build.rs >/dev/null; then
  pass "build.rs pure git SHA + UTC helpers present"
else
  bad "build.rs missing pure metadata helpers"
fi

# 3) which/where must not appear as Command::new in tests either.
if rg -n 'Command::new\("(which|where)"\)' tests/ --glob '*.rs' >/dev/null 2>&1; then
  bad "tests shell which/where — use PATH walk"
  rg -n 'Command::new\("(which|where)"\)' tests/ --glob '*.rs' || true
else
  pass "tests do not Command::new(which|where)"
fi

# 4) platform::which_bin is the sole PATH resolver (no which crate required).
# Pass F: platform is a directory (which_bin in path_util.rs).
if rg -n 'pub fn which_bin' src/platform/ >/dev/null 2>&1; then
  pass "platform::which_bin present"
else
  bad "platform::which_bin missing"
fi

# 5) ffmpeg: optional SO binary — must resolve via XDG then which_bin, not bare "ffmpeg" only.
# Pass F: media is a directory (screencast.rs).
if rg -n 'ffmpeg_path_from_config|which_bin\("ffmpeg"\)' src/browser/session/media/ >/dev/null 2>&1; then
  pass "ffmpeg resolved via XDG + which_bin"
else
  bad "ffmpeg path resolution incomplete"
fi

if rg -n 'ffmpeg_path' src/xdg >/dev/null; then
  pass "XDG ffmpeg_path config key present"
else
  bad "XDG ffmpeg_path missing"
fi

# 6) Core FS/HTTP/JSON use native crates (spot checks).
if rg -n 'use scraper::|scraper::' src/scrape_local >/dev/null; then
  pass "scraper used for HTML (not htmlq)"
else
  bad "scraper missing in scrape_local"
fi

if rg -n 'reqwest::' src/ --glob '*.rs' >/dev/null; then
  pass "reqwest used for HTTP (not curl/wget)"
else
  bad "reqwest missing"
fi

if rg -n 'use sha2::|sha2::' src/ --glob '*.rs' >/dev/null; then
  pass "sha2 used for hashing (not sha256sum)"
else
  bad "sha2 missing"
fi

if rg -n 'ignore::WalkBuilder|WalkBuilder' src/ --glob '*.rs' >/dev/null; then
  pass "ignore::WalkBuilder for recursive walk (not find/fd)"
else
  bad "ignore WalkBuilder missing"
fi

# 7) Documented allowlist: domain binaries only (chrome/lightpanda/lighthouse/ffmpeg/redis-server tests).
# Count production Command::new in src excluding unit-test modules.
# Drop lines from *tests*.rs and from process_util unit fixtures (/bin/true|/bin/sleep).
prod_cmds=$(rg -n 'Command::new\(|std::process::Command::new\(' src/ --glob '*.rs' \
  | rg -v '/tests?\.rs:|mod tests' \
  | rg -v 'platform/process_util\.rs:.*Command::new\("/bin/(true|sleep)"\)' \
  | rg -v '^\s*//' || true)
echo "INFO  production Command sites:"
echo "$prod_cmds" | while read -r line; do
  [ -n "$line" ] && printf '      %s\n' "$line"
done

# Pass M: timed capture helper must exist and be used by lighthouse + screencast.
if rg -n 'fn run_capture_with_timeout' src/platform/process_util.rs >/dev/null 2>&1 \
  && rg -n 'run_capture_with_timeout' "$LIGHTHOUSE" >/dev/null 2>&1 \
  && rg -n 'run_capture_with_timeout' src/browser/session/media/screencast.rs >/dev/null 2>&1; then
  pass "Pass M run_capture_with_timeout wired (lighthouse+ffmpeg)"
else
  bad "Pass M process helper missing or not wired"
fi

# Expect only: lightpanda launch, ffmpeg encode, lighthouse, redis-server (cfg test in cache).
# Fail if unexpected tool names appear as string literals in Command::new("...").
unexpected=$(echo "$prod_cmds" | rg 'Command::new\("([^"]+)"\)' -or '$1' | sort -u \
  | rg -v '^(ffmpeg|/bin/sh|/bin/true|/bin/sleep)$' || true)
# Note: most production uses Command::new(&path) not string literals — string literals
# of /bin/sh only in #[cfg(unix)] tests inside lightpanda/tests.rs.
if [ -z "$unexpected" ]; then
  pass "no unexpected Command::new(\"literal\") in production paths"
else
  # /bin/sh under cfg(test) is OK; filter those lines
  sh_in_test=$(rg -n 'Command::new\("/bin/sh"\)' src/native/cdp/lightpanda/ -B5 2>/dev/null \
    | rg -c '#\[cfg\(unix\)\]|#\[tokio::test\]|#\[test\]' || true)
  if echo "$unexpected" | rg -q '/bin/sh' && [ -n "$sh_in_test" ]; then
    pass "Command::new(\"/bin/sh\") only in unix unit tests (child lifecycle fixture)"
  else
    bad "unexpected Command::new string literals: $unexpected"
  fi
fi

if [ "$fail" -ne 0 ]; then
  echo "== natives-check FAILED =="
  exit 1
fi
echo "== natives-check OK =="
exit 0
