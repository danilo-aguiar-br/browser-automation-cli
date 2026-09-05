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
3. **Windows only:** `HKLM` then `HKCU` `SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{chrome.exe|msedge.exe|brave.exe}` (OS registry discovery via `windows-sys`, not product config)
4. `$PATH` names: `google-chrome`, `google-chrome-stable|beta|unstable`, `chromium`, `chromium-browser`, `chrome`, `microsoft-edge`, `msedge`, `brave-browser`, …
5. Known absolute layouts per OS (below)
6. Home-local Puppeteer / Playwright caches under `~/.cache/`

Override: `browser-automation-cli config set chrome_path /path/to/chrome`  
Diagnostics: `browser-automation-cli doctor --offline --quick --json` reports `path`, `sandbox`, `executable`, `version` (`--version` smoke), `windows_job_object`, and `host_environment`.

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
- Registry **App Paths** for `chrome.exe` / `msedge.exe` / `brave.exe` (before `$PATH` walk)
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
- Residual disk hygiene (v0.1.5 law still current in 0.1.9): BORN + FINALIZE scavenge owned Singleton-only Chromium tmp under process temp (commonly `/tmp/org.chromium.Chromium.*` and `/tmp/.org.chromium.Chromium.*`)
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


## Windows File-Permission Residual (declared, not fixed)
- Five files are created with restrictive POSIX modes on Unix and inherit the parent directory ACL on Windows
- `mitm_local/ca.rs` writes the MITM certificate authority PRIVATE KEY with `0o600`
- `xdg/config_write.rs` writes `config.toml` with `0o600`, and it holds `encryption_key`, `openrouter_api_key` and `proxy_password`
- `mitm_local/util.rs` writes captured request and response bodies with `0o600`
- `xdg/paths.rs` creates the XDG state directory with `0o700`
- `native/stealth/seed_cache.rs` writes the stealth identity seed with `0o600`
- Nothing BREAKS on Windows: the writes succeed and no code path panics
- What differs is the security POSTURE, and only for these five paths
- On Windows the effective protection is whatever the parent directory grants
- Store the product in a directory whose ACL you control when the host is shared
- This is a declared residual, not an oversight: an untested ACL implementation would be a worse answer than a measured statement of the gap
- Closing it needs a Windows host to verify against, which the measurement above did not have


## Anti-Detection Across Platforms
- `stealth` ships on by default and behaves the same on all three systems
- `stealth_profile auto` resolves against the host: Windows gives chrome-win, macOS gives chrome-mac
- Every other host resolves to chrome-linux, containers and WSL included
- A foreign profile is reported, never blocked, through `profile_contradicts_host`
- The headless override swaps only the `HeadlessChrome` product token for `Chrome`
- That swap keeps the real host platform and never invents another one
- `browser_mode auto` resolves to headless on Linux, macOS and Windows alike
- The private virtual display (Xvfb) is Linux only and needs an explicit headed launch
- macOS always has Quartz and Windows always has DWM, so neither uses Xvfb
- `doctor` reports the `xvfb` check as info on every non-Linux host
- Without Xvfb on PATH a headed launch falls back to the current display
- The install hint is read from `/etc/os-release` and the CLI never installs anything
- `DISPLAY` and `WAYLAND_DISPLAY` are host facts read only on Linux
- Reading them is not product configuration, which stays flags plus XDG
- The `virtual_display` check exposes `host_has_display` and `private_display_supported`
- Vulkan and ANGLE flags behind `--enable-unsafe-webgpu` are emitted on Linux only
- The proxy path does not vary by platform at any point
- `proxy_url` feeds Chrome `--proxy-server` and the shared HTTP client alike
- `HTTP_PROXY`, `HTTPS_PROXY` and `ALL_PROXY` are never inherited on any host
- Without `proxy_url` the client calls `no_proxy` and disables system proxy discovery
- `cdp_proxy_bypass_loopback` appends loopback to the bypass list on both sides
- Credentials come only from `proxy_username` and `proxy_password` under XDG
- With stealth on, Chrome receives `--disable-quic` on every platform
- HTTP/2 window and frame values are identical on every platform
- `http2_enabled false` drops the client to HTTP/1.1 and reports `http2_profile: disabled`
- `input_profile human` synthesizes events through CDP `Input` domain calls only
- No operating system input API is used, so kinematics are identical everywhere
- macOS asks for no accessibility permission because no native input API is touched
- Key codes travel in `windows_virtual_key_code` and `native_virtual_key_code` on every host
- The macOS Cmd modifier is the caller's bitmask choice, not a product default
- `stealth_seed` pins the identity across processes and its cache is 0600 on Unix
- Windows gets no such file permission tightening on that seed cache
- Chrome receives `--password-store=basic` and `--use-mock-keychain` by default on every host
- Both are suppressed together when a launch opts into the real keychain, which no product path does today
- Key names and defaults for this family live in [CONFIGURATION.md](CONFIGURATION.md)


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
- Discover live config keys with `config list-keys --json` (includes `dialog_settle_ms`; do not hard-code a fixed count such as “16 keys”)
- Product settings are flags and XDG `config` only — never product environment variables
- Product settings use flags and XDG CLI only (`config path|init|show|set|get|list-keys`)
- Language for human suggestions: `--lang` or XDG `lang` only
- Full command inventory (**71** agent names) and agent patterns: [docs/HOW_TO_USE.md](HOW_TO_USE.md)
- Redis cache: `cache_backend redis` + `cache_redis_url redis://…` only (`rediss://` fail-closed)
- Product logging: `--verbose` / `--debug` / `-q` or XDG `log_level`
- Color: `config set color`; Chrome path: `config set chrome_path`

## v0.1.9 agent surface (compact)

- Anti-detection family is live: `stealth`, `stealth_profile`, `stealth_seed`, `browser_mode`, `input_profile`
- Same family adds proxy keys, HTTP/2 `SETTINGS` keys and the ten `input_*` timing keys
- Global anti-detection flags: `--no-stealth`, `--stealth-profile`, `--stealth-seed`, `--input-profile`, `--input-seed`
- More of the same family: `--proxy`, `--proxy-bypass`, `--headed`, `--no-xvfb`, `--warmup`, `--warmup-url`
- Every one of those flags parses identically on Linux, macOS and Windows
- Scrape envelopes disclose `stealth`, `profile_contradicts_host`, `http2_profile` and `tls_impersonation`
- Platform behaviour for the whole family lives in [Anti-Detection Across Platforms](#anti-detection-across-platforms)
- **`dialog_settled`** boolean after real dialog accept/dismiss (GAP-054); multi-tab isolation via `Page::session_id` / `dialog_map_key`
- **`dialog_settle_ms`** via XDG `config set` only (flags + XDG; never product env vars)
- **`wait_timeout_ms`** public key on run wait steps (GAP-053)
- Scrape `format`/`formats` in run without HTML monster (GAP-057)
- Native select `pick`/`select-option` dispatches `input` then `change`, `via: native_select` (GAP-055)
- **Universal envelope flags:** `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes` on all 71 commands, identical on every platform
- **`agent_ops`** appears in the success envelope only when one of those flags ran; `unresolved_paths` names a path no row carried
- **`agent_ops` is omitted when there is nothing to report:** a flag that ran and resolved cleanly leaves the envelope shape untouched, on every platform
- **`--select`/`--filter`/`--limit`/`--sort` are NOT global:** they are per-command flags on scrape, crawl, map, search, batch-scrape and the media `info` verbs
- **XDG keys:** 206 documented in [CONFIGURATION.md](CONFIGURATION.md); discover live with `config list-keys --json`
- **`grab` encode:** png|jpeg|webp only; AVIF removed (breaking)
- Inventory **71** includes `submit` + `storage` + `image`+`video`+`audio`+`record`; residual-zero disk law from 0.1.5 still current
- GAP-021 partial (unit LHR fixtures; e2e lighthouse mock SKIP); GAP-022 residual ~53 dups accepted; GAP-023/024 intentional divergences

## Full agent inventory (71)

Discover live: `browser-automation-cli commands --json`

```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract feed fill-form
find-paths forward goto grab heap hover image video audio keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr record reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write sitemap storage submit text type
upload version view wait webmcp workflow write
```

Note: `pick` and `select-option` are multi-step inventory names used in `run` scripts; clap product subcommand count is **69** (71 agent names − 2 run-only).

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
