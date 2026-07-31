#!/usr/bin/env python3
"""Generate the three-layer parity matrix from live sources (GAP-044).

Sources, all live, none transcribed by hand:
  * reference handlers  -> scripts/extract-toolref-handlers.py
  * CLI inventory       -> `browser-automation-cli --json commands`
  * CLI capability gate -> `src/capability/table.rs`

Layers
  L1 name       every reference tool maps to a CLI command
  L2 parameters every reference parameter reaches the CLI under some spelling
  L3 semantics  precondition (capability gate, dialog block) and effect
                (read-only vs mutating) agree with the reference handler

A name-only matrix is what let GAP-041..GAP-043 stay open while the scoreboard
read green, so L3 is the reason this generator exists.

Exit 0 write, 65 unusable input, 1 divergence when --check is passed.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MATRIX = ROOT / "docs_prd" / "parity_devtools_matrix.md"
CAP_TABLE = ROOT / "src" / "capability" / "table.rs"
DIVERGENCES = ROOT / "docs_prd" / "parity_intentional_divergences.json"
PARAM_ALIASES = ROOT / "docs_prd" / "parity_param_aliases.json"

# Reference `conditions` token -> CLI capability gate flag.
CONDITION_TO_FLAG = {
    "memoryDebugging": "--category-memory",
    "experimentalVision": "--experimental-vision",
    "experimentalScreencast": "--experimental-screencast",
    "experimentalInteropTools": None,  # no CLI equivalent; reported as a gap
}


def die(msg: str, code: int = 65):
    print(msg, file=sys.stderr)
    sys.exit(code)


def load_reference() -> list:
    out = subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "extract-toolref-handlers.py")],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if out.returncode != 0:
        die(f"reference extraction failed: {out.stderr.strip()}")
    rows = [json.loads(l) for l in out.stdout.splitlines() if l.strip()]
    if not rows:
        die("reference extraction returned no tools")
    return rows


def load_cli(binary: Path) -> dict:
    out = subprocess.run(
        [str(binary), "--json", "commands"], capture_output=True, text=True
    )
    if out.returncode != 0:
        die(f"`commands` failed: {out.stderr.strip()}")
    env = json.loads(out.stdout)
    if not env.get("ok"):
        die("`commands` returned ok=false")
    return env["data"]


def load_capability_rows() -> dict:
    """cmd -> (capability consts, precondition const) from the gate table."""
    text = CAP_TABLE.read_text(encoding="utf-8")
    rows = {}
    for m in re.finditer(
        r'row\(\s*"([a-z0-9-]+)"\s*,\s*(None|Some\("[a-z0-9-]+"\))\s*,'
        r"\s*([A-Z_]+)\s*,\s*([A-Z_]+)\s*\)",
        text,
    ):
        cmd, action, cap, pre = m.groups()
        key = cmd if action == "None" else f"{cmd} {action[6:-2]}"
        rows[key] = (cap, pre)
    if not rows:
        die("capability table parsed to zero rows")
    return rows


def cli_root(cli_cmd: str) -> str:
    return cli_cmd.split()[0]


def snake(name: str) -> str:
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def cli_schema_props(binary: Path, cmd: str, cache: dict) -> set:
    """Property names the derived schema exposes for a command.

    The schema now comes from the parser rather than hand-written text, and it
    flattens subcommand arguments, so `heap close --path` shows up under `heap`.
    """
    root = cli_root(cmd.split("|")[0])
    if root not in cache:
        r = subprocess.run(
            [str(binary), "--json", "schema", root], capture_output=True, text=True
        )
        try:
            cache[root] = set(
                json.loads(r.stdout)["data"].get("properties", {}).keys()
            )
        except Exception:
            cache[root] = set()
    return cache[root]


def unreachable_params(tool: str, params: list, have: set, aliases: dict) -> list:
    """Reference parameters that reach the CLI under no declared spelling.

    Straight name matching reports 37 of 53 tools as broken because the CLI
    renames `uid` to `target`, `filePath` to `path` and so on. That much noise
    trains a reader to skip the gate, so every rename must be DECLARED in
    docs_prd/parity_param_aliases.json. What is left over is a real gap.
    """
    per_tool = aliases["per_tool"].get(tool, {})
    out = []
    for p in params:
        s = snake(p)
        candidates = {
            p,
            s,
            per_tool.get(s),
            per_tool.get(p),
            aliases["global"].get(s),
            aliases["global"].get(p),
        }
        if not (candidates & have):
            out.append(p)
    return out


def main() -> int:
    check = "--check" in sys.argv
    binary = ROOT / "target" / "debug" / "browser-automation-cli"
    if not binary.exists():
        die(f"binary not built: {binary}", 65)

    reference = load_reference()
    cli = load_cli(binary)
    caps = load_capability_rows()
    declared = json.loads(DIVERGENCES.read_text(encoding="utf-8"))
    aliases = json.loads(PARAM_ALIASES.read_text(encoding="utf-8"))
    schema_cache: dict = {}
    layer2_known = set(declared.get("layer2_open", []))
    declared_ids = {d["id"] for d in declared["tools"]}
    known_open_ids = {d["id"] for d in declared.get("known_open", [])}
    for d in declared.get("known_open", []):
        if not d.get("tracked_in"):
            die(f"known_open entry without tracked_in: {d['id']}")

    tool_to_cli = {e["tool"]: e["cli"] for e in cli["devtools_tool_map"]}
    cli_commands = set(cli["commands"])

    divergences = []
    intentional = []
    known_open = []
    layer2 = []
    lines = []

    def record(msg: str):
        """Classify a finding: deliberate, already-triaged, or new."""
        if msg in declared_ids:
            intentional.append(msg)
        elif msg in layer2_known:
            layer2.append(msg)
        elif msg in known_open_ids:
            known_open.append(msg)
        else:
            divergences.append(msg)
    for r in sorted(reference, key=lambda x: (x["category"], x["tool"])):
        tool = r["tool"]
        mapped = tool_to_cli.get(tool)

        # L1 — name
        if not mapped:
            l1 = "ABSENT"
            record(f"L1 {tool}: no CLI mapping")
        elif not all(
            cli_root(branch) in cli_commands for branch in mapped.split("|")
        ):
            l1 = "STALE"
            record(f"L1 {tool}: maps to `{mapped}`, absent from live inventory")
        else:
            l1 = "ok"

        # L3 — precondition: capability gate
        want_flags = [CONDITION_TO_FLAG.get(c, "?") for c in r["conditions"]]
        if any(f is None for f in want_flags):
            gate = "UNGATED"
            record(
                f"L3 {tool}: reference condition {r['conditions']} has no CLI gate flag"
            )
        elif want_flags:
            gate = ",".join(f for f in want_flags if f)
        else:
            gate = "-"

        # L3 — precondition: dialog block
        row = caps.get(mapped) or caps.get(cli_root(mapped or ""))
        cli_blocked = bool(row and row[1] == "BLOCKED")
        ref_blocked = r["blocked_by_dialog"]
        if ref_blocked and not cli_blocked:
            dialog = "DIVERGE"
            record(
                f"L3 {tool}: reference blockedByDialog=true, CLI `{mapped}` unguarded"
            )
        elif ref_blocked:
            dialog = "blocked"
        elif cli_blocked:
            # Stricter than the reference is a deliberate call, not a defect,
            # but it must be visible rather than silent.
            dialog = "stricter"
        else:
            dialog = "free"

        # L2 — parameters, against the parser-derived schema
        if mapped:
            have = cli_schema_props(binary, mapped, schema_cache)
            gaps = unreachable_params(tool, r["params"], have, aliases)
        else:
            gaps = []
        if gaps:
            l2 = "GAP:" + ",".join(gaps)
            record(f"L2 {tool}: params unreachable in CLI schema: {gaps}")
        else:
            l2 = "ok"

        effect = "read-only" if r["read_only"] else "mutates"
        # A `|` inside a cell breaks markdown column splitting, and the tool map
        # uses it for alternation (`goto|back|forward|reload`). Render with `/`;
        # the L1 check above still reads the raw alternation.
        cell = (mapped or "—").replace("|", " / ")
        lines.append(
            f"| {tool} | {cell} | {r['category'].lower()} | {l1} | {l2} | "
            f"{effect} | {gate} | {dialog} |"
        )

    body = [
        "# Matriz de paridade DevTools — três camadas",
        "",
        "",
        "## Como este arquivo é produzido",
        "- GERADO por `scripts/gen-parity-matrix.py`; NUNCA edite à mão",
        "- Camada 1 nome sai de `--json commands` e do mapa vivo de tools",
        "- Camada 2 parâmetro sai do `schema` de cada handler da referência",
        "- Camada 3 semântica sai de `readOnlyHint`, `conditions` e `blockedByDialog`",
        "- Regenere com `python3 scripts/gen-parity-matrix.py`",
        "- Verifique sem escrever com `python3 scripts/gen-parity-matrix.py --check`",
        "",
        "",
        "## Contagem viva",
        f"- Tools na referência: {len(reference)}",
        f"- Entradas no mapa tool→CLI: {len(tool_to_cli)}",
        "- O total de comandos do binário NÃO entra aqui: ele cresce com",
        "  superfície própria da CLI e faria a matriz envelhecer sem que a",
        "  paridade tivesse mudado. Use `--json commands` para esse número.",
        f"- Divergências abertas NÃO triadas: {len(divergences)}",
        f"- Divergências abertas já triadas: {len(known_open)}",
        f"- Lacunas de camada 2 já triadas: {len(layer2)}",
        f"- Divergências intencionais declaradas e ativas: {len(intentional)}",
        "",
        "",
        "## Legenda das colunas",
        "- `L1` ok quando a tool mapeia para comando presente no inventário vivo",
        "- `L2` ok quando todo parâmetro do handler alcança o schema derivado do parser",
        "- `L2` GAP: lista os parâmetros que a referência expõe e a CLI não alcança",
        "- Renomeações são DECLARADAS em `docs_prd/parity_param_aliases.json`",
        "- `efeito` vem de `readOnlyHint` do handler, não do nome do comando",
        "- `gate` é a flag de categoria exigida pela referência via `conditions`",
        "- `diálogo` blocked quando a referência declara `blockedByDialog: true`",
        "- `diálogo` stricter quando a CLI guarda algo que a referência deixa livre",
        "",
        "",
        "## Matriz",
        "| tool | cli | categoria | L1 | L2 | efeito | gate | diálogo |",
        "|---|---|---|---|---|---|---|---|",
        *lines,
    ]
    body += ["", "", "## Divergências intencionais de arquitetura"]
    for d in declared["architecture"]:
        body += [
            f"### {d['title']}",
            f"- REFERÊNCIA: {d['reference']}",
            f"- CLI: {d['cli']}",
            f"- POR QUÊ: {d['why']}",
            f"- CUSTO ACEITO: {d['cost']}",
        ]
    body += ["", "", "## Divergências intencionais por tool"]
    if intentional:
        for msg in intentional:
            d = next(x for x in declared["tools"] if x["id"] == msg)
            body += [
                f"### {d['title']}",
                f"- ACHADO: {msg}",
                f"- POR QUÊ: {d['why']}",
                f"- CUSTO ACEITO: {d['cost']}",
            ]
    else:
        body += ["- Nenhuma divergência declarada está ativa nesta geração"]

    body += ["", "", "## Divergências abertas já triadas"]
    if known_open:
        for msg in known_open:
            d = next(x for x in declared["known_open"] if x["id"] == msg)
            body += [
                f"### {d['title']}",
                f"- ACHADO: {d['finding']}",
                f"- IMPACTO: {d['impact']}",
                f"- DECISÃO PENDENTE: {d['decision_needed']}",
                f"- RASTREADO EM: {d['tracked_in']}",
            ]
    else:
        body += ["- Nenhuma"]

    body += ["", "", "## Lacunas de camada 2 já triadas"]
    body += [
        "- Parâmetro que a referência expõe e a CLI NÃO alcança sob nenhuma grafia",
        "- NÃO são renomeações: essas estão declaradas em `parity_param_aliases.json`",
        "- Cada linha é trabalho de implementação pendente, não permissão para omitir",
        "",
    ]
    body += [f"- {d}" for d in layer2] if layer2 else ["- Nenhuma"]

    body += ["", "", "## Divergências abertas NÃO triadas"]
    body += [f"- {d}" for d in divergences] if divergences else [
        "- Nenhuma; toda divergência ativa está declarada ou triada"
    ]
    text = "\n".join(body) + "\n"

    if check:
        current = MATRIX.read_text(encoding="utf-8") if MATRIX.exists() else ""
        if current != text:
            print("parity matrix is stale; run scripts/gen-parity-matrix.py", file=sys.stderr)
            return 1
        if divergences:
            for d in divergences:
                print(f"UNTRIAGED {d}", file=sys.stderr)
            return 1
        for d in known_open + layer2:
            print(f"known-open (tracked) {d}", file=sys.stderr)
        return 0

    MATRIX.write_text(text, encoding="utf-8")
    print(json.dumps({"written": str(MATRIX), "tools": len(reference),
                      "divergences": len(divergences)}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
