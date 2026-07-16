[English](HOW_TO_USE.md) | [Português Brasileiro](HOW_TO_USE.pt-BR.md)

# How to Use — browser-automation-cli

> Install once, launch Chrome once, finish the agent task, exit cleanly.

## Prerequisites
- Rust 1.88.0+ if building from source
- Chrome or Chromium on PATH
- Optional ffmpeg for screencast export
- Optional lighthouse binary for audits

## First Command in 60 Seconds
```bash
cargo install --path . --locked
browser-automation-cli doctor --offline --quick --json
browser-automation-cli --timeout 60 --json goto https://example.com
browser-automation-cli --json view
```
- Doctor proves Chrome discovery works
- Goto navigates in a fresh one-shot process
- View prints an accessibility snapshot with `@eN` refs

## Core Commands
- Navigate with `goto`, `back`, `forward`, `reload`
- Snapshot with `view`
- Click with `press @eN` or CSS selectors
- Fill with `write` and multi-field `fill-form`
- Capture pages with `grab out.png --full-page`
- Scrape body text with `scrape https://example.com`

## Multi-step in One Process
```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.jsonl
```
- Use `run` whenever refs must survive across steps
- Separate process launches cannot share `@eN` refs

## Advanced Patterns
- Capture network: `--capture-network net list --json`
- Capture console: `--capture-console console list --json`
- Emulate device and network with `emulate`
- Deep heap work requires `--category-memory`
- Extension tools require `--category-extensions`
- Coordinate clicks require `--experimental-vision`

## Configuration
- Prefer flags for one-off agent calls
- Prefer environment variables for CI defaults
- Keep robots dual-flag policy explicit when bypassing

## Subcommands Not Covered Above
- Use `browser-automation-cli commands --json` for the live inventory
- Use `browser-automation-cli schema --cmd <name> --json` for input shapes
- Use `browser-automation-cli help <cmd>` for flag-level detail

## Integration With AI Agents
- Always request `--json`
- Parse only stdout envelopes
- Treat stderr as diagnostics
- See [docs/AGENTS.md](AGENTS.md) and [INTEGRATIONS.md](../INTEGRATIONS.md)
