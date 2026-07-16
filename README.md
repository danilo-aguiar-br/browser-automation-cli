# browser-automation-cli

One-shot browser automation CLI for AI agents (Chrome CDP).

## Install

```bash
cargo install --path . --locked
browser-automation-cli --version
```

## Model

NASCE → EXECUTA → FINALIZE → MORRE in a single process.

No daemon. No npm packaging. crates.io path only (`publish = false` until first release).

## Examples

```bash
browser-automation-cli doctor --json
browser-automation-cli goto https://example.com
browser-automation-cli view --verbose
browser-automation-cli press "#btn" --include-snapshot --json
browser-automation-cli --capture-network net list --resource-types Document,XHR --json
browser-automation-cli type "hello" --target "input" --submit Enter --json
```

## DevTools surface (PRD §5C)
- One-shot CLI maps the official agent tool inventory to subcommands
- Multi-step browser work uses `run` (NDJSON) in a single process
- Category flags: `--category-memory`, `--category-extensions`, `--category-third-party`, `--category-webmcp`
- Experimental: `--experimental-vision`, `--experimental-screencast`
- Inventory: `docs_prd/parity_devtools_matrix.md`
- Schema gate: `tests/parity_toolref_schema.rs`
