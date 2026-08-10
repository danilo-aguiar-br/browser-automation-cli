#!/usr/bin/env python3
"""Reconcile the PRD global-flag list against the implemented surface (GAP-023).

For every flag the PRD section 7 declares global, decide where the capability
actually lives today:

  global  present in `GlobalOpts`
  local   present as a flag of one or more subcommands
  xdg     present as an XDG config key (per-host, not per-invocation)
  absent  nowhere

`xdg` is not a synonym for `global`: an XDG key cannot be varied per invocation,
which is the whole reason an agent would reach for a flag. That difference is
the point of the table.

Exit 0 write, 65 unusable input, 1 divergence with --check.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PRD = ROOT / "docs_prd" / "prd_browser-automation-cli.md"
OUT = ROOT / "docs_prd" / "flag_reconciliation.md"
GLOBAL_RS = ROOT / "src" / "cli" / "global.rs"

# PRD flag -> XDG key, where the capability moved to per-host configuration.
XDG_EQUIVALENT = {
    "--chrome-path": "chrome_path",
    "--ffmpeg-path": "ffmpeg_path",
    "--lighthouse-path": "lighthouse_path",
    "--artifacts-dir": "artifacts_dir",
    "--no-color": "color",
    "--max-body-bytes": "scrape_max_body_bytes",
    "--concurrency": "http_pool_max_idle_per_host",
    "--viewport": "default_viewport_width",
}


def die(msg, code=65):
    print(msg, file=sys.stderr)
    sys.exit(code)


def prd_global_flags() -> list:
    if not PRD.exists():
        die(f"PRD not found: {PRD}")
    text = PRD.read_text(encoding="utf-8", errors="replace").splitlines()
    try:
        start = next(i for i, l in enumerate(text) if l.strip() == "### Flags Globais")
    except StopIteration:
        die("PRD section `### Flags Globais` not found")
    flags = []
    for line in text[start + 1 :]:
        if line.startswith("###") or line.startswith("##"):
            break
        # The flag may carry a value placeholder inside the same backticks
        # (`--timeout <secs>`, `--robots-policy honor|ignore`). Anchoring on the
        # closing backtick silently dropped 26 of the 50 entries.
        m = re.match(r"-\s+`(--?[a-z0-9-]+)(?:[ =][^`]*)?`", line)
        if m:
            flags.append(m.group(1))
    if not flags:
        die("PRD global flag list parsed to zero entries")
    return flags


def short_flags(text: str) -> set:
    """Collect `short = 'q'` as `-q`.

    The PRD lists some flags only in short form. Matching long names alone made
    every one of them read as `absent`, which put `-q` and `-v` on the missing
    list while `src/cli/global.rs` declared both -- a fictional debt that made
    the whole scoreboard untrustworthy.
    """
    return {f"-{m}" for m in re.findall(r"short\s*=\s*'([A-Za-z0-9])'", text)}


def real_globals() -> set:
    text = GLOBAL_RS.read_text(encoding="utf-8")
    names = {f"--{m.replace('_', '-')}" for m in re.findall(r"^\s+pub ([a-z_0-9]+):", text, re.M)}
    names |= {f"--{m}" for m in re.findall(r'long\s*=\s*"([a-z0-9-]+)"', text)}
    names |= short_flags(text)
    if not names:
        die("GlobalOpts parsed to zero fields")
    return names


def local_flags() -> set:
    out = set()
    for path in (ROOT / "src" / "cli").glob("*.rs"):
        if path.name == "global.rs":
            continue
        text = path.read_text(encoding="utf-8")
        out |= {f"--{m}" for m in re.findall(r'long\s*=\s*"([a-z0-9-]+)"', text)}
        out |= short_flags(text)
        # `#[arg(long)]` derives the flag name from the field below it.
        for m in re.finditer(r"#\[arg\((?:[^)]*\blong\b)[^)]*\)\]\s*\n\s+([a-z_0-9]+):", text):
            out.add("--" + m.group(1).replace("_", "-"))
    return out


def xdg_keys(binary: Path) -> set:
    r = subprocess.run([str(binary), "--json", "config", "list-keys"],
                       capture_output=True, text=True)
    if r.returncode != 0:
        die(f"config list-keys failed: {r.stderr.strip()}")
    return {k["key"] for k in json.loads(r.stdout)["data"]["keys"]}


def main() -> int:
    check = "--check" in sys.argv
    binary = ROOT / "target" / "debug" / "browser-automation-cli"
    if not binary.exists():
        die(f"binary not built: {binary}")

    prd = prd_global_flags()
    g, l, x = real_globals(), local_flags(), xdg_keys(binary)

    rows, counts = [], {"global": 0, "local": 0, "xdg": 0, "absent": 0}
    for flag in prd:
        key = XDG_EQUIVALENT.get(flag)
        if flag in g:
            where, detail = "global", "`GlobalOpts`"
        elif flag in l:
            where, detail = "local", "flag de subcomando"
        elif key and key in x:
            where, detail = "xdg", f"`config set {key}` (por host, NÃO por invocação)"
        else:
            where, detail = "absent", "não existe em lugar nenhum"
        counts[where] += 1
        rows.append(f"| `{flag}` | {where} | {detail} |")

    extra = sorted(f for f in g if f not in prd and f not in {"--help", "--version"})

    body = [
        "# Reconciliação de flags globais do PRD",
        "",
        "",
        "## Como este arquivo é produzido",
        "- GERADO por `scripts/gen-flag-reconciliation.py`; NUNCA edite à mão",
        "- Lista do PRD sai da seção `### Flags Globais`",
        "- Superfície real sai de `src/cli/global.rs`, dos subcomandos e de `config list-keys`",
        "- Regenere com `python3 scripts/gen-flag-reconciliation.py`",
        "",
        "",
        "## Por que `xdg` não é o mesmo que `global`",
        "- Uma chave XDG vale para o host inteiro e NÃO varia por invocação",
        "- Um agente que precise alternar o valor entre duas chamadas não consegue",
        "- Marcar como resolvido o que virou chave XDG esconde essa perda",
        "",
        "",
        "## Placar",
        f"- Flags declaradas globais no PRD: {len(prd)}",
        f"- Existem como global: {counts['global']}",
        f"- Existem como flag local de subcomando: {counts['local']}",
        f"- Existem apenas como chave XDG: {counts['xdg']}",
        f"- Não existem em lugar nenhum: {counts['absent']}",
        "",
        "",
        "## Tabela",
        "| flag do PRD | onde vive | detalhe |",
        "|---|---|---|",
        *rows,
        "",
        "",
        "## Globais reais que o PRD não lista",
        *([f"- `{f}`" for f in extra] or ["- Nenhuma"]),
    ]
    text = "\n".join(body) + "\n"

    if check:
        current = OUT.read_text(encoding="utf-8") if OUT.exists() else ""
        if current != text:
            print("flag reconciliation is stale; run scripts/gen-flag-reconciliation.py",
                  file=sys.stderr)
            return 1
        return 0

    OUT.write_text(text, encoding="utf-8")
    print(json.dumps({"written": str(OUT), **counts, "prd_flags": len(prd)}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
