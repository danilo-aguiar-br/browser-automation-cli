#!/usr/bin/env bash
# Audit bilingual public docs: CLI invocations inside code fences must match EN vs PT.
# Usage:
#   bash scripts/audit_bilingual_docs.sh
# Exit:
#   0 all pairs match
#   1 invocation drift
#   2 missing pair file or fatal error
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 - "$ROOT" <<'PY'
from __future__ import annotations

import re
import sys
from collections import Counter
from pathlib import Path

root = Path(sys.argv[1])

PAIRS: list[tuple[str, str]] = [
    ("README.md", "README.pt-BR.md"),
    ("CHANGELOG.md", "CHANGELOG.pt-BR.md"),
    ("CONTRIBUTING.md", "CONTRIBUTING.pt-BR.md"),
    ("SECURITY.md", "SECURITY.pt-BR.md"),
    ("INTEGRATIONS.md", "INTEGRATIONS.pt-BR.md"),
    ("CODE_OF_CONDUCT.md", "CODE_OF_CONDUCT.pt-BR.md"),
    ("docs/HOW_TO_USE.md", "docs/HOW_TO_USE.pt-BR.md"),
    ("docs/AGENTS.md", "docs/AGENTS.pt-BR.md"),
    ("docs/COOKBOOK.md", "docs/COOKBOOK.pt-BR.md"),
    ("docs/CROSS_PLATFORM.md", "docs/CROSS_PLATFORM.pt-BR.md"),
    ("docs/MIGRATION.md", "docs/MIGRATION.pt-BR.md"),
    ("docs/TESTING.md", "docs/TESTING.pt-BR.md"),
    ("llms.txt", "llms.pt-BR.txt"),
]

# Fenced code: ```lang? ... ```
FENCE_RE = re.compile(r"```([^\n`]*)\n(.*?)```", re.DOTALL)
# Languages that may contain shell CLI examples
SHELL_LANGS = {"", "bash", "sh", "shell", "zsh", "fish", "console", "terminal"}
# Also scan rust only for Command::new("browser-automation-cli") strings? plan says bash fences.
# Scan all fences that contain browser-automation-cli for maximum safety.
CLI_TOKEN = "browser-automation-cli"


def collapse_continuations(text: str) -> str:
    # Join lines ending with backslash
    lines = text.splitlines()
    out: list[str] = []
    buf = ""
    for line in lines:
        stripped = line.rstrip()
        if stripped.endswith("\\"):
            buf += stripped[:-1].rstrip() + " "
            continue
        if buf:
            buf += stripped.lstrip()
            out.append(buf)
            buf = ""
        else:
            out.append(line)
    if buf:
        out.append(buf)
    return "\n".join(out)


def strip_shell_comment(line: str) -> str:
    # Remove unquoted # comments (simple heuristic)
    in_single = False
    in_double = False
    i = 0
    while i < len(line):
        c = line[i]
        if c == "'" and not in_double:
            in_single = not in_single
        elif c == '"' and not in_single:
            in_double = not in_double
        elif c == "#" and not in_single and not in_double:
            return line[:i].rstrip()
        i += 1
    return line.rstrip()


def normalize_ws(s: str) -> str:
    return re.sub(r"\s+", " ", s).strip()


def extract_invocations(path: Path) -> list[str]:
    if not path.is_file():
        return []
    text = path.read_text(encoding="utf-8")
    invs: list[str] = []
    for m in FENCE_RE.finditer(text):
        lang = (m.group(1) or "").strip().lower()
        body = m.group(2)
        if CLI_TOKEN not in body:
            continue
        # Prefer shell-like fences; still accept any fence that contains the binary
        if lang and lang not in SHELL_LANGS and lang not in {"json", "jsonl", "rust", "toml"}:
            # still process if binary present
            pass
        body = collapse_continuations(body)
        for raw_line in body.splitlines():
            line = strip_shell_comment(raw_line).strip()
            if not line or CLI_TOKEN not in line:
                continue
            # drop shell variable assignments prefixes only when not starting with binary
            # keep pipelines by taking each segment that contains the binary
            segments = re.split(r"\s*\|\s*", line)
            for seg in segments:
                if CLI_TOKEN not in seg:
                    continue
                # remove leading env VAR=value ...
                seg2 = re.sub(
                    r"^(?:[A-Za-z_][A-Za-z0-9_]*=\S+\s+)+",
                    "",
                    seg.strip(),
                )
                # remove leading sudo / command / time
                seg2 = re.sub(r"^(?:sudo|command|time)\s+", "", seg2)
                # capture from binary name
                idx = seg2.find(CLI_TOKEN)
                if idx < 0:
                    continue
                inv = normalize_ws(seg2[idx:])
                if inv:
                    invs.append(inv)
    return invs


def multiset_diff(a: list[str], b: list[str]) -> tuple[list[str], list[str]]:
    ca, cb = Counter(a), Counter(b)
    missing_in_b: list[str] = []
    missing_in_a: list[str] = []
    for k, n in ca.items():
        d = n - cb.get(k, 0)
        if d > 0:
            missing_in_b.extend([k] * d)
    for k, n in cb.items():
        d = n - ca.get(k, 0)
        if d > 0:
            missing_in_a.extend([k] * d)
    return sorted(missing_in_b), sorted(missing_in_a)


ok_pairs = 0
fail_pairs = 0
missing_files = 0
fatal = False

for en_rel, pt_rel in PAIRS:
    en_path = root / en_rel
    pt_path = root / pt_rel
    if not en_path.is_file() and not pt_path.is_file():
        print(f"SKIP  {en_rel} ↔ {pt_rel}  (both missing)")
        continue
    if not en_path.is_file():
        print(f"FAIL  {en_rel} ↔ {pt_rel}")
        print(f"  - missing file: {en_rel}")
        missing_files += 1
        fail_pairs += 1
        continue
    if not pt_path.is_file():
        print(f"FAIL  {en_rel} ↔ {pt_rel}")
        print(f"  - missing file: {pt_rel}")
        missing_files += 1
        fail_pairs += 1
        continue

    en_inv = extract_invocations(en_path)
    pt_inv = extract_invocations(pt_path)
    miss_pt, miss_en = multiset_diff(en_inv, pt_inv)

    if not miss_pt and not miss_en:
        print(f"OK    {en_rel} ↔ {pt_rel}  ({len(en_inv)} invocations)")
        ok_pairs += 1
        # optional order warning
        if en_inv != pt_inv and Counter(en_inv) == Counter(pt_inv):
            print("  warn: same multiset but different order")
    else:
        print(f"FAIL  {en_rel} ↔ {pt_rel}")
        print(f"  en_count={len(en_inv)} pt_count={len(pt_inv)}")
        for inv in miss_pt[:50]:
            print(f"  - missing_in_pt: {inv}")
        if len(miss_pt) > 50:
            print(f"  - missing_in_pt: ... +{len(miss_pt) - 50} more")
        for inv in miss_en[:50]:
            print(f"  - missing_in_en: {inv}")
        if len(miss_en) > 50:
            print(f"  - missing_in_en: ... +{len(miss_en) - 50} more")
        fail_pairs += 1

print(f"Summary: ok={ok_pairs} fail={fail_pairs} missing_files={missing_files}")
if missing_files and fail_pairs == missing_files and ok_pairs == 0:
    sys.exit(2)
if fail_pairs:
    sys.exit(1)
sys.exit(0)
PY
