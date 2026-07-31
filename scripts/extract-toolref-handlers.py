#!/usr/bin/env python3
"""Extract the three parity layers from the reference tool handlers.

Layer 1 (name)      -> `name:`
Layer 2 (params)    -> top-level keys of `schema: { ... }`
Layer 3 (semantics) -> `annotations.readOnlyHint` (effect),
                       `annotations.conditions`   (capability precondition),
                       `blockedByDialog`          (state precondition)

Name and parameters are enumerable by scanning. Semantics is not: it lives in
the handler declaration, which is exactly why a name-only matrix kept passing
while GAP-041..GAP-043 stayed open.

Emits NDJSON on stdout, one object per tool. Exit 65 when the reference tree
is unreadable, so a caller never mistakes an empty matrix for full parity.
"""
import json
import re
import sys
from pathlib import Path

TOOLS_DIR = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(
    "base_conhecimento_chrome-devtools-mcp-main/src/tools"
)

DEFINE = re.compile(r"\bdefine(?:Page)?Tool\s*\(")


def balanced(text: str, start: int) -> str:
    """Return the source of the object literal opened at/after `start`."""
    depth, i, n = 0, start, len(text)
    began = False
    while i < n:
        c = text[i]
        if c in "'\"`":
            quote, i = c, i + 1
            while i < n and text[i] != quote:
                i += 2 if text[i] == "\\" else 1
            i += 1
            continue
        if c == "{":
            depth += 1
            began = True
        elif c == "}":
            depth -= 1
            if began and depth == 0:
                return text[start : i + 1]
        i += 1
    return text[start:]


def top_level_keys(block: str) -> list:
    """Keys at nesting depth 1 of an object literal."""
    keys, depth, i, n = [], 0, 0, len(block)
    buf = ""
    while i < n:
        c = block[i]
        if c in "'\"`":
            quote, i = c, i + 1
            while i < n and block[i] != quote:
                i += 2 if block[i] == "\\" else 1
            i += 1
            buf = ""
            continue
        if c in "{([":
            depth += 1
            buf = ""
        elif c in "})]":
            depth -= 1
            buf = ""
        elif c == ":" and depth == 1:
            k = buf.strip().strip("'\"")
            if re.fullmatch(r"[A-Za-z_$][\w$]*", k):
                keys.append(k)
            buf = ""
        elif c == ",":
            buf = ""
        else:
            buf += c
        i += 1
    return keys


def field(block: str, name: str):
    m = re.search(rf"\b{name}\s*:\s*", block)
    if not m:
        return None
    rest = block[m.end() :]
    if rest.startswith("["):
        return [x for x in re.findall(r"'([^']*)'", balanced_brackets(rest))]
    m2 = re.match(r"(true|false)", rest)
    if m2:
        return m2.group(1) == "true"
    m3 = re.match(r"'([^']*)'", rest)
    if m3:
        return m3.group(1)
    m4 = re.match(r"ToolCategory\.([A-Z_]+)", rest)
    if m4:
        return m4.group(1)
    return None


def balanced_brackets(text: str) -> str:
    depth, i = 0, 0
    while i < len(text):
        if text[i] == "[":
            depth += 1
        elif text[i] == "]":
            depth -= 1
            if depth == 0:
                return text[: i + 1]
        i += 1
    return text


def main() -> int:
    if not TOOLS_DIR.is_dir():
        print(f"reference tools dir not found: {TOOLS_DIR}", file=sys.stderr)
        return 65
    rows = []
    for path in sorted(TOOLS_DIR.rglob("*.ts")):
        if path.name in {"ToolDefinition.ts", "categories.ts"}:
            continue
        # `slim/` is a separate reduced surface, not part of the main inventory.
        # Counting it inflates the total and invents tools the CLI never claimed.
        if "slim" in path.parts:
            continue
        text = path.read_text(encoding="utf-8")
        # Some tools name themselves through a module constant rather than a
        # literal. A literal-only scan silently drops them, which is how a
        # derived inventory can look complete while hiding a tool.
        consts = dict(
            re.findall(r"const\s+([A-Z][A-Z0-9_]*)\s*=\s*'([^']+)'", text)
        )
        for m in DEFINE.finditer(text):
            block = balanced(text, m.end())
            name = field(block, "name")
            if not name:
                cm = re.search(r"\bname\s*:\s*([A-Z][A-Z0-9_]*)\s*,", block)
                if cm:
                    name = consts.get(cm.group(1))
            if not name:
                continue
            ann = ""
            am = re.search(r"\bannotations\s*:\s*", block)
            if am:
                ann = balanced(block, am.end())
            sm = re.search(r"\bschema\s*:\s*", block)
            schema_keys = top_level_keys(balanced(block, sm.end())) if sm else []
            rows.append(
                {
                    "tool": name,
                    "file": path.name,
                    "category": field(ann, "category"),
                    "read_only": field(ann, "readOnlyHint"),
                    "conditions": field(ann, "conditions") or [],
                    "blocked_by_dialog": bool(field(block, "blockedByDialog")),
                    "params": sorted(set(schema_keys)),
                }
            )
    if not rows:
        print("no tool definitions extracted", file=sys.stderr)
        return 65
    seen = {}
    for r in rows:
        seen.setdefault(r["tool"], r)
    for r in sorted(seen.values(), key=lambda x: x["tool"]):
        print(json.dumps(r, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
