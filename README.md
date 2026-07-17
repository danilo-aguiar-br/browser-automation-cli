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
- XDG config and paths without product environment variables

## The Pain
- Agent workflows need multi-step browser work without a sticky daemon
- Node and npm browser stacks add runtime weight and supply-chain surface
- Session-based tools leave orphan Chrome processes and unclear ownership
- JSON contracts often drift from real CLI flags and exit codes
- Product settings scattered across env vars make agent prompts fragile

## Why browser-automation-cli
- One process owns one Chrome lifecycle from launch to kill fallback
- Multi-step work uses `run --script` NDJSON in the same process
- Accessibility snapshot refs `@eN` stay valid only inside that process
- `--json` envelopes are stable for programmatic agents
- Install path is pure Rust via cargo
- v0.1.1 ships config, mitm, workflow, batch-scrape, crawl, map, search, and parse

## Superpowers
- Navigation and page lifecycle: `goto`, `back`, `forward`, `reload`, `page`
- Input: `press`, `write`, `type`, `keys`, `hover`, `drag`, `fill-form`, `upload`
- Observation: `view`, `grab`, `extract`, `text`, `attr`, `scroll`, `assert`
- Wait: multi `--text` values resolve as OR (any text wins)
- Scrape: `scrape` with `--format text|markdown|html|links|metadata` and `--engine http|browser`
- Firecrawl-parity local surface: `batch-scrape`, `crawl`, `map`, `search`, `parse`
- Capture: `console` and `net` with optional global capture flags
- DevTools depth: `eval`, `emulate`, `resize`, `perf`, `lighthouse`, `heap`
- Optional categories: memory, extensions, third-party, webmcp
- Experimental: vision `click-at`, screencast with ffmpeg export
- MITM one-shot: `mitm start` binds `127.0.0.1` only (hudsucker)
- Workflow DAG: `workflow run|resume|status` with SQLite journal (resume skips ok)
- XDG config: `config path|init|show|set|get` for config.toml
- Discovery: `doctor` (incl. XDG browsers_dir), `commands`, `schema`, `completions`

## Quick Start
```bash
cargo install --path . --locked
browser-automation-cli --version
browser-automation-cli doctor --offline --quick --json
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
- Runtime needs Chrome or Chromium on PATH
- Optional: `ffmpeg` for screencast file export
- Optional: `lighthouse` binary for lighthouse audits

## Usage
- Always pass `--json` for agent pipelines
- Keep human diagnostics on stderr with `-q` when piping
- Use `--timeout` for wall-clock process budget in seconds
- Use `run --script` for multi-step sessions that need shared `@eN` refs
- Prefer CLI flags for one-off agent calls; use `config` for durable XDG defaults

```bash
browser-automation-cli --json goto https://example.com
browser-automation-cli --json wait --text Hello --text Welcome --ms 5000
browser-automation-cli --json scrape https://example.com --format markdown --engine http
browser-automation-cli --capture-network --json net list --resource-types Document,XHR
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json workflow resume --manifest workflow.toml
```

## Commands
- Discovery: `doctor`, `commands`, `schema`, `version`, `completions`
- Config: `config path`, `config init`, `config show`, `config set`, `config get`
- Navigate: `goto`, `back`, `forward`, `reload`
- Snapshot and input: `view`, `press`, `write`, `type`, `keys`, `wait`, `hover`, `drag`, `fill-form`, `upload`
- Content: `extract`, `text`, `scroll`, `attr`, `assert`, `grab`
- Scrape and discovery: `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
- Tabs and dialogs: `page`, `dialog`, `cookie`
- Capture: `console`, `net`
- MITM: `mitm status|list|get|har|export|domains|apis|init-ca|start`
- Workflow: `workflow run|resume|status`
- Advanced: `eval`, `emulate`, `resize`, `perf`, `lighthouse`, `screencast`, `heap`
- Categories: `extension`, `devtools3p`, `webmcp`
- Multi-step: `run`, `exec`

## Configuration
- Prefer CLI flags for one-off agent calls
- Use `config path|init|show|set|get` for XDG config.toml
- Product settings are NOT read from `BROWSER_AUTOMATION_CLI_*` environment variables
- OS-level only: `RUST_LOG` (tracing), `NO_COLOR` / color via config, `PATH` for Chrome/ffmpeg/lighthouse
- `config init` creates XDG layout and default config.toml
- `config path` prints resolved config, data, cache, state, and browsers_dir paths
- CLI flags override values stored in config.toml
- Doctor reports XDG browsers_dir among readiness checks

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
- Nightly docsrs cfg: `RUSTDOCFLAGS="--cfg docsrs" timeout 180 cargo +nightly doc --no-deps`

## Integration Patterns
- Claude Code, Codex, Cursor, and shell agents spawn one process per action
- Multi-step agent plans must use `run --script` instead of chaining separate processes
- Parse stdout with `jaq` and ignore stderr unless diagnosing failures
- Persist durable defaults with `config set` under XDG, not product env vars
- See [INTEGRATIONS.md](INTEGRATIONS.md) and [docs/AGENTS.md](docs/AGENTS.md)

## Performance
- Cold start is dominated by Chrome launch, not Rust binary size
- Prefer `doctor --offline --quick` for install checks without network
- Reuse multi-step scripts to avoid repeated Chrome launches
- Prefer `scrape --engine http` when CDP is not required
- Use `batch-scrape` concurrency for parallel HTTP fetches

## Memory Requirements
- Expect Chrome process memory far above the CLI binary itself
- Heap tools need `--category-memory` and larger snapshots increase RAM use
- Screencast export may invoke ffmpeg as an external helper
- Workflow journals and MITM captures land under XDG state/data paths

## Troubleshooting FAQ
- Chrome not found: install Chromium or Google Chrome, ensure it is on PATH, then re-run `doctor`
- Config / XDG: run `config init` then `config path` to inspect layout; use `config set|get` for values
- Exit 69 unavailable: browser binary missing, blocked, or not launchable
- Exit 124 timeout: raise `--timeout` or shorten the script
- Exit 2 usage: re-check flags with `browser-automation-cli help <cmd>`
- `@eN` refs invalid across commands: keep steps inside one `run` process; refs do not span processes
- Network empty: pass `--capture-network` on the same process that navigates
- Product env not supported: do not set `BROWSER_AUTOMATION_CLI_*` for settings; use flags or `config`
- Wait multi-text: repeat `--text` for OR semantics (any listed text unblocks)
- MITM bind: `mitm start` listens on `127.0.0.1` only with an ephemeral port
- Workflow resume: `workflow resume` skips steps already `ok` in the journal

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
- `255` unexpected fatal path

## Documentation Map
- [docs/HOW_TO_USE.md](docs/HOW_TO_USE.md) first command in 60 seconds
- [docs/AGENTS.md](docs/AGENTS.md) agent integration contract
- [docs/COOKBOOK.md](docs/COOKBOOK.md) practical recipes
- [docs/CROSS_PLATFORM.md](docs/CROSS_PLATFORM.md) platform matrix
- [docs/MIGRATION.md](docs/MIGRATION.md) version migration notes
- [docs/TESTING.md](docs/TESTING.md) test categories
- [docs/schemas/README.md](docs/schemas/README.md) JSON schema index
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
