[English](README.md) | [Português Brasileiro](README.pt-BR.md)

# browser-automation-cli

> One-shot Chrome CDP automation for AI agents. BORN, EXECUTE, FINALIZE, DIE.

[![docs.rs](https://img.shields.io/docsrs/browser-automation-cli)](https://docs.rs/browser-automation-cli)
[![crates.io](https://img.shields.io/crates/v/browser-automation-cli)](https://crates.io/crates/browser-automation-cli)
[![License](https://img.shields.io/crates/l/browser-automation-cli)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88.0-orange)](Cargo.toml)
[![Downloads](https://img.shields.io/crates/d/browser-automation-cli)](https://crates.io/crates/browser-automation-cli)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-blue)](https://www.rust-lang.org)
[![GitHub](https://img.shields.io/badge/github-browser--automation--cli-black.svg)](https://github.com/danilo-aguiar-br/browser-automation-cli)

## What is it
- Single-process browser automation CLI for AI agents
- Talks to system Chrome or Chromium through chromiumoxide CDP
- No daemon, no npm packaging, no remote telemetry
- Lifecycle is always BORN, EXECUTE, FINALIZE, DIE
- JSON envelopes on stdout for programmatic agents
- XDG config and paths via `config` commands only

## The Pain
- Agent workflows need multi-step browser work without a sticky daemon
- Node and npm browser stacks add runtime weight and supply-chain surface
- Session-based tools leave orphan Chrome processes and unclear ownership
- JSON contracts often drift from real CLI flags and exit codes
- Product settings outside XDG `config` make agent prompts fragile

## Why browser-automation-cli
- One process owns one Chrome lifecycle from launch to kill fallback
- Multi-step work uses `run --script` NDJSON or a JSON array of steps in the same process
- Accessibility snapshot refs `@eN` stay valid only inside that process
- `--json` envelopes are stable for programmatic agents; clap usage errors also emit JSON when `--json` is on argv
- Install path is pure Rust via cargo
- v0.1.7 is current: keeps residual-zero disk hygiene (0.1.5 RES-01…12 still true) and closes dialog settle, multi-tab dialog isolation, native select pick events, run wait/scrape fields, grab format lock, and lighthouse unit fixtures; inventory **69** agent names via `commands --json`

## Superpowers
- Navigation and page lifecycle: `goto` (init-script, beforeunload accept|dismiss), `back`, `forward`, `reload`, `page`
- Input: `press`, `write`, `type`, `keys`, `hover`, `drag`, `fill-form`, `select-option`, `pick` (native select + HIG badge/popover / `role=option` with pick events), `submit`, `upload`
- Observation: `view` (refuses empty about:blank unless `--allow-empty`), `grab` (formats `png|jpeg|webp` only; AVIF removed), `extract`, `text`, `attr`, `scroll`, `assert`
- Wait: multi `--text` OR; CSS multi-selector OR (`#a, #b`); run fields `url` / `url_contains` / `navigation` / `wait_timeout_ms`
- Assert: `url` / `text` / `console` plus `console_empty` / `console_no_match` (CLI `console-empty` / `console-no-match`)
- Scrape: multi-format `--format` / `--formats` (CSV or repeatable) with `--engine http|browser`; 14 live formats `text|markdown|html|rawHtml|links|metadata|screenshot|summary|product|branding|images|jsonld|json|feed` (`raw-html` stays an accepted alias of `rawHtml`); browser applies formats via outerHTML; `format`/`formats` also accepted in `run` scrape steps
- Local scrape/crawl/map/search/parse: `batch-scrape` and `crawl` accept `--engine http|browser`, `map`, `search` (cleans `uddg=`), `parse` (PDF/DOCX/xlsx/ods + `--redact-pii`)
- Extract LLM: `extract --llm --question --schema-json` (XDG `openrouter_api_key`, `llm_base_url`, `llm_model`)
- Capture: `console` (dump always writes `[]` when empty) and `net` with optional global capture flags
- Dialogs: `dialog accept|dismiss` returns `.data.dialog_settled`; XDG `dialog_settle_ms`; multi-tab dialog `session_id` isolation with e2e gate
- Storage: `storage export|import` for cookies + per-origin state within one process
- DevTools depth: `eval`, `emulate`, `resize`, `perf`, `lighthouse` (flag → XDG → PATH; `binary_source` real|mock; unit fixtures include chrome-captured LHR 13.4.1; e2e mock remains SKIP), `heap`
- PDF print: `print-pdf` one-shot and multi-step `run`; refuses blank PDF without navigated content
- Monitor: `monitor check --url --baseline [--write-baseline]`
- Utilities (no Chrome): `qr encode|decode`, `image info|convert|resize|download|exif`, `video info|download|convert|to-mp3|trim|thumbnail|manifest`, `audio info|download|convert|trim`, `find-paths` (`--glob`), `sheet-write`, `sg-scan`, `sg-rewrite`
- Assert aliases: `url_contains` / `text_contains`; `attr` falls back to DOM properties
- Scroll aliases in `run`: `dy`/`dx` for `delta_y`/`delta_x`
- Optional categories: memory, extensions, third-party, webmcp
- Experimental: vision `click-at`, screencast with ffmpeg export
- MITM one-shot: `status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact` (binds `127.0.0.1`; global `--mitm*`)
- Workflow DAG: `workflow run|resume|status` with SQLite journal (resume skips ok)
- XDG config: `config path|init|show|set|get|list-keys` for config.toml (discover full keys via `config list-keys --json`)
- Discovery: `doctor` (incl. `residual_disk`), `commands` (**69** agent names), `schema <cmd>` or `schema --cmd`, `version`, `locale`, `man`, `completions`
- Global flags: the global help declares **43** long flags, **41** of them product flags plus `--help` and `--version`; `browser-automation-cli --help` is the source of truth
- Multi-step observability: `run --json` final envelope includes `ok` + full `steps[].data`; global `--json-steps` streams one NDJSON line per step
- Fail-fast multi-step: `run` returns partial `data.steps` on error envelopes
- Residual-zero disk (still true from 0.1.5 RES-01…12): BORN auto-GC of stale Singleton-only Chromium dirs under `/tmp` older than 60s; FINALIZE dual scavenge + re-scan; never kills host Flatpak Chrome; marker prefix `browser-automation-cli-chrome-`
- Lifecycle: BORN + FINALIZE scavenge owned Chromium `/tmp` orphans; product law is residual-zero process + disk
- Cache: XDG `cache_backend` (`sqlite|memory|redis`) and `cache_redis_url`; `rediss://` fail-closed
- Intentional residual: GAP-022 ~53 transitive multi-version dups; GAP-023/024 PRD divergences registered

## Quick Start
```bash
cargo install --path . --locked
browser-automation-cli --version
browser-automation-cli doctor --offline --quick --json
browser-automation-cli doctor --offline --quick --json | jaq '.residual // .data.residual // .'
browser-automation-cli locale --json
browser-automation-cli goto https://example.com --json
browser-automation-cli view --json
```

## Installation
- Local development install:
```bash
git clone https://github.com/danilo-aguiar-br/browser-automation-cli
cd browser-automation-cli
cargo install --path . --locked
```
- From crates.io after the first publish:
```bash
cargo install browser-automation-cli --locked
```
- Runtime needs Chrome or Chromium on the shell path (or `config set chrome_path`)
- Optional: `ffmpeg` for screencast file export
- Optional: `lighthouse` binary for lighthouse audits (or `config set lighthouse_path`)

## Usage
- Always pass `--json` for agent pipelines
- Keep human diagnostics on stderr with `-q` when piping
- Use `--timeout` for wall-clock process budget in seconds
- Use `run --script` (NDJSON lines or a JSON array of steps) for multi-step sessions that need shared `@eN` refs
- Stream per-step progress with global `--json-steps` (NDJSON lines: `step`, `cmd`, `ok`, `result`)
- Prefer CLI flags for one-off agent calls; use `config` for durable XDG defaults
- Logging detail: `--verbose` / `--debug` / `-q`, or `config set log_level`
- Localize human suggestions with `--lang pt-BR` or `config set lang pt-BR`
- Optional scrape `--webhook-url` posts the result once to an operator URL (not product telemetry)
- Optional MITM: global `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`

```bash
browser-automation-cli config set openrouter_api_key sk-or-...
browser-automation-cli --json goto https://example.com
browser-automation-cli --json wait --text Hello --text Welcome --ms 5000
browser-automation-cli --json scrape https://example.com --format markdown --engine http
browser-automation-cli --json scrape https://example.com --format markdown,html,links --engine browser
browser-automation-cli --json scrape https://example.com --format markdown --engine http --webhook-url https://example.com/hook
browser-automation-cli --json extract --llm --question "What is the title?" https://example.com
browser-automation-cli --capture-network --json net list --resource-types Document,XHR
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm capture-url https://example.com --seconds 30 --har /tmp/browser-automation-cli-artifacts/cap.har
browser-automation-cli --json mitm har --out /tmp/browser-automation-cli-artifacts/capture.har
browser-automation-cli --json workflow resume --manifest workflow.toml
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/browser-automation-cli-artifacts/page.pdf
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/browser-automation-cli-artifacts/base.txt --write-baseline
browser-automation-cli --json parse ./doc.pdf --redact-pii
browser-automation-cli --json parse ./doc.ods
browser-automation-cli --json qr encode --text "hello" --path /tmp/browser-automation-cli-artifacts/qr.png
browser-automation-cli --json qr decode --path /tmp/browser-automation-cli-artifacts/qr.png
browser-automation-cli --json find-paths --glob '**/*.rs' '' src
browser-automation-cli --json sheet-write rows.csv --out /tmp/browser-automation-cli-artifacts/out.xlsx
browser-automation-cli --json sg-scan src
browser-automation-cli --json schema run
browser-automation-cli --json schema --cmd wait
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --engine browser --concurrency 2
browser-automation-cli --capture-console --json assert console-empty
browser-automation-cli --json record --url https://example.com --path /tmp/steps.jsonl --seconds 30 --max-events 200
```

- `--script` takes a file path, never inline JSON; write the steps file first (NDJSON, one step per line):
```json
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
```
- Then run that file in one process:
```bash
browser-automation-cli --json run --script /tmp/steps.jsonl
browser-automation-cli --json --json-steps run --script /tmp/steps.jsonl
```
- Reading an API payload needs the capture and the navigation in the same process, so put the `net` step in the script (`/tmp/net.jsonl`):
```json
{"cmd":"goto","url":"https://example.com"}
{"cmd":"net","action":"get","id":"0","response_path":"/tmp/browser-automation-cli-artifacts/res.json"}
```
```bash
browser-automation-cli --capture-network --json run --script /tmp/net.jsonl
```

## Commands
Full agent inventory (**69** names via `commands --json`, sorted):
`assert`, `attr`, `audio`, `back`, `batch-scrape`, `click-at`, `commands`, `completions`, `config`, `console`, `cookie`, `crawl`, `devtools3p`, `dialog`, `doctor`, `drag`, `emulate`, `eval`, `exec`, `extension`, `extract`, `find-paths`, `fill-form`, `forward`, `goto`, `grab`, `heap`, `hover`, `image`, `keys`, `lighthouse`, `locale`, `man`, `map`, `mitm`, `monitor`, `net`, `page`, `parse`, `perf`, `pick`, `press`, `print-pdf`, `qr`, `record`, `reload`, `resize`, `run`, `schema`, `scrape`, `screencast`, `scroll`, `search`, `select-option`, `sg-rewrite`, `sg-scan`, `sheet-write`, `storage`, `submit`, `text`, `type`, `upload`, `video`, `version`, `view`, `wait`, `webmcp`, `workflow`, `write`

Grouped for humans:
- Discovery: `doctor`, `commands`, `schema`, `version`, `locale`, `man`, `completions`
- Navigate: `goto`, `back`, `forward`, `reload`
- Interact: `press`, `write`, `type`, `keys`, `wait`, `hover`, `drag`, `fill-form`, `select-option`, `pick`, `submit`, `upload`, `click-at`
- Observe: `view`, `extract`, `text`, `scroll`, `attr`, `assert`, `grab`
- Scrape: `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
- Capture: `console`, `net`, `print-pdf`, `monitor`, `screencast`
- Tabs/Dialogs: `page`, `dialog`, `cookie`, `storage`
- Utils: `qr`, `image`, `video`, `audio`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`
- Advanced: `eval`, `emulate`, `resize`, `perf`, `lighthouse`, `heap`, `extension`, `devtools3p`, `webmcp`, `mitm`, `workflow`
- Config: `config path|init|show|set|get|list-keys`
- Multi-step: `run`, `exec`, `record`
- Record teaching: `browser-automation-cli --json record --url https://example.com --path /tmp/steps.jsonl --seconds 30 --max-events 200` writes page interactions as replayable NDJSON, then `browser-automation-cli --json run --script /tmp/steps.jsonl` replays them in one process
- Audio teaching: `browser-automation-cli --json audio info|download|convert|trim` runs the local audio pipeline without Chrome
- Inventory note: **69** agent-facing names via `commands --json` (includes `select-option`, `pick`, `submit`, `storage`, `image`+`video`+`audio`+`record`); DevTools e2e covers 53 tools (lighthouse mock SKIP)

## Configuration
- Prefer CLI flags for one-off agent calls
- Product settings only via flags and XDG `config path|init|show|set|get|list-keys`
- Discover the full key list (count is not fixed at 16) with `config list-keys --json`
- Important keys: `dialog_settle_ms`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`, `lang`, `log_level`
- Logging: `--verbose` / `--debug` / `-q`, or XDG `config set log_level` / `log_to_file`
- Color: `config set color true|false`
- Chrome binary: shell path or XDG `config set chrome_path`
- Lighthouse binary: flag `--lighthouse-path`, XDG `config set lighthouse_path`, or PATH (envelope reports `binary_source`)
- Dialog settle budget: XDG `config set dialog_settle_ms <ms>` (agent-visible `dialog_settled` on dialog accept|dismiss)
- Cache: `config set cache_backend sqlite|memory|redis` and optional `cache_redis_url` (`redis://` only; `rediss://` fail-closed)
- `config init` creates XDG layout and default config.toml
- `config path` prints resolved config, data, cache, state, and browsers_dir paths
- CLI flags override values stored in config.toml
- Doctor reports browsers_dir, lighthouse source, `cache_redis`, and `residual_disk` among readiness checks
- Doctor JSON top-level field `residual` reports: `scanned_roots`, `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legacy), `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`

## Features
- This crate has no Cargo feature flags
- Optional categories are process flags, not compile-time features
- `--category-memory` enables deep heap tools
- `--category-extensions` enables extension tools
- `--category-third-party` enables third-party DevTools helpers
- `--category-webmcp` enables webmcp tools
- `--experimental-vision` enables `click-at`
- `--experimental-screencast` enables screencast export with ffmpeg

## Targets
- Documented for `x86_64-unknown-linux-gnu`
- Documented for `x86_64-apple-darwin`
- Documented for `aarch64-apple-darwin`
- Documented for `x86_64-pc-windows-msvc`
- Documented for `aarch64-unknown-linux-musl`
- Not supported on `wasm32-unknown-unknown` (Chrome CDP requires a desktop browser)
- docs.rs metadata declares these targets explicitly after the 2026-05-01 multi-target change

## MSRV
- Minimum Supported Rust Version is 1.88.0
- Policy: bump MSRV only in minor or major releases with CHANGELOG note
- Local docs: `timeout 180 cargo doc --no-deps`

## Integration Patterns
- Claude Code, Codex, Cursor, and shell agents spawn one process per action
- Multi-step agent plans must use `run --script` (NDJSON or JSON array) instead of chaining separate processes
- Parse stdout with `jaq` and ignore stderr unless diagnosing failures
- Stream step progress with `--json-steps` when agents need progressive feedback
- Persist durable defaults with `config set` under XDG
- See [INTEGRATIONS.md](INTEGRATIONS.md) and [docs/AGENTS.md](docs/AGENTS.md)

## Performance
- Cold start is dominated by Chrome launch, not Rust binary size
- Prefer `doctor --offline --quick` for install checks without network
- Reuse multi-step scripts to avoid repeated Chrome launches
- Prefer `scrape --engine http` when CDP is not required
- Use `batch-scrape` concurrency for parallel fetches (`--engine http` default; `--engine browser` when JS render is required)

## Memory Requirements
- Expect Chrome process memory far above the CLI binary itself
- Heap tools need `--category-memory` and larger snapshots increase RAM use
- Screencast export may invoke ffmpeg as an external helper
- Workflow journals and MITM captures land under XDG state/data paths

## Troubleshooting FAQ
- Chrome not found: install Chromium or Google Chrome, ensure it is on the shell path, or `config set chrome_path`, then re-run `doctor`
- Config / XDG: run `config init` then `config path` to inspect layout; use `config set|get` for values
- Product settings only via flags and `config set` (XDG)
- Exit 69 unavailable: browser binary missing, blocked, or not launchable
- Exit 124 timeout: raise `--timeout` or shorten the script
- Exit 2 usage: re-check flags with `browser-automation-cli help <cmd>`; with `--json` on argv, clap usage errors emit JSON envelopes
- `@eN` refs invalid across commands: keep steps inside one `run` process; refs do not span processes
- Network empty: pass `--capture-network` on the same process that navigates
- API payload read: `net get <IDX>` writes bodies with `--response-path` and `--request-path`, but `net list` and `net get` only see traffic captured in the same process, so a standalone `net get 0` after a separate `goto` returns exit 65 with `count=0`; put a `net` step next to the `goto` step in one script and run `browser-automation-cli --capture-network --json run --script /tmp/net.jsonl`
- Wait multi-text: repeat `--text` for OR semantics (any listed text unblocks)
- Wait multi-selector / URL: CSS OR `#a, #b`; in `run` use `url` / `url_contains` / `navigation`
- View empty blank: empty about:blank refuses silent success unless `--allow-empty` / `allow_empty:true`
- MITM bind: `mitm start` and `mitm capture-url` listen on `127.0.0.1` only with an ephemeral port
- MITM HAR: `mitm har --out <path>` (required); or global `--mitm-har` on FINALIZE; or `capture-url --har`
- MITM redact: `mitm redact --secrets` and global `--mitm-redact-secrets`; CA under XDG data
- Workflow resume: `workflow resume` skips steps already `ok` in the journal
- Scrape multi-format: `--format markdown,html,links` (CSV or repeatable) returns per-format fields; 14 live formats are `text`, `markdown`, `html`, `rawHtml`, `links`, `metadata`, `screenshot`, `summary`, `product`, `branding`, `images`, `jsonld`, `json`, `feed` (`raw-html` remains an accepted alias of `rawHtml`)
- Scrape browser formats: `--engine browser` applies `--format` via outerHTML
- Batch/crawl browser engine: `batch-scrape --engine browser` and `crawl --engine browser` (GAP-010)
- Scroll aliases: in `run` scripts use `dy`/`dx` as aliases for `delta_y`/`delta_x`
- Schema discovery: `schema run` or `schema --cmd run`; expanded fragments for goto/eval/type/scroll/assert/wait
- Lang: `--lang pt-BR` or `config set lang pt-BR` localizes human suggestions
- Fail-fast partial steps: failed `run` error envelopes may include partial `data.steps`
- JSON steps stream: `--json-steps` emits one NDJSON object per step; final `--json` envelope still includes full `steps[]`
- Lighthouse path: flag, `config set lighthouse_path`, or PATH; envelope `binary_source` is `real` or `mock` (mock is e2e-only honesty, not production)
- Search redirects: `search` cleans `uddg=` wrappers to destination URLs
- Parse documents: `parse` supports PDF/DOCX/xlsx/ods and `--redact-pii`
- Extract LLM: requires XDG `openrouter_api_key` (optional `llm_base_url`, `llm_model`)
- Print PDF: `print-pdf --url <url> --path <file>` one-shot CDP; also valid inside `run`
- Monitor baseline: `monitor check --url <url> --baseline <file> [--write-baseline]`
- Assert console: `assert console-empty` / `assert console-no-match --pattern …` (needs `--capture-console`)
- Assert aliases: `url_contains` / `text_contains`; `attr` uses DOM property fallback when HTML attribute is null
- Pick / select-option: agent inventory names; native select dispatches input+change; HIG badge/popover / `role=option` via `pick`
- Submit / storage: `submit` for form submit; `storage export|import` for cookies + per-origin state
- Inventory size: `commands --json` lists **69** agent names (includes `select-option`, `pick`, `submit`, `storage`, `image`+`video`+`audio`+`record`)
- Locale: `locale --json` diagnoses resolved language; set with `--lang pt-BR` or `config set lang pt-BR`
- `file://` + `scrape --engine http`: Usage error — use browser engine or `parse` for local files
- `reload --ignore-cache`: CDP `Page.reload` with `ignoreCache` (not a JS no-op)
- `run` script formats: `--script` is always a file path (inline JSON is rejected with exit 66); the file is NDJSON one object per line, or a single JSON array of steps; supports `wait_timeout_ms` and scrape `format`/`formats`
- Grab formats: `png|jpeg|webp` only (AVIF removed in 0.1.6)
- Redis cache: set `cache_backend redis` and `cache_redis_url`; never use `rediss://`
- Residual /tmp disk hygiene (0.1.5 RES-01…12 still true in 0.1.7):
  - BORN auto-GC: `scavenge_stale_singleton_orphans` removes `/tmp` `org.chromium.Chromium.*` Singleton-only dirs older than 60s
  - FINALIZE dual scavenge + re-scan of owned marker dirs (`browser-automation-cli-chrome-` prefix)
  - Never kills host Flatpak Chrome or non-CLI browser processes
  - Doctor check `residual_disk` + top-level JSON field `residual` (`scanned_roots`, `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legacy), `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`)
  - Local gates: `scripts/residual-check.sh`, `scripts/residual-stress.sh` (no CI required)
- Dialog settle: `dialog accept|dismiss` → read `.data.dialog_settled`; budget via XDG `dialog_settle_ms`; multi-tab isolation by `session_id` (e2e gated)
- Lighthouse: unit fixtures include chrome-captured LHR 13.4.1; e2e mock path remains SKIP (contract-only)
- Intentional residual: GAP-022 ~53 transitive multi-version dups; GAP-023/024 PRD divergences registered
- Sheet/lint utils: `sheet-write <input> --out <file>`, `sg-scan <paths>`, `sg-rewrite <paths>` take positional inputs; `find-paths --glob` for shell globs
- `find-paths` positional order is `[PATTERN] [PATHS]...`, so a lone positional is read as the regex PATTERN and the roots silently fall back to the current directory; pass an empty pattern to target a root: `find-paths --glob '**/*.rs' '' src`
- Dialog soft path: `dialog accept --if-present` / run `if_present:true` soft-ok when no dialog is showing

## Exit Codes
- `0` success
- `2` usage or clap parse failure
- `65` data error
- `66` no input
- `69` unavailable
- `70` software, browser, or protocol failure
- `74` I/O failure
- `78` config error
- `124` timeout
- `130` cancelled by SIGINT
- `141` broken pipe
- `255` unexpected fatal path — plausible panic route, but not mapped by any `error.kind` and not observable through the discovery surface

## Documentation Map
- [docs/HOW_TO_USE.md](docs/HOW_TO_USE.md) first command in 60 seconds
- [docs/AGENTS.md](docs/AGENTS.md) agent integration contract
- [docs/COOKBOOK.md](docs/COOKBOOK.md) practical recipes
- [docs/CONFIGURATION.md](docs/CONFIGURATION.md) every XDG key, its default and its purpose
- [docs/CROSS_PLATFORM.md](docs/CROSS_PLATFORM.md) platform matrix
- [docs/MIGRATION.md](docs/MIGRATION.md) version migration notes
- [docs/TESTING.md](docs/TESTING.md) test categories
- [docs/schemas/README.md](docs/schemas/README.md) JSON schema index
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) module layout and lifecycle internals
- [docs/ROADMAP.md](docs/ROADMAP.md) what is planned and what is closed by physical limit
- [PRIVACY.md](PRIVACY.md) what stays local and what is never uploaded
- [skills/browser-automation-cli-en/SKILL.md](skills/browser-automation-cli-en/SKILL.md) imperative agent skill
- [CHANGELOG.md](CHANGELOG.md) Keep a Changelog history
- [SECURITY.md](SECURITY.md) vulnerability reporting
- [CONTRIBUTING.md](CONTRIBUTING.md) contributor workflow
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) Contributor Covenant 2.1
- [llms.txt](llms.txt) short LLM discovery map

## Contributing
- Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a PR
- Follow the Code of Conduct in [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

## Security
- Report vulnerabilities privately via [SECURITY.md](SECURITY.md)
- Maintainer contact: daniloaguiarbr@proton.me

## Changelog
- Version history lives only in [CHANGELOG.md](CHANGELOG.md)

## License
- Dual licensed under MIT OR Apache-2.0
- See [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT), and [LICENSE-APACHE](LICENSE-APACHE)
