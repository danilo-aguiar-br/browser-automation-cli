[English](INTEGRATIONS.md) | [Português Brasileiro](INTEGRATIONS.pt-BR.md)

# Integrations — browser-automation-cli

> One process, one Chrome, one JSON envelope. Built for agent subprocesses.

## Coverage Snapshot
- Works with any agent that can spawn a subprocess and read stdout plus stderr
- Primary surfaces: Claude Code, Codex, Cursor, local shell, editor agents
- Discovery helpers: `commands --json`, `schema <cmd>` / `schema --cmd`, `doctor --json`
- Integration path is local subprocess only
- Product settings are flags plus XDG config only

## Flag Aliases and Version Notes
- Product names stay fixed: `view`, `press`, `write`, `grab`
- Avoid inventing aliases such as `click` or `screenshot` in agent prompts (use `grab` for screenshots; scrape may accept a `screenshot` format token)
- Use `grab --path <file>` (not a bare positional path)
- Use repeatable `wait --text` for OR semantics across multiple strings
- Use `scrape --format` / `scrape --engine` for local scrape formats (multi-format CSV or repeatable)
- Browser scrape applies `--format` via outerHTML; 15 live formats: `text`, `markdown`, `html`, `rawHtml`, `links`, `metadata`, `screenshot`, `summary`, `product`, `branding`, `images`, `jsonld`, `json`, `feed`, `attributes` (`raw-html` remains an accepted alias of `rawHtml`)
- `0.1.0` ships the default-on DevTools parity surface plus category gates
- `0.1.1` adds XDG `config`, local MITM, workflow journal, and local scrape/crawl/map/search/parse surface (`batch-scrape`, `crawl`, `map`, `search`, `parse`, expanded `scrape`)
- `0.1.2` closes agent-first gaps and adds `print-pdf`, `monitor`, `qr`, `find-paths`, parse document types, extract LLM, and expanded config keys
- `0.1.3` hard-closes residual-zero and agent contracts: NDJSON|JSON-array `run`, CDP reload/beforeunload/init_script, Redis/Lighthouse honesty, `sheet-write`/`sg-scan`/`sg-rewrite`, `find-paths --glob` (59 clap top-level; 53 e2e DevTools tools)
- `0.1.4` hard-closes GAP-001…025: `--json-steps`, wait multi/url, `select-option`/`pick` run cmds, assert console kinds, `schema <cmd>` positional, MITM `capture-url` + global `--mitm*`, multi-format scrape, batch/crawl `--engine browser`, clap JSON usage errors
- `0.1.5` hard-closes residual-zero disk (RES-01…12): BORN auto-GC of stale Singleton-only Chromium `/tmp` dirs (age floor 60s), FINALIZE dual scavenge + re-scan, `doctor residual_disk` + top-level `residual` (`ResidualDiskReport`), never kills host Flatpak Chrome; inventory honesty with `locale`/`man`
- `0.1.6` hard-closes agent dialog/select/scrape/wait confidence: `dialog_settled` bool + XDG `dialog_settle_ms`, multi-tab dialog `session_id` isolation with e2e gate, native select `input`+`change`, `wait_timeout_ms` in `run`, scrape `format`/`formats` in `run`, grab `png|jpeg|webp` only (AVIF encode removed); inventory tip 0.1.8 was 69 via `commands --json` (0.1.6: `submit`/`storage` → 65; 0.1.7: `image`+`video`+`audio` → 68 then `record` → 69; also `select-option`, `pick`); e2e TOTAL=53 PASS=52 SKIP=1 (lighthouse mock honest SKIP)
- `0.1.8` hard-closes anti-detection and egress control: stealth family (`--no-stealth`, `--stealth-profile`, `--stealth-seed`), window mode via XDG `browser_mode` plus `--no-xvfb`, egress proxy (`--proxy`, `--proxy-bypass`) covering Chrome and the HTTP engine, constant HTTP/2 fingerprint keys, human input kinematics (`--input-profile`, `--input-seed`), session warmup (`--warmup`, `--warmup-url`), payload expectations (`--expect`, `--expect-exit-code`), and `config unset <KEY>`; config surface grows 176 → **204** keys while the 0.1.8 inventory tip stayed 69 via `commands --json`
- Live surface (v0.1.9): **217** XDG keys via `config list-keys --json` (the 204 figure belongs to the 0.1.8 paragraph above); `doctor --fingerprint` adds `measurement_scope` / `unmeasured_os` (not XDG keys); `emulate`/`resize` `screen` applies CDP; `--no-stealth` fingerprint plan matches the page
- Experimental tools require `--experimental-vision` or `--experimental-screencast`

## Summary Table

| Surface | Integration style | Required flags | Notes |
|---------|-------------------|----------------|-------|
| Claude Code | subprocess | `--json` | multi-step via `run --script` (NDJSON or JSON array); optional `--json-steps` |
| Codex | subprocess | `--json -q` | quiet stderr for cleaner transcripts |
| Cursor | shell tool | `--json` | keep timeouts explicit |
| Local shell | script | `--json` | parse with `jaq` |
| Continue / Cline | editor shell | `--json -q` | one-shot only |

## Claude Code
- Spawn one CLI process per atomic action
- Use `run --script` (NDJSON or JSON array) when `@eN` refs must survive multiple steps
- Prefer XDG `config set` for durable defaults
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli --json goto https://example.com
browser-automation-cli --json view
browser-automation-cli --json run --script /tmp/steps.jsonl
browser-automation-cli --json --json-steps run --script /tmp/steps.jsonl
```
- `--script` is a file path, never inline JSON; `/tmp/steps.jsonl` holds one step object per line:
```json
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
```

## Codex
- Prefer `-q --json` so only envelopes reach the agent transcript
```bash
browser-automation-cli -q --json goto https://example.com
```

## Cursor
- Call the binary from the shell tool with explicit `--timeout`
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine http
```

## Local Shell
- Always capture exit codes before parsing JSON
- Run validations on your local machine before release
```bash
out=$(browser-automation-cli --json version)
echo "$out" | jaq -e '.ok == true'
```

## Continue and Cline
- Use quiet JSON mode to keep editor transcripts clean
- Do not expect session stickiness between separate process launches

## New Flags by Version
- `0.1.0`: category gates, experimental vision and screencast, capture flags, schema discovery
- `0.1.1`: XDG `config` (`init`/`path`/`show`/`get`/`set`), `mitm` (local CA + one-shot `127.0.0.1` proxy), `workflow` (`run`/`resume`/`status`), local scrape surface (`scrape --format/--engine`, `batch-scrape`, `crawl`, `map`, `search`, `parse`), multi-text `wait --text` OR, `grab --path`
- `0.1.2`:
  - `scrape --engine browser` applies `--format` via outerHTML across the 15 live formats `text`, `markdown`, `html`, `rawHtml`, `links`, `metadata`, `screenshot`, `summary`, `product`, `branding`, `images`, `jsonld`, `json`, `feed`, `attributes` (`raw-html` remains an accepted alias of `rawHtml`)
  - `run` scroll aliases `dy`/`dx` for `delta_y`/`delta_x`; fail-fast error envelopes may include partial `data.steps`
  - `schema --cmd` expanded for `goto`/`eval`/`type`/`scroll`/`assert`
  - `--lang pt-BR` and `config set lang` localize human suggestions
  - Logging via `--verbose`/`--debug` and XDG `log_level`/`chrome_path`/`lighthouse_path` only
  - `search` cleans `uddg=` SERP redirects
  - `print-pdf` one-shot CDP; `monitor check --url --baseline [--write-baseline]`
  - `parse` PDF/DOCX/xlsx/ods + `--redact-pii`; `extract --llm --question --schema-json` (XDG `openrouter_api_key`, `llm_base_url`, `llm_model`)
  - `qr encode|decode`, `image info|convert|resize|download|exif`, `video info|download|convert|to-mp3|trim|thumbnail|manifest`, `find-paths`
  - `assert` aliases `url_contains`/`text_contains`; `attr` DOM property fallback
  - Config keys: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`
  - Command inventory is 56 top-level names (`commands --json`), including `print-pdf`, `monitor`, `qr`, `find-paths`
- `0.1.3`:
  - `run --script` accepts NDJSON or a JSON array of steps; fail-fast may return partial `data.steps`
  - `reload --ignore-cache` uses CDP `Page.reload` + `ignoreCache`
  - `init_script` is removed after navigation/reload; `handle_before_unload` auto-accepts via CDP dialog (no preventDefault inject)
  - `scrape --engine http` rejects `file://` with Usage + browser/parse suggestion
  - `find-paths --glob`; `sheet-write` CSV/JSON→XLSX; `sg-scan` / `sg-rewrite` structural lint (dry-run default)
  - Lighthouse resolve flag → XDG `lighthouse_path` → PATH; envelope `binary_source` real|mock; doctor reports source
  - Redis: XDG `cache_backend` / `cache_redis_url`; `rediss://` fail-closed; doctor `cache_redis`
  - FINALIZE scavenges owned Chromium `/tmp` orphans; residual e2e residual-zero
  - Config: `config list-keys`; keys add `log_to_file`, `cache_backend`, `cache_redis_url`
  - Command inventory is 59 top-level names (`commands --json`), including `sheet-write`, `sg-scan`, `sg-rewrite`
- `0.1.4`:
  - Global `--json-steps`: stream one NDJSON line per `run` step (`step`, `cmd`, `ok`, `result`)
  - `run --json` final envelope includes `ok` + full `steps[].data`
  - `wait`: CSS multi-selector OR (`#a, #b`), selectors arrays; run fields `url` / `url_contains` / `navigation`
  - `select-option` / `pick` multi-step cmds (inventory + run/schema; not standalone clap)
  - Assert kinds `console_empty` / `console_no_match` (CLI `assert console-empty` / `assert console-no-match --pattern`)
  - `schema <cmd>` positional in addition to `schema --cmd <cmd>`
  - `goto` / `reload` `--handle-before-unload accept|dismiss` (`BeforeUnloadAction`); dialog soft path `--if-present`
  - MITM: full surface `status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact`
  - Global MITM flags: `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`
  - Scrape multi-format (CSV/repeatable `--format`); `batch-scrape` / `crawl` `--engine browser`
  - `view --allow-empty`; `print-pdf` in multi-step `run`; blank PDF refused without navigated content
  - Clap usage errors emit JSON when `--json` is on argv; `console dump` always valid JSON array
  - Inventory: 61 agent names via `commands --json` (includes `select-option`, `pick`); clap top-level 59 without them as standalone
  - Contract gates: `tests/parity_run_inventory.rs`, `tests/clap_command_debug_assert.rs`
- `0.1.5`:
  - Residual-zero disk hygiene (product law: residual-zero process + disk)
  - BORN auto-GC: `scavenge_stale_singleton_orphans` of `/tmp` `org.chromium.Chromium.*` Singleton-only dirs older than 60s
  - FINALIZE dual scavenge + re-scan of owned marker dirs (prefix `browser-automation-cli-chrome-`); never kills host Flatpak Chrome
  - Doctor check `residual_disk` + top-level JSON field `residual` (`ResidualDiskReport`): `scanned_roots`, `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legacy), `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`
  - Local residual gates: `scripts/residual-check.sh`, `scripts/residual-stress.sh` (local only)
  - Discovery honesty: inventory includes `locale` and `man`
  - Inventory (historical 0.1.5): **63** agent names via `commands --json`
- `0.1.6`:
  - Dialog: `dialog accept|dismiss` emits `.data.dialog_settled` boolean on happy path; XDG `config set dialog_settle_ms` budgets wait for `Page.javascriptDialogClosed`
  - Multi-tab dialog isolation: page forwarders stamp `session_id`; gate `tests/dialog_multitab_gate.rs`; `tab_switch` best-effort domain enable under open page-modal dialog
  - Select: native `input`+`change` for `pick` / `select-option` (shared dispatch helper)
  - Run: public `wait_timeout_ms` on wait steps; scrape step `format`/`formats` (compact text without HTML dump when text-only)
  - Grab: `--format png|jpeg|webp` only — AVIF encode removed
  - Lighthouse: unit fixtures include chrome-captured LHR 13.4.1 shape; e2e mock remains SKIP (never claim parser PASS from mock)
  - Inventory tip at 0.1.6 was 65 agent names via `commands --json`, after `submit` and `storage` joined `select-option` and `pick`
  - Discover full config key set via `config list-keys --json` (not a fixed count of 16)
  - Intentional residual: GAP-022 ~53 dependency multi-versions; GAP-023/024 PRD wishlist flags/commands not full parity
- `0.1.8`:
  - Anti-detection: `--no-stealth`, `--stealth-profile auto|chrome-linux|chrome-win|chrome-mac`, `--stealth-seed <SEED>`; XDG `stealth` (default true), `stealth_profile`, `stealth_seed`
  - Window mode: XDG `browser_mode` (`auto|headed|headless`; `auto` resolves to headless and `doctor` reports the effective mode); `--no-xvfb` skips the private virtual display on Linux
  - Egress proxy: `--proxy <URL>` (`http`, `https`, `socks5`) and `--proxy-bypass <HOSTS>` apply to Chrome **and** to the HTTP engine; XDG `proxy_url`, `proxy_bypass`, `proxy_username`, `proxy_password`, `cdp_proxy_bypass_loopback` (default true)
  - HTTP/2 fingerprint: XDG `http2_enabled` (default true), `http2_initial_stream_window_size` (6291456), `http2_initial_connection_window_size` (15663105), `http2_max_header_list_size` (262144), `http2_max_frame_size` (16384), `http2_adaptive_window` (default false, because leaving it off keeps the fingerprint constant)
  - Human input kinematics: `--input-profile human|direct` (default `human`) and `--input-seed <SEED>`; XDG `input_profile`, `input_move_steps` (24), `input_move_gap_ms` (12), `input_click_dwell_ms` (65), `input_key_dwell_ms` (45), `input_type_delay_ms` (95), `input_scroll_tick_px` (100), `input_scroll_max_ticks` (40), `input_target_jitter_px` (3), `input_scroll_settle_rounds` (3)
  - Session warmup: `--warmup` and `--warmup-url <URL>`
  - Payload expectations: `--expect <EXPR>` with `key=value`, `key!=value` or `key~substring`, repeatable and AND-conjugated; `--expect-exit-code` exits 65 when any expectation is unmet, off by default because changing the exit code from data content would break callers silently
  - `config unset <KEY>` restores one key to its built-in default
  - New standalone keys: `robots_user_agent`, `scrape_no_cache`, `monitor_diff_max_bytes`
  - Config surface grows from 176 to **204** keys (`config list-keys --json`)
  - Inventory tip stayed at 69 agent names via `commands --json`: 0.1.8 added flags and keys, never a command
- `0.1.9`:
  - New commands `sitemap` and `feed`, the first inventory growth since 0.1.7. Neither adds capability: `sitemap <url>` is `map --sitemap-only` and `feed <url>` is `scrape --formats feed --engine http`. They exist for DISCOVERABILITY, because on an agent-facing CLI a capability reachable only by knowing that a flag on a differently-named verb carries it is, in practice, not reachable
  - `sitemap <url>` takes `--limit`, `--select`, `--include-path`, `--exclude-path`, `--search`, `--sort`, `--dedup-key`, `--include-subdomains`, `--ignore-query-params`. There is no `--depth`: a sitemap is a DECLARED list, not a frontier, so there is no link graph to bound
  - `feed <url>` takes `--select`, `--header`, `--no-cache`. The HTML-shaping flags are absent rather than ignored, because `ScrapeFormat::Feed` parses the RAW body and selector reduction would destroy an XML or JSON document; Chrome is not offered because rendering a feed produces the browser's XML viewer
  - `doctor --fingerprint` adds `stealth_installed`, `stealth_seed_active`, `measurement_scope_matches_host`, `measurement_scope` (`linux-headless-xvfb`), `unmeasured_os`, `stealth_profile_source`, `fonts_method` and derived `gpu_source` / `fonts_source` / `audio_source`; new coherence mismatch `stealth_not_installed`
  - `--stealth-profile list` prints the four tokens without launching Chrome; `commands --json` also emits `stealth_profiles`, `stealth_seed_fields` and `stealth_seed_does_not_vary`
  - `--min-delay-ms` sets the same-origin courtesy floor per invocation; the effective wait is the MAXIMUM of the flag, the XDG `scrape_min_delay_ms` floor and the site's `Crawl-delay`
  - `--max-items` is accepted as an alias of `--limit-rows`; it limits what is EMITTED, while a command's own `--limit` limits what is FETCHED
  - `crawl --include-regex` / `--exclude-regex` / `--sitemap-only`; `--webhook-url` on `crawl` and `batch-scrape`; `map --include-subdomains` and `map --ignore-query-params`; `search --include-domains`, `--exclude-domains`, `--country`, `--search-lang`, `--time-filter`
  - `map` restricts results to the seed host by default; `--include-subdomains` widens it, and there is no longer a way to have `map` return arbitrary external hosts
  - `parse --format` derives scrape formats from a parsed file; `heap take --url` navigates before capturing; `sheet-write --force`; `--paths-file` on thirteen `image` / `video` / `audio` actions; `mitm capture-url --capture-hosts`
  - `cookie clear` requires `--all`, and `mitm block` requires `--host` or `--path`: an irreversible verb takes its scope from argv, never from the absence of a flag
  - `--timeout` is bounded at 86400 seconds; `--schema-json` goes through the same filesystem jail as `run --script`
  - Every side-effecting verb publishes `target_resolved` and `target_source` (`argv` / `step` / `xdg` / `ambient`), the Explicit Target Designation contract, asserted by `tests/etd_gate.rs`
  - New XDG keys `screen` (`WxH`), `platform_child_poll_ms`, `extension_attach_poll_iters`, `user_data_dir` (opt-in persistent Chrome profile, unset by default, and leaving it unset is what keeps residual-zero true), `input_typo_permille` (`0`) and `capture_preserved_rings` (`3`); eighteen keys that were accepted and ignored at runtime are now wired
  - Config surface: **217** keys via `config list-keys --json` (the 204 figure belongs to the 0.1.8 paragraph above)
  - Inventory tip is **71** agent names via `commands --json`; the clap top-level surface is 69, because `select-option` and `pick` remain multi-step names without a standalone verb
