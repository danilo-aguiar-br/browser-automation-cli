[Português](CONFIGURATION.pt-BR.md)


# Configuration Reference
> Canonical XDG reference for every durable `browser-automation-cli` configuration key


## How Configuration Resolves
- The product reads no product environment variables at all
- Every durable setting lives in the XDG `config.toml` file
- Precedence is CLI flag first, XDG key second, built-in default last
- A CLI flag overrides the XDG key only for the invocation that carries it
- An XDG key overrides the built-in default for every invocation on that host
- A key with no built-in default stays unset until you write it or pass the matching flag
- Secrets such as `openrouter_api_key` and `encryption_key` are written with permission `0600`
- Secrets never appear in logs, in JSON envelopes or in human stderr
- Robots bypass still requires both `--ignore-robots` and `--i-accept-robots-risk` on the command line


## Configuration Commands
- `config init` creates the XDG configuration file when it is missing
- `browser-automation-cli --json config init`
- `config path` prints the resolved configuration and state paths
- `browser-automation-cli --json config path`
- `config show` prints the effective configuration after defaults and file merge
- `browser-automation-cli --json config show`
- `config get <key>` reads one resolved value
- `browser-automation-cli --json config get timeout`
- `config set <key> <value>` writes one durable value
- `browser-automation-cli --json config set dialog_settle_ms 2000`
- `config list-keys` enumerates every accepted key on the running binary
- `browser-automation-cli --json config list-keys`


## Core and Locale
- `lang` — Message locale override (`en` or `pt-BR`; bare `pt` rejected). Default: none
- `timeout` — Global timeout in seconds. Default: `0`
- `artifacts_dir` — Artifacts output directory. Default: none
- `namespace` — Isolated state namespace. Default: none
- `encryption_key` — Session encryption key material. Default: none
- `color` — ANSI colors on human stderr. Default: none


## Logging
- `log_level` — Tracing `EnvFilter` used when argv flags stay quiet. Default: `error`
- `log_to_file` — Rotated local JSON logs under XDG state, never remote. Default: none
- `max_log_files` — Retained rotated log files in range `1..=90`. Default: `14`
- `log_rotation` — Rolling policy `daily`, `hourly` or `never`. Default: `daily`


## External Binaries
- `chrome_path` — Absolute Chrome or Chromium path. Default: none
- `lighthouse_path` — Absolute `lighthouse` CLI path. Default: none
- `ffmpeg_path` — Absolute `ffmpeg` path for screencast encode and video convert or `to-mp3`. Default: none
- `lighthouse_timeout_secs` — Wall-clock `lighthouse` CLI timeout in seconds, range `1..=3600`. Default: `300`
- `ffmpeg_timeout_secs` — Wall-clock `ffmpeg` encode timeout in seconds, range `1..=3600`. Default: `120`


## LLM and Webhooks
- `openrouter_api_key` — LLM API key stored with permission `0600`. Default: none
- `llm_base_url` — OpenAI-compatible base URL. Default: none
- `llm_model` — Default LLM model id. Default: none
- `llm_http_timeout_secs` — LLM and webhook blocking HTTP timeout in seconds. Default: `60`
- `webhook_post_timeout_secs` — Operator webhook POST timeout in seconds. Default: `15`
- `webhook_retry_base_delay_ms` — Webhook retry base delay in milliseconds, doubled each attempt. Default: `50`
- `webhook_max_attempts` — Webhook max attempts including the first try. Default: `3`


## Cache and Redis
- `cache_backend` — Cache backend `sqlite`, `memory` or `redis`. Default: `sqlite`
- `cache_redis_url` — Redis URL required when the backend is `redis`. Default: none
- `redis_allow_remote` — Allow non-loopback Redis hosts, false when unset. Default: none
- `redis_connect_timeout_secs` — Redis TCP connect timeout in seconds. Default: `2`
- `redis_io_timeout_secs` — Redis RESP stream I/O timeout in seconds. Default: `3`
- `cache_max_resp_bulk_bytes` — Redis RESP bulk string size ceiling in bytes. Default: `16777216`
- `cache_max_resp_line_bytes` — Redis RESP line size ceiling in bytes. Default: `16777216`
- `scrape_http_cache_ttl_secs` — HTTP scrape response L2 cache TTL in seconds. Default: `3600`
- `file_parse_cache_ttl_secs` — Local file-parse L2 cache TTL in seconds. Default: `86400`


## HTTP and Network Safety
- `http_ssrf_mode` — HTTP SSRF policy `strict`, `allow_loopback` or `off`. Default: `strict`
- `http_timeout_secs` — Shared HTTP client total timeout in seconds. Default: `30`
- `http_connect_timeout_secs` — HTTP connect-phase timeout in seconds. Default: `10`
- `http_redirect_max` — Max HTTP redirects followed by product clients. Default: `10`
- `http_pool_max_idle_per_host` — HTTP pool max idle connections per host. Default: `4`
- `scrape_max_body_bytes` — Max HTTP scrape body bytes. Default: `5000000`
- `browser_scrape_max_body_bytes` — Max body bytes for browser-engine scrape helpers. Default: `2000000`
- `search_base_url` — HTML search endpoint base with `?q=` appended. Default: `https://html.duckduckgo.com/html/`


## Robots and Politeness
- `ignore_robots` — Default robots ignore, with both CLI risk flags still required. Default: none
- `robots_loopback_exempt` — Loopback hosts skip `robots.txt`; set false to enforce against localhost. Default: `true`
- `robots_probe_timeout_secs` — `robots.txt` HEAD or probe timeout in seconds. Default: `5`
- `robots_max_body_bytes` — Max `robots.txt` body bytes as anti-OOM guard. Default: `524288`
- `robots_fetch_timeout_secs` — Timeout for fetching `robots.txt` in seconds. Default: `30`
- `scrape_min_delay_ms` — Floor delay between same-origin GETs in milliseconds. Default: `0`
- `scrape_honor_meta_robots` — Honor meta robots and `X-Robots-Tag` noindex. Default: `true`
- `scrape_honor_nofollow` — Skip `rel=nofollow` links in crawl discovery. Default: `true`
- `scrape_delay_jitter_ratio` — Politeness delay jitter ratio in range `0.0..=1.0`, zero disables. Default: `0.2`


## Scrape and Crawl
- `scrape_default_engine` — Default scrape engine when the CLI omits `--engine`, `http` or `browser`. Default: `http`
- `scrape_use_sitemap` — Prefer `sitemap.xml` when mapping a site. Default: `true`
- `scrape_max_text_chars` — Max text or markdown chars in scrape envelopes, zero means no cap. Default: `32768`
- `scrape_summary_chars` — Max chars for scrape format `summary`. Default: `400`
- `scrape_feed_max_entries` — Max entries kept by scrape format `feed` for RSS, Atom and JSON Feed. Default: `50`
- `scrape_follow_rel_next` — Follow `rel=next` pagination links during crawl. Default: none
- `scrape_dedup_similar` — Collapse near-duplicate pages by content similarity in `crawl` and `batch-scrape`. Default: none
- `scrape_no_cache` — Ignore the response cache on READ and always fetch from origin. The fresh response is still written, so a bypassing call refreshes the entry for later callers instead of leaving a stale one. `--no-cache` on `scrape` overrides this per invocation. There is no way to express the same thing with `scrape_http_cache_ttl_secs`: a TTL of `0` already means "never expires", which is the opposite, and the key rejects it. `monitor check` bypasses unconditionally and ignores this key, because a cached body made it compare a stored page with itself and report `changed: false`. Default: `false`
- `scrape_dedup_similar_distance` — SimHash Hamming distance in range `0..=64` under which pages are near-duplicates. Default: `3`
- `scrape_sitemap_max_bytes` — Max sitemap body bytes. Default: `524288`
- `scrape_charset_peek_bytes` — Charset sniffing peek window in bytes. Default: `4096`
- `scrape_crawl_limit_max` — Max crawl page budget as anti-DoS clamp for `--limit`. Default: `500`
- `scrape_crawl_max_depth` — Max BFS depth for `crawl` and `map`. Default: `10`
- `scrape_search_limit_max` — Max search result budget as anti-DoS clamp. Default: `50`
- `scrape_max_parse_bytes` — Max local file parse size in bytes before reject. Default: `50000000`
- `max_urls_file_bytes` — Max bytes for the `batch-scrape --urls-file` list. Default: `8388608`


## Image
- `image_max_input_bytes` — Max bytes for local image decode, convert or resize input. Default: `32000000`
- `image_max_pixels` — Max width times height for image decode as anti-bomb guard. Default: `64000000`
- `image_default_format` — Default image convert format `png`, `jpeg`, `webp` or `gif`. Default: `png`
- `image_default_quality` — Default lossy quality in range `1..=100` for image convert and resize. Default: `85`
- `image_download_max_bytes` — Max HTTP body bytes for image download. Default: `32000000`
- `image_avif_speed` — AVIF encoder speed in range `1..=10` where one is slowest and best, needs the `image-avif` feature. Default: `6`
- `default_jpeg_quality` — JPEG quality in range `1..=100` when `grab` omits `--quality`. Default: `80`


## Video and Audio
- `video_max_input_bytes` — Max bytes for video stdin materialization or path pre-check. Default: `512000000`
- `video_download_max_bytes` — Max HTTP body bytes for video download. Default: `512000000`
- `video_default_container` — Default video convert container `mp4`, `webm`, `mkv`, `mov`, `avi` or `m4v`. Default: `mp4`
- `video_default_crf` — Default CRF in range `1..=51` for lossy video re-encode. Default: `23`
- `video_default_audio_bitrate` — Default bitrate for video `to-mp3`. Default: `192k`
- `audio_max_input_bytes` — Max bytes for audio stdin materialization or path pre-check. Default: `256000000`
- `audio_download_max_bytes` — Max HTTP body bytes for audio download. Default: `256000000`
- `audio_default_format` — Default audio convert format `mp3`, `m4a`, `ogg`, `opus`, `flac`, `wav` or `aac`. Default: `mp3`
- `audio_default_bitrate` — Default bitrate for lossy audio encode. Default: `192k`


## SVG, GIF and Manifests
- `svg_max_bytes` — Max SVG source bytes accepted before rasterisation. Default: `4000000`
- `svg_max_depth` — Max XML nesting depth accepted in an SVG source. Default: `128`
- `svg_max_entities` — Max `<!ENTITY>` declarations tolerated in an SVG DTD, zero rejects any. Default: `0`
- `gif_max_frames` — Max animation frames decoded from a GIF. Default: `2000`
- `manifest_max_bytes` — Max bytes accepted for an HLS or DASH manifest body. Default: `8000000`
- `manifest_max_variants` — Max variant or representation entries emitted per manifest envelope. Default: `500`


## Chrome Engine and Lifecycle
- `chrome_search_paths` — Ordered Chrome or Chromium discovery paths, platform-separated; empty uses the built-in per-OS layout. Default: none
- `chrome_legacy_oxide_launch` — Launch Chrome through the legacy path instead of the self-spawn path as a stabilization fallback that loses the residual kill target. Default: none
- `chrome_startup_timeout_secs` — Chrome self-spawn CDP readiness wait in seconds. Default: `20`
- `chrome_default_timeout_ms` — Default per-operation timeout for the Chrome engine in milliseconds. Default: `25000`
- `browser_close_wait_secs` — `Browser.close` and process wait budget during FINALIZE in seconds. Default: `5`
- `residual_orphan_min_age_secs` — Age floor in seconds before a dead-owner marker profile is collectable. Default: `60`
- `platform_child_wait_secs` — Platform child wait deadline in seconds. Default: `5`
- `shutdown_poll_ms` — Shutdown cooperative poll interval in milliseconds. Default: `5`
- `shutdown_deadline_secs` — Shutdown hard deadline waiting for browser exit in seconds. Default: `30`


## CDP and Events
- `cdp_connection_probe_timeout_secs` — CDP `Browser.getVersion` liveness probe timeout in seconds. Default: `3`
- `cdp_discovery_timeout_secs` — CDP HTTP discovery timeout for `/json/version` probes in seconds. Default: `2`
- `cdp_discovery_max_body_bytes` — Max CDP discovery HTTP body bytes for `/json/version` and `/json/list`. Default: `1048576`
- `cdp_event_broadcast_capacity` — Process-local CDP event broadcast channel capacity. Default: `4096`
- `cdp_event_drain_poll_ms` — CDP event drain poll slice during navigation wait in milliseconds. Default: `100`
- `cdp_network_idle_settle_ms` — CDP network-idle settle window in milliseconds. Default: `500`
- `cdp_target_event_wait_ms` — CDP target event short wait in milliseconds. Default: `600`
- `event_tracker_max_entries` — In-memory console and network tracker ring size per page session. Default: `1000`
- `event_pump_slice_ms` — Wait and eval event pump slice in milliseconds. Default: `50`
- `eval_drain_slice_ms` — Eval drain slice while waiting for `Runtime.evaluate` results in milliseconds. Default: `40`
- `extension_attach_poll_ms` — Extension attach poll slice in milliseconds. Default: `150`


## Lightpanda Engine
- `lightpanda_startup_timeout_secs` — Lightpanda process startup wait in seconds. Default: `10`
- `lightpanda_session_timeout_secs` — Lightpanda `--timeout` session max in seconds, range `1..=604800`. Default: `604800`
- `lightpanda_poll_interval_ms` — Lightpanda CDP readiness poll interval in milliseconds. Default: `100`
- `lightpanda_discovery_timeout_ms` — Per-probe CDP discovery timeout while waiting for Lightpanda in milliseconds. Default: `500`
- `lightpanda_max_log_lines` — Bounded Lightpanda launch log ring in lines per stream. Default: `40`
- `lightpanda_ready_slice_ms` — Drain slice after Lightpanda child exit before snapshotting logs in milliseconds. Default: `25`
- `lightpanda_cdp_connect_timeout_secs` — Lightpanda CDP connect attempt timeout in seconds. Default: `5`
- `lightpanda_target_init_timeout_secs` — Lightpanda target init wait after connect in seconds. Default: `10`


## Interaction and Waits
- `interact_settle_ms` — UI settle delay after click, type or extension action in milliseconds. Default: `200`
- `dialog_settle_ms` — Max wait after a JS dialog answer for `javascriptDialogClosed` in milliseconds. Default: `2000`
- `network_idle_window_ms` — Quiet window for `wait --network-idle` in milliseconds. Default: `500`
- `dom_stable_window_ms` — Quiet window for `wait --dom-stable-ms` in milliseconds. Default: `500`
- `drag_move_steps` — Intermediate mouse positions synthesized for one HTML5 drag. Default: `6`
- `drag_move_gap_ms` — Delay between synthesized drag positions in milliseconds. Default: `16`
- `input_profile` — Default input shaping when `--input-profile` is absent: `human` synthesizes a trajectory, wheel ticks and key events; `direct` keeps the pre-0.1.8 dispatch. The flag still wins. Default: `human`
- `browser_mode` — Window mode: `auto` resolves to `headless`; `headed` puts a real window on your display, and on Linux that window is rendered into a private virtual display when Xvfb is available; `headless` is cheapest and most detectable. `--headed` still wins. Inverting the `auto` default carries a latency bill and is a separate decision; `doctor` reports what `auto` resolves to on this host under the `virtual_display` check, so the answer never drifts from the binary. Default: `auto`
- `stealth` — Anti-detection patches applied before the first navigation. `--no-stealth` turns them off for one run. Default: `true`
- `stealth_profile` — Impersonated identity: `auto`, `chrome-linux`, `chrome-win`, `chrome-mac`. `auto` follows the host, which is the only value that cannot contradict the Canvas and WebGL hashes the real GPU produces. Default: `auto`
- `proxy_url` — Egress proxy for both Chrome and the HTTP engine (`http`, `https`, `socks5`, `socks5h`). Put credentials here rather than in `--proxy`, where the process table exposes them. Default: none
- `proxy_bypass` — Hosts bypassing the proxy, in Chrome's bypass-list syntax. Default: none
- `proxy_username` — Proxy account name, sent as basic auth. Kept here rather than in argv, where the process table would expose it. Default: none
- `proxy_password` — Proxy password, sent as basic auth. Never echoed by `config get` or `config show`. Default: none
- `cdp_proxy_bypass_loopback` — Always bypass loopback when Chrome runs behind `--proxy`. The CDP control channel is loopback, so a proxy that captures it produces a browser that never answers — reported as a Chrome startup timeout, which blames the wrong component. Default: `true`
- `stealth_seed` — Pins the stealth identity so the same fingerprint is reproduced across processes. Absent means a fresh identity each run, which is the default precisely because caching an identity writes it to disk. Default: none
- `http2_enabled` — Offer `h2` in ALPN. ALPN is visible in the clear during the TLS handshake and Chrome always lists `h2`, so a client that offers only `http/1.1` has answered "not a browser" before sending a byte. Default: `true`
- `http2_initial_stream_window_size` — `SETTINGS_INITIAL_WINDOW_SIZE` advertised to the peer. Library defaults are three orders of magnitude away from Chrome's. Default: Chrome's value
- `http2_initial_connection_window_size` — Connection-level flow-control window advertised to the peer. Default: Chrome's value
- `http2_max_header_list_size` — `SETTINGS_MAX_HEADER_LIST_SIZE` advertised to the peer. Default: Chrome's value
- `http2_max_frame_size` — `SETTINGS_MAX_FRAME_SIZE` advertised to the peer. Default: Chrome's value
- `http2_adaptive_window` — Let the HTTP/2 stack resize windows dynamically. Off keeps the advertised values fixed, which is what makes the fingerprint reproducible. Default: Chrome's value
- `robots_user_agent` — User-agent token that `robots.txt` rules are matched against. Set it when stealth sends a browser User-Agent, so the rules evaluated are the ones that apply to the request actually sent. Default: none
- `input_move_steps` — Intermediate pointer positions synthesized for one move (human profile). Default: `24`
- `input_move_gap_ms` — Delay between synthesized pointer positions in milliseconds. Default: `12`
- `input_click_dwell_ms` — Hold time between `mousePressed` and `mouseReleased` in milliseconds. Default: `65`
- `input_key_dwell_ms` — Hold time between `keyDown` and `keyUp` in milliseconds. Default: `45`
- `input_type_delay_ms` — Delay between characters while typing in milliseconds. Default: `95`
- `input_scroll_tick_px` — Scroll distance carried by one synthesized wheel tick in CSS pixels. Default: `100`
- `input_scroll_max_ticks` — Ceiling on the number of wheel ticks one scroll gesture synthesizes. Each tick is a CDP round trip, so without a ceiling the cost of a scroll grows linearly with the distance requested and a large `--delta-y` exhausts the command timeout. Past the ceiling the ticks carry more pixels each; total travel is unchanged and only the granularity degrades. Default: `40`
- `input_target_jitter_px` — Radius of the random offset applied to a click target in CSS pixels. Default: `3`
- `input_scroll_settle_rounds` — Extra rounds allowed to deliver a wheel delta the renderer dropped. Default: `3`
- `support_settle_ms` — Support-thread settle for sync helpers in milliseconds. Default: `80`
- `nav_micro_settle_ms` — Navigation micro-settle after page transitions in milliseconds. Default: `100`


## Screencast and Perf
- `screencast_jpeg_quality` — Screencast CDP JPEG quality in range `1..=100`. Default: `60`
- `screencast_ffmpeg_framerate` — Screencast `ffmpeg` input framerate in frames per second. Default: `10`
- `screencast_start_pump_iters` — Immediate pump iterations after `Page.startScreencast`. Default: `15`
- `screencast_stop_pump_iters` — Drain pump iterations before `Page.stopScreencast`. Default: `40`
- `perf_autostop_settle_ms` — Perf auto-stop settle after load or reload in milliseconds. Default: `500`
- `perf_trace_inner_slice_ms` — Perf trace poll inner slice in milliseconds. Default: `20`
- `perf_trace_outer_slice_ms` — Perf trace outer poll interval in milliseconds. Default: `50`
- `perf_trace_outer_iters` — Perf trace outer poll max iterations. Default: `100`
- `perf_trace_inner_iters` — Perf trace inner drain iterations after complete. Default: `5`


## Heap
- `heap_snapshot_max_bytes` — Offline heap snapshot file size ceiling in bytes. Default: `536870912`
- `heap_max_retainers` — Heap node-op max retainers returned. Default: `200`
- `heap_max_edges` — Heap node-op max edges returned. Default: `200`
- `heap_max_paths` — Heap paths enumeration max paths. Default: `32`
- `heap_max_path_depth` — Heap paths max depth. Default: `8`
- `heap_max_class_nodes` — Heap `class_nodes` list cap. Default: `500`
- `heap_dominator_max_states` — Dominator visited-state ceiling against pathological graphs. Default: `50000`
- `heap_outer_iters` — Heap snapshot outer poll max iterations. Default: `200`
- `heap_inner_iters` — Heap snapshot inner drain iterations after finished. Default: `10`
- `heap_final_iters` — Heap snapshot final drain iterations. Default: `20`


## MITM
- `monitor_diff_max_bytes` — Byte ceiling for the `monitor check --diff-mode` payload. A page rewritten wholesale diffs to the whole page twice, and the caller asked what changed, not for everything. `diff_truncated` says when the ceiling applied, and `added_count` / `removed_count` keep reporting the real size. Default: `65536`
- `mitm_list_limit_max` — MITM list and query max items clamp. Default: `10000`
- `mitm_proxy_seconds_max` — MITM proxy one-shot max window in seconds. Default: `600`
- `mitm_chrome_settle_ms` — MITM Chrome launch settle before navigation in milliseconds. Default: `150`
- `mitm_capture_wait_min_ms` — MITM capture wait floor after navigate in milliseconds. Default: `800`
- `mitm_capture_wait_max_ms` — MITM capture wait ceiling after navigate in milliseconds. Default: `8000`
- `mitm_ws_frames_cap` — Cap on in-memory WebSocket frames per capture process. Default: `500`
- `mitm_ws_preview_chars` — WebSocket text preview truncation in Unicode chars. Default: `256`
- `mitm_ca_cache_size` — MITM dynamic certificate cache size in hosts. Default: `1000`
- `mitm_rebind_attempts` — MITM proxy bind retries when the port is transiently in use. Default: `3`


## Local Files and Roots
- `allowed_roots` — Extra allowed roots for local reads and artifact writes, platform-separated; defaults cover cwd, XDG dirs and temp. Default: none
- `max_json_file_bytes` — Max bytes for JSON or NDJSON script and manifest files. Default: `33554432`
- `max_ndjson_line_bytes` — Max bytes for one NDJSON line in run scripts and traces. Default: `1048576`
- `max_cli_json_payload_bytes` — Max bytes for CLI flag JSON payloads. Default: `4194304`
- `max_sg_file_bytes` — Max bytes for one source file read by `sg-scan` and `sg-rewrite`. Default: `16777216`
- `run_max_include_depth` — Max nesting depth for `run --script` include chains. Default: `16`


## Retry Budgets
- `retry_default_max_attempts` — Default retry max attempts including the first try. Default: `3`
- `retry_base_delay_ms` — Default retry base delay in milliseconds. Default: `50`
- `retry_max_delay_secs` — Default retry max delay in seconds. Default: `2`
- `retry_budget_secs` — Default retry wall budget in seconds. Default: `10`
- `retry_cdp_max_attempts` — CDP retry max attempts. Default: `4`
- `retry_cdp_base_delay_ms` — CDP retry base delay in milliseconds. Default: `100`
- `retry_cdp_max_delay_secs` — CDP retry max delay in seconds. Default: `3`
- `retry_cdp_budget_secs` — CDP retry wall budget in seconds. Default: `15`
- `retry_http_max_attempts` — HTTP scrape retry max attempts. Default: `3`
- `retry_http_base_delay_ms` — HTTP scrape retry base delay in milliseconds. Default: `75`
- `retry_http_max_delay_secs` — HTTP scrape retry max delay in seconds. Default: `2`
- `retry_http_budget_secs` — HTTP scrape retry wall budget in seconds. Default: `12`
- `retry_llm_max_attempts` — LLM HTTP retry max attempts. Default: `2`
- `retry_llm_base_delay_ms` — LLM HTTP retry base delay in milliseconds. Default: `200`
- `retry_llm_max_delay_secs` — LLM HTTP retry max delay in seconds. Default: `4`
- `retry_llm_budget_secs` — LLM HTTP retry wall budget in seconds. Default: `20`


## Viewport and State
- `default_viewport_width` — Default headless Chrome window width when launch options omit the viewport. Default: `1280`
- `default_viewport_height` — Default headless Chrome window height when launch options omit the viewport. Default: `720`
- `state_collect_deadline_secs` — CDP storage collect outer deadline in seconds. Default: `5`
- `state_event_recv_secs` — CDP storage event recv slice in seconds. Default: `2`
- `state_load_settle_ms` — Settle delay after `load_state` navigation in milliseconds. Default: `500`


## Discovering Keys at Runtime
- Enumerate every accepted key on the running binary with the JSON envelope
- `browser-automation-cli --json config list-keys`
- Read one resolved value with `browser-automation-cli --json config get <key>`
- Inspect the whole effective set with `browser-automation-cli --json config show`
- Treat the live output as the source of truth when this document and the binary disagree


## See Also
- [README.md](../README.md)
- [docs/HOW_TO_USE.md](HOW_TO_USE.md)
- [docs/AGENTS.md](AGENTS.md)
