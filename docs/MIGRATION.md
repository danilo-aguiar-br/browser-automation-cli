[English](MIGRATION.md) | [Português Brasileiro](MIGRATION.pt-BR.md)

# Migration — browser-automation-cli

> Move to the one-shot process model without guessing the command map. Lifecycle: BORN EXECUTE FINALIZE DIE.


## What Changes
- `0.1.0` is the first public product line
- Canonical command names are `view`, `press`, `write`, and `grab`
- Multi-step automation must use `run --script` in one process
- Category and experimental surfaces are opt-in
- Lifecycle slogan is English only: BORN EXECUTE FINALIZE DIE


## Baseline 0.1.0
- One-shot Chrome launch and FINALIZE cleanup in a single process
- Core navigation and interaction: `goto`, `view`, `press`, `write`, `grab`, `run`
- DevTools parity surface for input, snapshot, network, console, pages, wait, perf, lighthouse, screencast, heap, extensions
- Schema discovery via `schema --cmd` and inventory via `commands --json`
- Robots dual-flag policy for explicit bypass
- Category gates such as `--category-memory` and `--category-extensions`
- Experimental gates such as `--experimental-vision` and `--experimental-screencast`
- Explicitly out of 0.1.0 only: local MITM, workflow journal, and local crawl/map/search surface


## 0.1.0 → 0.1.1
### Configuration and XDG
- Product settings use CLI flags and XDG via `config init|set|get|path|show` only
- `config path --json` reports `config_dir`, `data_dir`, `state_dir`, `mitm_ca_dir`, `mitm_capture_dir`, `workflow_dir`, and related paths
- Encryption key is set with `config set encryption_key`
- Product logging is flags + XDG (`--verbose` / `--debug` / `-q` or `config set log_level`)
- Color is `config set color`; Chrome path is `config set chrome_path`
- Doctor gains an XDG `browsers_dir` check

### MITM
- New local MITM surface on hudsucker
- `mitm start` binds `127.0.0.1` with an ephemeral port in one-shot mode
- Related commands: `status`, `init-ca`, `list`, `get`, `har`, `export`, `domains`, `apis`
- CA material lives under XDG data; captures under XDG state

### Workflow
- New workflow journal DAG (petgraph + SQLite)
- Commands: `workflow run`, `workflow resume`, `workflow status`
- Journals live under XDG state
- `workflow resume` skips steps already marked `ok`

### Local scrape surface
- New commands: `batch-scrape`, `crawl`, `map`, `search`, `parse`
- `scrape` gains `--format` (`text|markdown|html|links|metadata`)
- `scrape` gains `--engine` (`http|browser`) and `--only-main-content`
- Batch scrape uses bounded concurrency via Tokio `JoinSet`

### Interaction and capture flags
- `wait` accepts repeatable `--text` values with OR semantics (any match wins)
- `grab` uses `--path` (not a positional path)
- `emulate` uses `--user-agent`, `--viewport`, `--network-conditions` (no `--device` preset)
- `run` gains scrape parity options and enforces category gates inside script steps

### Packaging and docs
- Public bilingual documentation and skills package for crates.io
- Dual license `MIT OR Apache-2.0`
- Validation is local with cargo and e2e scripts


## 0.1.1 → 0.1.2
High-level GAP fixes and surface growth landed in `0.1.2`:

### Browser scrape and formats
- Browser engine scrape captures `outerHTML` and applies `--format` (markdown/html/links/metadata/…) instead of silent text-only
- Additional scrape format tokens: `summary`, `product`, `branding`, plus `raw-html` / `rawHtml` aliases and `screenshot` format token
- Optional scrape `--webhook-url` one-shot operator POST of result data (not product telemetry)

### Run script ergonomics
- Scroll NDJSON accepts `dy` / `dx` aliases for `delta_y` / `delta_x`
- Assert accepts `url_contains` / `text_contains` aliases
- Fail-fast `run` errors return partial `data.steps` on the error envelope for recovery
- `schema --cmd` expanded for goto/eval/type/scroll/assert tool-ref flags
- `exec` help describes the full step surface

### Logging, Chrome, and Lighthouse paths
- Product settings stay flags + XDG only
- Logging uses `--verbose` / `--debug` / `-q` and XDG `log_level`
- Chrome path via XDG `chrome_path`; Lighthouse via XDG `lighthouse_path` (plus flag)
- Color via XDG `color`

### i18n
- Human suggestions localize for `pt-BR` via `--lang` and XDG `config set lang`

### Search and attr
- Search cleans SERP redirect wrappers (`uddg=`) to destination URLs
- `attr` falls back to DOM properties when HTML attributes are null

### New commands and parse/LLM
- `print-pdf` — CDP `Page.printToPDF` one-shot artifact
- `monitor check` — baseline hash/text compare with optional `--write-baseline`
- `qr encode|decode` — no Chrome
- `find-paths` — fd-like path discovery (no Chrome)
- `parse` — PDF (lopdf), DOCX, xlsx/ods (calamine), plus `--redact-pii`
- `extract --llm` / `--question` / `--schema-json` with XDG-only keys: `openrouter_api_key`, `llm_base_url`, `llm_model` (fail-closed without key)
- MITM reports `ws_count`

### Config keys (full list in 0.1.2)
- `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`
- Plus: `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`

### Inventory
- Live inventory is **56 commands** (`commands --json`)
- DevTools tool-ref e2e suite remains **52 tools** (`scripts/e2e_all_52_tools.sh`)
- Static schemas regenerate via `bash scripts/generate_command_schemas.sh`


## Step-by-Step Migration
### From any older tree to 0.1.1
- Install or rebuild the binary for at least `0.1.1`
- Replace session-daemon calls with one-shot subprocess invocations
- Rewrite multi-step agent plans into NDJSON scripts for `run`
- Switch output consumers to `--json` envelopes
- Move durable defaults into `config set` or keep them as explicit flags
- Move encryption material to `config set encryption_key <secret>`
- Map old tool names through `commands --json` and the DevTools tool map
- Update screenshot callers to `grab --path <file>`
- Update waits that need alternate text to repeatable `--text` (OR)
- Update scrape callers to pass `--format` and `--engine` explicitly when needed

### From 0.1.1 to 0.1.2
- Rebuild/install `0.1.2`
- Use `--verbose`, `--debug`, `-q`, or `config set log_level` for product logging
- Prefer XDG `chrome_path` / `lighthouse_path` when PATH discovery is brittle
- Prefer `config set color` for ANSI color defaults
- Expect browser scrape formats to work (`--engine browser --format markdown|links|…`)
- Prefer scroll aliases `dy`/`dx` and assert aliases `url_contains`/`text_contains` in NDJSON
- On `run` failures, parse partial `data.steps` when present
- Discover new commands: `print-pdf`, `monitor`, `qr`, `find-paths`
- For operator scrape webhooks, pass `--webhook-url` on `scrape`
- For LLM extract, set XDG keys only via `config set`:
```bash
browser-automation-cli --json config set openrouter_api_key YOUR_KEY
browser-automation-cli --json config set llm_base_url https://openrouter.ai/api/v1
browser-automation-cli --json config set llm_model openai/gpt-4o-mini
browser-automation-cli --json extract https://example.com --llm --question 'What is the title?'
```
- Use `--lang pt-BR` or `config set lang pt-BR` for localized human suggestions
- Confirm inventory with `commands --json` (56) and regenerate schemas if packaging docs
- Re-run local validation with cargo and e2e scripts: `cargo test --lib`, e2e 52-tool script, residual smokes you care about


## JSON Schema Changes
- Before: free-form prose or ad-hoc JSON without `schema_version`
- After success:
```json
{"schema_version":1,"ok":true,"data":{}}
```
- After error with `--json`:
```json
{"schema_version":1,"ok":false,"error":{"message":"..."}}
```
- Error envelopes also carry `kind` and `exit_code` for programmatic branching
- Fail-fast multi-step errors may include partial `data` (for example `data.steps`)
- Live per-command input fragments come from `schema --cmd`
- Static snapshots under `docs/schemas/` are a convenience index and may lag the binary
- v0.1.1 static additions include `config`, `mitm`, `workflow`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`, and `wait`
- v0.1.2 static additions include `print-pdf`, `monitor`, `qr`, `find-paths` (regenerate with the generator)
- Prefer live `schema --cmd` after upgrades to confirm the installed binary


## Compatibility Notes
- No previous stable crates.io line exists for this repository before `0.1.0`
- Branding and history cleanup recreated a clean public root commit
- First crates.io publish still requires explicit maintainer approval
- Agents that hard-coded settings outside flags/`config` must migrate to flags + `config set`
- Agents that controlled product verbosity outside flags/`log_level` must migrate to `--verbose` / `--debug` / `config set log_level`
- Subprocess integration remains the only supported agent path
- Exit codes stay sysexits-style: `0`, `2`, `65`, `66`, `69`, `70`, `74`, `78`, `124`, `130`, `141`


## Rollback
- Pin to the previous local commit or installed binary path
- Keep scripts compatible with the success envelope fields `ok` and `schema_version`
- If rolling back from `0.1.2` to `0.1.1`, remove use of `print-pdf`, `monitor`, `qr`, `find-paths`, `parse --redact-pii`, `extract --llm`, and the new config keys
- If rolling back from `0.1.2`, also drop assumptions that browser scrape formats, scroll `dy`/`dx`, assert contains aliases, fail-fast `data.steps`, scrape `--webhook-url`, or flags/XDG logging always apply
- If rolling back from `0.1.1` to `0.1.0`, remove use of config, mitm, workflow, batch-scrape, crawl, map, search, parse
- If rolling back, also drop scrape `--format`/`--engine` assumptions that depend on `0.1.1`
- If rolling back, restore any wait or grab wrappers that assumed older argv shapes only if your fork had them
- Keep settings on flags and `config` even when targeting older trees


## See Also
- [CHANGELOG.md](../CHANGELOG.md)
- [docs/AGENTS.md](AGENTS.md)
- [docs/CROSS_PLATFORM.md](CROSS_PLATFORM.md)
- [docs/TESTING.md](TESTING.md)
- [docs/schemas/README.md](schemas/README.md)
