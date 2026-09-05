#!/usr/bin/env bash
# Orphan-module gate: every `src/**/*.rs` must be reachable from a crate root.
#
# WHY THIS GATE EXISTS
#   A Rust module is DECLARED, not discovered. A `.rs` file that no parent
#   declares with `mod` never enters the compilation graph, so it is not part of
#   the product — and nothing in the toolchain says so. `cargo build`, `cargo
#   clippy` and the entire test suite read the BINARY, and an undeclared file is
#   absent from the binary by construction. All three stay green.
#
#   Measured 2026-08-06: `src/scrape_local/content_kind.rs`, 276 code lines with
#   its own `#[cfg(test)] mod tests`, was written for G12 and never declared in
#   `src/scrape_local/mod.rs`. Build green, clippy green, 1095 tests green, and
#   the feature did not exist. In the same run `scripts/i18n-check.sh` FAILED on
#   `suggestion_key("scrape_opaque_content")` — that gate greps the SOURCE, so it
#   saw a key the binary can never emit.
#
#   One gate falsely green and another falsely red, both pointing at the same
#   dead file. The axis nothing measured was reachability.
#
# WHY NOT `cargo build` WITH A WARNING
#   There is no such warning. `dead_code` fires on unreachable items INSIDE the
#   graph; a file outside the graph is never parsed, so no lint can reach it.
#   The check has to compare the filesystem against the `mod` declarations, and
#   that comparison has no home in the compiler.
#
# THE RULE BEING ENFORCED (module resolution, not a heuristic)
#   - `src/foo.rs`        is declared by `mod foo;` in the crate root.
#   - `src/a/foo.rs`      is declared by `mod foo;` in `src/a/mod.rs` (or `src/a.rs`).
#   - `src/a/mod.rs`      is declared by `mod a;` in ITS parent, one level up.
#   - `#[path = "..."]`   overrides all of the above and is honoured explicitly.
#
#   Only the CORRECT parent counts. A `mod content_kind;` sitting in some other
#   module would not make `src/scrape_local/content_kind.rs` compile, so a
#   repo-wide grep for the name would be a false green of its own.
#
# CLEAN STDOUT: one status line on stdout; diagnostics on stderr.
set -euo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting (`--smart-case` widens matches, `--max-columns`
# truncates them away). Clearing the variable neutralizes the whole file; `-s`
# would close only one of those doors.
export RIPGREP_CONFIG_PATH=
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TARGET_DIR="${1:-src}"

# Crate roots need no declaration: Cargo names them in the manifest.
is_crate_root() {
  case "$1" in
  "$TARGET_DIR/lib.rs" | "$TARGET_DIR/main.rs") return 0 ;;
  *) return 1 ;;
  esac
}

# Files pulled in by `#[path = "..."]` anywhere in the tree. Collected once:
# the attribute is rare, and resolving it per-candidate would re-scan the tree.
#
# Paths in the attribute are relative to the DIRECTORY of the declaring file, so
# they are normalised against that directory rather than against the repo root.
#
# `declare -A` needs bash 4 and macOS ships bash 3.2, so the set is one path per
# line and membership below is an exact whole-line match (2026-09-04).
PATH_ATTR_TARGETS=""
while IFS= read -r hit; do
  [[ -z "$hit" ]] && continue
  decl_file="${hit%%:*}"
  rel="${hit#*:}"
  decl_dir="$(dirname "$decl_file")"
  # `realpath -m` resolves `..` without requiring the file to exist.
  abs="$(realpath -m "$decl_dir/$rel" 2>/dev/null || true)"
  [[ -z "$abs" ]] && continue
  PATH_ATTR_TARGETS="${PATH_ATTR_TARGETS}${abs#"$ROOT/"}"$'\n'
done < <(rg -n --no-heading -o '#\[path\s*=\s*"([^"]+)"\]' -r '$1' "$TARGET_DIR" 2>/dev/null |
  sd '^([^:]+):[0-9]+:' '$1:' || true)

# True when $2 declares module $1. Matches `mod x;`, `pub mod x;`,
# `pub(crate) mod x;` and inline `mod x {`, and tolerates leading indentation
# (a `#[cfg(...)] mod x;` inside a block is still a declaration).
declares_module() {
  local name="$1" parent="$2"
  [[ -f "$parent" ]] || return 1
  rg -q "^\s*(pub(\([^)]*\))?\s+)?mod\s+${name}\s*[;{]" "$parent"
}

echo "== orphan-module-check (target ${TARGET_DIR}) ==" >&2

orphans=0
checked=0
via_path_attr=0

while IFS= read -r file; do
  is_crate_root "$file" && continue

  # Honoured before the normal rule: `#[path]` deliberately breaks it.
  if printf '%s' "$PATH_ATTR_TARGETS" | rg -q -x -F -e "$file"; then
    via_path_attr=$((via_path_attr + 1))
    checked=$((checked + 1))
    continue
  fi

  dir="$(dirname "$file")"
  base="$(basename "$file" .rs)"

  if [[ "$base" == "mod" ]]; then
    # `a/b/mod.rs` is the module `b`, declared one level ABOVE `a/b`.
    name="$(basename "$dir")"
    owner_dir="$(dirname "$dir")"
  else
    name="$base"
    owner_dir="$dir"
  fi

  # Candidate parents, in Rust's own precedence: `dir/mod.rs`, then `dir.rs`,
  # then the crate roots when the owner is the crate directory itself.
  parents=()
  if [[ "$owner_dir" == "$TARGET_DIR" ]]; then
    parents+=("$TARGET_DIR/lib.rs" "$TARGET_DIR/main.rs")
  else
    parents+=("$owner_dir/mod.rs" "${owner_dir}.rs")
  fi

  checked=$((checked + 1))
  found=0
  for parent in "${parents[@]}"; do
    if declares_module "$name" "$parent"; then
      found=1
      break
    fi
  done

  if [[ "$found" -eq 0 ]]; then
    printf 'FAIL  %s  (no `mod %s;` in %s)\n' "$file" "$name" "${parents[*]}" >&2
    orphans=$((orphans + 1))
  fi
done < <(fd -e rs . "$TARGET_DIR" | sort)

{
  echo "----"
  printf 'checked=%d  via_path_attr=%d  orphans=%d\n' \
    "$checked" "$via_path_attr" "$orphans"
} >&2

if [[ "$orphans" -ne 0 ]]; then
  {
    echo "== orphan-module-check FAILED =="
    echo "An undeclared .rs file is NOT part of the product: it never reaches the"
    echo "compiler, so build, clippy and the whole test suite stay green over code"
    echo "that does not exist. Declare it with \`mod <name>;\` in the parent listed"
    echo "above, or delete the file."
  } >&2
  echo "orphan-module-check: FAIL"
  exit 1
fi

echo "orphan-module-check: OK (every module is reachable from a crate root)"
exit 0
