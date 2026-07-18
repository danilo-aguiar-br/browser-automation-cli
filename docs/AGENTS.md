[English](AGENTS.md) | [Português Brasileiro](AGENTS.pt-BR.md)

# Agents Guide — browser-automation-cli

> Cut browser-tool glue. Keep one Chrome lifecycle under your agent. Lifecycle: BORN EXECUTE FINALIZE DIE.


## Why Agents Choose This CLI
- Subprocess ownership is explicit and short-lived
- JSON envelopes reduce brittle stdout scraping
- Multi-step scripts preserve accessibility refs without a daemon
- Category gates keep experimental surfaces opt-in
- Local scrape / crawl / map / search / parse surface ships as first-class subcommands
- Artifact helpers (`print-pdf`, `monitor`, `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`) and XDG LLM keys extend agent workflows without daemons
- Durable defaults live in flags and XDG `config path|init|show|set|get`


## Economy
- Avoid long-lived browser servers that leak across agent turns
- Pay Chrome launch cost only when the task needs a real page
- Prefer HTTP `scrape` / `batch-scrape` / `crawl` / `map` when content alone is enough
- Collapse multi-step flows into one `run` process when refs matter
- Reuse `schema --cmd` once per session instead of re-deriving argv by guesswork


## Sovereignty
- No npm runtime dependency for the product binary
- No remote telemetry path in the CLI
- System Chrome remains under the operator host policy
- Product settings live in flags and XDG `config` only
- Product logging uses `--verbose` / `--debug` / `-q` and XDG `log_level`
- Color uses `config set color`; Chrome path uses `config set chrome_path`


## Compatible Agents and Orchestrators
- Integration mode for every entry below is one-shot subprocess plus `--json`
- This project validates locally with cargo and e2e scripts
- Claude Code
- Codex
- Gemini CLI
- Opencode
- Cursor
- Windsurf
- VS Code Copilot
- GitHub Copilot CLI
- Cline
- Continue
- Aider
- Zed AI assistant
- JetBrains AI Assistant
- Local shell scripts and Makefiles
- Any orchestrator that can spawn a process and read stdout exit codes


## Agent Integration Details
- Spawn `browser-automation-cli` as a one-shot subprocess
- Always pass `--json` for machine parsing
- Read success and error envelopes from stdout
- Keep stderr for human or debug logs only
- Use `commands --json` to discover the live inventory (**59 commands**)
- Inventory includes config, mitm, workflow, scrape, batch-scrape, crawl, map, search, parse, print-pdf, monitor, qr, find-paths, sheet-write, sg-scan, sg-rewrite, extract, and DevTools-parity tools (59 total; e2e 53 tools)
- Use `schema --cmd <name> --json` before generating argv for unfamiliar commands
- Prefer flags for one-off control
- Use `config init|set|get|path|show|list-keys` for durable XDG defaults
- Full config keys (16) via `config list-keys`: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- Resolve paths with `config path --json`
- For multi-step work that needs shared `@eN` refs, use one `run --script` process (NDJSON **or** JSON array of steps)
- Wait with OR text: `wait --text A --text B`
- Scroll aliases in NDJSON: `{"cmd":"scroll","dy":1500}`
- Assert aliases: `{"cmd":"assert","url_contains":"example.com"}` / `text_contains`
- On `run` fail-fast errors, inspect partial `data.steps` when present
- Scrape with `--format text|markdown|html|links|metadata|summary|product|branding|raw-html|screenshot` and `--engine http|browser`
- Optional operator webhook on scrape: `--webhook-url` (one-shot POST, not product telemetry)
- Capture screenshots with `grab --path <file>` (not a positional path)
- Print PDF with `print-pdf --url … --path …`
- LLM extract fails closed without XDG `openrouter_api_key`
- Localize human suggestions with `--lang pt-BR` or `config set lang pt-BR`


## Crate Integrations
- Binary name is always `browser-automation-cli`
- Install with `cargo install browser-automation-cli --locked` after crates.io publish
- During development install from path or git
- Any Rust agent crate integrates through `std::process::Command`
- Compatible pattern crates include `rig-core`, `genai`, `async-openai`, `ollama-rs`, `anthropic-sdk`, `agentai`, `autoagents`, `swarms-rs`, `graphbit`, `llm-agent-runtime`
- The CLI is not a Rust library dependency of those crates
- The shared contract is argv plus JSON stdout plus sysexits-style exit codes

### Minimal Rust Command Example
```rust
use std::process::Command;

fn main() {
    let out = Command::new("browser-automation-cli")
        .args(["-q", "--json", "version"])
        .output()
        .expect("spawn browser-automation-cli");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
}
```


## Surface Discovery for Agents
- Inventory: `browser-automation-cli commands --json` (59 commands)
- Input fragments: `browser-automation-cli schema --cmd <name> --json`
- Config paths: `browser-automation-cli config path --json`
- Config keys: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- MITM: `mitm status|init-ca|start|list|get|har|export|domains|apis`
- Workflow: `workflow run|resume|status`
- Local scrape surface: `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
- Artifacts and local IO: `print-pdf`, `monitor check`, `qr encode|decode`, `find-paths` (`--glob`), `sheet-write`, `sg-scan`, `sg-rewrite`
- LLM extract: `extract --llm --question …` (XDG keys only)
- Health: `doctor --json` (reports Chrome discovery, XDG browsers_dir, lighthouse source, and `cache_redis` when configured)
- Cache: XDG `cache_backend` (`sqlite|memory|redis`) and `cache_redis_url` (`redis://` only; `rediss://` fail-closed)
- Lighthouse: flag → XDG `lighthouse_path` → PATH; envelope `binary_source` is `real` or `mock`


## Full Command Inventory (59)
- Live source of truth: `browser-automation-cli commands --json` (59 top-level names)
- DevTools tool-ref e2e covers **53** tools (`scripts/e2e_all_52_tools.sh` filename is legacy; suite runs 53)
- Full top-level command list (every name is a real subcommand):
  - Meta: `doctor`, `commands`, `schema`, `version`, `completions`
  - Navigate: `goto`, `back`, `forward`, `reload`, `page`, `wait`, `dialog`
  - Interact: `press`, `click-at`, `write`, `keys`, `type`, `hover`, `drag`, `fill-form`, `upload`, `scroll`
  - Observe: `view`, `eval`, `text`, `attr`, `assert`, `cookie`, `console`, `net`
  - Capture: `grab`, `print-pdf`, `monitor`, `screencast`, `lighthouse`
  - Multi-step: `run`, `exec`
  - Extract/scrape: `extract`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
  - Local IO (no Chrome): `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`
  - Infra: `config`, `mitm`, `workflow`
  - Emulation/perf: `emulate`, `resize`, `perf`, `heap`
  - Category gates: `extension`, `devtools3p`, `webmcp`
- Discover argv with `schema --cmd <name> --json` for any name above

## Lifecycle
- Slogan (English): BORN EXECUTE FINALIZE DIE
- One process owns one Chrome session from launch through FINALIZE
- FINALIZE is idempotent (Browser.close, wait, kill fallback)
- Do not expect session or `@eN` refs to survive process exit


## Technical Contract
### REQUIRED
- Pass `--json` for programmatic consumption
- Treat one process as one Chrome lifecycle (BORN EXECUTE FINALIZE DIE)
- Use `run --script` for multi-step work that needs shared `@eN` refs (NDJSON or JSON array)
- Check process exit code before trusting stdout
- Branch on envelope field `ok`
- Keep category and experimental gates explicit when needed
- Configure durable product settings via `config` / flags only
- Discover unknown commands with `commands --json` and `schema --cmd`

### FORBIDDEN
- Do not keep a daemon between agent turns
- Do not invent product aliases such as `bac`, `click`, or `screenshot`
- Do not reuse `@eN` refs across separate process launches
- Do not parse stderr as the primary success channel
- Do not enable robots bypass without the dual-flag policy
- Use only flags and `config` for product settings
- Do not pass a positional path to `grab`; use `--path`
- Do not invent a `--device` preset on `emulate`; use `--user-agent`, `--viewport`, `--network-conditions`

### Correct Pattern
```bash
browser-automation-cli -q --timeout 60 --json goto https://example.com
browser-automation-cli -q --json view
out=$(browser-automation-cli -q --json version)
echo "$out" | jaq -e '.ok == true'
browser-automation-cli -q --json commands
browser-automation-cli -q --json config path
browser-automation-cli -q --json wait --text Example --text Domain --ms 5000
browser-automation-cli -q --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli -q --json grab --path /tmp/page.png --full-page
browser-automation-cli -q --json print-pdf --url https://example.com --path /tmp/page.pdf
browser-automation-cli -q --json find-paths 'Cargo.*' .
browser-automation-cli -q --json find-paths --glob '**/*.rs' .
browser-automation-cli -q --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx
browser-automation-cli -q --json sg-scan . --limit 50
browser-automation-cli -q --json config list-keys
```


## JSON Envelope
- Success: `{"schema_version":1,"ok":true,"data":...}`
- Error: `{"schema_version":1,"ok":false,"error":{...}}`
- Error objects include `kind`, `message`, and `exit_code` when `--json` is set
- Multi-step fail-fast errors may also include partial `data.steps`
- Schema index: [docs/schemas/README.md](schemas/README.md)
- Live input fragments always come from `schema --cmd`; static files may lag


## Exit Codes
- `0` success
- `2` usage
- `65` data
- `66` no input
- `69` unavailable
- `70` software, browser, protocol
- `74` I/O
- `78` config
- `124` timeout
- `130` cancelled
- `141` broken pipe
