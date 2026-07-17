[English](HOW_TO_USE.md) | [Português Brasileiro](HOW_TO_USE.pt-BR.md)

# How to Use — browser-automation-cli

> Install once, launch Chrome once per process, finish the task, exit clean. Lifecycle: BORN EXECUTE FINALIZE DIE.


## Prerequisites
- Rust 1.88.0 or newer when building from source
- Chrome or Chromium available on PATH for browser-engine commands
- Optional ffmpeg for experimental screencast file export
- Optional Lighthouse binary for audits, or pass `--lighthouse-path` to a mock
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
- Snapshot the page with `view`
- Click with `press` using a CSS selector or an `@eN` ref
- Fill inputs with `write` and multi-field forms with `fill-form`
- Wait with `wait --ms`, repeatable `--text` (OR), `--selector`, and optional `--state`
- Capture a screenshot with `grab --path /tmp/page.png` (flag, not a positional path)
- Scrape page content with `scrape` when you need text, markdown, html, links, or metadata
- List the live inventory with `commands --json`
- Discover argv shapes with `schema --cmd <name> --json`
- Print the product version with `version`

```bash
browser-automation-cli --timeout 60 --json goto https://example.com
browser-automation-cli --json view
browser-automation-cli --json wait --text "Example Domain" --ms 3000
browser-automation-cli --json grab --path /tmp/page.png --full-page
browser-automation-cli --json scrape https://example.com --format text --engine browser
```


## Multi-step Run
- Use `run --script` when `@eN` refs must survive across steps
- Separate process launches never share refs or the Chrome session
- One process is one lifecycle: BORN EXECUTE FINALIZE DIE
- There is no product daemon mode

```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500,"text":"Example Domain"}
{"cmd":"view"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.jsonl
```
- NDJSON lines use a `cmd` field matching a real subcommand name
- Global flags such as `--timeout` and `--step-timeout` apply to the whole script
- Prefer HTTP scrape paths when you only need content and not live refs


## Advanced Patterns
- Capture network in-process: `--capture-network` then `net list --json`
- Capture console in-process: `--capture-console` then `console list --json`
- Emulate without a named device profile:
  - `emulate --user-agent "Mozilla/5.0 ..."`
  - `emulate --viewport 390x844x3,mobile,touch`
  - `emulate --network-conditions "Slow 3G"`
- Wait for any of several texts (OR semantics): `wait --text A --text B --ms 5000`
- Scrape formats: `--format text|markdown|html|links|metadata`
- Scrape engines: `--engine http` (reqwest + scraper) or `--engine browser` (CDP)
- Prefer main content heuristics: `scrape ... --only-main-content`
- Batch scrape from a URL list: `batch-scrape --urls-file urls.txt --format text --concurrency 2`
- Discover sites with `crawl`, `map`, `search`, and local files with `parse`
- MITM one-shot proxy: `mitm start --seconds 30` (binds `127.0.0.1`)
- Workflow DAG journal: `workflow run|resume|status` (SQLite under XDG state)
- Deep heap tools require `--category-memory`
- Extension tools require `--category-extensions`
- Coordinate clicks require `--experimental-vision`
- Lighthouse with a mock path for CI: `lighthouse https://example.com --lighthouse-path mock --json`


## Configuration (XDG)
- Prefer flags for one-off agent calls
- Prefer XDG config via the `config` command for durable defaults
- There are no product `BROWSER_AUTOMATION_CLI_*` environment variables
- OS conventions only: `RUST_LOG` for tracing detail, `NO_COLOR` to disable ANSI color
- Layout commands: `config init`, `config path`, `config show`, `config set`, `config get`
- Supported keys: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`
- Color truthy values: `true`, `1`, `yes`
- Color falsy or other values resolve to off unless set truthy

```bash
browser-automation-cli --json config init
browser-automation-cli --json config path
browser-automation-cli --json config show
browser-automation-cli --json config set lang en
browser-automation-cli --json config set timeout 60
browser-automation-cli --json config set artifacts_dir /tmp/bac-artifacts
browser-automation-cli --json config set color true
browser-automation-cli --json config get lang
```
- Keep robots dual-flag policy explicit when bypassing: `--ignore-robots` plus `--i-accept-robots-risk`
- Config `ignore_robots` alone does not replace the dual-flag requirement on the command line


## Scrape, Crawl, Map, Search, Parse
```bash
# Single page as markdown over HTTP (no Chrome)
browser-automation-cli --json scrape https://example.com --format markdown --engine http --only-main-content

# Browser engine when JS rendering is required
browser-automation-cli --timeout 60 --json scrape https://example.com --format text --engine browser

# Many URLs (HTTP engine, one-shot)
printf '%s\n' 'https://example.com' 'https://example.org' > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2

# Crawl / map / search / parse local files
browser-automation-cli --json crawl https://example.com --same-host --limit 20 --max-depth 2 --format text
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
browser-automation-cli --json search "example domain" --limit 10
browser-automation-cli --json parse /tmp/page.html
```
- `scrape` defaults: `--format text`, `--engine browser`
- `batch-scrape` always uses the HTTP engine
- `crawl` stays on the seed host when you pass `--same-host`
- `parse` extracts text from local `html`, `md`, `txt`, and PDF paths
- Honor robots by default; dual-flag bypass when you intentionally skip policy


## MITM and Workflow
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list
browser-automation-cli --json mitm har

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
- Workflow journals live under XDG state (SQLite)
- Resume skips steps already marked `ok` in the journal
- Offline workflow steps are data-plane only
- Live multi-step browser work with shared `@eN` refs stays in `run --script`


## Common Errors
### Chrome missing
- Symptom: exit `69`, envelope kind `unavailable`, message about chrome not found
- Cause: Chrome or Chromium is not installed or not on PATH
- Fix: install Chromium or Google Chrome, ensure PATH, re-run `doctor --offline --quick --json`

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
- Fix: use only `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`

### Wrong schema or command name
- Symptom: exit `2`, message `unknown command for schema: ...` or clap `unrecognized subcommand`
- Cause: typo or invented subcommand / schema name
- Fix: run `commands --json`, then `schema --cmd <name> --json` with a listed name

### Grab path mistaken as positional
- Symptom: clap usage error around unexpected arguments
- Cause: screenshot destination was passed positionally
- Fix: use `grab --path /tmp/page.png` (and optional `--full-page`)


## Integration With Shell Scripts
- Always request machine-readable stdout with `--json`
- Inspect `$?` (or `$LASTEXITCODE`) before trusting the payload
- Pipe stdout into `jaq` / `jq` for field extraction
- Keep diagnostics on stderr with `--quiet` when you only want envelopes

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
- Discover inventory with `commands --json`
- Discover argv with `schema --cmd <name> --json`
- Collapse multi-step browser work into one `run --script` process when refs matter
- Prefer flags for one-off control; use `config` for durable XDG defaults
- Do not invent a daemon between agent turns
- Do not invent product env vars such as `BROWSER_AUTOMATION_CLI_*`
- OS env only when needed: `RUST_LOG`, `NO_COLOR`
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


## Next Steps
- Recipes and longer flows: [docs/COOKBOOK.md](COOKBOOK.md)
- Agent contract and lifecycle rules: [docs/AGENTS.md](AGENTS.md)
- JSON contracts: [docs/schemas/README.md](schemas/README.md)
- Platform and agent catalog: [INTEGRATIONS.md](../INTEGRATIONS.md)
- Version changes: [docs/MIGRATION.md](MIGRATION.md)
- Portuguese mirror: [docs/HOW_TO_USE.pt-BR.md](HOW_TO_USE.pt-BR.md)
