#!/usr/bin/env bash
# Config round-trip gate: every declared XDG key must survive being written and read.
#
# WHY THIS GATE EXISTS
#   `config set` mutates an in-memory `ProductConfig`, then `write_config`
#   rebuilds the ENTIRE `config.toml` from a hand-written template that names
#   each key one by one. A key absent from that template is not "left alone" —
#   it is dropped, because the file is rewritten from scratch every time. The
#   reader `apply_toml_kv` is the mirror image: a hand-written `match` whose
#   catch-all only rescues promoted policy keys, so a key absent from the arms
#   is discarded on load.
#
#   Neither half is derived from `CONFIG_KEYS`. Nothing keeps the three lists
#   aligned, and the failure is SILENT in the worst possible way: `config set`
#   answers `ok: true`, exit 0, and the next process reads back `null`.
#
#   Measured 2026-08-09: seventeen keys were missing from both halves, including
#   `browser_mode` and `stealth`. They were added. Measured 2026-08-10: SIX MORE
#   were still missing — `proxy_url`, `proxy_bypass`, `proxy_username`,
#   `proxy_password`, `stealth_seed`, `robots_user_agent`. Fixing instances did
#   not close the class, and the second sweep found the residue of the first.
#
#   Two of those six are proxy credentials. `docs/CONFIGURATION.md` and
#   `config_ops::key_entries` both tell the operator to keep them here BECAUSE
#   argv is visible in the process table. The channel documented as the safe one
#   silently discarded the value, leaving the leaky one as the only one that
#   worked. That is the cost of a class left open.
#
# WHY NOT A RUST TEST
#   A round-trip test needs a type-appropriate, validation-passing value for
#   each of ~97 keys, so in practice it degenerates into a hardcoded sample —
#   which is exactly what `tests/v018_parity_gate.rs` does with three keys, and
#   exactly why it missed the six. The invariant that actually matters is a
#   static set comparison: CONFIG_KEYS ⊆ writer AND CONFIG_KEYS ⊆ reader. A
#   static comparison belongs in a static check.
#
# WHAT IS CHECKED
#   For every key literal in `CONFIG_KEYS`:
#     writer — `src/xdg/config_write.rs` must contain either
#              `<key> = ` at the start of a template line, or
#              `"<key> = ` inside a `push_str` append.
#     reader — `src/xdg/config_io.rs` must contain `"<key>" =>`.
#   Promoted policy keys are exempt on both sides: they are generated from one
#   table by the `policy_knobs!` macro, which is the mechanism this gate exists
#   to substitute for where the macro does not reach.
#
# EXIT
#   0 every declared key is present on both sides
#   1 at least one key is missing; the key and the missing side are named
set -euo pipefail

# Gate determinism: the user's ripgrep config is outside version control and
# changes RESULTS, not formatting (`--smart-case` widens matches, `--max-columns`
# truncates them away). Clearing the variable neutralizes the whole file; `-s`
# would close only one of those doors.
export RIPGREP_CONFIG_PATH=

cd "$(dirname "${BASH_SOURCE[0]}")/.."

KEYS_FILE="src/xdg/config_ops/keys.rs"
# The writer is TWO files: the template in `config_write.rs` and the
# emitted-only-when-set keys in `config_write_optional.rs`. They split on
# 2026-08-10 when the six added keys pushed the first past the 300-line gate.
# Scanning only the first would report every optional key as missing, which is
# the instrument-drift failure this gate exists to prevent — applied to itself.
WRITER="src/xdg/config_write.rs"
WRITER_OPT="src/xdg/config_write_optional.rs"
READER="src/xdg/config_io.rs"

for f in "$KEYS_FILE" "$WRITER" "$WRITER_OPT" "$READER"; do
  if [[ ! -f "$f" ]]; then
    echo "== config-roundtrip-check FAILED: missing $f =="
    exit 1
  fi
done

echo "== config-roundtrip-check (CONFIG_KEYS vs writer and reader) =="

# Key literals live in the `CONFIG_KEYS` slice as `    "name",` lines. Taking the
# whole file would also catch `config_keys_description`, so the extraction is
# bounded to the slice body.
# `mapfile` is a bash 4 builtin and macOS ships bash 3.2, so the read loop
# below is the portable equivalent (2026-09-04).
KEYS=()
while IFS= read -r __line; do KEYS+=("$__line"); done < <(
  awk '
    /pub const CONFIG_KEYS/ { inside = 1; next }
    inside && /^\];/        { inside = 0 }
    inside && match($0, /"[a-z0-9_]+"/) {
      print substr($0, RSTART + 1, RLENGTH - 2)
    }
  ' "$KEYS_FILE"
)

if [[ ${#KEYS[@]} -eq 0 ]]; then
  echo "== config-roundtrip-check FAILED: extracted zero keys from $KEYS_FILE =="
  echo "   The slice shape changed; fix the extraction rather than deleting the gate."
  exit 1
fi

missing=0

for key in "${KEYS[@]}"; do
  # Template line (`  key = {placeholder}`) or conditional append (`"key = {v}\n"`).
  if ! rg -q "^\s*${key} = |\"${key} = " "$WRITER" "$WRITER_OPT"; then
    echo "MISSING WRITER  ${key}  (not emitted by write_config; value is dropped on every write)"
    missing=$((missing + 1))
  fi
  if ! rg -q "\"${key}\" =>" "$READER"; then
    echo "MISSING READER  ${key}  (no arm in apply_toml_kv; value is discarded on load)"
    missing=$((missing + 1))
  fi
done

printf 'checked=%d  missing=%d\n' "${#KEYS[@]}" "$missing"

if [[ "$missing" -gt 0 ]]; then
  echo "== config-roundtrip-check FAILED: ${missing} key/side pair(s) missing =="
  echo "   A declared key absent from either side makes \`config set\` answer ok"
  echo "   while the value never survives the process. Add the key to both."
  exit 1
fi

echo "== config-roundtrip-check OK =="
