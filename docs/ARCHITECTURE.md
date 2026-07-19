[English](ARCHITECTURE.md) | [Português Brasileiro](ARCHITECTURE.pt-BR.md)

# Architecture — browser-automation-cli

- One-shot Chrome CDP automation for AI agents
- Lifecycle is always: BORN → EXECUTE → FINALIZE → DIE (single process; no daemon)
- Full agent command list (63 names): see [docs/HOW_TO_USE.md](HOW_TO_USE.md) and `browser-automation-cli commands --json`

## Layers

| Layer | Path | Role |
|-------|------|------|
| Binary thin | `src/main.rs` | panic hook, `run_from_args`, exit code |
| Lib entry | `src/lib.rs` | `run` / `run_from_args`, telemetry hold, lifecycle |
| CLI surface | `src/cli.rs` | Clap derive (`Parser` / `Subcommand`); help = agent UX |
| Dispatch | `src/commands_prd/` | PRD handlers (`mod.rs` match + `meta` + `run`) |
| Session | `src/browser/` | One-shot Chrome session, actions, residual ledger hooks |
| Native CDP | `src/native/` | chromiumoxide client, snapshot, heap, cookies, … |
| Contract I/O | `src/output.rs`, `src/envelope.rs`, `src/json_util.rs` | stdout envelopes; BrokenPipe → 141 |
| Lifecycle | `src/lifecycle.rs` | cancel token, BORN/FINALIZE orchestration, SIGINT/SIGTERM |
| Residual disk/process | `src/residual.rs` | marker + Chromium tmp Singleton GC; `ResidualDiskReport` |
| Telemetry | `src/telemetry.rs` | tracing dual sink (stderr + optional rotated JSON) |
| XDG config | `src/xdg.rs`, `src/config.rs` | product settings: flags + XDG `config` only |
| i18n | `src/i18n/`, `locales/*.ftl` | `--lang` + XDG `lang` → negotiate → OnceLock; human suggestions only |
| Platform | `src/platform.rs` | PATH `which_bin`, console UTF-8/VT, HostEnvironment, browser sandbox |
| Windows jobs | `src/win_job.rs` | Job Object residual process kill (stubs on non-Windows) |

## Residual product law (process + disk)

Product law residual-zero covers **both** live Chrome trees and **disk** hygiene after DIE:

1. **Process residual** — ledger-owned Chrome PID (Unix SIGTERM → grace → SIGKILL; Windows Job Object kill-on-close).
2. **Marker residual** — CLI-owned temp profiles under `browser-automation-cli-chrome-*`.
3. **Chromium tmp Singleton residual** — owned `/tmp/org.chromium.Chromium.*` and `/tmp/.org.chromium.Chromium.*` that are Singleton-only (or empty), same uid, with no live process holding the path.

Never kill or wipe **host Flatpak** Chrome trees (for example `com.google.Chrome.*` temp prefixes). Cross-run GC is Singleton-shape + uid + age + no live holder only.

### Role of `src/residual.rs`

- Marker prefix and Chromium tmp prefix constants (public, anti-hardcode).
- Discovery of invocation-window side-channels (pid/profile attribution).
- Cross-run stale GC: `scavenge_stale_singleton_orphans` with age floor **60s** (`STALE_MIN_AGE_SECS`).
- Live-process checks via a single `/proc` cmdline index (no O(N×P) rescans).
- Machine report: `ResidualDiskReport` / `residual_disk_report()` for doctor and agents.

### BORN and FINALIZE dual scavenge

| Phase | Residual work |
|-------|----------------|
| **BORN** (`Lifecycle::new`) | `scavenge_stale_singleton_orphans` — wipe cross-run Singleton-only orphans older than 60s |
| **FINALIZE** (`Lifecycle::finalize`) | Ledger residual kill/wipe; re-discover invocation-window side-channels; `scavenge_owned_chromium_tmp_orphans`; **second** `scavenge_stale_singleton_orphans` |
| **Drop** | Sync safety net calling the same idempotent finalize path |

FINALIZE dual scavenge = invocation-window orphans **plus** stale Singleton GC so a one-shot cannot leave disk litter for the next process.

### Doctor residual surface

- Check id: `residual_disk` (path-light; no Chrome launch for the report itself).
- Top-level doctor JSON field: `residual` (`ResidualDiskReport`).
- Fields:
  - `cli_marker_dirs` — count of `browser-automation-cli-chrome-*` under temp
  - `chromium_tmp_singleton_orphans` — Singleton-only Chromium tmp that looks orphaned
  - `scavenge_safe_candidates` — paths stale GC would wipe now (age ≥ 60s, owned, no live holder)
  - `live_cli_marker_processes` — live processes whose cmdline contains the CLI chrome marker prefix
- Status: `fail` if live marker processes; `warn` if marker dirs or singleton orphans remain; else `pass`.

Local maintainer gates (no CI/GHA requirement): `scripts/residual-check.sh`, `scripts/residual-stress.sh`.

## i18n (human suggestions)

Precedence for product docs and agents: **`--lang` → XDG `lang` → OS locale (`sys-locale` + `fluent-langneg`) → default `en`**.

- MVP packs: `en` + `pt-BR` (`Idioma` / `Mensagem` exhaustive match + FTL parity).
- Machine JSON `error.message` and tracing stay English (agent contract).
- Optional packs: features `i18n-cjk` / `i18n-rtl` / `i18n-europe` / `i18n-full` (scaffold).
- Diagnostics: subcommand `locale` (+ `--json`).
- Man page generation: subcommand `man` (roff via clap_mangen; no Chrome).

Product settings (including language) use **flags + XDG only**. Do not invent or promote product environment variables for durable config.

## Module map (`commands_prd`)

- `mod.rs` — `dispatch` match on `Commands` + browser/session handlers  
- `meta.rs` — `commands` / `schema` inventory for agents (**63** names via `commands --json`)  
- `run.rs` — multi-step `run` / `exec` script engine (NDJSON steps)

Large handler surface remains in `mod.rs` by design (single match table for agent
parity). Prefer extracting **new** command families into sibling modules rather
than growing unrelated helpers.

## Macros / codegen

- **No** public `macro_rules!` / `proc-macro` crate.  
- CDP protocol stubs: `build.rs` + `include!(concat!(env!("OUT_DIR"), "/cdp_generated.rs"))`.  
- Event forwarders: generic functions (`spawn_cdp_event_forwarder`), not macros.

## Browser discovery (multiplatform)

Order: XDG `chrome_path` → product browsers cache → `$PATH` names → known absolute
layouts (Linux `/usr`/`/opt`/snap/flatpak, macOS `/Applications`, Windows
`%ProgramFiles%` / LocalAppData including Edge/Beta/Canary/Brave) → home
Puppeteer/Playwright caches.

- No product `CHROME_PATH` env (product law: flags + XDG only).  
- Snap/Flatpak paths warn via `tracing` and doctor `sandbox` field.  
- Containers/root get Chrome `--no-sandbox` + `--disable-dev-shm-usage`.  
- Host probe: `doctor --json` → `host_environment` (wsl/container/ci/termux/snap/flatpak).

## Product law (non-negotiable)

- stdout = JSON envelopes only (agent-first)  
- stderr = diagnostics / tracing  
- zero remote telemetry / no MCP server  
- residual zero after DIE: Chrome process + CLI markers + Chromium Singleton tmp (process **and** disk)  
- never kill host Flatpak Chrome residual  
- product settings: flags + XDG only (no product env catalogs)  
- no GitHub Actions / CD in-repo (local gates under `scripts/*-check.sh`)  
- host-only Chrome CDP (no WASM automation target)

## Related docs

- `docs/COOKBOOK.md` — agent recipes
- `docs/TESTING.md` — how to run gates
- `docs/CROSS_PLATFORM.md` — OS matrix, browser paths, sandboxes
- `docs/HOW_TO_USE.md` — full inventory of 63 commands
- `docs/ARCHITECTURE.pt-BR.md` — Portuguese mirror
- `gaps.md` — `/r-auditoria` catalogue (RES-01…12 closed Pass 27)
- `PRIVACY.md` — local-only data handling
