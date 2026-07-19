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
- v0.1.5 residual-zero disk: BORN + FINALIZE Singleton GC, doctor `residual_disk` / JSON `residual`, meta cmds `locale` and `man`, inventory 63
- Carry-forward from v0.1.4 agent contracts: `--json-steps`, wait multi/url, pick/select-option, assert console, schema positional, MITM capture-url, clap JSON usage errors


## Economy
- Avoid long-lived browser servers that leak across agent turns
- Pay Chrome launch cost only when the task needs a real page
- Prefer HTTP `scrape` / `batch-scrape` / `crawl` / `map` when content alone is enough
- Collapse multi-step flows into one `run` process when refs matter
- Stream progressive feedback with `--json-steps` instead of re-spawning for status
- Reuse `schema <cmd>` once per session instead of re-deriving argv by guesswork


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
- Use `commands --json` to discover the live inventory (**63 agent names**)
- Inventory includes config, mitm, workflow, scrape, batch-scrape, crawl, map, search, parse, print-pdf, monitor, qr, find-paths, sheet-write, sg-scan, sg-rewrite, extract, select-option, pick, locale, man, and DevTools-parity tools (63 total; e2e 53 tools)
- Note: `select-option` and `pick` are multi-step/schema surface only (not standalone clap subcommands; clap top-level lists **61** without them)
- Use `schema <name> --json` or `schema --cmd <name> --json` before generating argv for unfamiliar commands
- Prefer flags for one-off control
- Use `config init|set|get|path|show|list-keys` for durable XDG defaults
- Full config keys (16) via `config list-keys`: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- Resolve paths with `config path --json`
- For multi-step work that needs shared `@eN` refs, use one `run --script` process (NDJSON **or** JSON array of steps)
- Final `run --json` envelope includes `ok` and full `steps[].data`
- Stream per-step NDJSON with global `--json-steps` (`step`, `cmd`, `ok`, `result`)
- Wait with OR text: `wait --text A --text B`
- Wait multi-selector CSS OR and run fields `url` / `url_contains` / `navigation: true` (boolean); may return `matched_selector`
- Pick option menus: `{"cmd":"pick","target":"…","option":"…"}` or `select-option`
- Scroll aliases in NDJSON: `{"cmd":"scroll","dy":1500}`
- Assert aliases: `{"cmd":"assert","url_contains":"example.com"}` / `text_contains`
- Assert console: `{"cmd":"assert","kind":"console_empty"}` or `console_no_match` + `pattern` (needs `--capture-console`)
- CLI assert: `assert console-empty` / `assert console-no-match --pattern …`
- On `run` fail-fast errors, inspect partial `data.steps` when present
- Scrape with multi-format `--format text|markdown|html|links|metadata|summary|product|branding|raw-html|screenshot` and `--engine http|browser`
- Batch/crawl: optional `--engine browser` (default http)
- Optional operator webhook on scrape: `--webhook-url` (one-shot POST, not product telemetry)
- Capture screenshots with `grab --path <file>` (not a positional path)
- Print PDF with `print-pdf --url … --path …` (also inside `run`)
- View blank pages: pass `--allow-empty` only when intentional
- LLM extract fails closed without XDG `openrouter_api_key`
- Localize human suggestions with `--lang pt-BR` or `config set lang pt-BR` (flags + XDG only)
- Inspect resolved locale with `locale --json`; generate man page with `man`
- After browser work, expect residual-zero disk: doctor check `residual_disk` and top-level `residual` (`cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes`)
- Clap usage errors emit JSON when `--json` is already on argv (GAP-002)
- Beforeunload (GAP-003): `goto`/`reload --handle-before-unload accept|dismiss`; run field `handle_before_unload`
- Isolated context (GAP-004): `page new --isolated-context [name]` (flag alone → `default-isolated`); run `isolated_context` string or `true`
- Extension install/uninstall intentionally outside `run` (GAP-007); discover via `schema`/`commands`
- Assert dual surface (GAP-014): CLI `assert url|text|console|console-empty|console-no-match` vs run kinds
- `console dump` always writes a valid JSON array (`[]` when empty) (GAP-021)
- Wait multi-selector success may include `matched_selector`; run `navigation` is boolean `true`
- Scrape multi-format alias `--formats` where supported (GAP-018)
- `print-pdf` refuses blank without navigated content/`url` (GAP-013)


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
- Inventory: `browser-automation-cli commands --json` (**63** agent names)
- Input fragments: `browser-automation-cli schema <name> --json` or `schema --cmd <name> --json`
- Config paths: `browser-automation-cli config path --json`
- Config keys: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- MITM: `mitm status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact`
- Global MITM: `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`
- Workflow: `workflow run|resume|status`
- Local scrape surface: `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
- Artifacts and local IO: `print-pdf`, `monitor check`, `qr encode|decode`, `find-paths` (`--glob`), `sheet-write`, `sg-scan`, `sg-rewrite`
- Multi-step only: `select-option`, `pick`
- Meta: `locale` (UI locale diagnostics), `man` (roff man page; no Chrome)
- LLM extract: `extract --llm --question …` (XDG keys only)
- Health: `doctor --json` (Chrome discovery, XDG browsers_dir, lighthouse source, `cache_redis` when configured, residual disk hygiene)
- Residual: top-level `residual` + check `residual_disk` with fields `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes`
- Cache: XDG `cache_backend` (`sqlite|memory|redis`) and `cache_redis_url` (`redis://` only; `rediss://` fail-closed)
- Lighthouse: flag → XDG `lighthouse_path` → PATH; envelope `binary_source` is `real` or `mock`


## Full Command Inventory (63)
- Live source of truth: `browser-automation-cli commands --json` (**63** agent-facing names)
- Clap top-level help lists **61** without `select-option` and `pick` as standalone
- DevTools tool-ref e2e covers **53** tools (`scripts/e2e_all_52_tools.sh` filename is legacy; suite runs 53)
- Full agent command list:
  - Meta / discovery: `doctor`, `commands`, `schema`, `version`, `locale`, `completions`, `man`
  - Navigate: `goto`, `back`, `forward`, `reload`, `page`, `wait`, `dialog`
  - Interact: `press`, `click-at`, `write`, `keys`, `type`, `hover`, `drag`, `fill-form`, `upload`, `scroll`
  - Multi-step / schema only: `select-option`, `pick`
  - Observe: `view`, `eval`, `text`, `attr`, `assert`, `cookie`, `console`, `net`
  - Capture: `grab`, `print-pdf`, `monitor`, `screencast`, `lighthouse`
  - Multi-step: `run`, `exec`
  - Extract / scrape: `extract`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
  - Local IO (no Chrome): `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`
  - Infra: `config`, `mitm`, `workflow`
  - Emulation / perf: `emulate`, `resize`, `perf`, `heap`
  - Category gates: `extension`, `devtools3p`, `webmcp`
- Discover argv with `schema <name> --json` for any name above

## Lifecycle
- Slogan (English): BORN EXECUTE FINALIZE DIE
- One process owns one Chrome session from launch through FINALIZE
- BORN scavenges stale Singleton-only Chromium tmp (age floor 60s)
- FINALIZE is idempotent (Browser.close, wait, kill fallback) and dual-scavenges invocation-window + stale Singleton orphans
- Residual contract for agents: after DIE expect zero live CLI marker processes, zero CLI marker dirs, zero owned Singleton-only Chromium tmp litter
- Host Flatpak Chrome is never killed or wiped by product residual GC
- Do not expect session or `@eN` refs to survive process exit
- Verify with `doctor --offline --quick --json` → `residual` / check `residual_disk`


## Technical Contract (v0.1.5)
### REQUIRED
- Pass `--json` for programmatic consumption
- Treat one process as one Chrome lifecycle (BORN EXECUTE FINALIZE DIE)
- Use `run --script` for multi-step work that needs shared `@eN` refs (NDJSON or JSON array)
- Prefer `--json-steps` when the agent needs progressive step feedback (stream per-step NDJSON)
- Prefer schema positional: `schema <cmd> --json` (also `schema --cmd <cmd> --json`)
- Use dialog soft path when optional: `dialog accept --if-present` / `dialog dismiss --if-present`
- Check process exit code before trusting stdout
- Branch on envelope field `ok`
- Keep category and experimental gates explicit when needed
- Configure durable product settings via `config` / flags only (`--lang` + XDG for language)
- Discover unknown commands with `commands --json` and `schema <cmd>` or `schema --cmd`
- After browser one-shots, treat residual-zero as part of success: inspect doctor `residual` when diagnosing leaks

### FORBIDDEN
- Do not keep a daemon between agent turns
- Do not invent product aliases such as `bac`, `click`, or `screenshot`
- Do not reuse `@eN` refs across separate process launches
- Do not parse stderr as the primary success channel
- Do not enable robots bypass without the dual-flag policy
- Use only flags and `config` for product settings
- Do not invent product environment variables for config (flags + XDG `config` only)
- Do not pass a positional path to `grab`; use `--path`
- Do not invent a `--device` preset on `emulate`; use `--user-agent`, `--viewport`, `--network-conditions`
- Do not treat `select-option` / `pick` as clap standalone subcommands; use `run` / `exec` steps
- Do not assume silent success for empty `view` on about:blank without `--allow-empty`
- Do not assume `print-pdf` succeeds without a navigated page or an explicit `url` (GAP-013); residual smokes may use `print-pdf --url about:blank` as a light one-shot when `url` is present
- Do not kill or ask the CLI to wipe host Flatpak Chrome residual

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
browser-automation-cli -q --json schema run
browser-automation-cli -q --json --json-steps run --script '[{"cmd":"goto","url":"https://example.com"},{"cmd":"view"}]'
browser-automation-cli -q --json mitm capture-url https://example.com --seconds 20
browser-automation-cli -q --capture-console --json assert console-empty
browser-automation-cli -q --timeout 60 --json goto https://example.com --handle-before-unload accept
browser-automation-cli -q --json page new --isolated-context
browser-automation-cli -q --json dialog accept --if-present
browser-automation-cli -q --capture-console --json console dump --path /tmp/console.json
browser-automation-cli -q --json schema pick
browser-automation-cli -q --json locale
browser-automation-cli -q --json doctor --offline --quick
```


## JSON Envelope
- Success: `{"schema_version":1,"ok":true,"data":...}`
- Error: `{"schema_version":1,"ok":false,"error":{...}}`
- Error objects include `kind`, `message`, and `exit_code` when `--json` is set
- Multi-step fail-fast errors may also include partial `data.steps`
- `run --json` success includes `ok` and full `steps[].data`
- `--json-steps` streams one NDJSON object per step: `step`, `cmd`, `ok`, `result`
- Clap usage errors with `--json` on argv emit JSON error envelopes
- Schema index: [docs/schemas/README.md](schemas/README.md)
- Live input fragments always come from `schema <cmd>` / `schema --cmd`; static files may lag


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
