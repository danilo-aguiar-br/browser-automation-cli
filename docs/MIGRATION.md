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
- Explicitly out of 0.1.0 only: local MITM, workflow journal, and Firecrawl crawl/map/search surface


## 0.1.0 → 0.1.1
### Configuration and XDG
- Product environment variable contract is removed
- Do not set `BROWSER_AUTOMATION_CLI_*` for settings
- Configure with CLI flags and XDG via `config init|set|get|path|show`
- `config path --json` reports `config_dir`, `data_dir`, `state_dir`, `mitm_ca_dir`, `mitm_capture_dir`, `workflow_dir`, and related paths
- Encryption key moves to `config set encryption_key`, not env
- OS env remains only for host conventions: `RUST_LOG`, `NO_COLOR`
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

### Firecrawl-local surface
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
- Hosted GitHub Actions workflows removed; validation is local


## Step-by-Step Migration
- Install or rebuild the binary for `0.1.1`
- Replace session-daemon calls with one-shot subprocess invocations
- Rewrite multi-step agent plans into NDJSON scripts for `run`
- Switch output consumers to `--json` envelopes
- Drop any scripts that export `BROWSER_AUTOMATION_CLI_*` product settings
- Move durable defaults into `config set` or keep them as explicit flags
- Move encryption material to `config set encryption_key <secret>`
- Map old tool names through `commands --json` and the DevTools tool map
- Update screenshot callers to `grab --path <file>`
- Update waits that need alternate text to repeatable `--text` (OR)
- Update scrape callers to pass `--format` and `--engine` explicitly when needed
- Discover new v0.1.1 surfaces with `schema --cmd <name> --json`
- Confirm doctor XDG path health with `doctor --json`
- Re-run local validation: `cargo test --lib`, e2e script, and smoke pillars you care about


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
- Live per-command input fragments come from `schema --cmd`
- Static snapshots under `docs/schemas/` are a convenience index and may lag the binary
- v0.1.1 static additions include `config`, `mitm`, `workflow`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`, and `wait`
- Prefer live `schema --cmd` after upgrades to confirm the installed binary


## Compatibility Notes
- No previous stable crates.io line exists for this repository before `0.1.0`
- Branding and history cleanup recreated a clean public root commit
- First crates.io publish still requires explicit maintainer approval
- Agents that hard-coded product env vars must migrate to flags + `config`
- Subprocess integration remains the only supported agent path
- Exit codes stay sysexits-style: `0`, `2`, `65`, `66`, `69`, `70`, `74`, `78`, `124`, `130`, `141`


## Rollback
- Pin to the previous local commit or installed binary path
- Keep scripts compatible with the success envelope fields `ok` and `schema_version`
- If rolling back from `0.1.1` to `0.1.0`, remove use of config, mitm, workflow, batch-scrape, crawl, map, search, parse
- If rolling back, also drop scrape `--format`/`--engine` assumptions that depend on `0.1.1`
- If rolling back, restore any wait or grab wrappers that assumed older argv shapes only if your fork had them
- Do not reintroduce product env vars; even on older trees, prefer flags when possible


## See Also
- [CHANGELOG.md](../CHANGELOG.md)
- [docs/AGENTS.md](AGENTS.md)
- [docs/CROSS_PLATFORM.md](CROSS_PLATFORM.md)
- [docs/TESTING.md](TESTING.md)
- [docs/schemas/README.md](schemas/README.md)
