[English](CHANGELOG.md) | [Português Brasileiro](CHANGELOG.pt-BR.md)

# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.8] - 2026-08-10

### Added
- Anti-detection patches applied before the first navigation, on by default, masking the automation markers a real Chrome never exposes: `--no-stealth` opts out for one run, `--stealth-profile` picks the impersonated identity (`auto`, `chrome-linux`, `chrome-win`, `chrome-mac`, with `auto` following the host) and `--stealth-seed` pins that identity across processes. Without a seed each execution draws a new identity, so a crawl of 50 URLs spread over 50 one-shot processes presented itself as 50 different machines. Backed by the XDG keys `stealth`, `stealth_profile` and `stealth_seed`
- `browser_mode` (`auto`, `headed`, `headless`; `auto` resolves to headless and `doctor` reports which one was used) and `--no-xvfb`, which skips the private virtual display on Linux and uses the current one — the only case where that is meaningful is headed mode on Linux
- Egress proxy for both the Chrome engine and the HTTP engine: `--proxy <URL>` accepting `http`, `https` and `socks5`, and `--proxy-bypass <HOSTS>` in Chrome's own bypass-list syntax. The XDG keys `proxy_url` and `proxy_bypass` carry the same values, while `proxy_username` and `proxy_password` exist in XDG only, because argv is visible in the process table. `cdp_proxy_bypass_loopback` defaults to `true` so the CDP control channel survives a proxy that would otherwise swallow it
- HTTP/2 fingerprint control on the shared HTTP client, so the `http` engine stops announcing a settings frame no browser sends: `http2_enabled` (default `true`, because Chrome always offers h2), `http2_initial_stream_window_size` (6291456), `http2_initial_connection_window_size` (15663105), `http2_max_header_list_size` (262144), `http2_max_frame_size` (16384, range 16384..=16777215) and `http2_adaptive_window` (default `false`, because a window that resizes at runtime moves the fingerprint between requests)
- Human input kinematics, on by default: `--input-profile human|direct` interpolates pointer trajectories, holds between press and release and paces typing, and `--input-seed` makes a `human` run reproduce exactly. Ten XDG keys expose the model: `input_profile`, `input_move_steps` (24), `input_move_gap_ms` (12), `input_click_dwell_ms` (65), `input_key_dwell_ms` (45), `input_type_delay_ms` (95), `input_scroll_tick_px` (100), `input_scroll_max_ticks` (40), `input_target_jitter_px` (3) and `input_scroll_settle_rounds` (3)
- `--warmup` visits the origin root before the target URL so the session already carries cookies when the request that matters is made, and `--warmup-url <URL>` warms a different URL instead of that root
- `--expect <EXPR>` asserts that the emitted payload matches `key=value`, `key!=value` or `key~substring`, repeatable and conjoined with AND, so a caller stops re-reading the whole envelope to decide whether the run was useful. `--expect-exit-code` is separate and off by default: turning a data-content mismatch into exit **65** would silently break every existing caller that only branches on transport failure
- `config unset <KEY>`, the inverse of `set`. `config set <key> ""` was never an inverse — for a string key it stored an empty value the normal path never produces, and for a numeric key it was a parse error. Unsetting a key that is already absent succeeds, so a script never has to know the prior state
- `robots_user_agent`, which names the token robots.txt rules are matched against; `scrape_no_cache`, which ignores the response cache on read and always fetches from origin; and `monitor_diff_max_bytes` (65536), a byte ceiling for the `monitor check --diff-mode` payload
- `scripts/config-roundtrip-check.sh`, auto-discovered by `ci-check`, requiring every key of `CONFIG_KEYS` to appear in the writer AND in the reader. Two controls in `scripts/verifier-controls-check.sh` prove the gate accuses each side. The pre-existing `every_declared_key_survives_being_set` iterates three fixed keys and would have caught none of the six keys that were broken
- `tests/phantom_flag_gate.rs`, the 411-line `phantom_flag_scan.py` ported to Rust and deleted, covering 242 declared flags with the same three properties. The universe floor rose from 20 to 200, because 20 is satisfied by a walk that fails on every subcommand. `scripts/phantom-flag-gate.sh` is the adapter that keeps the control runner pointed at a gate that still exists — moving a check without moving its control is how a gate becomes a rubber stamp

### Changed
- The XDG surface grew from **176** keys to **204**, in five families: anti-detection, window mode, egress proxy, HTTP/2 fingerprint and human input kinematics, plus `robots_user_agent`, `scrape_no_cache` and `monitor_diff_max_bytes`. The agent inventory stays at **69** commands
- `src/xdg/config_write_optional.rs` extracted: the six repaired keys pushed `config_write.rs` to 301 lines against the 300 limit. The boundary is semantic rather than arithmetic — rendering the template and appending what the template cannot express are different jobs. The round-trip gate scans both files, otherwise it would report every optional key as absent
- `json_escape` in `scripts/docs-check.sh` now uses `jaq -R -c .`. Measured: `bash scripts/ci-check.sh` FAILED on a host without `python3`, through two chains — `agent-ops-check.sh:152` reaching `phantom_flag_scan.py`, and `docs-check.sh:39` running `python3 -c` under `set -euo pipefail`. Both chains are gone; five `.py` scripts totalling 1030 lines and inline `python3` in seven shells remain, none of which breaks `ci-check`, and all of which stay as named debt against the rust-native rule
- `tests/v018_parity_gate.rs` resolves the binary with `env!("CARGO_BIN_EXE_browser-automation-cli")` instead of the hard-coded `target/debug/browser-automation-cli`. The skip path used `eprintln!`, which libtest swallows, so all 14 tests would have reported `ok` while measuring nothing. Eight other gates inherit the fragile pattern and stay as named debt
- Two comments that contradicted the code they described: `scrape_view.rs` claimed "BODY SIGNALS ONLY" while the call already passed body, URL and title, and `Cargo.toml` framed text recognition as a dependency deferred by MSRV when the product rule forbids it permanently — the comment even cited `src/image_local/ocr_rs.rs`, a file that does not exist

### Fixed
- Six declared XDG keys accepted `config set` with `ok:true` and never persisted: `proxy_url`, `proxy_bypass`, `proxy_username`, `proxy_password`, `stealth_seed` and `robots_user_agent`. The next process read `null`. Two of them are credentials that `docs/CONFIGURATION.md:204` tells the operator to keep in XDG precisely because argv leaks, so the channel documented as safe discarded the value and left only the channel that leaks. Repaired on both the writer and the reader side, with `proxy_password` added to the zeroize sweeps in `secrets.rs`. The 2026-08-09 round had fixed 17 keys without fixing the mechanism: `write_config` followed a hand-written template and `apply_toml_kv` a literal match, with no single source of truth between them
- `scrape` answered in two different shapes depending on how many values `--format` carried. Measured on 2026-08-10 against the same URL: `--format markdown` returned twenty top-level keys, content plus the whole diagnosis (`status_code`, `http_error`, `cache_hit`, `robots_policy`, `charset`, `http_version`, `stealth`, `tls_impersonation`, `http2_profile`, `header_order_controlled`, `change_status`), while `--format markdown,links` returned four (`engine`, `format_list`, `formats`, `source_url`) with every diagnostic field gone. Asking for MORE data returned LESS, and `--fields markdown` worked in the first case but returned an empty `data` with `ok: true`, no `agent_ops` and exit **0** in the second — a silent wrong answer. The shape is now the union: `formats` and `format_list` are always present, each format is also mirrored at the top level, and a transport field wins over a derived one of the same name so the field keeps meaning "what came back on the wire"
- `--mitm-max-body-bytes`, `--mitm-no-media-bodies` and `--mitm-redact-secrets` were declared on the CLI and read by nobody: `--help` promised a body ceiling, a media filter and a redaction switch, and the capture applied none of them. Redaction did happen, but by accident of call site — every caller passed `true` literally, so the flag could neither turn it on nor off. `src/mitm_local/policy.rs` now publishes the policy once from CLI dispatch, with a default ceiling of 65536 bytes per body, and resolves the contradiction of asking for masking and asking to turn it off by masking

## [0.1.7] - 2026-08-04

### Added
- Universal agent data operations on the success envelope, applied to `data` before it reaches stdout and therefore covering all **69** commands with one implementation: `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes`. The four row-scoped names carry the `-rows` suffix because `--select`, `--filter`, `--limit` and `--sort` were already taken as per-command flags on `scrape`, `crawl`, `map`, `search`, `batch-scrape` and the media `info` verbs; promoting those spellings to the global scope would have collided with 32 existing declarations. Previously only 8 of 69 commands offered any of these and they disagreed — `crawl` had eight, `scrape` had one, `doctor` had none. Measured on `doctor --offline --quick`: 26_277 bytes → **80** bytes with `--fields residual.ghost_marker_processes`. The envelope gains `agent_ops` (`total`, `matched`, `truncated`, `omitted_rows`) only when a flag ran, so untouched envelopes keep their exact previous shape
- `scripts/natives-check.sh` Pass N pins the `*-sys` / native-crate allowlist and forbids `openssl` and `nasm-rs`; it also fires when `aws-lc-sys` LEAVES, so `cmake` gets retired from the documented prerequisites instead of outliving its cause. Four new controls in `scripts/verifier-controls-check.sh` prove the gate detects each mutation
- `docs/CONFIGURATION.md` and `docs/CONFIGURATION.pt-BR.md`: the complete XDG reference, all **176** keys with default and purpose. **132** of them appeared in no public document, so the only way to learn the surface was `config list-keys --json` — serviceable for an agent, invisible to a human comparing the product against alternatives
- `scripts/doc-coverage-check.sh`: reads the LIVE binary for keys and commands and fails when the prose drifts from either. `scripts/docs-check.sh` validates rustdoc and never opens README; `scripts/inventory-flat-check.sh` pins the command COUNT without checking that each name is documented anywhere, and says nothing at all about configuration keys. The flag-scope assertion is deliberately scope-aware: a naive "does this flag exist" check would pass on `--select`, because it exists on `scrape`
- `PRIVACY.pt-BR.md`: the policy existed in English only and was the one root document with no bilingual mirror
- `agent_ops.unresolved_paths`, which names the flag and the path exactly as the caller typed them whenever a requested key resolves on no row. A bare count would not have been actionable
- `scripts/agent-ops-check.sh` and `tests/agent_ops_cli.rs`: ten assertions driven through argv against the compiled binary. Integration coverage of the eight envelope flags was previously zero — the only `--fields` match under `tests/` was `--fields-json` from `fill-form`
- Nine XDG keys promoted out of source literals: `max_urls_file_bytes`, `run_max_include_depth`, `mitm_rebind_attempts`, `network_idle_window_ms`, `dom_stable_window_ms`, `chrome_default_timeout_ms`, `drag_move_steps`, `drag_move_gap_ms`, `robots_fetch_timeout_secs`. The two wait budgets are the most user-visible in the product, and the Lightpanda engine already had a session-timeout key while the Chrome engine had none
- `scrape --format metadata` now harvests Open Graph, Dublin Core, `article:`, Twitter card, canonical, favicon, charset and `html_lang`. It emitted five fixed fields while those tags sat in the same parsed document and were discarded, so a page with no author and no publish date was indistinguishable from a page the CLI never inspected. Qualified prefixes use a literal selector match, because the shared helper adds an implicit `og:` fallback that would make `dc:title` silently answer with `og:title`
- AVIF encode via `ravif` with `default-features = false` (feature `image-avif`), keeping `rav1e/asm`, `nasm-rs` and `cc` out of the tree
- HEIC decode via `heif-oxide` over `rust_h265` (feature `image-heic`), pure Rust with zero C
- SVG sanitise and rasterise via `resvg` and `tiny-skia` (feature `image-svg`)
- SIMD resize via `fast_image_resize` (feature `image-simd-resize`, on by default)
- GIF multi-frame extraction and reassembly, retiring the `frame_count: 1` placeholder
- IPTC IIM and XMP reading, written from scratch over `quick-xml`: no pure-Rust crate exposes them and `xmp_toolkit` is FFI to Adobe's C++ SDK
- HLS and DASH manifest parsing via `m3u8-rs` and `dash-mpd` (feature `media-manifest`, on by default)
- `video manifest`, which summarises an HLS `.m3u8` or DASH `.mpd` without fetching a single media segment. `video` now exposes 7 actions: `info`, `download`, `convert`, `to-mp3`, `trim`, `thumbnail`, `manifest`
- `source_hash` in the `version` envelope, so an agent can pin the exact source tree behind a binary instead of trusting the version string alone
- `scrape --format feed` for RSS, Atom and JSON Feed via `feed-rs`
- `crawl --follow-rel-next` for `rel=next` pagination, bounded by the existing limit, depth, robots and politeness rules
- `crawl`/`batch-scrape --dedup-similar`, a from-scratch SimHash that collapses near-identical content rather than identical URLs, reporting how many pages were collapsed
- XDG key `chrome_startup_timeout_secs`, defaulting to 20 to match chromiumoxide's `LAUNCH_TIMEOUT`
- `tests/fuzz_magic_parsers_gate.rs`: deterministic fuzzing of every magic parser over a xorshift corpus of 15 real container prefixes, truncated and bit-flipped. Replaces a `cargo fuzz` recipe that had been in `docs/TESTING.md` since auditoria-04 without a `fuzz/` directory ever existing — it needed nightly, needed libFuzzer from LLVM in a rust-native crate, and no gate invoked it
- `scripts/lib/rust-regions.sh`: shared `#[cfg(test)]` span detection for verifiers. Spanning to end-of-file was wrong twice — `mod tests;` without a body declares the tests elsewhere, and Rust allows items after the test module
- Residual scrape agent-native CLEAN STDOUT (wave 04): fix `--filter http_error=false` on OK pages; multi-format `--select` promotes nested fields; `build_formats_map` propagates selectors/redact/hash; format `json` real LLM extract via XDG OpenRouter; `--header` / browser `--wait-ms`; map `--sitemap-only`; `change_status` (fresh|unchanged) + content_hash; URL-normalize dedup trailing slash; gate expanded (10 tests); schemas residual flags; orphan `src/src` removed
- Residual scrape agent-native CLEAN STDOUT (wave 03): crawl multi-format; `--include-selector`/`--exclude-selector`; formats `jsonld`/`json`; `--redact-pii`; `--with-content-hash`; batch/crawl `--output-mode csv`; `--sort`/`--dedup-key`; map `--search`; crawl `--ignore-query-params`; default scrape engine `http`; politeness delay jitter (XDG `scrape_delay_jitter_ratio`); XDG keys `scrape_default_engine`, `scrape_summary_chars`, `scrape_sitemap_max_bytes`, `scrape_charset_peek_bytes`
- Residual scrape local scraping agent-native (CLEAN STDOUT): `--select`, `--max-text-chars` on scrape/batch-scrape/crawl/map/search; `--filter` / `--output-mode ndjson` on batch/crawl; `--include-path` / `--exclude-path` / `--use-sitemap` on crawl/map; batch multi-format CSV; format `images`
- Politeness: Crawl-delay honor (`robots/politeness.rs`) + XDG `scrape_min_delay_ms`; encoding_rs charset pipeline; meta/X-Robots noindex; nofollow skip; HTTP 4xx/5xx structured (`http_error`)
- XDG keys: `scrape_max_text_chars`, `scrape_min_delay_ms`, `scrape_honor_meta_robots`, `scrape_honor_nofollow`, `scrape_use_sitemap`
- i18n EN+PT: `http_status_scrape`, `meta_robots_noindex`; gate `tests/scrape_agent_native_gate.rs`
- WAVE-C TREATED honesty: CAPTCHA/proxy/agent SaaS/async jobs not in product; feed/ETag deferred TREATED
- Local audio pipeline (no Chrome): `audio info|download|convert|trim` (magic-first; ffprobe/ffmpeg optional via XDG `ffmpeg_path`; path→path; agent JSON only; no PCM/base64 stdout)
- XDG keys: `audio_max_input_bytes`, `audio_download_max_bytes`, `audio_default_format`, `audio_default_bitrate`
- Schema: `docs/schemas/audio.schema.json`; concurrency matrix `audio` = `sequential_justified`
- Inventory agent surface: **68** names via `commands --json` (adds `audio`); recipe download→convert→`upload`
- i18n EN+PT: `audio_too_large`, `audio_magic_invalid`, `audio_format_unsupported`, `audio_lossy_transcode`
- Integration `tests/audio_local_gate.rs`; inventory flat gate EXPECTED=68 + has_audio (renamed to `scripts/inventory-flat-check.sh`; `scripts/verify-inventory-flat.sh` kept as shim so `scripts/ci-check.sh` glob `scripts/*-check.sh` finally discovers it)
- Local video pipeline (no Chrome): `video info|download|convert|to-mp3|trim|thumbnail` (magic-first; ffprobe/ffmpeg optional via XDG `ffmpeg_path`; path→path; agent JSON only)
- Residual discovery/docs Locale-Parity (auditoria-04): flat lists/Utils/HOW_TO/README inventário **67** + `video`; clap tip **65**; schemas README `video.schema.json`; run INTENTIONAL_RUN_EXCLUDE video
- Residual auditoria-05: `video --select` agent aliases (`format`/`bytes`/`path`); compact ffmpeg error messages; `run` unknown cmd uses INTENTIONAL_RUN_EXCLUDE reasons; ROADMAP Wave C honesty + inventory `video`; skills formulas image/video; TESTING.pt-BR inventário 67
- Residual auditoria-06: flat inventory blocks **67** + `video` (TESTING/MIGRATION/CROSS); MIGRATION jaq/timeline **67**; schema `--select` aliases; magic open Permission denied suggestion (i18n input+output); AGENTS.pt-BR + COOKBOOK local-IO video
- Residual auditoria-07: MIGRATION.pt-BR inventory heading `+ video`; schema per-action `--select` aliases; magic read uses `io_open_err`; gaps naming image-06 vs execucao-06
- Residual auditoria-08: `scripts/verify-inventory-flat.sh` local gate (67+image+video); hash/stat I/O uses `io_open_err` suggestion; image backlog hard-TREATED; TESTING pointer
- Residual auditoria-09: FTL/enum PT media parity; FS path I/O uses `io_path_err` suggestion (stat/mkdir/rename/stdin); verify script README.pt-BR; gaps mid-08 hygiene
- Residual auditoria-10: image path FS uses `io_path_err` suggestion (parity video); pt_br media indent hygiene; unit coverage mkdir/rename/open
- Residual auditoria: schema Wave B (trim/thumbnail/`no_faststart`), filesize SRP split (`ffmpeg_ops`/`ops`/`resolve_media`/`set_media`), `ffmpeg_io_failed` i18n, integration `tests/video_local_gate.rs`, soft tip **67** Locale-Parity
- Smart convert: stream-copy when muxable; auto re-encode when copy incompatible (e.g. H.264→WebM) with honesty fields `auto_reencoded` / `reencode_reason`
- Atomic ffmpeg outputs (`.ba-partial.<ext>` → rename); fail cleans residual; dedicated `ffmpeg_failed` i18n suggestion
- Faststart default for MP4-family (`--no-faststart` opt-out); doctor reports optional `ffprobe`
- XDG keys: `video_max_input_bytes`, `video_download_max_bytes`, `video_default_container`, `video_default_crf`, `video_default_audio_bitrate`
- Schema: `docs/schemas/video.schema.json`; concurrency matrix `video` = `sequential_justified`
- Shared agent projection helper `json_util::project_fields` (DRY image+video)
- Local image pipeline (no Chrome): `image info|convert|resize|download|exif`
- Pure-Rust EXIF via `kamadak-exif` (GPS omitted by default; `--include-gps`)
- `image download` over the shared SSRF-guarded HTTP path, bounded by `image_download_max_bytes`
- XDG keys: `image_max_input_bytes`, `image_max_pixels`, `image_default_format`, `image_default_quality`, `image_download_max_bytes`
- Magic-byte format probe (png/jpeg/webp/gif; AVIF/HEIC detect-and-reject)
- Agent projection: `image info --select` CSV fields; `image convert --strip-exif` / `--keep-exif`
- `grab --include-base64` opt-in (default off; key omitted from JSON when off)
- Unit tests: `image_local` (**17**) for magic, limits, convert, resize, atomic, select, SSRF, EXIF APP1, webp quality honesty, select aliases, magic-first path

### Changed
- `scripts/verify-inventory-flat.sh` became `scripts/inventory-flat-check.sh`. `scripts/ci-check.sh` discovers verifiers with the glob `scripts/*-check.sh`, which the old name never matched, so the gate never ran in the bundle. The old path is kept as a delegating shim
- The inventory gate now also covers `docs/HOW_TO_USE.md`, `docs/HOW_TO_USE.pt-BR.md` and `docs/schemas/README.md`, and asserts the clap surface as well as the agent inventory
- `scripts/schema-drift-check.sh` wires the generator's long-existing `--check` into the bundle, closing a drift of 8 schemas in 68
- `scripts/filesize-check.sh` discounts inline `#[cfg(test)] mod tests`; it was demanding that production code shrink to make room for table-driven tests
- `scripts/ci-check.sh` writes a citable artifact to `target/gates/ci-check.txt`, so a close can cite an execution instead of prose
- `scripts/network-check.sh`, `scripts/json-ndjson-check.sh` and `scripts/natives-check.sh` stopped flagging test code as production. The network gate was failing on the very test that proves the product rejects a `0.0.0.0` bind
- `#![recursion_limit = "256"]` on the crate root; the XDG key catalog crossed `serde_json::json!`'s default expansion ceiling
- Inventory agent surface: **68** names via `commands --json` (adds `image`, `video`, `audio`); clap product **66**
- `ScreenshotResult.base64` is `Option<String>` (None by default)
- Human image line includes `w=`/`h=` when present
- `gaps.md` versioned living inventory (image closed/open + auditoria-02 + auditoria-03 residual)
- Docs/skills/CLAUDE inventory honesty: flat lists + Local IO include `image`+`video`; clap product surface **65**; schemas `image.schema.json`+`video.schema.json`
- Docs honesty residual: MIGRATION timeline 65→66 Unreleased `image`; PT 0.1.5 as-of 63; CONTRIBUTING/INTEGRATIONS tip Unreleased (not “0.1.6=66”); CLAUDE image playbook; rustdoc `ImageSource` link fixed (docs-check PASS)
- Agent-honest convert envelope: `quality_applied`, `keep_exif_honored` (local webp lossless per image 0.25 docs.rs)
- Doctor budget matrix includes `image` (sequential_justified single-file)
- `image exif --select` field projection (DRY with info)
- COOKBOOK recipes for local image pipeline (agent-native, no pixel base64 default)

### Fixed
- Residual process identity now comes from the kernel-reported executable, never from argv. A shell script carrying `--user-data-dir=<marker> --type=renderer` satisfied the old substring classifier, so `ghost_marker_processes` reported it and `doctor --offline --quick` exited **1** on a host with no Chrome running; the same classification also put an unrelated pid in front of the reaper. `sysinfo` documents `cmd[0]` as untrustworthy for exactly this reason. The predicate is now split by consequence: verdict and reaping are strict (unknown executable is never a browser), wipe protection stays permissive (anything that might hold a profile keeps it alive)
- `reconcile` no longer signals a process it cannot identify: `browsers_pinning` stays unfiltered because the reparenting proof reads it as tree topology (a Flatpak root is `bwrap`, not a browser, and filtering it out would re-read its children as orphaned roots), while the new `browsers_reapable` gates the kill. When any holder is unidentified the pass declines the whole directory — killing the identified subset and wiping anyway would have manufactured a `ghost_marker_processes` of its own
- `foreign_root_orphans` counts marker PROFILES, not processes. It used to count entries, so one invocation with renderer, GPU and utility children reported three orphans for one directory — the same Chrome-subprocess inflation already fixed twice elsewhere in the module
- `residual::proc` enumerates with `without_tasks()` instead of collecting every Linux thread and discarding it afterwards; the index is built during BORN on every invocation, so that cost was charged to every run
- The `cargo fmt` diff left by the `residual-honesty-04` wave is cleared, and the wave's own smoke (`cargo test --lib residual::` plus one integration test) is replaced by the canonical bundle — a literal recurrence of `NC-GATE-BUNDLE-NUNCA-RODOU`
- Residual field list and status ladder corrected in **16** documentation sites that still described four fields and a `fail` rule retired by GAP-002/GAP-006 (ARCHITECTURE, COOKBOOK, HOW_TO_USE, README, INTEGRATIONS, llms-full; EN+PT). `MIGRATION` keeps the 0.1.5 text as historical record and gains a tip annotation instead
- The `Cargo.toml` note claiming `cc`/`cmake` reach the graph "only via the pre-existing TLS stack" was measured wrong: `libsqlite3-sys` (bundled), `libmimalloc-sys` and `zstd-sys` compile C too — five units, not two. Removing `cmake` was attempted and reverted with the measurement recorded: `reqwest/rustls-no-provider` does not drop `aws-lc-sys`, because `hudsucker` declares `tokio-rustls` without `default-features = false` and Cargo feature unification is additive. hudsucker 0.25.0 is the newest release, so `cmake` stays a documented prerequisite rather than an undocumented surprise
- `scripts/inventory-flat-check.sh` no longer false-green: `STALE_COUNT=EXPECTED-1` (68), requires live `record`, README `**69**`+`record`, anti-stale targets include README/ARCHITECTURE/llms and secondary tip docs
- Residual honesty-02: skills EN+PT `all/estes 69`+`record`; CONTRIBUTING/INTEGRATIONS tip **69**+`record`; llms* flat 69 unique (no `record, record`); gate phrase-family + flat uniqueness
- Residual honesty-03: TESTING EN+PT bare inventory notes **69**+`record`; MIGRATION jaq comment/timeline/tip paren include `record`→69; gate bare-phrase `(inventory N)` / `commands --json` (N) + skills set-equality vs live
- Residual honesty-04: agent residual contract aligns with doctor — fail on `orphan_marker_dirs` + `ghost_marker_processes` (live CLI Chrome with missing marker dir); skills/AGENTS/TESTING stop requiring zero `live_cli_marker_processes`; `sibling_live_processes` documented as healthy concurrency; TESTING documents `RUST_MIN_STACK` for clap-tree stack overflow
- `doctor` reported success for a payload it never emitted. `src/doctor/run.rs` discarded the over-budget error in `Err(_) => {}` because `run_doctor` returns `i32` rather than `Result`, so `--max-output-bytes 1000`, `4000` and `10000` each returned exit **0** with an empty stdout and an empty stderr — which an agent reads as "the host is healthy". Seven other commands already returned exit 2 on the same input; `doctor` is the one agents use to validate residual-zero, and the silent band reached roughly 20000 bytes, covering every operationally plausible value
- A requested path that resolves on no row is no longer indistinguishable from success. `--fields NAO.EXISTE` returned `{"ok":true,"data":{}}`; `--sort-rows` with an absent key fell into `(None, None) => Ordering::Equal` and, because `sort_by` is stable, produced a perfect no-op reported as `matched == total`; `--dedupe-by` with an absent key reported every row as unique. All three now report `unresolved_paths`, while a path that does resolve keeps the envelope byte-identical
- The three `agent-ops-*` recovery messages suggested `--select`, which is not a global flag. On 61 of the 69 commands, following the advice produced `error: unexpected argument '--select' found`, so the message meant to recover from one error produced a second
- `scrape --format rawHtml` returns raw HTML under the `rawHtml` key. The alias collapsed into `ScrapeFormat::Html`, so a caller asking for raw received the body after main-content extraction and selector filtering, under the `html` key. The `"rawHtml"` match arm was also unreachable: it sat after `to_ascii_lowercase()`
- `batch-scrape --urls-file` had no size ceiling and was the only reader in the product without one, over user-controlled input. It now checks file metadata first, like every sibling reader already did, against the new `max_urls_file_bytes`
- `verify_image_magic` read the whole file to inspect its first bytes; `IMAGE_MAGIC_PROBE_BYTES` existed for exactly this purpose and was never used
- `scrape_local::emit` cloned the entire result array in order to iterate it, doubling peak memory on a crawl of hundreds of markdown pages, and carried a dead conditional whose two branches were identical
- `src/output.rs` acquired the stdout lock and flushed once per line, so an NDJSON batch of N items cost N lock acquisitions and N syscalls. Per-line flushing protects long-lived streaming; this CLI is one-shot and emits in batches, so batch emission now takes one lock and one flush
- `batch-scrape` published a `concurrency_budget` it never spent: the loop is serial by construction on a single CDP session, so the envelope advertised parallelism beside a note stating it is sequential. It now reports the effective value
- The browser engine matched robots rules under a different identity than the HTTP engine. `nav.rs` passed a bare product-name literal while the HTTP path passed the versioned `HTTP_USER_AGENT`, so the same site could be allowed on one engine and denied on the other
- BORN reconciliation now reaps orphans it could only *report* before. The reaper listed profiles under `residual_scan_roots()`, which derives from `XDG_CACHE_HOME`, while `foreign_root_orphans` already found them by reading command lines. Detection and action had different scopes, so a tree from an earlier build survived every invocation. It now works over the union of both views
- Legacy profiles carrying no owner-pid marker are collectable again. Requiring the marker failed closed, leaving every pre-GAP-052 profile permanently pinned. When the marker is absent the proof is read from the kernel instead: the tree root's parent must not be a live CLI of this product, and the age floor is multiplied tenfold because the substitute replaces the weaker half of the proof
- `ERROR chromiumoxide::handler: WS Connection error` no longer appears on a successful run. `Browser.close` drops the socket without a handshake and chromiumoxide logs that from inside its handler; FINALIZE now stops the event pump first, so the reset is never observed rather than observed and filtered
- `run --script <(…)` explains itself. Shell process substitution hands over a path under `/proc/<pid>/fd/<n>`, which no allowed root can contain; the refusal now points at `run --script -`, which reads NDJSON steps from stdin
- One-shot DIE now holds under hard kill. Chrome is spawned by the product instead of by chromiumoxide, with `PR_SET_PDEATHSIG` plus `setpgid(0, 0)` on Linux, a Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` on Windows, and a `kqueue` `NOTE_EXIT` watchdog on macOS. `SIGKILL` on the CLI now takes the browser down through the kernel, not through `Drop`
- `BrowserProcess` gained a `Chrome` variant, so `chrome_pid()` stops returning `None` on the Chrome path. FINALIZE reaches `residual_kill_child` for the first time; previously only chromiumoxide's `kill_on_drop` did any reaping, and `panic = "abort"` bypassed even that
- Residual kill escalates from a single pid to the process group. `kill(-pgid, …)` reaches zygote, GPU, network and renderer children, with a `sysinfo` parent-child walk as fallback when the pgid is unavailable
- `doctor` emits `residual.scanned_roots[]`. Residual-zero was silently relative to the caller's `XDG_CACHE_HOME`: the same binary on the same host reported `cli_marker_dirs: 0` from one shell and `2` from another
- `doctor` emits `foreign_root_orphans`, which counts marker-holding browsers whose profile sits outside every scanned root — residue every other field was blind to
- `live_cli_marker_processes` no longer inflates by the thread count. `sysinfo` enumerates `/proc/<pid>/task`, so each Chrome thread arrived as its own process; the reported figure was 382 on a host with 22. Threads are now filtered via `Process::thread_kind()`, a no-op outside Linux
- `cargo test` no longer aborts with SIGABRT. Building the 68-subcommand clap tree overflowed the 2 MiB test-thread stack, so the whole suite was unrunnable and gates were only ever invoked per-module — which is how a global `--lang` collision survived ten audits
- `std::thread::sleep` on an async path in `src/browser/support.rs` no longer blocks a Tokio worker
- Residual-audio-03 (agent-native honesty): AGENTS/CONTRIBUTING inventário **68**+`audio`; convert/trim (+video) omit null Option keys; media max write uses DEFAULT_* (not 0); full_dump omits JSON null; libvorbis uses `-q:a` (8 kHz ogg); verify-flat checks AGENTS
- BUG-IMG-001: `grab --format webp` default path now uses `.webp` extension
- BUG-IMG-002: QR decode is magic-first (no longer trusts file extension)
- BUG-IMG-003: screenshot save is atomic (tmp + fsync + rename)
- BUG-IMG-004: drop retained CDP base64 after disk write (agent-native; no pixel dump)
- BUG-AUD-001/002: `image` registered in agent `COMMANDS` inventory + categories (schema discoverable)
- BUG-AUD-003: clippy clean on image pipeline (`-D warnings`)
- grab lossy quality applied for `webp` as well as `jpeg`

### Removed
- The `image ocr` action. The agent that consumes this CLI reads images natively, so an OCR pass in the middle only spent tokens restating what the caller could already see
- OCR was also the one path that dragged an external C binary — `tesseract` — into a tool whose whole premise is rust-native and self-contained
- The XDG keys `ocr_engine`, `ocr_lang` and `tesseract_path` went with it. A legacy `config.toml` still carrying them loads without error, because the config model is `#[serde(default)]` and never sets `deny_unknown_fields`
- `image` now exposes 5 actions: `info`, `convert`, `resize`, `download`, `exif`

### Documentation
- Inventory honesty tip: **69** plus `record` across README, ARCHITECTURE, HOW_TO_USE, AGENTS, schemas, llms, TESTING and COOKBOOK (EN+PT); clap product surface **67**; README current version **0.1.7**
- The XDG configuration surface is documented for the first time. Public prose described 44 of 176 keys and pointed readers at `config list-keys --json` for the rest
- `CLAUDE.md` stopped requiring zero `live_cli_marker_processes`, which `docs/AGENTS.md` and both skills already contradicted. The field is legacy and counts Chrome child processes, so a healthy concurrent run inflates it
- Every public document now has a `.pt-BR` mirror, and every link in `llms.txt`, `llms.pt-BR.txt`, `llms-full.txt` and `llms-full.pt-BR.txt` resolves to a file that exists
- Inventory corrected from **67** to **68** and the clap product surface from **65** to **66** across eighteen files, EN and pt-BR. Historical statements were left intact; only live claims were changed
- The `CHANGELOG` no longer carries two consecutive `### Added` sections under one `## [Unreleased]`

## [0.1.6] - 2026-07-31

### Added
- `dialog_settled` boolean on dialog accept/dismiss happy path (GAP-054); agent-native compact signal — no artificial wait after settle
- XDG key `dialog_settle_ms` (max wait after JS dialog answer for `Page.javascriptDialogClosed`)
- Multi-tab dialog isolation: page forwarders stamp `Page::session_id`; pure helper `dialog_map_key`; unit coverage (2 session ids, fallback, empty, map isolation)
- Gate `tests/dialog_multitab_gate.rs` (isolation tab1 + accept owner; `tab_switch` best-effort domain enable under open dialog with budget)
- Lighthouse fixture `chrome_captured_lhr.json` (Lighthouse 13.4.1 real, sanitized) + unit `scores_from_lhr` with minimal + chrome-captured fixtures (GAP-021 partial↑)
- Run step field `wait_timeout_ms` honored for wait steps (GAP-053)
- Scrape `format`/`formats` in multi-step `run` via shared `build_formats_map` (GAP-057)
- Native select events DRY: `DISPATCH_INPUT_AND_CHANGE` shared by pick + fill-form select (GAP-055)
- Inventory honesty: **65** agent command names via `commands --json` (includes `submit`, `storage`, `select-option`, `pick`, `locale`, `man`, …)

### Fixed
- GAP-054: suppress Opening + listener `Page.javascriptDialogClosed` (browser+page); dialog settle under load 20/20
- GAP-055: native `input`+`change` for option pick / select
- GAP-050: doctor production path without `.unwrap()`
- Multi-tab dialog: session_id stamping so non-active tab dialogs map correctly
- `tab_switch` under open page-modal dialog: best-effort `Page.enable` domains with timeout budget, cached url/title

### Changed
- Version `0.1.6`
- **Breaking (encode):** `grab --format` is `png|jpeg|webp` only — AVIF encode removed (crate `image` without avif/core2 yanked chain)
- GAP-022 dependency duplicates ~53 multi-version residual accepted (lopdf/hudsucker/human-panic/criterion/tungstenite) — measured, cheap prune exhausted
- GAP-023/024 PRD wishlist flags/commands remain intentional divergences (`parity_intentional_divergences.json`) — do not claim full PRD parity
- GAP-052 residual/doctor path `contains` typed via cmdline markers (intentional process classification)
- e2e placar: TOTAL=53 PASS=52 FAIL=0 SKIP=1 (lighthouse mock SKIP honest, never PASS of parser)

### Documentation
- Public bilingual docs synchronized to 0.1.6 (this release)
- Skills EN/PT operational playbooks for `dialog_settled`, grab formats, XDG `dialog_settle_ms`, full command surface
- `gaps.md` Status v0.1.6 placar + historical 0.1.5 archive disclaimer

## [0.1.5] - 2026-07-19

### Added
- Automatic **BORN** cross-run GC of stale Chromium Singleton-only temp dirs (`scavenge_stale_singleton_orphans`) so prior one-shot runs cannot accumulate `/tmp/org.chromium.Chromium.*` litter (PRD §5N residual-zero disk)
- `ResidualDiskReport` + `doctor` check `residual_disk` / top-level JSON `residual` (path-light; no Chrome launch for the report itself)
- Public residual constants (marker prefix, age floor, size caps) — anti-hardcode
- Local gates: `scripts/residual-check.sh`, `scripts/residual-stress.sh` (no CI/GHA)
- Integration coverage: Singleton side-channel non-growth, BORN fixture wipe, doctor residual fields

### Fixed
- **RES-01:** `Lifecycle::finalize` now copies `chrome_pid` **before** `.take()` so invocation-window scavenge can attribute side-channels
- **RES-02/RES-10:** cross-run GC by Singleton-only shape + uid + no live `/proc` holder (not weak path_references alone)
- **RES-05:** FINALIZE re-discovers side-channels before wipe
- Host Flatpak `com.google.Chrome.*` temp prefixes are **never** deleted by stale GC

### Changed
- Version `0.1.5`
- FINALIZE dual scavenge: invocation-window + stale Singleton GC
- Product law residual-zero extended from process/marker to Chromium tmp disk hygiene


## [0.1.4] - 2026-07-18

### Added
- `run --json-steps` (global `--json-steps`): stream one NDJSON line per step (`step`, `cmd`, `ok`, `result`) for agent-first observability (GAP-020)
- `wait` supports CSS multi-selector OR (`#a, #b`), `selectors` arrays, `url` / `url_contains` / `navigation` (GAP-019, GAP-024)
- `select-option` / `pick` multi-step cmds for HIG badge/popover / `role=option` (GAP-023)
- Assert kinds `console_empty` and `console_no_match` (GAP-025)
- `schema <cmd>` positional in addition to `schema --cmd` (GAP-022)
- `BeforeUnloadAction` accept|dismiss on `goto` / `reload` (GAP-003)
- MITM `capture-url` one-shot compose + global `--mitm*` flags (GAP-011)
- `print-pdf` in multi-step `run` + run inventory gate (GAP-001, GAP-017)
- Scrape multi-format and batch/crawl `--engine browser` (GAP-009, GAP-010)

### Fixed
- `console dump` always writes a valid JSON array (`[]` when empty; never 0-byte) (GAP-021)
- `run --json` final envelope includes `ok` + full `steps[].data` (GAP-020)
- Clap usage errors emit JSON envelope when `--json` is on argv (GAP-002)
- `view` empty about:blank refuses silent success unless `--allow-empty` (GAP-012)
- `print-pdf` refuses blank PDF without navigated content (GAP-013)
- Dialog soft path with `--if-present` (GAP-006)
- Chrome privacy launch flags; no `metrics-recording-only` (GAP-016)

### Changed
- Version `0.1.4`
- `parity_run_inventory` test enforces `RUN_DISPATCHED_CMDS` ∪ intentional exclude
- Clap surface audit (`rules_rust_cli_com_clap`): `GlobalOpts` uses `Args` + flatten; explicit `ArgAction::SetTrue`; `value_hint` on paths/URLs; help headings; `after_help` examples; `-v` alias; `author` metadata
- `CliError` derives `thiserror::Error`; binary installs `human-panic` for release panic reports
- Integration gate `tests/clap_command_debug_assert.rs` runs `Cli::command().debug_assert()`

### Documentation
- Public bilingual docs (README, INTEGRATIONS, llms*, HOW_TO_USE, AGENTS, COOKBOOK, MIGRATION, TESTING, SECURITY, CONTRIBUTING) synchronized to v0.1.4 surface
- Inventory documented as 61 agent names via `commands --json` (includes run/schema-only `select-option` and `pick`; clap top-level lists 59 without them as standalone)
- Skills EN/PT rewritten as imperative playbooks with formulas for all 61 commands (XDG + flags only; no product env catalogs)
- `docs/schemas` regenerated; live `schema` fragments for `batch-scrape`/`crawl`/`scrape` document `--engine browser` and multi-format
- `gaps.md` banner marks GAP-001…025 Closed while keeping pre-fix audit history

## [0.1.3] - 2026-07-17


### Documentation
- Public root docs (README, INTEGRATIONS, llms*, SECURITY, CONTRIBUTING) synchronized to v0.1.3 surface (59 commands, Redis/Lighthouse honesty, A001–A012)
- `CHANGELOG.pt-BR.md` mirrors full 0.1.3 hard-close; added `llms-full.pt-BR.txt`
### Fixed (Redis live + Lighthouse real polish)
- Redis cache: always-on RESP mock TCP roundtrip (no `#[ignore]`, no product env); optional real `redis-server` spawn when on PATH; doctor `cache_redis` health from XDG
- Lighthouse: resolve flag → XDG → PATH; envelope `binary_source`/`binary_present`; doctor reports source; e2e labels `source=real|mock`

### Fixed (hard-close GAP-A001…A012)
- E2E residual assert no longer self-matches scanners; pipefail-safe empty match (GAP-A001)
- FINALIZE scavenges owned Chromium tmp orphans (GAP-A002)
- `run --script` accepts NDJSON or JSON array of steps (GAP-A003)
- `scrape --engine http` rejects `file://` with Usage + browser/parse suggestion (GAP-A004)
- `reload` uses CDP `Page.reload` + `ignoreCache` (GAP-A005)
- `init_script` removed after navigation/reload (GAP-A006)
- Redis `rediss://` fail-closed (GAP-A007); always-on RESP mock roundtrip + optional live redis when `redis-server` is on PATH (GAP-A008)
- `handle_before_unload` auto-accepts via CDP without `preventDefault` inject (GAP-A009)
- Doctor lighthouse reports XDG path suggestion honestly (GAP-A010)
- Unknown modern CDP events ignored so capture continues (GAP-A012)

### Added (GAP-A011 PRD pillars)
- `find-paths --glob` shell-style filter
- `sheet-write` CSV/JSON to XLSX via `rust_xlsxwriter`
- `sg-scan` / `sg-rewrite` one-shot structural lint (dry-run default)

### Fixed
- `goto` wires `--init-script`, `--handle-before-unload`, and `--navigation-timeout-ms` (no silent discard) via CDP `Page.addScriptToEvaluateOnNewDocument`
- Doctor never suggests `npm`; `--fix` / `--offline` are wired; lighthouse fix points to `config set lighthouse_path`
- `console list` / `net list` `--include-preserved` uses a process-local navigation ring buffer with honest `include_preserved_mode`
- Lighthouse `--mode snapshot` maps to `--gather-mode=snapshot` (mock echoes argv)
- `reload --init-script` single-shot rejects blank sessions; multi-step `run` applies init on reload
- Extension uninstall unloads in-process targets with explicit `effect` (`unloaded` | `metadata_only`)
- Residual ledger fills `profile_dir` + Singleton side-channels; FINALIZE wipes owned paths only
- Windows Job Object helpers for residual-zero reap (`win_job`)
- i18n pt-BR critical suggestions use correct accents (invocação, propósito, obrigatórios, não)
- Parse path uses XDG HTTP/parse cache (no discarded cache dir)

### Added
- `page tab-id` (tool-ref `get_tab_id`) — inventory 53 tools
- `eval --service-worker-id` evaluates in extension service worker targets
- `config list-keys` for XDG key discovery
- `RetryConfig` module with backoff/jitter; proptest offline parsers
- Layered HTTP cache (memory L1 + SQLite L2 under XDG); optional `log_to_file` rotated logs
- `scripts/inventory_diff_base.sh` local inventory gate; e2e harness cleans `/tmp/ba-e2e-*` on success

## [0.1.2] - 2026-07-17

### Fixed
- Public bilingual documentation and skills synchronized to the full v0.1.2 surface (print-pdf, monitor, qr, find-paths, parse PDF/DOCX/xlsx/ods, extract LLM, 13 XDG keys, browser scrape formats, fail-fast data.steps, scrape webhook-url)
- Public docs teach product settings only via flags and XDG `config path|init|show|set|get` (no product env catalogs)
- Live `schema --cmd` and static `docs/schemas/` regenerated for print-pdf/monitor/qr/find-paths and expanded scrape/config fragments (including scrape `webhook_url`)
- Browser scrape now captures `outerHTML` and applies `--format` (markdown/html/links/metadata/raw-html/screenshot/summary/product/branding) instead of silent text-only (GAP-001)
- `run` scroll accepts `dy`/`dx` aliases for `delta_y`/`delta_x` (GAP-002)
- `schema --cmd` expanded for goto/eval/type/scroll/assert tool-ref flags (GAP-003)
- Human suggestions localize for `pt-BR` via `--lang` and XDG `config set lang` (GAP-004)
- Product runtime no longer reads `RUST_LOG`, `CI`, `PUPPETEER_*`, or `PLAYWRIGHT_*`; logging uses flags + XDG `log_level`; Chrome via XDG `chrome_path` (GAP-005)
- `run` fail-fast returns partial `data.steps` on error envelopes (GAP-006/016)
- Lighthouse resolves XDG `lighthouse_path` and localized install suggestion (GAP-007)
- Search cleans SERP redirect wrappers (`uddg=`) to destination URLs (GAP-008)
- Scrape accepts `raw-html` / `rawHtml` aliases and `screenshot` format token (GAP-009/021)
- `exec` help describes full step surface (GAP-011)
- `assert` accepts `url_contains` / `text_contains` aliases (GAP-012)
- Clippy `manual_clamp` cleanups in MITM helpers (GAP-013)
- `attr` falls back to DOM properties when HTML attributes are null (GAP-018)
- Docs examples use `/tmp/browser-automation-cli-artifacts` instead of `bac-` prefix (GAP-019)
- Tool-reference fixture synced to 52 official tools from knowledge base (GAP-017/020)

### Added
- `print-pdf` one-shot CDP `Page.printToPDF` artifact command
- `monitor check` one-shot baseline hash compare with optional `--write-baseline`
- XDG config keys: `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model` (full key set also includes lang, timeout, artifacts_dir, ignore_robots, namespace, encryption_key, color)
- Error envelopes may include partial `data` for multi-step fail-fast recovery
- `parse` PDF (lopdf), DOCX, xlsx/ods (calamine), `--redact-pii`
- `extract --llm` / `--question` / `--schema-json` (XDG key only; fail-closed without key)
- `qr encode|decode` and `find-paths` (no Chrome)
- Scrape formats `summary`/`product`/`branding`; MITM `ws_count`
- Command inventory documents 56 top-level names (`commands --json`), including `print-pdf`, `monitor`, `qr`, `find-paths` beyond the 52 DevTools parity tools

### Changed
- clap feature set drops unused `env` (product settings stay XDG + argv)
- Version bumped to `0.1.2`

## [0.1.1] - 2026-07-17

### Added
- XDG config surface: `config path`, `config init`, `config show`, `config set`, and `config get` for resolved paths and `config.toml` keys (lang, timeout, artifacts_dir, ignore_robots, namespace)
- Local MITM surface on hudsucker: `mitm start` (bind `127.0.0.1` with ephemeral port, one-shot), `list`, `get`, `har`, `export`, `domains`, `apis`, and `init-ca`
- Workflow journal DAG (petgraph + SQLite): `workflow run`, `workflow resume`, and `workflow status`; resume skips steps already marked ok
- Local scrape/crawl/map/search/parse HTTP commands: `batch-scrape`, `crawl`, `map`, `search`, and `parse`
- `scrape` formats `text|markdown|html|links|metadata`, engines `http|browser`, and `--only-main-content`
- `wait` multi `--text` with OR semantics (any listed text resolves the wait)
- Doctor check for XDG `browsers_dir`
- Batch scrape bounded concurrency via Tokio `JoinSet`
- Public bilingual documentation framework for crates packaging (`docs/` guides, `docs/schemas/` index, dual-language skill packages)
- Dual license files `LICENSE-MIT` and `LICENSE-APACHE`
- Crate-level rustdoc with Overview, Features, Targets, MSRV, Safety, and Examples
- rustdoc lints on the crate root (`missing_docs`, broken/private links, invalid HTML/codeblocks)
- docs.rs `targets` and `default-target` for multiplatform builds
- README Features, Targets, and MSRV sections with local `cargo doc` formulas
- `aquamarine` Mermaid lifecycle diagram on `run()` rustdoc
- Vendored tool-ref fixture `tests/fixtures/tool-reference.md` (52 tools) for inventory/e2e parity
- English product lifecycle slogan **BORN EXECUTE FINALIZE DIE** in crate description, CLI about text, and agent docs

### Changed
- Product settings no longer use runtime product environment variables; configuration is XDG-backed (`config.toml` + flags)
- `run` gains scrape parity with standalone scrape options and enforces category gates (`category_memory`, `category_extensions`, `category_third_party`, `category_webmcp`) inside script steps
- `Cargo.toml` metadata now includes authors, repository, homepage, documentation, and MSRV
- License declared as `MIT OR Apache-2.0`
- README badge order now starts with docs.rs and crates.io
- Public API docs expanded for `error`, `envelope`, and `lifecycle`
- Release profile uses fat LTO (`lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`)
- Clap help shows zero product env suggestions (`BROWSER_AUTOMATION_CLI_*` no longer advertised on flags)
- Enabled crates packaging by removing `publish = false`

### Fixed
- Build blockers: `RunFlags.category_extensions` field wiring and `Selector` lifetime
- `run` + scrape parity end-to-end; multi-text wait OR; category gates in `run`
- XDG config/paths without product env for settings; doctor reports XDG `browsers_dir`
- MITM hudsucker one-shot bind on `127.0.0.1` with ephemeral port
- Workflow resume correctly skips completed ok steps
- Batch concurrency shutdown-friendly via `JoinSet`
- rustdoc broken intra-doc links in `emulate --viewport` help text
- `tests/parity_inventory.rs` reads vendored `tests/fixtures/tool-reference.md` (52 tools)
- Formatting drift under `cargo fmt`

### Removed
- GitHub Actions workflows under `.github/workflows/`
- Cargo `[profile.ci]` used only by removed CI
- Hosted CI and GitHub Actions integration guidance from public docs
- Product settings bound to `BROWSER_AUTOMATION_CLI_*` environment variables (settings live under XDG + CLI flags)

## [0.1.0] - 2025-07-16

### Added
- One-shot Chrome launch via `chromiumoxide::Browser::launch`
- Launch flags for proxy, webgpu, extensions, and sandbox on the oxide path
- FINALIZE path with close, wait, and kill fallback
- Core commands: `doctor`, `open`/`goto`, `extract`, `scrape`, `run`, `grab`, `view`, `click`/`press`, `fill`/`write`, `robots`
- Optional console and network capture
- Robots policy with dual-flag acceptance
- DevTools parity surface for navigation, input, snapshot, screenshot, eval, pages, wait, perf, lighthouse, screencast, heap, extensions
- Tool-ref flags such as `--include-snapshot` on hover, drag, keys, upload, and fill-form
- `net` and `console` list filters with pagination
- `eval` with `--args`, `--dialog-action`, and `--file-path`
- `perf start --auto-stop` and `perf insight`
- `screencast stop --path` with ffmpeg-backed webm or mp4 export
- Heap deep analysis gated by `--category-memory`
- Page management with `--background` and `--isolated-context`
- Schema discovery via `schema --cmd` and inventory gate tests

### Changed
- `src/install.rs` slimmed to local discovery only
- CDP stack is 100 percent chromiumoxide Chrome

### Removed
- Dual-spawn monólito `launch_chrome` / `ChromeProcess`
- Residual branding and non-product dump artifacts from the public tree

### Fixed
- Clean public git history recreated without legacy branding commits

### Notes
- Explicitly out of **0.1.0 only**: PRD local scrape crawl/map/search surface, MITM, and workflow SQLite journal (these landed in 0.1.1)
