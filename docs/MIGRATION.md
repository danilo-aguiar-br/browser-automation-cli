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
- Live inventory is **59 commands** (`commands --json`)
- DevTools tool-ref e2e suite remains **53 tools** (`scripts/e2e_all_52_tools.sh`)
- Static schemas regenerate via `bash scripts/generate_command_schemas.sh`



## 0.1.2 → 0.1.3
Hard-close residual-zero, Redis/Lighthouse honesty, and PRD write/lint surface landed in `0.1.3`:

### Residual e2e and scavenger (A001–A002)
- Residual e2e measurement no longer self-matches; pipefail-safe residual harness
- FINALIZE scavenges owned Chromium `/tmp` orphans (`scavenge_owned_chromium_tmp_orphans`)

### Run script contract (A003)
- `run --script` accepts **NDJSON** (one object per line) **or** a top-level **JSON array** of step objects
- Fail-fast errors still return partial `data.steps` when present

### Navigation / CDP honesty (A004–A006, A009, A012)
- `scrape --engine http` rejects `file://` with Usage + suggestion (`browser` engine or `parse`)
- `reload` uses CDP `Page.reload` with `ignoreCache` when `--ignore-cache` is set
- `init_script` is removed after navigation/reload
- `handle_before_unload` auto-accepts via CDP dialog pump (no inject `preventDefault`)
- Unknown CDP events are ignored so network capture continues

### Redis / cache (A007–A008)
- New XDG keys: `cache_backend`, `cache_redis_url`, plus `log_to_file`
- `rediss://` is fail-closed (plain TCP only)
- Doctor reports `cache_redis` when Redis cache is configured
- Unit RESP mock always-on; optional real redis-server when present on the host

### Lighthouse honesty (A010)
- Resolve order: flag `--lighthouse-path` → XDG `lighthouse_path` → PATH
- Envelope reports `binary_source` as `real` or `mock`
- Doctor reports lighthouse source honestly

### PRD write/lint surface (A011)
- `find-paths --glob` shell-style glob filter
- `sheet-write` CSV/JSON → XLSX (no Chrome)
- `sg-scan` structural lint; `sg-rewrite` dry-run default with `--apply`

### Other 0.1.3 surface
- `page tab-id` (tool-ref `get_tab_id`) expands e2e to **53** tools
- `config list-keys` lists supported keys and defaults
- Live inventory is **59 commands** (`commands --json`)
- DevTools tool-ref e2e is **53 tools** (`scripts/e2e_all_52_tools.sh` legacy filename)

### Config keys (full list in 0.1.3)
- `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`


## 0.1.3 → 0.1.4
Hard-close GAP-001…025 for agent-first observability, wait/assert depth, MITM compose, and clap honesty:

### Run observability (GAP-020)
- Global `--json-steps`: stream one NDJSON line per step (`step`, `cmd`, `ok`, `result`)
- `run --json` final envelope includes `ok` and full `steps[].data`
- Fail-fast still returns partial `data.steps` on error envelopes

### Wait multi-selector and URL (GAP-019, GAP-024)
- CSS multi-selector OR: `#a, #b` and `selectors` arrays in run
- Run wait fields: `url` (exact), `url_contains`, `navigation: true` (boolean — not a string like `"load"`)
- Successful multi-selector wait may include `matched_selector` in result data
- Existing multi `--text` OR remains

### Select / pick multi-step (GAP-023)
- New inventory names: `select-option`, `pick` (HIG badge/popover / `role=option`)
- Available in `run` / `exec` / schema discovery with `target` + `option`
- Not standalone clap subcommands (clap top-level help lists 59 without them)

### Assert console kinds (GAP-025)
- Run kinds: `console_empty`, `console_no_match` (requires `--capture-console`)
- CLI: `assert console-empty`, `assert console-no-match --pattern <re>`

### Schema positional (GAP-022)
- `schema <cmd>` positional in addition to `schema --cmd <cmd>`

### Navigation / dialog / view / PDF honesty (GAP-003, GAP-006, GAP-012, GAP-013, GAP-001, GAP-017)
- `BeforeUnloadAction` accept|dismiss on `goto` / `reload` (`--handle-before-unload`; run `handle_before_unload`)
- Dialog soft path: `dialog accept --if-present` / run `if_present:true`
- `view` refuses empty about:blank unless `--allow-empty` / `allow_empty:true` (GAP-012 only — not print-pdf)
- `print-pdf` in multi-step `run`; refuses blank PDF without navigated content or step/CLI `url` (GAP-013)
- `parity_run_inventory` enforces `print-pdf` in `RUN_DISPATCHED_CMDS`

### Isolated context (GAP-004)
- `page new --isolated-context` (flag alone → `default-isolated`) or `--isolated-context <name>`
- Run: `{"cmd":"page","action":"new","isolated_context":true}` or named string

### Extension install/uninstall outside run (GAP-007)
- `extension install` / `extension uninstall` intentionally excluded from `run` dispatch
- Use top-level `extension` commands; discover via `schema extension` / `commands --json`

### Assert dual surface (GAP-014)
- CLI subcommands: `assert url|text|console|console-empty|console-no-match`
- Run kinds: `url` / `text` / `console` / `console_empty` / `console_no_match` (+ aliases)

### MITM capture-url and globals (GAP-011)
- Full MITM surface: `status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact`
- `mitm capture-url <url> [--seconds N] [--har path] [--hosts …]` one-shot compose
- Global flags: `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`
- `mitm har --out <path>` required for HAR export path

### Scrape multi-format and batch/crawl browser engine (GAP-009, GAP-010, GAP-018)
- `scrape --format` accepts CSV or repeatable multi-format in one invocation
- Alias `--formats` accepted where supported (GAP-018)
- `batch-scrape --engine http|browser` (default http)
- `crawl --engine http|browser` (default http)

### Clap / console / privacy (GAP-002, GAP-021, GAP-016)
- Clap usage errors emit JSON envelope when `--json` is on argv
- `console dump` always writes a valid JSON array (`[]` when empty)
- Chrome privacy launch flags; no `metrics-recording-only`

### Inventory and contract gates
- Live inventory is **61** agent names via `commands --json` (includes `select-option`, `pick`)
- Carry-forward honesty (closed earlier, still required in 0.1.4): lighthouse `binary_source` real|mock (GAP-008); `extract --llm` fail-closed on XDG keys only (GAP-015)
- Clap top-level help lists **59** without `select-option`/`pick` as standalone
- DevTools tool-ref e2e remains **53 tools**
- Gates: `tests/parity_run_inventory.rs`, `tests/clap_command_debug_assert.rs`
- Clap surface audit: `GlobalOpts` uses `Args` + flatten; explicit `ArgAction::SetTrue`; `value_hint`; help headings; `after_help` examples; `-v` alias

### Config keys (unchanged full list of 16 in 0.1.4)
- `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`

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
- Confirm inventory with `commands --json` (59) and regenerate schemas if packaging docs
- Re-run local validation with cargo and e2e scripts: `cargo test --lib`, e2e 52-tool script, residual smokes you care about

### From 0.1.2 to 0.1.3
- Rebuild/install `0.1.3`
- Update agents: `run --script` may use a JSON array of steps as well as NDJSON
- Do not pass `file://` to `scrape --engine http`
- Discover new commands: `sheet-write`, `sg-scan`, `sg-rewrite`, and `find-paths --glob`
- Configure Redis only via XDG: `config set cache_backend redis` and `config set cache_redis_url redis://…`
- Never use `rediss://` (fail-closed)
- Expect lighthouse envelopes to include `binary_source`
- Confirm inventory with `commands --json` (59) and regenerate schemas if packaging docs
- Re-run local validation: `cargo test --lib`, e2e 53-tool script, residual PRD smokes

### From 0.1.3 to 0.1.4
- Rebuild/install `0.1.4`
- Prefer progressive agent feedback with global `--json-steps` on `run`
- Expect `run --json` success envelopes to include full `steps[].data` and `ok`
- Update wait scripts for multi-selector OR and `url` / `url_contains` / `navigation: true` (boolean)
- Use `page new --isolated-context` / run `isolated_context` for named isolated contexts (GAP-004)
- Keep `extension install|uninstall` as top-level only (not inside `run`) (GAP-007)
- Prefer dual assert surfaces: CLI `assert console-empty` and run `kind: console_empty` (GAP-014)
- Prefer scrape `--format` multi/CSV or alias `--formats` (GAP-018)
- Use `select-option` / `pick` only inside `run` / `exec` (not as standalone clap cmds)
- Adopt assert console kinds: `console_empty` / `console_no_match` (CLI `console-empty` / `console-no-match`)
- Prefer `schema run` positional; `schema --cmd run` still works
- For MITM one-shot navigate+capture: `mitm capture-url <url>`; export with `mitm har --out <path>`
- Optional global MITM flags when routing Chrome: `--mitm`, `--mitm-har`, `--mitm-redact-secrets`, …
- Pass multi-format scrape: `--format markdown,html,links`
- Prefer `batch-scrape --engine browser` / `crawl --engine browser` when JS render is required (default remains http)
- Handle empty `view` / blank `print-pdf` honestly (`--allow-empty` only when intentional)
- Confirm inventory with `commands --json` (61) and regenerate schemas if packaging docs
- Re-run local validation: `cargo test --lib`, `parity_run_inventory`, `clap_command_debug_assert`, e2e 53-tool script, residual smokes

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
- Live per-command input fragments come from `schema <cmd>` or `schema --cmd`
- Static snapshots under `docs/schemas/` are a convenience index and may lag the binary
- v0.1.1 static additions include `config`, `mitm`, `workflow`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`, and `wait`
- v0.1.2 static additions include `print-pdf`, `monitor`, `qr`, `find-paths` (regenerate with the generator)
- v0.1.3 static additions include `sheet-write`, `sg-scan`, `sg-rewrite`; `find-paths` gains `glob`; config keys include cache/log_to_file
- v0.1.4: wait/assert/schema/run fragments expand for multi-selector, url wait, console asserts, json-steps; inventory adds `select-option`/`pick` as run/schema names
- Prefer live `schema <cmd>` after upgrades to confirm the installed binary


## Compatibility Notes
- No previous stable crates.io line exists for this repository before `0.1.0`
- Branding and history cleanup recreated a clean public root commit
- First crates.io publish still requires explicit maintainer approval
- Agents that hard-coded settings outside flags/`config` must migrate to flags + `config set`
- Agents that controlled product verbosity outside flags/`log_level` must migrate to `--verbose` / `--debug` / `config set log_level`
- Subprocess integration remains the only supported agent path
- Exit codes stay sysexits-style: `0`, `2`, `65`, `66`, `69`, `70`, `74`, `78`, `124`, `130`, `141`
- Agents that assumed `batch-scrape` was HTTP-only must accept optional `--engine browser` in 0.1.4
- Agents that treated `select-option`/`pick` as clap subcommands must use `run`/`exec` steps instead


## Rollback
- Pin to the previous local commit or installed binary path
- Keep scripts compatible with the success envelope fields `ok` and `schema_version`
- If rolling back from `0.1.4` to `0.1.3`, remove use of `--json-steps`, wait `url`/`url_contains`/`navigation`, multi-selector wait arrays, `select-option`/`pick` steps, assert `console_empty`/`console_no_match`, `schema <cmd>` positional-only flows, `mitm capture-url` / `graphql` / `ws` / `block` / `allow` / `redact`, global `--mitm*` flags, multi-format scrape assumptions, `batch-scrape`/`crawl` `--engine browser`, `view --allow-empty`, and clap-JSON-usage-error assumptions
- If rolling back from `0.1.3` to `0.1.2`, remove use of `sheet-write`, `sg-scan`, `sg-rewrite`, `find-paths --glob`, JSON-array-only `run` scripts, cache XDG keys, and `binary_source` assumptions
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
