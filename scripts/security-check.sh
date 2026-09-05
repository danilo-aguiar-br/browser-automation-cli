#!/usr/bin/env bash
# Security gate: the three defects of the 2026-08-31 audit, pinned as invariants.
#
# WHY THIS GATE EXISTS
#   Each rule below encodes a defect that was FOUND, not one that was imagined.
#   Fixing the instances closes the instances; only a gate closes the class. The
#   filesize gate in this same repository failed in August, was fixed by editing
#   the files it named, and failed again four weeks later naming different files
#   — the mechanism had never been touched. These three are the mechanisms.
#
# CLEAN STDOUT: one status line on stdout; diagnostics on stderr.
set -uo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting. Clearing it neutralizes the whole file.
export RIPGREP_CONFIG_PATH=
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

FAILED=0
CHECKED=0

fail() {
  FAILED=1
  {
    echo "== security-check FAILED: $1 =="
    shift
    printf '%s\n' "$@"
  } >&2
}

# RULE 1 — no media artifact reaches disk without the allowed-root check.
#
#   Measured 2026-08-31 with an independent control: `image resize --out
#   /dev/shm/x.png` returned `ok: true` and the file existed, while `run
#   --script /dev/shm/x.ndjson` refused the SAME directory with exit 64. The
#   policy was live and this path went around it.
#
#   `/tmp` would have been an invalid target for that control, because
#   `fs_roots::default_roots` includes `std::env::temp_dir()` — the refusal has
#   to be demonstrated on a directory that is genuinely outside.
#
#   The funnel is `image_local::atomic::write_bytes_atomic`, which the whole
#   `image` family plus the `audio` and `video` downloaders write through.
CHECKED=$((CHECKED + 1))
if ! rg -q 'ensure_write_allowed' src/image_local/atomic.rs 2>/dev/null; then
  fail "media writer bypasses fs_roots" \
    "src/image_local/atomic.rs no longer calls ensure_write_allowed." \
    "Every image, audio and video artifact reaches disk through it, so the" \
    "allowed-root policy is off for all three families at once. A root that" \
    "one code path can walk around is not a root."
fi

# RULE 2 — a permission change never discards its own error.
#
#   `let _ = fs::set_permissions(...)` on the MITM root CA private key meant a
#   chmod that failed left that key world-readable, permanently and in silence.
#   A security control that cannot report its own failure is indistinguishable
#   from one that is absent.
#
#   The fix was structural — the file is born `0600` via `create_private_file`
#   — and this rule keeps the discarded-error shape from coming back.
#   The pattern is matched by the MODE and not by the call, and the first draft
#   of this rule got that wrong. Matching every discarded `set_permissions`
#   flagged a unit test doing `from_mode(0o755)` on a mock script — an
#   ELEVATION, where a discarded error means the test skips rather than a secret
#   leaks. A gate that reports that is a gate someone turns off, and the reason
#   this file gives for warning instead of failing on transitive advisories
#   applies to itself. So the match is restricted to modes that close group and
#   other, which is what protecting a secret looks like.
CHECKED=$((CHECKED + 1))
discarded="$(rg -n 'let _ = .*from_mode\(0o[0-7]00\)|let _ = .*restrict_to_owner' src/ 2>/dev/null || true)"
if [[ -n "$discarded" ]]; then
  fail "permission change with discarded error" \
    "$discarded" \
    "Propagate with \`?\`. Prefer platform::create_private_file, which makes" \
    "the restriction a property of creation instead of a later adjustment."
fi

# RULE 3 — no structured format is produced by string concatenation.
#
#   `config.toml` was built with `push_str(&format!("k = \"{v}\"\n"))` over nine
#   free-text keys. A value carrying a quote and a newline closes its string and
#   opens a LINE of TOML: `config set proxy_password` could declare
#   `ffmpeg_path`, and the next media command executes that binary. A config
#   write escalating to code execution.
#
#   Same class as SQL assembled with `format!`.
#
#   The fix is a refusal at the setter rather than an escape at the writer, and
#   the reader is why: `config_io` parses with `split_once('=')` and
#   `trim_matches`, which decodes no escapes at all, so escaping on write alone
#   would trade a security defect for a silent write/read divergence.
CHECKED=$((CHECKED + 1))
if ! rg -q 'reject_untransportable_value' src/xdg/config_ops/set.rs 2>/dev/null; then
  fail "config setter accepts untransportable values" \
    "src/xdg/config_ops/set.rs no longer refuses quotes, backslashes and" \
    "control characters. The config writer interpolates values into TOML, so" \
    "an unfiltered value can declare a key the operator never asked for —" \
    "including ffmpeg_path, which the next media command executes."
fi

# RULE 4 — every operator-named path is bounded on the READ side too.
#
#   Rule 1 closed the WRITE axis and the read axis stayed open, which is the
#   more instructive half: `parse /dev/shm/leak.txt` answered `ok: true` with the
#   file's text inline, so it was arbitrary file DISCLOSURE and not merely an
#   unbounded read. Measured 2026-08-31, with the same control: `run --script`
#   refused that identical directory with exit 64.
#
#   Fixing one axis and declaring the class closed is exactly how the write gap
#   survived its own audit. These five are the funnels every `parse`, `image`,
#   `video` and `audio` verb that names a file arrives through.
CHECKED=$((CHECKED + 1))
#   The list grew five -> eight -> ten, over THREE separate rounds, and each
#   round ended with the class being declared closed. It was wrong every time.
#
#   Round 2 added `sheet-write` and `workflow run --manifest`. Round 3 added
#   `batch-scrape --urls-file` and `scrape --schema-json`, and those two are the
#   ones that say why the method kept failing: the list was being built by
#   READING CODE and enumerating the helpers a reader could imagine. Each family
#   uses its own helper, so that method structurally undercounts. The surface to
#   enumerate is the list of COMMANDS that accept a path -- in this repo, the 15
#   PathBuf fields under `src/cli/` -- never the list of functions that read.
#
#   `--urls-file` is the worst of the ten and was found last: measured
#   2026-08-31, `batch-scrape --urls-file /etc/passwd` exited 0 with `ok: true`
#   and echoed all 59 lines inside `data.errors[].error`, because every
#   unfetchable line is reported verbatim. `--schema-json` was unguarded on the
#   `scrape` path while its `nav` twin was guarded, and a comment there asserted
#   a parity that did not exist.
#
#   Round 4 added three more, and they share one shape the earlier rounds did
#   not: the WRITE of the same path was already guarded and only the READ was
#   open. `monitor check --baseline` read the file with no check while writing
#   it through the guarded `write_bytes_sync` eight lines below, in the same
#   function. `SnapshotGraph::load` is the single funnel all eight `heap` verbs
#   reach, including the two paths `heap compare` takes at once.
#
#   Asymmetry inside one function is the cheapest thing to look for and it was
#   not looked for: the guarded write is a signpost saying the path is operator
#   input, sitting next to the unguarded read of that same variable.
#
#   Deliberate exclusions, so a later audit does not "fix" what is already right:
#   `mitm` reads its allowlist from `xdg::mitm_capture_dir()`, and the perf
#   analyser `native::perf_insight::analyze_file` is also handed a trace the
#   product generated itself. Both are internal paths a guard would wrongly
#   refuse.
#
#   `monitor_diff.rs` is excluded too, and the reason is worth the lines: its
#   `build_diff` reads `<baseline>.content`, but `content_path` derives that
#   from the SAME value `handle_monitor` already bounded, and the function has
#   no other caller and no error channel. A check there could only agree with
#   the one upstream or drift from it, and a refusal would have to be laundered
#   into `diff_available: false`, reporting a wrong reason.
#
#   `browser/helpers.rs` is excluded for a different reason: `verify_image_magic`
#   returns `bool` and reads at most `IMAGE_MAGIC_PROBE_BYTES`, so it echoes
#   nothing, and the path it is handed is what the product just wrote through an
#   already-guarded write. A guard there would refuse the product its own file.
#
#   `browser::session::media::perf` is in the list because `perf_insight_file`
#   is the operator-facing door. Measured 2026-08-31 it had ZERO callers and its
#   guard was unproven by execution; `perf insight --path` now reaches it, so
#   the guard is executable and can be proven.
read_funnels=(
  src/scrape_local/parse.rs
  src/scrape_local/urls.rs
  src/commands/scrape/page/llm.rs
  src/image_local/decode.rs
  src/image_local/ops.rs
  src/video_local/ops/source.rs
  src/audio_local/ops/source.rs
  src/sheet_local.rs
  src/workflow_local/dag.rs
  src/browser/session/media/perf.rs
  src/commands/scrape/local_fs.rs
  src/commands/nav/capture/monitor.rs
  src/native/heap_snapshot/graph/load.rs
)
missing_read=""
for f in "${read_funnels[@]}"; do
  rg -q 'ensure_read_allowed' "$f" 2>/dev/null || missing_read="$missing_read $f"
done
if [[ -n "$missing_read" ]]; then
  fail "read funnel bypasses fs_roots" \
    "these files no longer call ensure_read_allowed:$missing_read" \
    "Each one is a funnel an operator-named path reaches disk through. Without" \
    "the check, the command reads a file outside the allowed roots and returns" \
    "its contents with ok: true, which is disclosure rather than a bad read."
fi

# RULE 5 — operator-named WRITE destinations outside the media funnel.
#
#   Rule 1 pins `image_local::atomic`, which is the funnel the media family
#   shares. Two other commands name their own destination and reach disk through
#   a DIFFERENT helper, which is the same shape as the read-axis misses:
#   `mitm --har` writes through `mitm_local::util::atomic_write`, and
#   `sg-rewrite --apply` through a private `atomic_write` in `sg_local`.
#
#   The guard is NOT in `mitm_local::util::atomic_write` itself on purpose: its
#   three other callers write the root CA, the rules file and captured bodies to
#   directories the product derives, so bounding the shared helper would refuse
#   the product its own state. It is bounded in `har::export_har`, the one
#   caller whose destination comes from argv.
#
#   The `video`/`audio` transform families are pinned here because they escaped
#   the write axis in a way no search for a write primitive can find. Rule 1
#   pins `image_local::atomic`, whose own comment called it "the single funnel
#   every media artifact reaches disk through" -- false the day it was written.
#   It is the funnel of the DOWNLOADERS. The transform path hands `--out` to
#   ffmpeg as argv and lets the SUBPROCESS write, so `ensure_write_allowed` had
#   zero occurrences under `video_local/` and `audio_local/` while the write axis
#   was being called closed.
#
#   A path that becomes a subprocess argument is a write that no search for
#   `File::create` or `fs::write` finds. That is the lesson this entry exists to
#   keep.
CHECKED=$((CHECKED + 1))
#   Round 5 added `screencast`, and it is the third member of the delegated-write
#   family after the ffmpeg transforms and the lighthouse runner. `screencast
#   start --path` created the frame DIRECTORY through `create_dir_all_blocking`,
#   whose write twin `write_bytes_blocking` in the same module IS guarded, and
#   `screencast stop --path` handed the video target to ffmpeg as argv.
#
#   MEASURED 2026-08-31 against the release binary, before the fix:
#   `screencast start --path /dev/shm/scprobe` exited 0 with `"dir":
#   "/dev/shm/scprobe"` and the directory existed afterwards, outside the roots.
#
#   Both verbs funnel into `browser/session/media/screencast.rs`, and the run
#   step `{"cmd":"screencast"}` reaches the same two functions, so one guard per
#   function covers the CLI and the script surface at once.
#
#   Note which primitive escaped: `create_dir_all` is a write that creates no
#   file, so it appears in no search for `File::create`, `fs::write` or an
#   atomic-write helper. Directory creation is the third way a write hides.
write_funnels=(
  src/mitm_local/har.rs
  src/sg_local.rs
  src/video_local/ops/common.rs
  src/audio_local/ops/common.rs
  src/browser/session/media/screencast.rs
)
missing_write=""
for f in "${write_funnels[@]}"; do
  rg -q 'ensure_write_allowed' "$f" 2>/dev/null || missing_write="$missing_write $f"
done
if [[ -n "$missing_write" ]]; then
  fail "operator-named write destination bypasses fs_roots" \
    "these files no longer call ensure_write_allowed:$missing_write" \
    "Each one writes to a path the operator named, through a helper the media" \
    "funnel does not share. Without the check the command writes outside the" \
    "allowed roots and reports ok: true."
fi

if [[ $FAILED -ne 0 ]]; then
  echo "security-check: FAIL"
  exit 1
fi

echo "security-check: OK ($CHECKED invariants held)"
exit 0
