#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# POSITIVE CONTROLS for the verifiers re-anchored after GAP-051.
#
# # Why this file exists
#
# Seven verifiers were anchored on FILE PATHS (`rg 'fn init_tracing_local'
# src/tracing_local.rs`). Splitting a module into a directory made every one of
# them fail with `No such file or directory` while printing a message that
# blamed a missing function that was present and working. Red for the wrong
# reason.
#
# Re-anchoring them on symbols fixes that — and opens a worse failure mode. A
# widened search can match something unrelated, or an anchor can be deleted
# outright, and the verifier then passes forever. GREEN for the wrong reason is
# worse than red for the wrong reason, because nobody investigates green.
#
# So each re-anchored assertion gets a control here: copy the tree, DELETE the
# property, run the verifier, and require it to complain. A control that does
# not fail is a verifier that does not verify.
#
# This is mutation testing scoped to the gates. It never touches the real tree.
#
# Usage: ./scripts/verifier-controls-check.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail=0
pass() { printf 'PASS  %s\n' "$1"; }
bad() { printf 'FAIL  %s\n' "$1"; fail=1; }

echo "== verifier-controls (each gate must DETECT the property's absence) =="

# run_control <label> <script+args> <expected-substring> <mutation-command...>
#
# The mutation runs inside a throwaway copy with $WORK as cwd. The verifier is
# then run there and must print <expected-substring>. Matching on the MESSAGE
# rather than the exit code keeps the control honest for gates that continue
# past a failed assertion to run cargo, which would muddy the exit status.
run_control() {
  local label="$1" script="$2" expected="$3"
  shift 3
  local work
  work="$(mktemp -d "${TMPDIR:-/tmp}/bac-verifier-control-XXXXXX")"
  # Copy the whole working tree minus the heavy, irrelevant parts.
  #
  # A partial copy is a trap: a gate that aborts early because the sandbox lacks
  # `benches/` or `llms.txt` never reaches the assertion under control, and the
  # control then reports the gate as blind when it is merely unreachable. That
  # false alarm cost two rounds here — the sandbox must be complete enough that
  # the gate fails for the reason the mutation created.
  fd -H -d 1 . "$ROOT" \
    -E target -E .git -E '*.sqlite*' -E '*.7z' \
    -E 'base_conhecimento*' -E 'docs_rules*' -E graphrag -E node_modules \
    -x cp -r {} "$work/" \; 2>/dev/null

  (cd "$work" && "$@") || {
    bad "$label: mutation command failed, control is inconclusive"
    rm -rf "$work"
    return
  }

  local out
  # Share the real target dir so a gate that runs cargo reuses the existing
  # build instead of compiling the dependency graph from scratch per control.
  #
  # Also hand the sandbox a binary from THIS tree. Every gate that inspects the
  # live product resolves `$BIN` relative to its own `$ROOT`, and the sandbox is
  # copied WITHOUT `target/`, so the lookup fell through to whatever
  # `browser-automation-cli` sits on `PATH` — usually a release from an older
  # version. The gate then measured a product that is not the one under test,
  # and reported flags as missing that the tree declares. Newest of debug and
  # release wins, because either may be the one just built.
  local tree_bin=""
  local candidate
  for candidate in "$ROOT/target/debug/browser-automation-cli" \
                   "$ROOT/target/release/browser-automation-cli"; do
    if [[ -x "$candidate" ]]; then
      if [[ -z "$tree_bin" || "$candidate" -nt "$tree_bin" ]]; then
        tree_bin="$candidate"
      fi
    fi
  done

  # shellcheck disable=SC2086  # $script carries its own flags on purpose
  out="$(cd "$work" && CARGO_TARGET_DIR="$ROOT/target/verifier-controls" \
    BIN="$tree_bin" timeout 900 bash $script 2>&1 || true)"
  if printf '%s' "$out" | rg -qF "$expected"; then
    pass "$label"
  else
    bad "$label: gate stayed silent after the property was removed"
    printf '      expected to see: %s\n' "$expected"
  fi
  rm -rf "$work"
}

# 1) tracing-check must notice init_tracing_local disappearing.
run_control "tracing-check detects missing init_tracing_local" \
  "scripts/tracing-check.sh --inventory-only" \
  "missing tracing_local module / init_tracing_local" \
  bash -c "sd 'fn init_tracing_local' 'fn renamed_away' \$(rg -l 'fn init_tracing_local' src/)"

# 2) tracing-check must notice the correlation span leaving the entry surface.
run_control "tracing-check detects missing cli_run span" \
  "scripts/tracing-check.sh --inventory-only" \
  "missing cli_run correlation span" \
  bash -c "sd 'cli_run' 'span_renamed' \$(rg -l 'cli_run' src/)"

# 3) shutdown-check must notice the dual flush going away.
run_control "shutdown-check detects missing dual flush" \
  "scripts/shutdown-check.sh --inventory-only" \
  "missing dual flush before exit" \
  bash -c "sd 'flush_stdout' 'flush_renamed' \$(rg -l 'flush_stdout' src/)"

# 4) process-check must notice lighthouse dropping the timed capture helper.
run_control "process-check detects lighthouse without run_capture_with_timeout" \
  "scripts/process-check.sh" \
  "lighthouse/ffmpeg missing run_capture_with_timeout" \
  bash -c "sd 'run_capture_with_timeout' 'capture_renamed' \$(rg -l 'run_capture_with_timeout' src/commands/ops/lighthouse)"

# 5) network-check must notice read_body_limited losing its wiring.
run_control "network-check detects unwired read_body_limited" \
  "scripts/network-check.sh" \
  "read_body_limited not wired" \
  bash -c "sd 'read_body_limited' 'body_renamed' \$(rg -l 'read_body_limited' src/robots)"

# 6) ownership-check must notice the pages() slice API vanishing.
run_control "ownership-check detects missing pages() slice API" \
  "scripts/ownership-check.sh" \
  "missing pages() slice API" \
  bash -c "sd 'fn pages' 'fn pages_renamed' \$(rg -l 'fn pages' src/native/browser/tabs)"

# 7) natives-check must notice its lighthouse property being removed.
run_control "natives-check detects missing lighthouse process helper" \
  "scripts/natives-check.sh" \
  "Pass M process helper missing or not wired" \
  bash -c "sd 'run_capture_with_timeout' 'capture_renamed' \$(rg -l 'run_capture_with_timeout' src/commands/ops/lighthouse)"

# 8) json-ndjson-check must notice a raw parser reappearing in a SPLIT module.
#
# This is the control that matters most here: before the fix the gate reported
# "no raw serde_json::from_str" for `discovery` and `lighthouse` because it read
# paths that no longer existed. It was green while asserting nothing.
run_control "json-ndjson-check detects raw serde_json in a split module" \
  "scripts/json-ndjson-check.sh" \
  "still calls serde_json::from_str directly" \
  bash -c "printf '\nfn __control() { let _ = serde_json::from_str::<serde_json::Value>(\"{}\"); }\n' >> src/native/cdp/discovery/mod.rs"

# 9) docs-check must notice the aquamarine feature gate being dropped.
#
# The mutation drops the gate and keeps the cfg VALID. It used to rename the
# feature to `docs-mermaid` -> `other`, which is not a declared feature, so
# `unexpected_cfgs` fired under `-D warnings` and `cargo +nightly doc` failed in
# an EARLIER phase than the assertion under control. The gate then reported a
# doc failure instead of the ungated attribute, the control saw the wrong
# message, and the verdict read "gate stayed silent" — the exact
# unreachable-assertion trap this file's own header warns about.
#
# `all(doc, feature = "docs-mermaid")` -> `doc` ungates aquamarine while
# remaining something rustc accepts, so phase 5 is actually reached.
run_control "docs-check detects ungated aquamarine" \
  "scripts/docs-check.sh" \
  "aquamarine must be gated behind feature docs-mermaid" \
  bash -c "sd 'all\(doc, feature = \"docs-mermaid\"\)' 'doc' \$(rg -l 'docs-mermaid' src/ --glob '*.rs')"

# 10) natives-check Pass N must notice a NEW native dependency arriving.
#
# The allowlist exists because the previous claim ("cc/cmake only via TLS") was
# folklore that nobody re-measured — SQLite, mimalloc and zstd compile C too. An
# allowlist that cannot detect an addition would be the same folklore in list form.
run_control "natives-check detects a new *-sys dependency" \
  "scripts/natives-check.sh" \
  "new native dependency not in the documented allowlist" \
  bash -c "printf '\n[[package]]\nname = \"totally-new-sys\"\nversion = \"1.0.0\"\n' >> Cargo.lock"

# 11) natives-check must notice openssl reaching the graph.
run_control "natives-check detects openssl in the graph" \
  "scripts/natives-check.sh" \
  "openssl reached the graph" \
  bash -c "printf '\n[[package]]\nname = \"openssl\"\nversion = \"0.10.0\"\n' >> Cargo.lock"

# 12) natives-check must notice aws-lc-sys LEAVING, which retires a prerequisite.
#
# The only control here that fires on good news. Without it, cmake would stay in
# the documented prerequisites forever after upstream stopped requiring it.
run_control "natives-check detects aws-lc-sys disappearing" \
  "scripts/natives-check.sh" \
  "aws-lc-sys is GONE" \
  bash -c "sd 'name = \"aws-lc-sys\"' 'name = \"aws-lc-sys-renamed\"' Cargo.lock"

# The agent-ops gate is the newest verifier and it exists precisely because a
# green unit suite coexisted with a broken binary. Giving it no control here
# would repeat the mistake one level up: a gate nobody proved can detect the
# absence of the property it claims to check.
#
# Deleting the FTL suggestion's `--fields` and putting `--select` back is the
# exact regression the gate was written for — `--select` is a real flag on
# scrape/crawl/map/search, so a naive "flag exists somewhere" check passes and
# only the scope-aware one fires.
run_control "agent-ops-check detects a suggestion citing a non-global flag" \
  "scripts/agent-ops-check.sh" \
  "absent from the global help" \
  bash -c "sd -- '--fields' '--select' locales/en.ftl"

# doc-coverage-check: the documentation surface is the largest thing in the
# product with no gate before this wave — 132 of 176 XDG keys appeared in no
# public document. Three controls, one per class of drift, because the three
# assertions fail for different reasons and a single mutation would only prove
# one of them alive.
# `heap_dominator_max_states` is chosen because it appears EXACTLY ONCE in the
# reference. A key such as `dialog_settle_ms` also appears inside an example
# command, so deleting its definition would leave the gate finding the other
# occurrence and passing — an inconclusive control that reads like a healthy one.
# The replacement must not begin with a hyphen either: `sd` parses that as a
# flag and exits 2, which the harness reports as "mutation failed" rather than
# as a gate defect. Both traps fired here before this comment existed.
# NEVER pin a live COUNT in an expected string.
#
# These two controls used to expect `omits 1 of 176 live XDG keys` and `never
# names 1 of 69 live commands`. The counts are printed by the gate from the LIVE
# binary, so the day the product grew to 185 keys the control failed — and it
# failed while reporting "gate stayed silent", which reads as a blind verifier.
# The verifier was fine. The INSTRUMENT had aged. Chasing that cost a full
# investigation into a gate that was working correctly the whole time.
#
# The sibling command control only survived because 69 happened not to change,
# which is luck, not design.
#
# Anchor on the property instead: the gate must name the key it could not find.
# That string carries no count, so it survives the product growing, and it is
# STRICTER than the old one because it proves the gate found the right key
# rather than merely some key. If the mutated key is later renamed in the
# product, `sd` matches nothing, the copy stays unmutated, and the control
# reports silence — which is the correct alarm, not a false one.
# The phantom-flag scan is itself a re-anchored gate, so it needs its own
# control. Two mutations, one per property it asserts.
#
# It moved out of `agent-ops-check.sh` on 2026-08-10: it was a Python script, and
# the tool must be Rust end to end. These two controls kept pointing at the old
# host for exactly one run and reported "gate stayed silent" — correctly, because
# the property really had left that gate. Moving a check without moving its
# control is how a gate quietly becomes a stamp; the control caught it in the
# same round, which is what controls are for.
#
# Mutation A puts a flag nobody declares into the English catalog. This is the
# `--proxy` defect reproduced deliberately: prose that names a route the product
# does not offer.
run_control "phantom-flag-scan detects a suggestion citing an undeclared flag" \
  "scripts/phantom-flag-gate.sh every_cited_flag_exists_among_the_declared_flags" \
  "which the product does not declare" \
  bash -c "sd 'config list-keys' 'config list-keys --nonexistent-flag' src/i18n/en.rs"

# Mutation B rebuilds suggestion prose outside the catalog, which is how the
# original defect escaped a gate that only ever read the FTL files.
run_control "phantom-flag-scan detects suggestion prose assembled outside the catalog" \
  "scripts/phantom-flag-gate.sh no_suggestion_prose_is_assembled_outside_the_catalog" \
  "builds suggestion prose naming a flag outside" \
  bash -c "sd 'crate::i18n::suggestion_key\(\"perf_trace_path\", None\),' '\"Pass a path from perf stop --path\",' src/browser/session/media/perf.rs"

run_control "doc-coverage-check detects an undocumented XDG key" \
  "scripts/doc-coverage-check.sh" \
  "live XDG keys (first: heap_dominator_max_states)" \
  bash -c "sd 'heap_dominator_max_states' 'KEY_REMOVED_FOR_CONTROL' docs/CONFIGURATION.md"

run_control "doc-coverage-check detects a command that no entry-point document names" \
  "scripts/doc-coverage-check.sh" \
  "live commands (first: screencast)" \
  bash -c "sd -- '\`screencast' 'SCREENCAST_REMOVED_FOR_CONTROL' README.md"

# The scope assertion is the one most likely to rot into a false-green, because
# the naive version of it passes: `--select` really does exist on `scrape`.
run_control "doc-coverage-check detects a per-command flag presented as global" \
  "scripts/doc-coverage-check.sh" \
  "present a per-command flag as global" \
  bash -c "printf '%s\n' '- These flags are GLOBAL on every one of the 69 commands: \`--select\`' >> docs/ROADMAP.md"

# macros-check must notice a production panic! outside every test block.
#
# Check 9 used to filter by LINE, so a panic! inside an inline
# `#[cfg(test)] mod tests` read as production code, and the remedy at the time
# was to tolerate whole paths: src/cache/, src/lifecycle/, src/concurrency/,
# src/sync_util.rs. Making the gate block-aware let all four tolerances be
# DELETED — and a rewrite that widens what a gate ignores is exactly the shape
# that goes green forever.
#
# So the mutation plants the panic! in src/cache/, one of the paths that used to
# be tolerated wholesale, at file scope where no `#[cfg(test)]` covers it. If the
# block tracker ever swallows a production panic, or if a path tolerance creeps
# back, this control stops passing.
run_control "macros-check detects a production panic!" \
  "scripts/macros-check.sh" \
  "unexpected panic! in production code" \
  bash -c "printf '\nfn __control_panic() { panic!(\"control\"); }\n' >> src/cache/mod.rs"

# The defect this control models is the one that shipped twice: a key declared in
# CONFIG_KEYS but absent from the writer, so `config set` answers ok and the value
# never survives the process. Deleting the emission line is exactly that state.
run_control "config-roundtrip-check detects a key dropped by the writer" \
  "scripts/config-roundtrip-check.sh" \
  "MISSING WRITER" \
  bash -c "sd -F '\"proxy_url = ' '\"proxy_url_control = ' src/xdg/config_write_optional.rs"

# Same invariant, other side: an arm removed from apply_toml_kv means the value is
# written to disk and then discarded on load, which reads as "it forgot my setting".
run_control "config-roundtrip-check detects a key dropped by the reader" \
  "scripts/config-roundtrip-check.sh" \
  "MISSING READER" \
  bash -c "sd -F '\"stealth_seed\" =>' '\"stealth_seed_control\" =>' src/xdg/config_io.rs"

# The count assertion is the one whose SIBLING passes while it fails, which is
# the shape that hides longest: assertion 1 confirmed all 204 keys were present
# on the same run that the prose still said 176. So the mutation must not touch
# any key — it plants a sentence that merely COUNTS wrong.
#
# The expected string quotes the STALE number the mutation writes, never the
# live one. This file already paid for that lesson twice: two controls pinned
# `176` and `69` from the gate's own output, and the day the product grew they
# failed while reporting "gate stayed silent", which reads as a blind verifier.
# `176 XDG keys` is fixed by the mutation itself, so it survives the product
# growing to any size.
#
# The sentence carries no version token, because a version-bound line is history
# and the gate exempts it by design; a mutation that wrote `0.1.7 had 176 XDG
# keys` would be correctly ignored and the control would report false silence.
run_control "doc-coverage-check detects a stale count transcribed into prose" \
  "scripts/doc-coverage-check.sh" \
  'says "176 XDG keys"' \
  bash -c "printf '%s\n' '- The product exposes 176 XDG keys.' >> docs/ROADMAP.md"

# The embedded skills ship inside the crate, so their drift reaches an agent
# directly. `heap_dominator_max_states` is reused here for the reason the
# sibling control states: it appears EXACTLY ONCE in the reference, so deleting
# it cannot leave a second occurrence behind that keeps the gate green.
#
# The expected string is the KEY NAME with no count attached — the failure has
# to be actionable, and a control that accepted a bare number would let the gate
# regress into reporting one.
run_control "doc-coverage-check detects a live XDG key missing from the embedded skill" \
  "scripts/doc-coverage-check.sh" \
  "missing: heap_dominator_max_states" \
  bash -c "sd 'heap_dominator_max_states' 'KEY_REMOVED_FOR_CONTROL' skills/browser-automation-cli-en/references/xdg-keys.md"

# `--headed` is chosen because it is present in BOTH skills today, so the
# mutation is what creates the gap. Picking a flag that is already missing would
# produce a control that passes no matter what the mutation does — green for the
# wrong reason, which is the failure mode this whole file exists to prevent.
run_control "doc-coverage-check detects a global flag missing from the embedded skills" \
  "scripts/doc-coverage-check.sh" \
  "never names --headed" \
  bash -c "sd -F -- '--headed' 'FLAG_REMOVED_FOR_CONTROL' \$(rg -l -F -- '--headed' skills/)"

# The sibling control above proves the SKILL side. This one proves the human
# contract side, and it exists because assertion 8 searches its four usage
# documents in one ripgrep call: an OR that answers "written down anywhere"
# while reading like "written where the agent looks".
#
# The mutation touches ONLY the English contract document. That is the whole
# point — if assertion 12 ever collapses back into searching the bilingual pair
# together, the Portuguese file still carries `--correlation-id` and the gate
# goes quiet while a real gap sits in the English one.
#
# `--correlation-id` is chosen for the reason the sibling states: it occurs
# EXACTLY ONCE in `docs/AGENTS.md`, so the rename cannot leave a second
# occurrence behind that keeps the gate green for the wrong reason.
run_control "doc-coverage-check detects a global flag missing from one agent contract document" \
  "scripts/doc-coverage-check.sh" \
  "docs/AGENTS.md never names --correlation-id" \
  bash -c "sd -F -- '--correlation-id' 'FLAG_REMOVED_FOR_CONTROL' docs/AGENTS.md"

# `scripts/audit_bilingual_docs.sh` compares the two languages by their COMMAND
# INVOCATIONS, so a section of pure explanation can exist in one language only
# and that audit stays green. This control models exactly that: the mutation
# adds a heading with NO command in it, which the invocation audit cannot see.
#
# `ARCHITECTURE` is chosen because it is in structural parity today and is not
# a document whose heading count is expected to move on every release, so the
# control keeps modelling a defect rather than tracking product growth.
#
# The expected string omits the heading NUMBERS on purpose. Pinning "26 headings"
# would make the control fail the day the document legitimately grows, and this
# file has already paid for that lesson with two counts pinned from gate output.
run_control "doc-coverage-check detects a section that exists in one language only" \
  "scripts/doc-coverage-check.sh" \
  "bilingual drift: docs/ARCHITECTURE.md has" \
  bash -c "printf '\n## Control Section\n- control bullet with no command in it\n' >> docs/ARCHITECTURE.md"

# Assertion 13 discovers bilingual pairs through the `X.md` / `X.pt-BR.md`
# sibling convention. The embedded skills express the SAME invariant through a
# different one — two sibling directories, `-en` and `-pt` — so that assertion
# is structurally unable to see them, and no other assertion compares the two
# skills to each other. Measured on 2026-08-10: 54 headings against 53, with a
# whole section of ready-made `run` payloads present in English only.
#
# The mutation touches ONLY the English skill. If assertion 14 ever collapses
# into comparing each skill against the binary instead of against its sibling,
# the Portuguese file still matches the binary and this control goes silent.
run_control "doc-coverage-check detects a section present in one skill language only" \
  "scripts/doc-coverage-check.sh" \
  "skill pair drift: skills/browser-automation-cli-en/SKILL.md has" \
  bash -c "printf '\n## Control Section\n- control bullet\n' >> skills/browser-automation-cli-en/SKILL.md"

# A skill in this repository carries instruction and inline references, never
# code. Nothing measured that before: the fenced-block ban lived in prose and in
# review habit, so the first fence to land would have shipped unnoticed.
run_control "doc-coverage-check detects a fenced code block inside a skill" \
  "scripts/doc-coverage-check.sh" \
  "carries a fenced code block" \
  bash -c "printf '\n\`\`\`bash\necho control\n\`\`\`\n' >> skills/browser-automation-cli-pt/SKILL.md"

# A skill is a CONSOLIDATED instruction set, so a gap number or release label
# dates content that must read as the permanent contract of the tool. Measured
# on 2026-08-10: `GAP-054` sat in both `references/xdg-keys.md` files and no
# gate objected, because every assertion until now audited coverage and none
# audited the KIND of content the skill is allowed to carry.
run_control "doc-coverage-check detects a version-specific identifier inside a skill" \
  "scripts/doc-coverage-check.sh" \
  "cites a version-specific gap identifier" \
  bash -c "printf -- '- control line citing GAP-999\n' >> skills/browser-automation-cli-en/references/formulas.md"

if [ "$fail" -eq 0 ]; then
  echo "== verifier-controls OK =="
  exit 0
fi
echo "== verifier-controls FAILED ==" >&2
exit 65
