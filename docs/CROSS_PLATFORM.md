[English](CROSS_PLATFORM.md) | [Português Brasileiro](CROSS_PLATFORM.pt-BR.md)

# Cross Platform — browser-automation-cli


- Stop rewriting browser automation for every host OS
- Lifecycle: BORN EXECUTE FINALIZE DIE


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
- The resolution order never reads product environment variables, because the product law is flags plus XDG only
- Step 1 is XDG `chrome_path` (`config set chrome_path /absolute/path`) when the file is executable
- Step 2 is the product browsers cache under XDG data (`browsers/`)
- Step 3 runs on Windows only: `HKLM` then `HKCU` `SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{chrome.exe|msedge.exe|brave.exe}` (OS registry discovery via `windows-sys`, not product config)
- Step 4 is the `$PATH` names `google-chrome`, `google-chrome-stable|beta|unstable`, `chromium`, `chromium-browser`, `chrome`, `microsoft-edge`, `msedge`, `brave-browser` and similar
- Step 5 is the known absolute layouts per OS listed below
- Step 6 is the home-local Puppeteer and Playwright caches under `~/.cache/`
- Override the cascade with `browser-automation-cli config set chrome_path /path/to/chrome`
- Undo the override with `browser-automation-cli config unset chrome_path`
- `browser-automation-cli doctor --offline --quick --json` reports `path`, `sandbox`, `executable`, `version` (`--version` smoke), `windows_job_object` and `host_environment`

### Linux known paths
- `/usr/bin/google-chrome`, `google-chrome-stable|beta|unstable`, `chromium`, `chromium-browser`
- `/opt/google/chrome/chrome`, `/opt/google/chrome/google-chrome`
- `/usr/bin/microsoft-edge`, `/opt/microsoft/msedge/msedge`
- Snap: `/snap/bin/chromium` (emits sandbox warn — prefer APT/RPM)
- Flatpak exports: `/var/lib/flatpak/exports/bin/com.google.Chrome`, `org.chromium.Chromium`, user `~/.local/share/flatpak/exports/bin/…`

### macOS known paths
- `/Applications/Google Chrome.app/…`, Beta, Canary
- `/Applications/Chromium.app/…`, `Microsoft Edge.app`, `Brave Browser.app`
- `~/Applications/Google Chrome.app/…` (per-user installs)

### Windows known paths
- Registry App Paths for `chrome.exe` / `msedge.exe` / `brave.exe` (before `$PATH` walk)
- `%ProgramFiles%`, `%ProgramFiles(x86)%` and `%LOCALAPPDATA%` are joined with `Google\Chrome\Application\chrome.exe`
- The same roots are joined with `Google\Chrome Beta\…` and `Google\Chrome SxS\…` (Canary)
- The same roots are joined with `Microsoft\Edge\Application\msedge.exe` and `BraveSoftware\Brave-Browser\Application\brave.exe`
- Hardcoded `C:\Program Files\…` only as last-resort fallback when the OS path variables are missing
- Console boot: UTF-8 code page 65001 + `ENABLE_VIRTUAL_TERMINAL_PROCESSING` for ANSI
- Residual Chrome trees: Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`)

### Snap / Flatpak sandboxes
- Detected by path prefix (`/snap/`, `/var/lib/flatpak/`, `~/.var/app/`) and `$SNAP` / `$FLATPAK_ID`
- Doctor status becomes warn when sandbox is restricted
- Prefer system packages
- CDP + temp user-data-dir often break under confinement


## Linux Notes
- Common binaries include `chromium-browser`, `chromium`, and `google-chrome`
- Run `doctor` after package install to confirm discovery
- Override discovery with `config set chrome_path /path/to/chrome` when PATH is messy
- `browser_mode auto` is headless unless the host is Linux with `Xvfb` on PATH and no `--no-xvfb`, as stated in [Anti-Detection Across Platforms](#anti-detection-across-platforms)
- On Alpine or other musl hosts, cross-compile or build natively for the musl target
- Provide a real Chrome or Chromium binary
- The CLI does not bundle a browser
- Containers auto-add Chrome `--no-sandbox` and `--disable-dev-shm-usage` when root or docker/podman/k8s markers are present
- Residual disk hygiene (v0.1.5 law still current in 0.2.0): BORN + FINALIZE scavenge owned Singleton-only Chromium tmp under process temp (commonly `/tmp/org.chromium.Chromium.*` and `/tmp/.org.chromium.Chromium.*`)
- Stale Singleton GC age floor is 60s
- Only same-uid Singleton-only (or empty) dirs with no live `/proc` holder are wiped
- CLI markers use prefix `browser-automation-cli-chrome-*` under the process temp dir
- Host Flatpak Chrome temp prefixes are never deleted by product residual GC
- Inspect with `doctor --offline --quick --json` → top-level `residual` and check `residual_disk`

### Headed launch under Wayland and the private Xvfb
- A headed launch on Linux draws into a private Xvfb display when the `Xvfb` binary is on PATH and `--no-xvfb` is absent
- `Xvfb` is a host package the CLI never installs, and `doctor` reports it with the install hint for the distribution
- Without `Xvfb` the launch warns, continues headed on the current display, and `display_backend` reports `host`
- With the private display up, `display_backend` reports `xvfb`
- `display_backend` follows the display the launch actually used since 0.2.0, so a run whose Xvfb failed reports `host`
- A headed run with an extension starts the private display too since 0.2.0
- Under a Wayland session Chromium picks Wayland from `XDG_SESSION_TYPE` when no `--ozone-platform` is given
- Removing `WAYLAND_DISPLAY` alone does not stop that, because Chromium then finds `$XDG_RUNTIME_DIR/wayland-0` by itself
- Measured on Fedora 44 with Chromium 152, that left the window on the real compositor while Xvfb drew nothing
- The CLI therefore adds `--ozone-platform=x11` when the private display started, and also removes `WAYLAND_DISPLAY` from Chrome's environment
- The pin enters only when Xvfb actually came up, because forcing X11 with no X server fails with `Missing X server or $DISPLAY`
- A platform already present in Chrome's arguments is kept
- `--no-xvfb` starts no private display, adds no pin, and lets Chrome use the current display, Wayland included
- `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY` and `XDG_RUNTIME_DIR` are host facts Chromium reads, never product configuration
- Concurrent headed runs each get their own display, searched from `:99` over 32 numbers
- A run owns a display only when `/tmp/.X{n}-lock` names the pid of its own Xvfb
- A server that loses the race for a number exits, and the launch retries on the next free number
- The lock and the socket are removed at teardown only when the lock names this run's Xvfb
- A lock left by a dead process is reused, because a CLI killed with `SIGKILL` kills its Xvfb the same way
- The private Xvfb stops with `SIGTERM` and a 2-second grace before `SIGKILL`, so the server removes its own lock and socket
- A server that does not come up within 10 seconds ends the attempt
- The private Xvfb starts with `-nolisten tcp` and writes its output to the null device
- The private display demands a `MIT-MAGIC-COOKIE-1` held in a mode 0600 file in the per-user runtime directory, with the temp directory as fallback
- The server receives that file through `-auth`, and Chrome receives it through `XAUTHORITY`
- The file name carries the creator's pid, so the next launch removes a cookie left by a CLI killed with `SIGKILL`
- The file is removed at teardown after the server that reads it is gone
- `doctor`'s `virtual_display` message names the value `auto` resolves to on this host since 0.2.0
- macOS and Windows never start Xvfb

### DevTools pipe per platform
- A self-spawned Chrome gets DevTools through `--remote-debugging-pipe` and opens no TCP port
- The CLI talks to Chrome through a loopback WebSocket bridge on a path holding the 122 random bits of a v4 UUID
- The bridge relays exactly one valid client, answers `403` on any other path, and serves no `/json/version`
- The bridge gives a client 2 seconds to finish the WebSocket handshake
- The bridge caps each DevTools message at 256 MiB in either direction
- The bridge waits at most 2 seconds for its pipe threads at teardown
- Chrome's and Lightpanda's stdout and stderr drainers get the same 2-second bound at teardown
- On Linux and macOS Chrome reads the pipe on descriptors 3 and 4
- On Linux both pipes are created close-on-exec atomically with `pipe2`
- On macOS the pipes are created with `pipe` and the close-on-exec flag is set right after, which leaves a narrow window where a fork on another thread inherits the ends
- On Windows Chrome receives two inheritable handles in `--remote-debugging-io-pipes`, and the handles the CLI keeps are marked non-inheritable
- A host with neither POSIX descriptors nor Windows handles refuses the pipe with an error
- On Unix a failed Chrome launch kills Chrome's whole process group, and on Windows that group kill does nothing
- On every platform the teardown of a failed launch runs on a blocking thread through `kill_off_the_runtime` in `src/native/cdp/chrome/spawn.rs`, so `--timeout` still ends the command with exit 124 instead of 69
- Linux is the only host where the pipe was measured
- macOS was NOT validated live for the pipe
- Windows was NOT compiled or tested for the pipe
- KDE and Sway sessions were NOT validated live for the pipe or the X11 pin
- The Lightpanda engine and the legacy launch behind `chrome_legacy_oxide_launch` keep their previous transport


## macOS Notes
- Install Google Chrome from the official channel
- Prefer full binary path via XDG `chrome_path` only when PATH discovery fails
- Apple Silicon and Intel both use system Chrome discovery
- Grant accessibility or screen permissions only if you use headed debugging outside agents
- Universal binary / notarization are release-ops (not required for source builds)


## Windows Notes
- Use PowerShell or cmd with explicit quoting around URLs
- Prefer `--json` to avoid locale-dependent prose parsing
- Keep argv UTF-8 clean
- Avoid mojibake when piping through legacy code pages
- Quote paths with spaces: `"C:\Users\me\out.png"`
- Prefer `grab --path` with a full path rather than relying on cwd
- Windows process helpers live behind `cfg(windows)` and do not change the JSON contract
- Path basenames reserved on Windows (`CON`, `NUL`, `COM1`, …) are rejected on all hosts for portable scripts
- Residual process hygiene uses Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) so Chrome trees die with the CLI process
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
- `browser_mode auto` resolves to headed inside a private virtual display on Linux with Xvfb on PATH and without `--no-xvfb`
- In every other case, macOS and Windows included, `browser_mode auto` resolves to headless
- macOS always has Quartz and Windows always has DWM, so neither uses Xvfb
- `doctor` reports the `xvfb` check as info on every non-Linux host
- Without Xvfb on PATH a headed launch falls back to the current display
- The install hint is read from `/etc/os-release` and the CLI never installs anything
- `DISPLAY` and `WAYLAND_DISPLAY` are host facts read only on Linux
- Reading them is not product configuration, which stays flags plus XDG
- The `virtual_display` check exposes `host_has_display`, `browser_mode_auto_resolves` and `private_display_supported`
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
- Do not assume a host-mounted product settings file outside XDG
- Use flags and XDG mounts to carry settings into the container
- Example shape: package `browser-automation-cli` plus Chromium, then call `doctor --json`
- Optional: Redis server when testing `cache_backend redis`
- Optional: Lighthouse binary or mock for audits
- Host probe: `doctor --json` → `host_environment.container` / `.wsl` / `.termux`


## Host environment probe
- Module `platform::HostEnvironment` detects WSL, container, Termux, Flatpak and Snap
- Used by doctor diagnostics and Chrome launch flags (container → sandbox/dev-shm flags)
- Host markers are observability only, and never product settings


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
- MITM CA material lives under XDG data (`mitm/ca`)
- MITM captures live under XDG state (`mitm/`)
- Workflow journals live under XDG state (`workflows`)
- Encryption key is set with `config set encryption_key <value>`
- Discover live config keys with `config list-keys --json`, and do not hard-code a fixed count such as “16 keys”
- Product settings are flags and XDG `config` only — never product environment variables
- Product settings use flags and the XDG CLI only (`config path|init|show|set|unset|get|list-keys`)
- Undo any key you wrote with `config unset <key>`
- Language for human suggestions: `--lang` or XDG `lang` only
- Full command inventory (71 agent names) and agent patterns: [docs/HOW_TO_USE.md](HOW_TO_USE.md)
- Redis cache: `cache_backend redis` + `cache_redis_url redis://…` only (`rediss://` fail-closed)
- Product logging: `--verbose` / `--debug` / `-q` or XDG `log_level`
- Color: `config set color`
- Chrome path: `config set chrome_path`


## v0.2.0 agent surface (compact)
- Anti-detection family is live: `stealth`, `stealth_profile`, `stealth_seed`, `browser_mode`, `input_profile`
- Same family adds proxy keys, HTTP/2 `SETTINGS` keys and the ten `input_*` timing keys
- Global anti-detection flags: `--no-stealth`, `--stealth-profile`, `--stealth-seed`, `--input-profile`, `--input-seed`
- More of the same family: `--proxy`, `--proxy-bypass`, `--headed`, `--no-xvfb`, `--warmup`, `--warmup-url`
- Every one of those flags parses identically on Linux, macOS and Windows
- Scrape envelopes disclose `stealth`, `profile_contradicts_host`, `http2_profile` and `tls_impersonation`
- Platform behaviour for the whole family lives in [Anti-Detection Across Platforms](#anti-detection-across-platforms)
- A self-spawned Chrome uses the DevTools pipe on every platform, as stated in [DevTools pipe per platform](#devtools-pipe-per-platform)
- `dialog_settled` boolean after real dialog accept/dismiss (GAP-054)
- Multi-tab dialog isolation goes through `Page::session_id` / `dialog_map_key`
- `dialog_settle_ms` via XDG `config set` only (flags + XDG, never product env vars)
- `wait_timeout_ms` public key on run wait steps (GAP-053)
- Scrape `format`/`formats` in run without HTML monster (GAP-057)
- Native select `pick`/`select-option` dispatches `input` then `change`, `via: native_select` (GAP-055)
- Universal envelope flags: `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes` on all 71 commands, identical on every platform
- `agent_ops` appears in the success envelope only when one of those flags ran
- `unresolved_paths` names a path no row carried
- `agent_ops` is omitted when there is nothing to report: a flag that ran and resolved cleanly leaves the envelope shape untouched, on every platform
- `--select`/`--filter`/`--limit`/`--sort` are NOT global: they are per-command flags on scrape, crawl, map, search, batch-scrape and the media `info` verbs
- XDG keys: 217 documented in [CONFIGURATION.md](CONFIGURATION.md)
- Discover live with `config list-keys --json`
- `grab` encode: png|jpeg|webp only
- AVIF removed (breaking)
- Inventory 71 includes `submit` + `storage` + `image`+`video`+`audio`+`record`
- The residual-zero disk law from 0.1.5 is still current
- GAP-021 partial (unit LHR fixtures, e2e lighthouse mock SKIP)
- GAP-022 residual ~53 dups accepted
- GAP-023/024 intentional divergences


## Full agent inventory (71)
- Discover live: `browser-automation-cli commands --json`
```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract feed fill-form
find-paths forward goto grab heap hover image video audio keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr record reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write sitemap storage submit text type
upload version view wait webmcp workflow write
```
- `pick` and `select-option` are multi-step inventory names used in `run` scripts
- The clap product subcommand count is 69 (71 agent names − 2 run-only)


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
