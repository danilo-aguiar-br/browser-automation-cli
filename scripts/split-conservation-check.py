#!/usr/bin/env python3
"""Conservation audit: every significant line of a pre-split file must still
exist somewhere under the directory that replaced it.

# Why compiling is not enough

Splitting `commands/ops/lighthouse.rs` silently dropped the doc comment of
`enum LighthouseSource`. Every gate stayed green: the crate compiled, 487 tests
passed, `clippy -D warnings` passed, and `cargo doc -D warnings` passed.

`missing_docs` cannot catch that class: it fires on a PUBLIC item that has no
doc, and the enum is `pub(crate)`. A gate that measures the PRESENCE of
documentation is blind to documentation that was DELETED from an item it does
not cover. The same reasoning applies to any content whose absence is still
valid Rust — a dropped match arm with a catch-all sibling, a dropped comment,
a dropped test helper that nothing else calls.

So the invariant here is conservation, not compilation: run this BEFORE removing
the original, and treat a non-zero exit as a lost hunk, not as a nit.

# Trivial lines

Excluded because a split legitimately rewrites them: imports, lone delimiters,
blank lines and module attributes. Keep this list tight — every prefix added
here is a place a real loss could hide.
"""
import sys
import pathlib

TRIVIAL_PREFIX = ("use ", "#![", "mod ", "pub use ", "pub(crate) use ", "pub(super) use ")


def significant(line: str) -> bool:
    s = line.strip()
    if not s:
        return False
    if s in {"}", "{", "};", ")", "),", "()", "],", "]"}:
        return False
    if s.startswith(TRIVIAL_PREFIX):
        return False
    return True


def audit(original: pathlib.Path, new_root: pathlib.Path) -> int:
    old_lines = [l for l in original.read_text().split("\n") if significant(l)]
    haystack = "\n".join(
        p.read_text() for p in sorted(new_root.rglob("*.rs"))
    )
    missing = [l for l in old_lines if l.strip() not in haystack]
    name = original.name
    if missing:
        print(f"FAIL  {name}: {len(missing)}/{len(old_lines)} linhas ausentes")
        for l in missing[:5]:
            print(f"        {l.strip()[:78]}")
    else:
        print(f"ok    {name}: {len(old_lines)} linhas significativas preservadas")
    return len(missing)


if __name__ == "__main__":
    total = 0
    for orig, root in zip(sys.argv[1::2], sys.argv[2::2]):
        o, r = pathlib.Path(orig), pathlib.Path(root)
        if not o.exists() or not r.exists():
            print(f"skip  {o.name}: sem referencia")
            continue
        total += audit(o, r)
    print(f"\ntotal ausente: {total}")
    sys.exit(1 if total else 0)
