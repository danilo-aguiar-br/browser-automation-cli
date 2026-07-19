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


## Browser Discovery Cascade

Resolution order (never product env vars — product law is **flags + XDG only**):

1. XDG `chrome_path` (`config set chrome_path /absolute/path`) when the file is executable
2. Product browsers cache under XDG data (`browsers/`)
3. `$PATH` names: `google-chrome`, `google-chrome-stable|beta|unstable`, `chromium`, `chromium-browser`, `microsoft-edge`, `msedge`, `brave-browser`, …
4. Known absolute layouts per OS (below)
5. Home-local Puppeteer / Playwright caches under `~/.cache/`

Override: `browser-automation-cli config set chrome_path /path/to/chrome`  
Diagnostics: `browser-automation-cli doctor --offline --quick --json` reports `path`, `sandbox`, `executable`, and `host_environment`.

### Linux known paths
- `/usr/bin/google-chrome`, `google-chrome-stable|beta|unstable`, `chromium`, `chromium-browser`
- `/opt/google/chrome/chrome`, `/opt/google/chrome/google-chrome`
- `/usr/bin/microsoft-edge`, `/opt/microsoft/msedge/msedge`
- Snap: `/snap/bin/chromium` (emits sandbox **warn** — prefer APT/RPM)
- Flatpak exports: `/var/lib/flatpak/exports/bin/com.google.Chrome`, `org.chromium.Chromium`, user `~/.local/share/flatpak/exports/bin/…`

### macOS known paths
- `/Applications/Google Chrome.app/…`, Beta, Canary
- `/Applications/Chromium.app/…`, `Microsoft Edge.app`, `Brave Browser.app`
- `~/Applications/Google Chrome.app/…` (per-user installs)

### Windows known paths
- `%ProgramFiles%` / `%ProgramFiles(x86)%` / `%LOCALAPPDATA%` joined with:
  - `Google\Chrome\Application\chrome.exe`
  - `Google\Chrome Beta\…`, `Google\Chrome SxS\…` (Canary)
  - `Microsoft\Edge\Application\msedge.exe`
  - `BraveSoftware\Brave-Browser\Application\brave.exe`
- Hardcoded `C:\Program Files\…` only as last-resort fallback when env vars are missing
- Console boot: UTF-8 code page **65001** + `ENABLE_VIRTUAL_TERMINAL_PROCESSING` for ANSI
- Residual Chrome trees: Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`)

### Snap / Flatpak sandboxes
- Detected by path prefix (`/snap/`, `/var/lib/flatpak/`, `~/.var/app/`) and `$SNAP` / `$FLATPAK_ID`
- Doctor status becomes **warn** when sandbox is restricted
- Prefer system packages; CDP + temp user-data-dir often break under confinement


## Linux Notes
- Common binaries include `chromium-browser`, `chromium`, and `google-chrome`
- Run `doctor` after package install to confirm discovery
- Override discovery with `config set chrome_path /path/to/chrome` when PATH is messy
- Headless is default for local agent runs
- On Alpine or other musl hosts, cross-compile or build natively for the musl target
- Provide a real Chrome or Chromium binary; the CLI does not bundle a browser
- Containers auto-add Chrome `--no-sandbox` and `--disable-dev-shm-usage` when root or docker/podman/k8s markers are present
- Residual disk hygiene (v0.1.5): BORN + FINALIZE scavenge owned Singleton-only Chromium tmp under process temp (commonly `/tmp/org.chromium.Chromium.*` and `/tmp/.org.chromium.Chromium.*`)
- Stale Singleton GC age floor is **60s**; only same-uid Singleton-only (or empty) dirs with no live `/proc` holder are wiped
- CLI markers use prefix `browser-automation-cli-chrome-*` under the process temp dir
- Host Flatpak Chrome temp prefixes are **never** deleted by product residual GC
- Inspect with `doctor --offline --quick --json` → top-level `residual` and check `residual_disk`


## macOS Notes
- Install Google Chrome from the official channel
- Prefer full binary path via XDG `chrome_path` only when PATH discovery fails
- Apple Silicon and Intel both use system Chrome discovery
- Grant accessibility or screen permissions only if you use headed debugging outside agents
- Universal binary / notarization are **release-ops** (not required for source builds)


## Windows Notes
- Use PowerShell or cmd with explicit quoting around URLs
- Prefer `--json` to avoid locale-dependent prose parsing
- Keep argv UTF-8 clean; avoid mojibake when piping through legacy code pages
- Quote paths with spaces: `"C:\Users\me\out.png"`
- Prefer `grab --path` with a full path rather than relying on cwd
- Windows process helpers live behind `cfg(windows)` and do not change the JSON contract
- Path basenames reserved on Windows (`CON`, `NUL`, `COM1`, …) are rejected on **all** hosts for portable scripts
- Residual **process** hygiene uses Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) so Chrome trees die with the CLI process
- Disk residual report fields (`residual` / `residual_disk`) remain available via doctor for marker and temp hygiene diagnostics


## Containers
- Install Chrome or Chromium in the image before runtime tests
- Provide enough shared memory for Chrome (`/dev/shm` or equivalent)
- Keep one-shot process cleanup expectations under orchestration restarts
- Do not assume a host-mounted product settings file outside XDG; use flags and XDG mounts if needed
- Example shape: package `browser-automation-cli` plus Chromium, then call `doctor --json`
- Optional: Redis server when testing `cache_backend redis`; Lighthouse binary or mock for audits
- Host probe: `doctor --json` → `host_environment.container` / `.wsl` / `.ci` / `.termux`


## Host environment probe
- Module `platform::HostEnvironment` detects WSL, container, CI markers, Termux, Flatpak, Snap
- Used by doctor diagnostics and Chrome launch flags (container → sandbox/dev-shm flags)
- CI env keys are **observability only** — never product settings


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
- Product settings are flags and XDG `config` only — never product environment variables
- Product settings use flags and XDG CLI only (`config path|init|show|set|get|list-keys`)
- Language for human suggestions: `--lang` or XDG `lang` only
- Full command inventory (63 agent names) and agent patterns: [docs/HOW_TO_USE.md](HOW_TO_USE.md)
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
