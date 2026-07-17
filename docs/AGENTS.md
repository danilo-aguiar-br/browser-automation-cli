[English](AGENTS.md) | [Português Brasileiro](AGENTS.pt-BR.md)

# Agents Guide — browser-automation-cli

> Cut browser-tool glue. Keep one Chrome lifecycle under your agent. Lifecycle: BORN EXECUTE FINALIZE DIE.


## Why Agents Choose This CLI
- Subprocess ownership is explicit and short-lived
- JSON envelopes reduce brittle stdout scraping
- Multi-step scripts preserve accessibility refs without a daemon
- Category gates keep experimental surfaces opt-in
- Firecrawl-local discovery surface ships as first-class subcommands
- XDG config replaces product environment variables


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
- Product settings live in flags and XDG `config`, not product environment variables
- There is no product env contract such as `BROWSER_AUTOMATION_CLI_*`


## Compatible Agents and Orchestrators
- Integration mode for every entry below is one-shot subprocess plus `--json`
- This project validates locally with cargo and e2e scripts; it does not claim hosted CI coverage per agent
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
- Use `commands --json` to discover the live inventory
- Inventory includes config, mitm, workflow, scrape, batch-scrape, crawl, map, search, parse
- Use `schema --cmd <name> --json` before generating argv for unfamiliar commands
- Prefer flags for one-off control
- Use `config init|set|get|path|show` for durable XDG defaults
- OS env only when needed: `RUST_LOG` for tracing, `NO_COLOR` to disable color
- For multi-step work that needs shared `@eN` refs, use one `run --script` process
- Wait with OR text: `wait --text A --text B`
- Scrape with `--format text|markdown|html|links|metadata` and `--engine http|browser`
- Capture screenshots with `grab --path <file>` (not a positional path)


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
- Inventory: `browser-automation-cli commands --json`
- Input fragments: `browser-automation-cli schema --cmd <name> --json`
- Config paths: `browser-automation-cli config path --json`
- Config keys: `config set|get|show` for lang, timeout, artifacts_dir, ignore_robots, namespace, encryption_key, color
- MITM: `mitm status|init-ca|start|list|get|har|export|domains|apis`
- Workflow: `workflow run|resume|status`
- Firecrawl-local: `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
- Health: `doctor --json` (reports Chrome discovery and XDG browsers_dir)


## Lifecycle
- Slogan (English): BORN EXECUTE FINALIZE DIE
- One process owns one Chrome session from launch through FINALIZE
- FINALIZE is idempotent (Browser.close, wait, kill fallback)
- Do not expect session or `@eN` refs to survive process exit


## Technical Contract
### REQUIRED
- Pass `--json` for programmatic consumption
- Treat one process as one Chrome lifecycle (BORN EXECUTE FINALIZE DIE)
- Use `run --script` for multi-step work that needs shared `@eN` refs
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
- Do not rely on product `BROWSER_AUTOMATION_CLI_*` environment variables
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
browser-automation-cli -q --json scrape https://example.com --format markdown --engine http
browser-automation-cli -q --json grab --path /tmp/page.png --full-page
```


## JSON Envelope
- Success: `{"schema_version":1,"ok":true,"data":...}`
- Error: `{"schema_version":1,"ok":false,"error":{...}}`
- Error objects include `kind`, `message`, and `exit_code` when `--json` is set
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
