[English](HOW_TO_USE.md) | [Português Brasileiro](HOW_TO_USE.pt-BR.md)

# How to Use — browser-automation-cli

> Install once, launch Chrome once per process, finish the task, exit clean. Lifecycle: BORN EXECUTE FINALIZE DIE.


## Prerequisites
- Rust 1.88.0 or newer when building from source
- Chrome or Chromium available on PATH (or set XDG `chrome_path`) for browser-engine commands
- Optional ffmpeg for experimental screencast file export
- Optional Lighthouse binary for audits, or pass `--lighthouse-path` / XDG `lighthouse_path` to a mock
- A shell that can pipe stdout and inspect exit codes


## First Command in 60 Seconds
```bash
cargo install --path . --locked
browser-automation-cli doctor --offline --quick --json
browser-automation-cli --timeout 60 --json goto https://example.com
browser-automation-cli --json view
```
- Doctor checks Chrome discovery and one-shot readiness without a long network probe
- Goto navigates in a fresh one-shot process (BORN → EXECUTE → FINALIZE → DIE)
- View prints an accessibility snapshot with `@eN` refs for the current process only
- Prefer `--json` from the first call when a machine will parse stdout


## Core Commands
- Navigate with `goto`, `back`, `forward`, `reload`
- Snapshot the page with `view` (empty about:blank refuses silent success unless `--allow-empty`)
- Click with `press` using a CSS selector or an `@eN` ref
- Fill inputs with `write` and multi-field forms with `fill-form`
- Wait with `wait --ms`, repeatable `--text` (OR), `--selector` (CSS multi-selector OR), and optional `--state`
- Capture a screenshot with `grab --path /tmp/page.png` (flag, not a positional path)
- Print the page to PDF with `print-pdf --url <url> --path /tmp/page.pdf` (also valid inside `run`)
- Scrape page content with multi-format `scrape --format markdown,html,links` when you need several shapes at once
- Parse local files with `parse` (html/md/txt/pdf/docx/xlsx/ods; optional `--redact-pii`)
- Encode or decode QR codes with `qr encode|decode` (no Chrome)
- Discover filesystem paths with `find-paths` (regex pattern and/or `--glob '**/*.rs'`; no Chrome)
- Write XLSX from CSV/JSON with `sheet-write <input> -o <out.xlsx>` (no Chrome)
- Structural lint with `sg-scan [paths…]` and dry-run rewrite with `sg-rewrite [paths…]` (`--apply` to write)
- Check page change against a baseline with `monitor check`
- List the live inventory (61 agent names) with `commands --json`
- Discover argv shapes with `schema <name> --json` or `schema --cmd <name> --json`
- Print the product version with `version`
- Resolve XDG keys with `config list-keys --json`

```bash
browser-automation-cli --timeout 60 --json goto https://example.com
browser-automation-cli --json view
browser-automation-cli --json wait --text "Example Domain" --ms 3000
browser-automation-cli --json grab --path /tmp/page.png --full-page
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/page.pdf
browser-automation-cli --json schema run
```


## Multi-step Run
- Use `run --script` when `@eN` refs must survive across steps
- Separate process launches never share refs or the Chrome session
- One process is one lifecycle: BORN EXECUTE FINALIZE DIE
- There is no product daemon mode
- On fail-fast error, the error envelope may include partial `data.steps` for recovery
- Script body accepts **NDJSON** (one JSON object per line) **or** a top-level **JSON array** of step objects
- Final `--json` envelope includes `ok` and full `steps[].data`
- Global `--json-steps` streams one NDJSON line per step (`step`, `cmd`, `ok`, `result`)
- Multi-step only cmds: `select-option` / `pick` with `target` + `option` (not standalone clap)

```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500,"text":"Example Domain"}
{"cmd":"scroll","dy":1500}
{"cmd":"assert","url_contains":"example.com"}
{"cmd":"assert","text_contains":"Example Domain"}
{"cmd":"view"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.jsonl

# Same steps as a JSON array (GAP-A003)
cat > /tmp/demo.browser-automation.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"view"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.array.json

# Progressive step stream (GAP-020)
browser-automation-cli --timeout 60 --json --json-steps run --script /tmp/demo.browser-automation.array.json
```
- NDJSON lines and array elements use a `cmd` field matching a real subcommand or run inventory name
- Scroll accepts `dy`/`dx` as aliases for `delta_y`/`delta_x`
- Assert accepts `url_contains` / `text_contains` aliases and console kinds
- Wait accepts multi-selector OR and run fields `url` / `url_contains` / `navigation: true` (boolean); multi-selector success may include `matched_selector`
- Fill multi-field forms in run: `{"cmd":"fill-form","fields":[{"target":"…","value":"…"}]}`
- Beforeunload in run: `handle_before_unload` on `goto` / `reload`; isolated page: `{"cmd":"page","action":"new","isolated_context":true}`
- Global flags such as `--timeout` and `--step-timeout` apply to the whole script
- Prefer HTTP scrape paths when you only need content and not live refs


## Advanced Patterns
- Capture network in-process: `--capture-network` then `net list --json`
- Capture console in-process: `--capture-console` then `console list --json`
- Assert console clean: `assert console-empty` / `assert console-no-match --pattern TypeError` (needs capture)
- Emulate without a named device profile:
  - `emulate --user-agent "Mozilla/5.0 ..."`
  - `emulate --viewport 390x844x3,mobile,touch`
  - `emulate --network-conditions "Slow 3G"`
- Wait for any of several texts (OR semantics): `wait --text A --text B --ms 5000`
- Scrape formats: `--format text|markdown|html|links|metadata|summary|product|branding|raw-html|screenshot` (CSV or repeatable multi-format; alias `--formats`)
- Scrape engines: `--engine http` (reqwest + scraper) or `--engine browser` (CDP; formats apply to captured HTML)
- Optional operator webhook POST of scrape result data: `scrape ... --webhook-url https://127.0.0.1:9000/hook` (one-shot operator destination, not product telemetry)
- Prefer main content heuristics: `scrape ... --only-main-content`
- Batch scrape from a URL list: `batch-scrape --urls-file urls.txt --format text --concurrency 2` (default `--engine http`; use `--engine browser` for JS-rendered pages)
- Discover sites with `crawl` (`--engine http|browser`), `map`, `search`, and local files with `parse`
- LLM extract (fail-closed without keys): set XDG `openrouter_api_key`, optional `llm_base_url` / `llm_model`, then `extract <url> --llm --question '...'`
- MITM one-shot proxy: `mitm start --seconds 30` (binds `127.0.0.1`)
- MITM compose navigate+capture: `mitm capture-url https://example.com --seconds 30 --har /tmp/cap.har`
- MITM HAR export: `mitm har --out /tmp/capture.har` (required `--out`)
- MITM full surface: `status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact`
- Global MITM flags: `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`
- Workflow DAG journal: `workflow run|resume|status` (SQLite under XDG state)
- Deep heap tools require `--category-memory`
- Extension tools require `--category-extensions`
- Coordinate clicks require `--experimental-vision`
- Lighthouse binary resolve order: flag `--lighthouse-path` → XDG `lighthouse_path` → PATH
- Lighthouse envelope reports `binary_source` as `real` or `mock` (mock is honesty for e2e/smoke, not production)
- Lighthouse with a mock path: `lighthouse https://example.com --lighthouse-path ./scripts/mock-lighthouse.sh --json`
- Cache backend via XDG only: `config set cache_backend sqlite|memory|redis` and optional `config set cache_redis_url redis://127.0.0.1:6379`
- `rediss://` is fail-closed (plain TCP only; do not use rediss URLs)
- Doctor reports Chrome, lighthouse source, and `cache_redis` when Redis cache is configured
- Localize human suggestions: `--lang pt-BR` or `config set lang pt-BR`
- Verbosity: `--verbose` (info), `--debug` (max), `-q`/`--quiet`, or `config set log_level debug`
- Color: `config set color true|false` (truthy values: `true`, `1`, `yes`)
- Chrome path: `config set chrome_path /path/to/chrome` when PATH discovery is not enough
- Dialog soft path: `dialog accept --if-present` / `dialog dismiss --if-present` when a dialog may be absent
- Beforeunload (GAP-003): `goto --handle-before-unload accept|dismiss` and `reload --handle-before-unload accept|dismiss`; run field `handle_before_unload`
- Goto options: `--init-script`, `--handle-before-unload`, `--navigation-timeout-ms`
- Reload ignore cache (GAP-005): `reload --ignore-cache`
- Isolated context (GAP-004): `page new --isolated-context` (flag alone → `default-isolated`) or `page new --isolated-context <name>`; run `isolated_context` string or `true`
- Wait multi-selector CSS OR: `wait --selector '#a, #b'`; run fields `url`, `url_contains`, `navigation: true`; success may include `matched_selector` in result data
- Clap usage errors with `--json` emit JSON error envelopes (GAP-002)
- `console dump --path …` always writes a valid JSON array (`[]` when empty) (GAP-021)
- `print-pdf` refuses blank PDF without navigated content or `url` (GAP-013)
- Extension `install` / `uninstall` intentionally outside `run` (GAP-007); discover via `schema` / `commands`
- Assert dual surface (GAP-014): CLI `assert url|text|console|console-empty|console-no-match` vs run kinds (`url` / `text` / `console` / `console_empty` / `console_no_match`)
- Scrape multi-format (GAP-018): `--format` multi/CSV and alias `--formats` where supported


## Configuration (XDG)
- Prefer flags for one-off agent calls
- Prefer XDG config via the `config` command for durable defaults
- Product settings are flags and XDG CLI only: `config init`, `config path`, `config show`, `config set`, `config get`, `config list-keys`
- Resolve live config/data/state paths with `config path --json`
- Product logging is controlled by `--verbose` / `--debug` / `-q` and XDG `log_level`
- Supported keys (full list of 16): `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- Color truthy values: `true`, `1`, `yes`
- Color falsy or other values resolve to off unless set truthy

```bash
browser-automation-cli --json config init
browser-automation-cli --json config path
browser-automation-cli --json config show
browser-automation-cli --json config set lang en
browser-automation-cli --json config set timeout 60
browser-automation-cli --json config set artifacts_dir /tmp/browser-automation-cli-artifacts
browser-automation-cli --json config set ignore_robots false
browser-automation-cli --json config set namespace demo
browser-automation-cli --json config set color true
browser-automation-cli --json config set log_level info
browser-automation-cli --json config set chrome_path /usr/bin/chromium
browser-automation-cli --json config set lighthouse_path ./scripts/mock-lighthouse.sh
browser-automation-cli --json config set openrouter_api_key YOUR_KEY
browser-automation-cli --json config set llm_base_url https://openrouter.ai/api/v1
browser-automation-cli --json config set llm_model openai/gpt-4o-mini
browser-automation-cli --json config set log_to_file false
browser-automation-cli --json config set cache_backend sqlite
browser-automation-cli --json config set cache_redis_url redis://127.0.0.1:6379
browser-automation-cli --json config list-keys
browser-automation-cli --json config get lang
```
- Use `redis://` only for Redis cache; `rediss://` is rejected fail-closed
- Discover keys and defaults with `config list-keys --json`
- Keep robots dual-flag policy explicit when bypassing: `--ignore-robots` plus `--i-accept-robots-risk`
- Config `ignore_robots` alone does not replace the dual-flag requirement on the command line


## Scrape, Crawl, Map, Search, Parse, PDF, QR, Paths
```bash
# Single page as markdown over HTTP (no Chrome)
browser-automation-cli --json scrape https://example.com --format markdown --engine http --only-main-content

# Browser engine formats apply to captured outerHTML (markdown, links, …)
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --timeout 60 --json scrape https://example.com --format links --engine browser

# Multi-format in one invocation (GAP-009)
browser-automation-cli --json scrape https://example.com --format markdown,html,links --engine http

# Optional one-shot operator webhook POST of scrape result data (not product telemetry)
browser-automation-cli --json scrape https://example.com --format markdown --engine http \
  --webhook-url https://127.0.0.1:9000/hook

# Many URLs: default HTTP engine; optional browser engine per URL (GAP-010)
printf '%s\n' 'https://example.com' 'https://example.org' > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format markdown --engine browser --concurrency 1

# Crawl / map / search / parse local files
browser-automation-cli --json crawl https://example.com --same-host --limit 20 --max-depth 2 --format text
browser-automation-cli --timeout 120 --json crawl https://example.com --same-host --limit 5 --engine browser
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
browser-automation-cli --json search "example domain" --limit 10
browser-automation-cli --json parse tests/fixtures/hello.pdf
browser-automation-cli --json parse tests/fixtures/hello.docx --redact-pii
# xlsx/ods spreadsheets are also supported:
# browser-automation-cli --json parse /tmp/sheet.xlsx
# browser-automation-cli --json parse /tmp/sheet.ods --redact-pii

# PDF print, monitor baseline, QR, path discovery
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/page.pdf
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/mon.base --write-baseline
browser-automation-cli --json qr encode --text 'hello' --format png --path /tmp/qr.png
browser-automation-cli --json qr decode --path /tmp/qr.png
browser-automation-cli --json find-paths 'Cargo.*' .
browser-automation-cli --json find-paths --glob '**/*.rs' .
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data
browser-automation-cli --json sg-scan . --limit 100
browser-automation-cli --json sg-rewrite .
# dry-run by default; write only with --apply
# browser-automation-cli --json sg-rewrite . --apply
```
- `scrape` defaults: `--format text`, `--engine browser`
- Browser engine respects `--format` (not silent text-only)
- Multi-format returns per-format fields in the envelope when more than one format is requested
- `batch-scrape` defaults to HTTP engine; pass `--engine browser` for CDP per URL
- `crawl` defaults to HTTP BFS; pass `--engine browser` when JS rendering is required
- `crawl` stays on the seed host when you pass `--same-host`
- `parse` extracts text from local `html`, `md`, `txt`, `pdf`, `docx`, `xlsx`, and `ods` paths
- `--redact-pii` redacts common PII patterns in parse output
- `--webhook-url` on `scrape` POSTs the result data once to an operator URL (not product telemetry)
- Honor robots by default; dual-flag bypass when you intentionally skip policy


## LLM Extract (XDG keys)
```bash
browser-automation-cli --json config set openrouter_api_key YOUR_KEY
browser-automation-cli --json config set llm_base_url https://openrouter.ai/api/v1
browser-automation-cli --json config set llm_model openai/gpt-4o-mini
browser-automation-cli --json extract https://example.com --llm --question 'What is the title?'
```
- Keys are stored only under XDG via `config set`
- Without `openrouter_api_key`, `extract --llm` fails closed with a usage envelope
- Optional `--schema-json` points at a local JSON Schema file for structured answers


## i18n
```bash
browser-automation-cli --lang pt-BR --json click-at --x 1 --y 1
# usage error shows localized suggestion when lang is pt-BR (needs --experimental-vision for success)
browser-automation-cli --json config set lang pt-BR
```
- Human messages and suggestions honor `--lang` and XDG `lang`
- Machine envelopes keep English-stable `kind` / `exit_code` fields


## MITM and Workflow
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list
browser-automation-cli --json mitm har --out /tmp/capture.har
browser-automation-cli --json mitm capture-url https://example.com --seconds 30 --har /tmp/cap.har
browser-automation-cli --json mitm redact --secrets
browser-automation-cli --json mitm graphql
browser-automation-cli --json mitm ws

cat > /tmp/wf.json <<'JSON'
{
  "name": "demo",
  "steps": [
    {"id": "a", "cmd": "echo", "args": {"message": "hello"}},
    {"id": "b", "cmd": "scrape", "args": {"url": "https://example.com", "engine": "http"}, "depends_on": ["a"]}
  ]
}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
```
- MITM binds loopback only (`127.0.0.1`) with an ephemeral port
- MITM CA lives under XDG data; captures live under XDG state
- `mitm har` requires `--out <path>`
- `mitm capture-url` one-shot: proxy + Chrome + navigate + capture
- Workflow journals live under XDG state (SQLite)
- Resume skips steps already marked `ok` in the journal
- Offline workflow steps are data-plane only
- Live multi-step browser work with shared `@eN` refs stays in `run --script`


## Common Errors
### Chrome missing
- Symptom: exit `69`, envelope kind `unavailable`, message about chrome not found
- Cause: Chrome or Chromium is not installed or not on PATH / `chrome_path`
- Fix: install Chromium or Google Chrome, set `config set chrome_path`, re-run `doctor --offline --quick --json`

### Timeout
- Symptom: exit `124`, envelope kind `timeout`
- Cause: navigation or step exceeded `--timeout` / wait budget
- Fix: raise `--timeout`, use targeted `wait --text` / `--selector`, or prefer `--engine http` when CDP is unnecessary

### Robots dual-flag incomplete
- Symptom: exit `2`, message `--ignore-robots requires --i-accept-robots-risk`
- Cause: only one robots bypass flag was passed
- Fix: pass both `--ignore-robots` and `--i-accept-robots-risk` together when intentional

### Broken pipe (exit 141)
- Symptom: exit `141`, envelope kind `broken-pipe` when the consumer closes stdout early
- Cause: piping into a closed reader (for example a head that exits mid-stream)
- Fix: read full stdout before closing, or avoid early pipe teardown; treat `141` as expected pipe semantics

### Unknown config key
- Symptom: exit `2`, message `unknown config key: ...`
- Cause: `config set` received a key outside the supported set
- Fix: use only `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`

### LLM keys missing
- Symptom: exit `2`, message `LLM extract requires XDG openrouter_api_key`
- Cause: `extract --llm` without XDG key
- Fix: `config set openrouter_api_key YOUR_KEY` (and optional `llm_base_url` / `llm_model`)

### Redis rediss URL rejected
- Symptom: exit non-zero / config or cache error when `cache_redis_url` uses `rediss://`
- Cause: Redis client is plain TCP only; `rediss://` is fail-closed (GAP-A007)
- Fix: use `config set cache_redis_url redis://127.0.0.1:6379` for local Redis

### HTTP scrape rejects file://
- Symptom: exit `2` usage when `scrape --engine http` receives a `file://` URL
- Cause: HTTP engine is network-only (GAP-A004)
- Fix: use `--engine browser` for file pages, or `parse` for local files

### Empty view on about:blank
- Symptom: exit non-zero / usage when `view` runs without navigation
- Cause: empty about:blank refuses silent success (GAP-012)
- Fix: navigate with `goto` first, or pass `--allow-empty` only when intentional

### Wrong schema or command name
- Symptom: exit `2`, message `unknown command for schema: ...` or clap `unrecognized subcommand`
- Cause: typo or invented subcommand / schema name
- Fix: run `commands --json`, then `schema <name> --json` with a listed name
- Note: `select-option` and `pick` are run/schema inventory only (not clap standalone)

### Grab path mistaken as positional
- Symptom: clap usage error around unexpected arguments
- Cause: screenshot destination was passed positionally
- Fix: use `grab --path /tmp/page.png` (and optional `--full-page`)

### Clap usage errors under --json (GAP-002)
- Symptom: usage failures still need machine parsing
- Cause: invalid argv with `--json` already on the command line
- Fix: read the JSON error envelope on stdout (`ok: false`); do not scrape clap prose alone

### Blank print-pdf (GAP-013)
- Symptom: exit non-zero when printing without page content
- Cause: `print-pdf` refuses blank about:blank without navigated content or a `url`
- Fix: pass `--url` / step `url`, or `goto` first in the same `run` process

### Wait navigation is boolean
- Symptom: wait step ignored or rejected when using a string like `"load"`
- Cause: run field `navigation` is boolean `true`, not a string lifecycle name
- Fix: use `{"cmd":"wait","navigation":true}`


## Integration With Shell Scripts
- Always request machine-readable stdout with `--json`
- Inspect `$?` (or `$LASTEXITCODE`) before trusting the payload
- Pipe stdout into `jaq` / `jq` for field extraction
- Keep diagnostics on stderr with `--quiet` when you only want envelopes
- On `run` errors, inspect partial `data.steps` when present
- Use `--json-steps` when progressive step lines are easier to stream than a single final envelope

```bash
browser-automation-cli --timeout 60 --json goto https://example.com \
  | jaq -e '.ok == true'

browser-automation-cli --json scrape https://example.com --format markdown --engine http \
  | jaq -r '.data // .'

printf '%s\n' 'https://example.com' > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2 \
  | jaq .
```
- Broken pipe exit `141` means the reader closed early, not necessarily a CLI bug
- Prefer HTTP scrape / batch / crawl paths in pure shell pipelines that do not need CDP


## Integration With AI Agents
- Spawn `browser-automation-cli` as a one-shot subprocess per task boundary
- Pass `--json` on every programmatic call
- Parse only stdout envelopes; treat stderr as diagnostics
- Branch on envelope field `ok` and process exit code
- Discover inventory with `commands --json` (61 agent names)
- Discover argv with `schema <name> --json` or `schema --cmd <name> --json`
- Collapse multi-step browser work into one `run --script` process when refs matter
- Prefer flags for one-off control; use `config` for durable XDG defaults
- Do not invent a daemon between agent turns
- Configure product settings only with flags and `config set` / `config get` / `config path`
- Product logging uses `--verbose` / `--debug` / `-q` or `config set log_level`
- Color uses `config set color`; Chrome path uses `config set chrome_path`
- Compatible editors and runners include Claude Code, Codex, Cursor, Continue, and Cline via shell or subprocess
- Full agent contract: [docs/AGENTS.md](AGENTS.md) and [INTEGRATIONS.md](../INTEGRATIONS.md)


## Integration With Rust Crates
- Call the binary with `std::process::Command`
- Capture stdout, check status, deserialize with `serde_json`
- Keep the binary name exact: `browser-automation-cli`

```rust
use serde_json::Value;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("browser-automation-cli")
        .args([
            "--json",
            "scrape",
            "https://example.com",
            "--format",
            "text",
            "--engine",
            "http",
        ])
        .output()?;

    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(output.status.code().unwrap_or(1));
    }

    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    if envelope.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        eprintln!("envelope not ok: {envelope}");
        std::process::exit(1);
    }

    println!("{envelope}");
    Ok(())
}
```
- Prefer HTTP `scrape` in unit-style checks that must not launch Chrome
- Use `run --script` when the crate orchestrates multi-step CDP flows
- See crate-oriented notes in [docs/AGENTS.md](AGENTS.md) and [INTEGRATIONS.md](../INTEGRATIONS.md)


## Full Command Inventory (61)
- Live source of truth: `browser-automation-cli commands --json` (61 agent-facing names)
- Clap top-level help lists 59 without `select-option` and `pick` as standalone subcommands
- DevTools tool-ref e2e covers **53** tools (`scripts/e2e_all_52_tools.sh` filename is legacy; suite runs 53)
- Full agent command list:
  - Meta: `doctor`, `commands`, `schema`, `version`, `completions`
  - Navigate: `goto`, `back`, `forward`, `reload`, `page`, `wait`, `dialog`
  - Interact: `press`, `click-at`, `write`, `keys`, `type`, `hover`, `drag`, `fill-form`, `upload`, `scroll`
  - Multi-step / schema only: `select-option`, `pick`
  - Observe: `view`, `eval`, `text`, `attr`, `assert`, `cookie`, `console`, `net`
  - Capture: `grab`, `print-pdf`, `monitor`, `screencast`, `lighthouse`
  - Multi-step: `run`, `exec`
  - Extract/scrape: `extract`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
  - Local IO (no Chrome): `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`
  - Infra: `config`, `mitm`, `workflow`
  - Emulation/perf: `emulate`, `resize`, `perf`, `heap`
  - Category gates: `extension`, `devtools3p`, `webmcp`
- Discover argv with `schema <name> --json` for any name above

## Next Steps
- Recipes and longer flows: [docs/COOKBOOK.md](COOKBOOK.md)
- Agent contract and lifecycle rules: [docs/AGENTS.md](AGENTS.md)
- JSON contracts: [docs/schemas/README.md](schemas/README.md)
- Platform and agent catalog: [INTEGRATIONS.md](../INTEGRATIONS.md)
- Version changes: [docs/MIGRATION.md](MIGRATION.md)
- Portuguese mirror: [docs/HOW_TO_USE.pt-BR.md](HOW_TO_USE.pt-BR.md)
