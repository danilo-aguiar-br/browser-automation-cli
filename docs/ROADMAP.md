[English](ROADMAP.md) | [Português Brasileiro](ROADMAP.pt-BR.md)

# Roadmap (maintainer notes)

- This project ships as a stable one-shot CLI
- The roadmap is intentionally short

## Near term (local quality)

- v0.1.8 is the CURRENT release and the lines below describe the live state
- v0.1.8 shipped the anti-detection family and closed gaps G2, G4, G8, G9, G11 and G13
- Live XDG surface: 204 keys documented in `docs/CONFIGURATION.md`
- Live agent inventory: 69 names via `commands --json`
- The inventory includes `submit`, `storage`, `image`, `video`, `audio`, `record`, `locale` and `man`
- Product settings are flags plus XDG `config` only, never product environment variables
- Discover keys with `config list-keys --json` instead of trusting any static list
- Delivered since v0.1.7: `scrape --format attributes` with `--attribute-selector` and `--attribute-name`
- Keep `scripts/*-check.sh` gates green on every audit pass
- Residual-zero disk gates are `scripts/residual-check.sh` and `scripts/residual-stress.sh`
- Optional confidence suite: rerun `dialog_multitab_gate`, `option_pick_gate`, `wait_conditions_gate` and `scrape_step_gate`
- Also rerun the lighthouse unit LHR fixtures after large refactors
- `grab` encode is png, jpeg and webp only, because AVIF was removed
- Grow unit coverage for pure helpers such as `dialog_map_key` and `scores_from_lhr`
- Optional: split large `commands` handler families when a new domain lands

### Anti-detection family (v0.1.8)

- v0.1.8 patches the browser before first navigation under XDG `stealth`, default true
- v0.1.8 added `stealth_profile`, default `auto`, and `stealth_seed`, which has no default
- `stealth_seed` pins the impersonated identity across processes when you set it
- Global flags `--no-stealth`, `--stealth-profile` and `--stealth-seed` override those XDG values
- v0.1.8 added HTTP/2 fingerprint control under `http2_enabled`, default true
- `http2_initial_stream_window_size` defaults to 6291456
- `http2_initial_connection_window_size` defaults to 15663105
- `http2_max_header_list_size` defaults to 262144 and `http2_max_frame_size` to 16384
- `http2_adaptive_window` completes that HTTP/2 fingerprint family
- v0.1.8 added egress proxy keys `proxy_url`, `proxy_bypass`, `proxy_username` and `proxy_password`
- Proxy credentials belong in XDG, because argv is visible in the process table
- `cdp_proxy_bypass_loopback` defaults to true so the CDP control channel survives a proxy
- Global flags `--proxy` and `--proxy-bypass` cover the argv side of the same family
- v0.1.8 added human input kinematics under `input_profile`, default `human`
- `input_move_steps` is 24, `input_move_gap_ms` is 12 and `input_click_dwell_ms` is 65
- `input_key_dwell_ms` is 45, `input_type_delay_ms` is 95 and `input_scroll_tick_px` is 100
- `input_scroll_max_ticks` is 40, `input_target_jitter_px` is 3 and `input_scroll_settle_rounds` is 3
- Global flags `--input-profile` and `--input-seed` override the kinematics per process
- v0.1.8 added `browser_mode`, default `auto`, reachable through XDG and NOT through any flag
- v0.1.8 also added `robots_user_agent`, `scrape_no_cache` and `monitor_diff_max_bytes`, default 65536
- v0.1.8 gave real consumers to `--mitm-max-body-bytes`, `--mitm-no-media-bodies` and `--mitm-redact-secrets`
- v0.1.8 added `--mitm-no-redact-secrets`, the only way to turn secret masking off
- Asking to mask and to unmask at once resolves to masking, because that is the safe reading
- v0.1.8 unified the `scrape` envelope so `--format` arity no longer changes the key set
- `formats` and `format_list` are always present, and `--fields` now projects in both cases
- The single-format `scrape` envelope now reports `stealth`, `http2_profile` and `tls_impersonation`
- It also reports `header_order_controlled`, `fingerprint_stable_across_processes` and `profile_contradicts_host`
- `cookie_jar_persistent` closes that telemetry block, measured 2026-08-10

### History (do NOT read as current state)

- v0.1.6 closed GAP-054 dialog settle and multi-tab, with `dialog_settled` and XDG `dialog_settle_ms`
- v0.1.6 closed GAP-055 native select, GAP-057 scrape format in run and GAP-053 `wait_timeout_ms`
- v0.1.6 kept the residual-zero disk law inherited from v0.1.5
- v0.1.7 closed the ceiling error that `doctor` used to swallow
- v0.1.7 closed the silent unresolved path in agent-ops, now reported as `unresolved_paths`
- v0.1.7 closed the i18n suggestion that cited a flag which does not exist
- v0.1.7 closed the false alias of `rawHtml`, now raw against processed `html`
- v0.1.7 expanded `metadata` past its five original fields
- v0.1.7 closed `--urls-file` accepting input without a ceiling
- v0.1.7 promoted nine XDG keys and documented the whole XDG surface
- v0.1.7 added two new gates to the audit pass

## Intentional residuals (do not claim closed as full parity)

- **GAP-021 partial:** lighthouse parser confidence is unit fixtures (minimal + chrome-captured LHR); e2e mock remains **SKIP** — never claim full e2e lighthouse parser PASS
- **GAP-022 residual dups:** ~53 multi-version dependency duplicates measured; cheap prune exhausted; residual accepted
- **GAP-023 / GAP-024:** PRD wishlist flags/commands remain intentional divergences in `parity_intentional_divergences.json` — not full PRD parity
- **AVIF encode:** removed from `grab` (webp remains); document as intentional breaking residual of 0.1.6
- AVIF decode stays closed by physical limit, not by priority
- HEIC encode stays closed by the same physical limit
- Media extraction that needs obfuscated JavaScript execution stays closed
- Any feature that depends on a remote service stays closed by design
- Anti-detection is best effort, and NO stealth profile guarantees evasion of a given detector

## Open, no committed date
- `scrape` has no `changeTracking` format
- `search` has no temporal filter, and ten dimensions are still missing
- `crawl` include and exclude take no regex, and there is no `regexOnFullURL`
- `parse` does not apply scrape formats to the parsed file
- `crawl` and `batch-scrape` have no `--webhook-url`, which `scrape` already has
- `browser_mode` is reachable only through XDG, because no CLI flag exposes it
- These items carry no date and MUST NOT be read as a promise

## Full agent inventory (69)

Discover live: `browser-automation-cli commands --json`

```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract fill-form
find-paths forward goto grab heap hover image video audio keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr record reload resize run schema scrape screencast
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
