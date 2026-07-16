[English](README.md) | [Português Brasileiro](README.pt-BR.md)

# browser-automation-cli

> One-shot Chrome CDP automation for AI agents. NASCE, EXECUTA, FINALIZE, MORRE.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![MSRV 1.88.0](https://img.shields.io/badge/MSRV-1.88.0-blue.svg)](Cargo.toml)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![GitHub](https://img.shields.io/badge/github-browser--automation--cli-black.svg)](https://github.com/danilo-aguiar-br/browser-automation-cli)

## What is it
- Single-process browser automation CLI for AI agents
- Talks to system Chrome or Chromium through chromiumoxide CDP
- No daemon, no npm packaging, no remote telemetry
- Lifecycle is always NASCE, EXECUTA, FINALIZE, MORRE

## The Pain
- Agent workflows need multi-step browser work without a sticky daemon
- Node and npm browser stacks add runtime weight and supply-chain surface
- Session-based tools leave orphan Chrome processes and unclear ownership
- JSON contracts often drift from real CLI flags and exit codes

## Why browser-automation-cli
- One process owns one Chrome lifecycle from launch to kill fallback
- Multi-step work uses `run --script` NDJSON in the same process
- Accessibility snapshot refs `@eN` stay valid only inside that process
- `--json` envelopes are stable for programmatic agents
- Install path is pure Rust via cargo

## Superpowers
- Navigation and page lifecycle: `goto`, `back`, `forward`, `reload`, `page`
- Input: `press`, `write`, `type`, `keys`, `hover`, `drag`, `fill-form`, `upload`
- Observation: `view`, `grab`, `extract`, `text`, `attr`, `scroll`, `assert`
- Capture: `console` and `net` with optional global capture flags
- DevTools depth: `eval`, `emulate`, `resize`, `perf`, `lighthouse`, `heap`
- Optional categories: memory, extensions, third-party, webmcp
- Experimental: vision `click-at`, screencast with ffmpeg export
- Discovery: `doctor`, `commands`, `schema`, `completions`

## Quick Start
```bash
cargo install --path . --locked
browser-automation-cli --version
browser-automation-cli doctor --offline --quick --json
browser-automation-cli goto https://example.com --json
browser-automation-cli view --json
```

## Installation
- Prefer local path while `publish = false` on crates.io
```bash
git clone https://github.com/danilo-aguiar-br/browser-automation-cli
cd browser-automation-cli
cargo install --path . --locked
```
- After first crates.io release use:
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

```bash
browser-automation-cli --json goto https://example.com
browser-automation-cli --capture-network --json net list --resource-types Document,XHR
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
```

## Commands
- Discovery: `doctor`, `commands`, `schema`, `version`, `completions`
- Navigate: `goto`, `back`, `forward`, `reload`, `scrape`
- Snapshot and input: `view`, `press`, `write`, `type`, `keys`, `wait`, `hover`, `drag`, `fill-form`, `upload`
- Content: `extract`, `text`, `scroll`, `attr`, `assert`, `grab`
- Tabs and dialogs: `page`, `dialog`, `cookie`
- Capture: `console`, `net`
- Advanced: `eval`, `emulate`, `resize`, `perf`, `lighthouse`, `screencast`, `heap`
- Categories: `extension`, `devtools3p`, `webmcp`
- Multi-step: `run`, `exec`

## Environment Variables
- `BROWSER_AUTOMATION_CLI_JSON` enables JSON envelopes
- `BROWSER_AUTOMATION_CLI_QUIET` suppresses non-error stderr prose
- `BROWSER_AUTOMATION_CLI_VERBOSE` raises tracing to info
- `BROWSER_AUTOMATION_CLI_DEBUG` raises tracing to debug
- `BROWSER_AUTOMATION_CLI_TIMEOUT` sets global timeout seconds
- `BROWSER_AUTOMATION_CLI_STEP_TIMEOUT` sets per-step timeout for `run`
- `BROWSER_AUTOMATION_CLI_HEADED` launches visible Chrome
- `BROWSER_AUTOMATION_CLI_ARTIFACTS_DIR` stores artifacts
- `BROWSER_AUTOMATION_CLI_LANG` selects locale messaging
- `BROWSER_AUTOMATION_CLI_CAPTURE_CONSOLE` enables console capture
- `BROWSER_AUTOMATION_CLI_CAPTURE_NETWORK` enables network capture
- `BROWSER_AUTOMATION_CLI_IGNORE_ROBOTS` and `BROWSER_AUTOMATION_CLI_I_ACCEPT_ROBOTS_RISK` control robots policy
- `BROWSER_AUTOMATION_CLI_CATEGORY_MEMORY` enables deep heap tools
- `BROWSER_AUTOMATION_CLI_CATEGORY_EXTENSIONS` enables extension tools
- `BROWSER_AUTOMATION_CLI_CATEGORY_THIRD_PARTY` enables third-party tools
- `BROWSER_AUTOMATION_CLI_CATEGORY_WEBMCP` enables webmcp tools
- `BROWSER_AUTOMATION_CLI_EXPERIMENTAL_VISION` enables `click-at`
- `BROWSER_AUTOMATION_CLI_EXPERIMENTAL_SCREENCAST` enables screencast
- `BROWSER_AUTOMATION_CLI_ENCRYPTION_KEY` encrypts optional native state
- `BROWSER_AUTOMATION_CLI_NAMESPACE` scopes native state namespaces
- `BROWSER_AUTOMATION_CLI_COLOR` and `NO_COLOR` control color on stderr
- `RUST_LOG` overrides tracing filters when needed

## Integration Patterns
- Claude Code, Codex, Cursor, and shell agents spawn one process per action
- Multi-step agent plans must use `run --script` instead of chaining separate processes
- Parse stdout with `jaq` and ignore stderr unless diagnosing failures
- See [INTEGRATIONS.md](INTEGRATIONS.md) and [docs/AGENTS.md](docs/AGENTS.md)

## Performance
- Cold start is dominated by Chrome launch, not Rust binary size
- Prefer `doctor --offline --quick` for install checks without network
- Reuse multi-step scripts to avoid repeated Chrome launches

## Memory Requirements
- Expect Chrome process memory far above the CLI binary itself
- Heap tools need `--category-memory` and larger snapshots increase RAM use
- Screencast export may invoke ffmpeg as an external helper

## Troubleshooting FAQ
- Chrome not found: install Chromium or Google Chrome and re-run `doctor`
- Exit 69 unavailable: browser binary missing, blocked, or not launchable
- Exit 124 timeout: raise `--timeout` or shorten the script
- Exit 2 usage: re-check flags with `browser-automation-cli help <cmd>`
- `@eN` refs invalid across commands: keep steps inside one `run` process
- Network empty: pass `--capture-network` on the same process that navigates

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
- [skill/browser-automation-cli-en/SKILL.md](skill/browser-automation-cli-en/SKILL.md) imperative agent skill
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
