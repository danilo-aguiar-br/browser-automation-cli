[English](CROSS_PLATFORM.md) | [Português Brasileiro](CROSS_PLATFORM.pt-BR.md)

# Cross Platform — browser-automation-cli

> Stop rewriting browser automation for every host OS. Lifecycle: BORN EXECUTE FINALIZE DIE.


## The Pain You Already Know
- Browser tooling often assumes one OS path layout
- Local agents fail when Chrome discovery is host-specific and undocumented
- Shell quoting and path separators break fragile wrappers
- Settings scattered outside flags and XDG `config` multiply across shells without a single source of truth


## Support Matrix

| Platform | Arch | Status | Notes |
|----------|------|--------|-------|
| Linux | x86_64 | primary | Chromium and Google Chrome common paths |
| Linux | aarch64 | supported | requires local Chrome or Chromium |
| macOS | x86_64 | supported | system Chrome discovery |
| macOS | aarch64 | supported | system Chrome discovery |
| Windows | x86_64 | supported | Windows-specific process helpers |
| Windows | aarch64 | compile-time | build from source when the Rust target is available |

- docs.rs documents `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, and `aarch64-unknown-linux-musl`
- musl and Alpine are compile-time target possibilities (`aarch64-unknown-linux-musl` and similar)
- This repository does not ship prebuilt musl or multi-arch release artifacts by default
- Validate the binary on your host with `doctor --json` after install


## Linux Notes
- Common binaries include `chromium-browser`, `chromium`, and `google-chrome`
- Run `doctor` after package install to confirm discovery
- Override discovery with `config set chrome_path /path/to/chrome` when PATH is messy
- Headless is default for local agent runs
- On Alpine or other musl hosts, cross-compile or build natively for the musl target
- Provide a real Chrome or Chromium binary; the CLI does not bundle a browser


## macOS Notes
- Install Google Chrome from the official channel
- Prefer full binary path via XDG `chrome_path` only when PATH discovery fails
- Apple Silicon and Intel both use system Chrome discovery
- Grant accessibility or screen permissions only if you use headed debugging outside agents


## Windows Notes
- Use PowerShell or cmd with explicit quoting around URLs
- Prefer `--json` to avoid locale-dependent prose parsing
- Keep argv UTF-8 clean; avoid mojibake when piping through legacy code pages
- Quote paths with spaces: `"C:\Users\me\out.png"`
- Prefer `grab --path` with a full path rather than relying on cwd
- Windows process helpers live behind `cfg(windows)` and do not change the JSON contract


## Containers
- Install Chrome or Chromium in the image before runtime tests
- Provide enough shared memory for Chrome (`/dev/shm` or equivalent)
- Keep one-shot process cleanup expectations under orchestration restarts
- Do not assume a host-mounted product settings file outside XDG; use flags and XDG mounts if needed
- Example shape: package `browser-automation-cli` plus Chromium, then call `doctor --json`
- Optional: Redis server when testing `cache_backend redis`; Lighthouse binary or mock for audits


## Shell Support
- bash, zsh, fish, and PowerShell can spawn the binary
- Completions are generated through `completions <shell>`
- Supported completion shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`
```bash
browser-automation-cli completions bash
browser-automation-cli completions zsh
browser-automation-cli completions fish
browser-automation-cli completions powershell
```


## File Paths and XDG
- Resolve live paths with `browser-automation-cli config path --json`
- Init layout with `browser-automation-cli config init`
- Config file is XDG `config.toml` under the product config dir
- `config path --json` includes fields such as `config_dir`, `data_dir`, `state_dir`, `mitm_ca_dir`, `mitm_capture_dir`, `workflow_dir`
- Related fields also include `config_file`, `cache_dir`, `browsers_dir`, `sessions_dir`, `home_dir`, and `layout`
- Artifacts follow `--artifacts-dir` when provided (flag or config key)
- Cache, state, sessions, and workflow journals stay under user-local XDG trees
- MITM CA material lives under XDG data (`mitm/ca`); captures under XDG state (`mitm/`)
- Workflow journals live under XDG state (`workflows`)
- Encryption key is set with `config set encryption_key <value>`
- Full config keys (16): `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- Product settings use flags and XDG CLI only (`config path|init|show|set|get|list-keys`)
- Redis cache: `cache_backend redis` + `cache_redis_url redis://…` only (`rediss://` fail-closed)
- Product logging: `--verbose` / `--debug` / `-q` or XDG `log_level`
- Color: `config set color`; Chrome path: `config set chrome_path`


## Performance by Target
- Linux desktop and servers are the primary optimization target
- Cold start remains Chrome-bound on every OS when using the browser engine
- Prefer `--engine http` on scrape-style commands when a full browser is unnecessary
- Local maintainer validation uses `cargo build --release`, host Chrome, and e2e scripts


## Agents Validated per Platform
- Integration mode everywhere: one-shot subprocess plus `--json`
- Linux: Claude Code, Codex, Gemini CLI, Cursor, shell local, editor agents
- macOS: local shell agents and editor integrations
- Windows: shell and editor integrations with explicit quoting
- Expanded agent lists in [docs/AGENTS.md](AGENTS.md) are subprocess-compatible via local validation with cargo and e2e scripts
