#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Resolve a Rust module to wherever it currently lives (GAP-051 fallout).
#
# # The bug this exists to kill
#
# A verifier written as `rg -q 'fn init_tracing_local' src/tracing_local.rs`
# measures LOCATION. The property it wants to measure is EXISTENCE OF
# BEHAVIOUR. The two agree only until someone splits the module, and then the
# gate fails with `No such file or directory` while printing a message that
# blames a missing function which is right there and working. Worse: the
# obvious "fix" is to delete the check, and then it passes forever without the
# property.
#
# Rust gives a module two legal shapes, `src/x.rs` and `src/x/`, and both are
# the same module. A verifier must accept both.
#
# # Contract
#
# `mod_path src/x` echoes `src/x.rs` or `src/x`, whichever exists.
#
# When NEITHER exists it echoes a sentinel path that cannot exist and warns on
# stderr. That is deliberate: `rg` then fails on the missing path and the check
# goes red. A module that vanished must break the gate LOUDLY. Echoing nothing
# would hand `rg` an empty argument, which makes it read stdin and hang.

if [[ -n "${_BAC_MODULE_PATHS_SOURCED:-}" ]]; then
  return 0
fi
_BAC_MODULE_PATHS_SOURCED=1

# mod_path <module-root-without-extension>  →  path on stdout
mod_path() {
  local base="$1"
  if [ -f "$base.rs" ]; then
    printf '%s\n' "$base.rs"
    return 0
  fi
  if [ -d "$base" ]; then
    printf '%s\n' "$base"
    return 0
  fi
  printf 'mod_path: no module at %s.rs nor %s/\n' "$base" "$base" >&2
  printf '%s\n' "$base.__MODULE_NOT_FOUND__"
  return 1
}

# files_defining <regex> [root]  →  every file under root whose text matches
#
# For assertions of the form "X must be wired where Y is defined". Anchoring
# those on a file name breaks the moment `Y` moves; anchoring on `Y` itself
# follows it. Echoes nothing and returns non-zero when `Y` is gone, which is the
# correct failure: the anchor itself disappeared.
files_defining() {
  local pattern="$1" root="${2:-src/}" found
  found="$(rg -l --glob '*.rs' "$pattern" "$root" 2>/dev/null || true)"
  if [ -z "$found" ]; then
    printf 'files_defining: nothing matches %s under %s\n' "$pattern" "$root" >&2
    return 1
  fi
  printf '%s\n' "$found"
}

# crate_doc_files  →  every file that makes up the crate-level documentation
#
# `//!` prose can live in `lib.rs` or in a file spliced in with
# `#![doc = include_str!("...")]`, and both render as the same page. A gate that
# reads only `lib.rs` therefore reports missing sections that are present.
# This follows the `include_str!` directive instead of guessing the file name.
crate_doc_files() {
  local lib="src/lib.rs" inc
  [ -f "$lib" ] || { printf 'crate_doc_files: no %s\n' "$lib" >&2; return 1; }
  printf '%s\n' "$lib"
  # Only `#![doc = include_str!(...)]` counts: an inner attribute on the crate.
  inc="$(rg -o '#!\[doc *= *include_str!\("([^"]+)"\)\]' -r '$1' "$lib" || true)"
  local rel
  for rel in $inc; do
    [ -f "src/$rel" ] && printf '%s\n' "src/$rel"
  done
}

# POSITIVE CONTROL for the resolver itself.
#
# Re-anchoring a gate can turn "red for the wrong reason" into "green for the
# wrong reason", which is worse because nobody investigates green. This proves,
# every run, that `mod_path` still resolves a module that exists AND still
# refuses one that does not. Call it before using `mod_path` for assertions.
module_paths_self_test() {
  local probe missing
  probe="$(mod_path src/lib 2>/dev/null || true)"
  if [ ! -e "$probe" ]; then
    printf 'FAIL  mod_path self-test: cannot resolve src/lib (got %s)\n' "$probe" >&2
    return 1
  fi
  missing="$(mod_path src/__definitely_not_a_module__ 2>/dev/null || true)"
  if [ -e "$missing" ]; then
    printf 'FAIL  mod_path self-test: resolved a module that does not exist\n' >&2
    return 1
  fi
  if files_defining '__definitely_not_a_symbol__' src/ >/dev/null 2>&1; then
    printf 'FAIL  mod_path self-test: files_defining matched an absent symbol\n' >&2
    return 1
  fi
  if ! files_defining 'pub fn ' src/ >/dev/null 2>&1; then
    printf 'FAIL  mod_path self-test: files_defining found no public fn in src/\n' >&2
    return 1
  fi
  return 0
}
