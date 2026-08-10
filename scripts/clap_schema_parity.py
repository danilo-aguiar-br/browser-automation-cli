#!/usr/bin/env python3
"""Clap-vs-schema parity: every flag clap accepts must appear in `schema --cmd`.

WHY THIS EXISTS AS A SECOND GATE
  `schema-drift-check.sh` compares `docs/schemas/*.json` against the live binary
  and is correct on its own axis. It cannot see this failure, because both sides
  of that comparison derive from the SAME hand-written schema module: the
  generator asks the binary, the binary asks `src/commands/meta/schema/*.rs`.
  When a flag is added to the clap enum and wired in the dispatcher, nothing in
  that loop forces a schema edit, so the file and the binary agree perfectly
  about a surface neither of them describes.

  An agent discovers what it may pass by reading `schema`. A flag absent there
  does not exist for the agent, whatever the parser happens to accept.

WHAT THIS GATE FIRST REPORTED, AND WHY IT WAS WRONG
  The first cut reported 29 undocumented flags across `console`, `heap`, `mitm`,
  `net`, `perf` and `storage`, and this docstring asserted that
  `schema --cmd console` published only `action`, `id` and `path`.

  Re-measured 2026-08-06 against the same binary: the top level does carry only
  those three, but `--page-idx`, `--page-size`, `--types`, `--include-preserved`
  and `--service-worker-id` are all published under
  `action.actions.list.properties`. The schema described them the whole time.

  Every one of the 29 was a false positive with a single cause: the clap side
  unions `<cmd> --help` with `<cmd> <sub> --help`, so it sees two levels, while
  the schema side read one. The six failing commands are exactly the six that
  dispatch on an action word -- which is the signature of a depth bug, not of a
  documentation gap. See `_collect_flags`.

  Recorded rather than deleted, because the failure mode matters more than the
  finding: a gate is a measuring instrument, and an instrument that disagrees
  with the artifact is not evidence about the artifact until the instrument
  itself has been checked.

WHY `--help` AND NOT A PARSE OF THE RUST SOURCE
  The same reason the drift gate trusts the binary: `--help` is what clap
  actually built, so it cannot disagree with the parser. A regex over
  `src/cli/` would introduce a third opinion, and a third opinion is a third
  thing that can be wrong on its own.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys

# A flag line in clap help: two or more leading spaces, an optional short alias,
# then the long name. `--flag[=<V>]`, `--flag <V>` and bare `--flag` all match.
FLAG_RE = re.compile(r"^\s{2,}(?:-[a-zA-Z], )?(--[a-z0-9][a-z0-9-]*)")

# A subcommand line inside the `Commands:` block: two spaces, the name, then at
# least two spaces before its description.
SUBCMD_RE = re.compile(r"^\s{2}([a-z][a-z0-9-]*)\s{2,}\S")

# Sections that follow the command's own `Options:` and carry global flags only.
# Kept for documentation value; the global set is subtracted wholesale below, so
# correctness does not depend on this list being complete.
GLOBAL_SECTIONS = ("Output:", "Timeouts:", "Capabilities:", "Network:")


def run(binary: str, args: list[str]) -> str:
    """Capture stdout+stderr of a help invocation; clap may use either."""
    proc = subprocess.run(
        [binary, *args],
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.stdout + proc.stderr


def flags_in(text: str) -> set[str]:
    """Every long flag clap prints for this invocation."""
    return {m.group(1) for line in text.splitlines() if (m := FLAG_RE.match(line))}


def subcommands_in(text: str) -> set[str]:
    """Names listed under the `Commands:` block, if the command has one."""
    out: set[str] = set()
    in_block = False
    for line in text.splitlines():
        if line.startswith("Commands:"):
            in_block = True
            continue
        if in_block:
            # A non-indented line ends the block (`Options:`, `Arguments:`, ...).
            if line and not line.startswith(" "):
                break
            if m := SUBCMD_RE.match(line):
                out.add(m.group(1))
    return out - {"help"}


def _collect_flags(props: dict, out: set[str]) -> None:
    """Add every long flag a properties object declares, at any nesting depth.

    WHY THE WALK RECURSES
      A command that dispatches on an action word keeps each action's arguments
      under `action.actions.<name>.properties`, not at the top level. Reading
      only the top level while the clap side already unions `<cmd> <sub> --help`
      compares two different depths, and every nested flag is then reported as
      undocumented.

      Measured 2026-08-06: that asymmetry alone produced all 29 findings of this
      gate's first cut. `storage export --path` was cited as unreachable by an
      agent while the schema published it at
      `action.actions.export.properties.path`; `console list --page-idx` was
      cited the same way and sits at `action.actions.list.properties.page_idx`.

      This is the fictional-debt failure named in the module docstring, repeated
      one level down: an instrument that reads the wrong depth does not merely
      miss real debt, it invents debt that costs real work to disprove. The
      first cut of this gate paid that price on the `argv` field; this is the
      same lesson on the nesting axis.
    """
    for key, prop in props.items():
        if not isinstance(prop, dict):
            out.add("--" + key.replace("_", "-"))
            continue
        argv = prop.get("argv", "")
        if isinstance(argv, str) and argv.startswith("--"):
            out.add(argv)
        else:
            out.add("--" + key.replace("_", "-"))
        actions = prop.get("actions")
        if isinstance(actions, dict):
            for spec in actions.values():
                if isinstance(spec, dict):
                    nested = spec.get("properties")
                    if isinstance(nested, dict):
                        _collect_flags(nested, out)


def schema_flags(binary: str, command: str) -> set[str] | None:
    """Long flags the published schema declares for `command`.

    Returns None when the command has no schema at all, which is a different
    failure from a schema that omits flags and is reported separately.

    WHY THE KEY IS A FALLBACK FOR `argv`
      Only some properties carry an explicit `argv`; most identify themselves by
      key alone. A first cut of this gate collected `argv` and nothing else, and
      reported 127 undocumented flags -- including `qr --text`, which the schema
      documents as the property `text`. That is the fictional-debt failure this
      repository already paid for once: an instrument that measures the wrong
      field does not merely miss real debt, it invents debt that costs real work
      to disprove. The property key IS the documentation when `argv` is absent,
      so it counts.
    """
    proc = subprocess.run(
        [binary, "--json", "schema", "--cmd", command],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        return None
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return None
    props = payload.get("data", {}).get("properties", {})
    declared: set[str] = set()
    _collect_flags(props, declared)
    return declared


def command_names(binary: str) -> list[str]:
    """Top-level verbs, straight from the binary's own inventory."""
    proc = subprocess.run(
        [binary, "--json", "commands"],
        capture_output=True,
        text=True,
        check=False,
    )
    proc.check_returncode()
    data = json.loads(proc.stdout).get("data", {})
    raw = data.get("commands", [])
    names = []
    for item in raw:
        if isinstance(item, str):
            names.append(item)
        elif isinstance(item, dict) and "name" in item:
            names.append(item["name"])
    return names


def main() -> int:
    binary = sys.argv[1] if len(sys.argv) > 1 else "./target/release/browser-automation-cli"

    globals_ = flags_in(run(binary, ["--help"])) | {"--help", "--version"}

    missing: dict[str, set[str]] = {}
    no_schema: list[str] = []
    checked = 0

    for name in command_names(binary):
        top_help = run(binary, [name, "--help"])
        exposed = flags_in(top_help)
        for sub in sorted(subcommands_in(top_help)):
            exposed |= flags_in(run(binary, [name, sub, "--help"]))
        exposed -= globals_
        if not exposed:
            continue

        declared = schema_flags(binary, name)
        if declared is None:
            no_schema.append(name)
            continue

        checked += 1
        if gap := exposed - declared:
            missing[name] = gap

    for name in sorted(missing):
        for flag in sorted(missing[name]):
            print(f"FAIL  {name}  {flag}  (accepted by clap, absent from schema)")
    for name in sorted(no_schema):
        print(f"FAIL  {name}  (no schema published at all)")

    total = sum(len(v) for v in missing.values()) + len(no_schema)
    print("----")
    print(f"commands_with_flags={checked}  undocumented_flags={total}")

    if total:
        print("== clap-schema-parity FAILED ==")
        print("Add the flag to src/commands/meta/schema/, then regenerate:")
        print("  bash scripts/generate_command_schemas.sh")
        return 1
    print("== clap-schema-parity PASS ==")
    return 0


if __name__ == "__main__":
    sys.exit(main())
