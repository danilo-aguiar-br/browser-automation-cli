[English](ROADMAP.md) | [Português Brasileiro](ROADMAP.pt-BR.md)

# Roadmap (maintainer notes)

- This project ships as a stable one-shot CLI
- The roadmap is intentionally short

## Near term (local quality)

- **v0.1.6 residual DoD achieved** (measured 2026-07-31): GAP-054 dialog settle + multi-tab (`dialog_settled` boolean; XDG `dialog_settle_ms`), GAP-055 native select, GAP-057 scrape format in run, GAP-053 `wait_timeout_ms`, residual-zero disk law from 0.1.5 still current
- v0.1.7 closed the ceiling error that `doctor` used to swallow
- v0.1.7 closed the silent unresolved path in agent-ops, now reported as `unresolved_paths`
- v0.1.7 closed the i18n suggestion that cited a flag which does not exist
- v0.1.7 closed the false alias of `rawHtml`, now raw against processed `html`
- v0.1.7 expanded `metadata` past its five original fields
- v0.1.7 closed `--urls-file` accepting input without a ceiling
- v0.1.7 promoted nine XDG keys and documented the whole XDG surface
- v0.1.7 added two new gates to the audit pass
- Live XDG surface: 176 keys documented in `docs/CONFIGURATION.md`
- Keep `scripts/*-check.sh` gates green on every audit pass (incl. `scripts/residual-check.sh` / `scripts/residual-stress.sh` for residual-zero disk)
- Optional confidence suite: re-run `dialog_multitab_gate`, `option_pick_gate`, `wait_conditions_gate`, `scrape_step_gate`, lighthouse unit LHR fixtures after large refactors
- Live agent inventory: **69** names via `commands --json` (includes `submit`, `storage`, `image`+`video`+`audio`+`record`, `locale`, `man`)
- Product settings: flags + XDG `config` only (no product environment variables); discover keys via `config list-keys --json`
- **`grab` encode:** png|jpeg|webp only; AVIF removed (breaking, keep residual noted)
- Grow unit coverage for pure helpers (filter, JSON, residual ledger, `dialog_map_key`, `scores_from_lhr`)
- Optional: split large `commands` handler families when a new domain lands

## Intentional residuals (do not claim closed as full parity)

- **GAP-021 partial:** lighthouse parser confidence is unit fixtures (minimal + chrome-captured LHR); e2e mock remains **SKIP** — never claim full e2e lighthouse parser PASS
- **GAP-022 residual dups:** ~53 multi-version dependency duplicates measured; cheap prune exhausted; residual accepted
- **GAP-023 / GAP-024:** PRD wishlist flags/commands remain intentional divergences in `parity_intentional_divergences.json` — not full PRD parity
- **AVIF encode:** removed from `grab` (webp remains); document as intentional breaking residual of 0.1.6
- AVIF decode stays closed by physical limit, not by priority
- HEIC encode stays closed by the same physical limit
- Media extraction that needs obfuscated JavaScript execution stays closed
- Any feature that depends on a remote service stays closed by design

## Open, no committed date
- `scrape` has no `attributes` format
- `scrape` has no `changeTracking` format
- `search` has no temporal filter; ten dimensions are still missing
- `crawl` include and exclude take no regex, and there is no `regexOnFullURL`
- `parse` does not apply scrape formats to the parsed file
- `crawl` and `batch-scrape` have no `--webhook-url`, which `scrape` already has
- These items carry no date and MUST NOT be read as a promise

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

Note: `pick` and `select-option` are multi-step inventory names used in `run` scripts; clap product subcommand count is **67** (69 agent names − 2 run-only).

### Local media (image/video) — intentional non-goals (Wave C TREATED)

- **In product now:** path→path `image` / `video` (magic, download SSRF, convert/remux, to-mp3, trim, thumbnail, manifest); optional OS ffmpeg/ffprobe via XDG `ffmpeg_path` (no linked libav).
- **In product now:** `video manifest` summarises HLS `.m3u8` and DASH `.mpd` structure without fetching any media.
- **Not in product (honesty):** adaptive HLS/DASH playback, yt-dlp/site downloaders, pure-Rust production encode, multi-file JoinSet batch media. Agents use external tools or future optional design — do not claim these as shipped.

## Explicitly out of scope

- Daemon / long-lived browser service
- Remote OpenTelemetry / SaaS dashboards
- MCP server embedding
- In-repo remote release orchestration / cargo-dist multi-arch matrix
- HLS/DASH / yt-dlp core / pure-Rust video encode (see Wave C TREATED above)

## Profiling (on demand)

```bash
./scripts/profile-cdp.sh
# or: cargo flamegraph --bin browser-automation-cli -- goto about:blank
```

- Capture artefacts are not committed
- Use them locally to justify micro-opts
