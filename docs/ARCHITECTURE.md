[English](ARCHITECTURE.md) | [Português Brasileiro](ARCHITECTURE.pt-BR.md)

# Architecture — browser-automation-cli

- One-shot Chrome CDP automation for AI agents
- Lifecycle is always: BORN → EXECUTE → FINALIZE → DIE (single process; no daemon)
- Full agent command list (**69** names): see [docs/HOW_TO_USE.md](HOW_TO_USE.md) and `browser-automation-cli commands --json`

## Layers

| Layer | Path | Role |
|-------|------|------|
| Binary thin | `src/main.rs` | panic hook, `run_from_args`, exit code |
| Lib entry | `src/lib.rs` | `run` / `run_from_args`, tracing_local guard hold, lifecycle |
| CLI surface | `src/cli/` | Clap derive (`Parser` / `Subcommand`); help = agent UX |
| Dispatch | `src/commands/` | PRD handlers (`mod.rs` match + `meta` + `run`) |
| Session | `src/browser/` | One-shot Chrome session, actions, residual ledger hooks |
| Native CDP | `src/native/` | chromiumoxide client, snapshot, heap, cookies, … |
| Contract I/O | `src/output.rs`, `src/envelope.rs`, `src/json_util.rs` | stdout envelopes; BrokenPipe → 141 |
| Lifecycle | `src/lifecycle/` | cancel token, BORN/FINALIZE orchestration, SIGINT/SIGTERM |
| Residual disk/process | `src/residual/` | marker + Chromium tmp Singleton GC; `ResidualDiskReport` |
| Local tracing | `src/tracing_local/` | tracing dual sink (stderr + optional rotated JSON) |
| XDG config | `src/xdg/`, `src/config.rs` | product settings: flags + XDG `config` only |
| i18n | `src/i18n/`, `locales/*.ftl` | `--lang` + XDG `lang` → negotiate → OnceLock; human suggestions only |
| Platform | `src/platform/` | PATH `which_bin`, console UTF-8/VT, HostEnvironment, browser sandbox |
| Windows jobs | `src/win_job.rs` | Job Object residual process kill (stubs on non-Windows) |

## Residual product law (process + disk)

Product law residual-zero covers **both** live Chrome trees and **disk** hygiene after DIE:

1. **Process residual** — ledger-owned Chrome PID (Unix SIGTERM → grace → SIGKILL; Windows Job Object kill-on-close).
2. **Marker residual** — CLI-owned temp profiles under `browser-automation-cli-chrome-*`.
3. **Chromium tmp Singleton residual** — owned `/tmp/org.chromium.Chromium.*` and `/tmp/.org.chromium.Chromium.*` that are Singleton-only (or empty), same uid, with no live process holding the path.

Never kill or wipe **host Flatpak** Chrome trees (for example `com.google.Chrome.*` temp prefixes). Cross-run GC is Singleton-shape + uid + age + no live holder only.

### Role of `src/residual/`

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
- Fields (all ten; a shorter list here used to disagree with the struct):
  - `scanned_roots` — the roots this report actually walked; a zero without them is unfalsifiable
  - `cli_marker_dirs` — count of `browser-automation-cli-chrome-*` under the scanned roots
  - `chromium_tmp_singleton_orphans` — Singleton-only Chromium tmp that looks orphaned
  - `scavenge_safe_candidates` — paths stale GC would wipe now (age ≥ 60s, owned, no live holder)
  - `live_cli_marker_processes` — legacy per-process count; agents MUST NOT require zero
  - `sibling_live_processes` — concurrent invocations; informational, never fails
  - `orphan_marker_dirs` — marker dir past the age floor whose owner pid is dead
  - `foreign_root_orphans` — held marker PROFILES outside the scanned roots
  - `ghost_marker_processes` — live CLI browser whose marker profile dir is gone
  - `process_table_unavailable` — enumeration failed, so every wipe is refused
- Status: `fail` on `orphan_marker_dirs` or `ghost_marker_processes`; `warn` on marker dirs or
  Singleton orphans; else `pass`. A live sibling invocation is healthy and never fails the check.

### How a process is identified as a browser

- Identity comes from the kernel-reported executable path, never from argv.
- argv is written by the process itself; `sysinfo` documents `cmd[0]` as untrustworthy for this.
- The predicate is split by consequence, because the same error costs opposite things:
  - Verdict and reaping are STRICT — an unknown executable is never treated as a browser.
  - Wipe protection is PERMISSIVE — anything that might hold a profile keeps it alive.
- Known blind spot: sandbox wrappers report `bwrap` (Flatpak) or `snap` as the tree root, so the
  strict counts under-report those roots. Under-reporting cannot fail a healthy host or signal an
  innocent process, which is why the trade goes this way.

Local maintainer gates (local maintainer scripts only): `scripts/residual-check.sh`, `scripts/residual-stress.sh`.

## i18n (human suggestions)

Precedence for product docs and agents: **`--lang` → XDG `lang` → OS locale (`sys-locale` + `fluent-langneg`) → default `en`**.

- MVP packs: `en` + `pt-BR` (`UiLocale` / `UiMessage` exhaustive match + FTL parity).
- Machine JSON `error.message` and tracing stay English (agent contract).
- Optional packs: features `i18n-cjk` / `i18n-rtl` / `i18n-europe` / `i18n-full` (scaffold).
- Diagnostics: subcommand `locale` (+ `--json`).
- Man page generation: subcommand `man` (roff via clap_mangen; no Chrome).

Product settings (including language) use **flags + XDG only**. Do not invent or promote product environment variables for durable config.

## Module map (`commands`)

- `mod.rs` — `dispatch` match on `Commands` + browser/session handlers  
- `meta/` — `commands` / `schema` inventory for agents (**69** names via `commands --json`; schema SRP dir)  
- `run/` — multi-step `run` / `exec` script engine (NDJSON steps)

### Dialog multi-tab and settle (v0.1.6)

- **`dialog_map_key`:** pure helper maps open JS dialogs by CDP session identity. Event `session_id` wins; browser-scoped `None` falls back to the active page session id.
- **Page forwarders stamp `Page::session_id`:** so `Page.javascriptDialogOpening` / `Closed` from non-active tabs do not collide with the active tab map entry. Multi-tab isolation is via `Page::session_id` / `dialog_map_key`.
- **`dialog_settled`:** after accept/dismiss, the session waits up to XDG `dialog_settle_ms` for `javascriptDialogClosed`, then returns a compact boolean (agent-first; no invented post-settle wait by consumers). GAP-054.
- **`dialog_settle_ms`:** XDG config key only (`config set dialog_settle_ms`); never a product env var.
- **`tab_switch` domain enable budget:** when switching tabs under a page-modal dialog, domain enable is best-effort under `TAB_SWITCH_DOMAIN_ENABLE_BUDGET_MS` so modal ownership does not hang the switch path.

### Run wait / scrape / select (v0.1.6)

- **`wait_timeout_ms`:** public key on run wait steps (GAP-053); parser honors it (not silent discard).
- **Scrape `format`/`formats` in run:** without HTML monster when only text is requested (GAP-057).
- **Native select:** `pick` / `select-option` dispatch `input` then `change`, report `via: native_select` (GAP-055).
- **`grab` encode:** **png|jpeg|webp** only; AVIF removed (breaking).
- Inventory **69** includes `submit` + `storage` + `image` + `video` + `audio` + `record`; clap product surface is **67** (`pick` / `select-option` are inventory/run multi-step names).

### Lighthouse LHR pure parse (v0.1.6)

- **`scores_from_lhr`:** pure function extracts category scores from Lighthouse Result JSON (0–1 or null audits). Unit fixtures: `scripts/fixtures/lighthouse/minimal_lhr.json` and real sanitized `chrome_captured_lhr.json`. E2e mock path stays SKIP (not a parser PASS claim). GAP-021 partial.
- **GAP-022 residual:** ~53 multi-version dups accepted (cheap prune exhausted).
- **GAP-023/024:** intentional PRD divergences in `parity_intentional_divergences.json`.
- Residual-zero disk law from 0.1.5 still current.
- Product config: flags + XDG only (never product env vars).

## Agent output operations (`agent_ops`)
- `src/agent_ops/` applies eight universal operations over `data` before stdout
- One implementation covers all 69 commands, including the ones nobody wired locally
- Four of the global flags are `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`
- The other four are `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes`
- `--select`, `--filter`, `--limit` and `--sort` are not global flags
- They exist only as per-command flags on `scrape`, `crawl`, `map` and `search`
- `batch-scrape` and the media info verbs also declare that per-command set
- The `-rows` suffix stops clap from handing one value to both projections
- Order of operations: select, resolve rows, filter, sort, dedupe, limit
- Then string truncation, the count-only collapse and the byte ceiling
- `select` runs first because it also disambiguates data holding two arrays
- The envelope gains the `agent_ops` field only when some flag actually ran
- An untouched envelope keeps its exact previous shape for existing consumers
- A flag that ran and resolved cleanly also omits the field, so running one is necessary and not sufficient
- Members are all optional: `total`, `matched`, `truncated`, `omitted_rows`, `unresolved_paths`
- `unresolved_paths` is a list of `{flag, path}` entries
- It names a requested dotted path that no row actually carries
- Without it a mistyped path returns exit 0 and looks like success
- `src/agent_ops/path.rs` has `project()` returning `(Value, Vec<String>)`
- The second element is the set of paths that did not resolve
- `src/agent_ops/filter.rs` has the pure `rows_with_key()` probe
- It runs before dedupe and sort, which would otherwise erase the evidence
- A row operation against data with no single list fails as `Usage`

## Document metadata harvest (`html_meta`)
- `src/scrape_local/html_meta.rs` backs the `metadata` scrape format
- `collect_metadata()` reads the document already parsed by `build_scrape_payload`
- The extra coverage costs one selector pass per field and no new dependency
- Harvested families: Open Graph, Dublin Core, `article:` and Twitter card
- Also harvested: canonical URL, favicon, declared charset and `html_lang`
- Simple `<meta>` names cover keywords, author, language, robots, viewport, generator and theme-color
- Qualified prefixes are keyed as `prefix_name`, without the colon
- Absent fields are omitted rather than emitted as null (CLEAN stdout)
- Favicon tries `icon`, then `shortcut icon`, then `apple-touch-icon`
- `html_lang` comes from the `<html lang>` attribute, not the `language` meta
- `meta_property()` uses a literal selector match for qualified prefixes
- The shared helper adds an implicit `og:` fallback to every lookup
- That fallback would make `dc:title` silently answer with `og:title`
- Literal matching keeps the harvest from reporting fields the page never declared

## Full agent inventory (69)

Discover live: `browser-automation-cli commands --json`

```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract fill-form
find-paths forward goto grab heap hover image video audio keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write storage submit text type
upload version view wait webmcp workflow write
```

Note: `pick` and `select-option` are multi-step inventory names used in `run` scripts; clap product subcommand count is 67.

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
- no remote release orchestration pipelines in-repo (local gates under `scripts/*-check.sh`)  
- host-only Chrome CDP (no WASM automation target)

## Related docs

- `docs/COOKBOOK.md` — agent recipes
- `docs/TESTING.md` — how to run gates
- `docs/CROSS_PLATFORM.md` — OS matrix, browser paths, sandboxes
- `docs/HOW_TO_USE.md` — full inventory of **69** commands
- `docs/ARCHITECTURE.pt-BR.md` — Portuguese mirror
- `gaps.md` — Status v0.1.6 residual DoD + historical 0.1.5 audit catalogue
- `PRIVACY.md` — local-only data handling
