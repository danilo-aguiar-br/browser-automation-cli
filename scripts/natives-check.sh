#!/usr/bin/env bash
# Local hygiene gate: rules_rust crates nativas — no external CLI when a crate exists.
# No GitHub Actions — run manually or from scripts/ci-check.sh.
set -euo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting (`--smart-case` widens matches, `--max-columns`
# truncates them away). Clearing the variable neutralizes the whole file; `-s`
# would close only one of those doors.
export RIPGREP_CONFIG_PATH=
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
#
# ASSERT THE PROPERTY, NOT THE SPELLING
#   The old form matched the literal names `ffmpeg_path_from_config` and
#   `which_bin("ffmpeg")` inside `src/browser/session/media/`. GAP-VID-007 then
#   did exactly what this gate wants — it centralised resolution behind
#   `video_local::resolve_ffmpeg_bin` (XDG `ffmpeg_path` first, PATH second) and
#   made screencast call that one helper. The property got STRONGER and the gate
#   went red, because it was pinned to the old spelling.
#
#   A verifier that encodes the implementation instead of the invariant punishes
#   the refactor it was written to encourage. Accept either the shared resolver
#   or the original inline pair.
if rg -n 'resolve_ffmpeg_bin|ffmpeg_path_from_config|which_bin\("ffmpeg"\)' src/browser/session/media/ >/dev/null 2>&1; then
  pass "ffmpeg resolved via XDG + which_bin (shared resolver or inline)"
else
  bad "ffmpeg path resolution incomplete"
fi

# The shared resolver itself must still consult XDG before PATH.
if rg -n 'fn resolve_ffmpeg_bin' src/video_local/ >/dev/null 2>&1; then
  if rg -n -A8 'fn resolve_ffmpeg_bin' src/video_local/ | rg -q 'ffmpeg_path'; then
    pass "resolve_ffmpeg_bin consults XDG ffmpeg_path"
  else
    bad "resolve_ffmpeg_bin ignores XDG ffmpeg_path"
  fi
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

# Pass N: the C-toolchain surface is PINNED, not merely "checked once".
#
# The product law is rust-native / self-contained, and the note in Cargo.toml used
# to claim `cc`/`cmake` arrived "only via the pre-existing TLS stack". Auditing
# every `*-sys` crate in the lock disproved it: SQLite (bundled), mimalloc and
# zstd compile C too. A claim nobody re-measures decays into folklore, so the
# allowlist below is the measurement, executable.
#
# Reads Cargo.lock directly: no network, no resolution, no build. `cargo tree`
# would be the obvious tool and is the wrong one here — it needs a resolve, and
# this gate runs inside verifier-controls sandboxes where that is pure latency.
SYS_ALLOWLIST=(
  # Real C compilation units — each one is a deliberate, documented trade.
  aws-lc-sys        # rustls provider; cc + cmake; upstream-pinned via hudsucker
  ring              # rcgen / MITM CA; cc + asm
  libsqlite3-sys    # rusqlite "bundled"; vendoring beats a system-library hunt
  libmimalloc-sys   # default allocator; see scripts/rss-baseline.sh
  zstd-sys          # async-compression via hudsucker "decoder"
  # Bindings only — no C is compiled for these.
  core-foundation-sys dirs-sys jni-sys js-sys libfuzzer-sys linux-raw-sys
  security-framework-sys web-sys windows-sys
)
observed_sys="$(rg -o '^name = "([a-z0-9_-]+-sys)"' -r '$1' Cargo.lock | sort -u)"
# `ring` has no `-sys` suffix, so the regex above cannot see it; add it explicitly
# rather than widening the pattern and dragging in every unrelated crate.
rg -q '^name = "ring"$' Cargo.lock && observed_sys="$(printf '%s\nring\n' "$observed_sys" | sort -u)"
unexpected_sys=""
for crate in $observed_sys; do
  found=0
  for allowed in "${SYS_ALLOWLIST[@]}"; do
    [[ "$crate" == "$allowed" ]] && { found=1; break; }
  done
  [ "$found" -eq 0 ] && unexpected_sys="$unexpected_sys $crate"
done
if [ -z "$unexpected_sys" ]; then
  pass "Pass N no unexpected *-sys / native crate in Cargo.lock"
else
  bad "Pass N new native dependency not in the documented allowlist:$unexpected_sys"
  echo "      Add it to SYS_ALLOWLIST with a justification, or remove the dependency."
  echo "      The Cargo.toml note above [dependencies] is the prose half of this gate."
fi

# openssl is forbidden outright; TLS is rustls-only.
if rg -q '^name = "openssl(-sys)?"$' Cargo.lock; then
  bad "Pass N openssl reached the graph (TLS must stay rustls-only)"
else
  pass "Pass N no openssl in Cargo.lock"
fi

# A system assembler must never become a build prerequisite (see the AVIF note).
if rg -q '^name = "nasm-rs"$' Cargo.lock; then
  bad "Pass N nasm-rs reached the graph (a system NASM would become required)"
else
  pass "Pass N no nasm-rs in Cargo.lock"
fi

# Good-news detector. cmake is the ONLY prerequisite beyond a C compiler, and it
# rides in on aws-lc-sys. When upstream stops pulling it, this fires so the
# prerequisite gets dropped from the docs instead of outliving its cause.
if rg -q '^name = "aws-lc-sys"$' Cargo.lock; then
  pass "Pass N aws-lc-sys present as documented (cmake stays a build prerequisite)"
else
  bad "Pass N aws-lc-sys is GONE — this is good news, not a defect."
  echo "      Drop cmake from the documented build prerequisites, update the"
  echo "      Cargo.toml note above [dependencies], and remove it from SYS_ALLOWLIST."
fi

if [ "$fail" -ne 0 ]; then
  echo "== natives-check FAILED =="
  exit 1
fi
echo "== natives-check OK =="
exit 0
