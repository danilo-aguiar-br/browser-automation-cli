# XDG Configuration Keys


## Configuration Contract
- MUST treat this file as the complete inventory of the product XDG configuration surface
- MUST configure this product ONLY through CLI flags and `config init|path|show|get|set|unset|list-keys`
- NEVER treat any environment variable as product configuration; the product reads none
- MUST apply precedence CLI flag first, then the XDG config file, then the built-in default
- MUST discover the live key set with `browser-automation-cli --json config list-keys`
- MUST resolve the config file location with `browser-automation-cli --json config path`
- MUST inspect current values with `browser-automation-cli --json config show` and `config get <KEY>`
- MUST write a value with `browser-automation-cli config set <KEY> <VALUE>`
- MUST restore one key to its built-in default with `browser-automation-cli --json config unset <KEY>`
- MUST know unsetting an already absent key succeeds, so a script never has to know the prior state
- MUST read `Default: none` as a key with no built-in default; behaviour then falls back to the per-command logic
- NEVER invent a key absent from this list or from `config list-keys`


## Core and Identity
- `lang` — Message locale override (en|pt-BR; bare pt rejected). Default: none
- `timeout` — Global timeout seconds. Default: `0`
- `artifacts_dir` — Artifacts output directory. Default: none
- `ignore_robots` — Default robots ignore (flags still required). Default: `false`
- `namespace` — Isolated state namespace. Default: none
- `encryption_key` — Session encryption key material. Default: none
- `color` — ANSI colors on human stderr. Default: none


## Anti-Detection and Identity
- `stealth` — Anti-detection patches before first navigation (--no-stealth opts out). Default: `true`
- `stealth_profile` — Impersonated identity: auto|chrome-linux|chrome-win|chrome-mac. Default: `auto`
- `stealth_seed` — Pin the stealth identity across processes (absent = redrawn per process). Default: none
- `screen` — Default screen `WxH` for device metrics (absent = mirror the viewport). Default: none
- `browser_mode` — Window mode: auto|headed|headless (auto resolves to headless; doctor reports it). Default: `auto`


## Local Logging
- `log_level` — Tracing EnvFilter when argv flags quiet (no RUST_LOG). Default: `error`
- `log_to_file` — Rotated local JSON logs under XDG state (never remote). Default: `false`
- `max_log_files` — Retained rotated log files (1..=90). Default: `14`
- `log_rotation` — Rolling policy: daily|hourly|never. Default: `daily`


## External Binaries
- `chrome_path` — Absolute Chrome/Chromium path. Default: none
- `lighthouse_path` — Absolute lighthouse CLI path. Default: none
- `ffmpeg_path` — Absolute ffmpeg path (optional screencast encode + video convert/to-mp3). Default: none
- `lighthouse_timeout_secs` — Wall-clock lighthouse CLI timeout (seconds, 1..=3600). Default: `300`
- `ffmpeg_timeout_secs` — Wall-clock ffmpeg encode timeout (seconds, 1..=3600). Default: `120`
- `chrome_search_paths` — Ordered Chrome/Chromium discovery paths (platform-separated); empty uses the built-in per-OS layout. Default: none


## LLM
- `openrouter_api_key` — LLM API key (stored 0600). Default: none
- `llm_base_url` — OpenAI-compatible base URL. Default: none
- `llm_model` — Default LLM model id. Default: none
- `llm_http_timeout_secs` — LLM/webhook blocking HTTP timeout (seconds). Default: `60`


## Cache and Redis
- `cache_backend` — sqlite|memory|redis. Default: `sqlite`
- `cache_redis_url` — Redis URL when backend=redis. Default: none
- `redis_allow_remote` — Allow non-loopback Redis hosts (default false). Default: `false`
- `redis_connect_timeout_secs` — Redis TCP connect timeout (seconds). Default: `2`
- `redis_io_timeout_secs` — Redis/RESP stream I/O timeout (seconds). Default: `3`
- `cache_max_resp_bulk_bytes` — Redis RESP bulk string size ceiling (bytes). Default: `16777216`
- `cache_max_resp_line_bytes` — Redis RESP line size ceiling (bytes). Default: `16777216`
- `scrape_http_cache_ttl_secs` — HTTP scrape response L2 cache TTL (seconds). Default: `3600`
- `file_parse_cache_ttl_secs` — Local file-parse L2 cache TTL (seconds). Default: `86400`


## Web Search
- `search_base_url` — HTML search endpoint base (?q= appended). Default: `https://html.duckduckgo.com/html/`
- `user_data_dir` — persistent Chrome profile dir, opt-in; unset keeps residual-zero. Mode 0700 on Unix; `--profile` wins. Default: unset


## Payload Limits and Roots
- `max_json_file_bytes` — Max bytes for JSON/NDJSON script or manifest files. Default: `33554432`
- `max_ndjson_line_bytes` — Max bytes for one NDJSON line (run scripts / traces). Default: `1048576`
- `max_cli_json_payload_bytes` — Max bytes for CLI flag JSON payloads. Default: `4194304`
- `max_sg_file_bytes` — Max bytes for one source file read by sg scan/rewrite. Default: `16777216`
- `max_urls_file_bytes` — Max bytes for the batch-scrape --urls-file list. Default: `8388608`
- `run_max_include_depth` — Max nesting depth for run --script include chains. Default: `16`
- `allowed_roots` — Extra allowed roots for local reads and artifact writes (platform-separated); defaults cover cwd, XDG dirs and temp. Default: none


## Visual Capture and Screencast
- `default_jpeg_quality` — JPEG quality 1..=100 when grab omits --quality. Default: `80`
- `screencast_jpeg_quality` — Screencast CDP JPEG quality 1..=100. Default: `60`
- `screencast_ffmpeg_framerate` — Screencast ffmpeg input framerate (frames per second). Default: `10`
- `screencast_start_pump_iters` — Screencast start: immediate pump iterations after Page.startScreencast. Default: `15`
- `screencast_stop_pump_iters` — Screencast stop: drain pump iterations before Page.stopScreencast. Default: `40`


## Interaction and Waiting
- `event_pump_slice_ms` — Wait/eval event pump slice (milliseconds). Default: `50`
- `interact_settle_ms` — UI settle delay after click/type/extension (ms). Default: `200`
- `dialog_settle_ms` — Max wait after JS dialog answer for javascriptDialogClosed (ms). Default: `2000`
- `network_idle_window_ms` — Quiet window for wait --network-idle (milliseconds). Default: `500`
- `dom_stable_window_ms` — Quiet window for wait --dom-stable-ms (milliseconds). Default: `500`
- `drag_move_steps` — Intermediate mouse positions synthesized for one HTML5 drag. Default: `6`
- `drag_move_gap_ms` — Delay between synthesized drag positions (milliseconds). Default: `16`
- `eval_drain_slice_ms` — Eval drain slice while waiting for Runtime.evaluate results (milliseconds). Default: `40`
- `support_settle_ms` — Support-thread settle for sync helpers (milliseconds). Default: `80`
- `nav_micro_settle_ms` — Navigation micro-settle after page transitions (milliseconds). Default: `100`


## Input Kinematics
- `input_profile` — Default input shaping: human|direct. Default: `human`
- `input_move_steps` — Intermediate pointer positions synthesized for one move (human profile). Default: `24`
- `input_move_gap_ms` — Delay between synthesized pointer positions (milliseconds). Default: `12`
- `input_click_dwell_ms` — Hold time between mousePressed and mouseReleased (milliseconds). Default: `65`
- `input_key_dwell_ms` — Hold time between keyDown and keyUp (milliseconds). Default: `45`
- `input_type_delay_ms` — Delay between characters while typing (milliseconds). Default: `95`
- `input_scroll_tick_px` — Scroll distance carried by one synthesized wheel tick (CSS pixels). Default: `100`
- `input_scroll_max_ticks` — Ceiling on wheel ticks per scroll gesture (one CDP round trip each). Default: `40`
- `input_target_jitter_px` — Radius of the random offset applied to a click target (CSS pixels). Default: `3`
- `input_scroll_settle_rounds` — Extra rounds allowed to deliver a wheel delta the renderer dropped. Default: `3`
- `input_timing_distribution` — Shape of the dispersion around input delays: lognormal|normal|uniform; governs the fast rhythm only, and the long-pause tail is `input_word_pause_permille`. Default: `lognormal`
- `input_move_steps_stddev` — Standard deviation of the per-gesture pointer sample budget. Default: `6`
- `input_move_gap_stddev_ms` — Standard deviation of the delay between pointer positions (milliseconds). Default: `5`
- `input_click_dwell_stddev_ms` — Standard deviation of the press-to-release hold (milliseconds). Default: `26`
- `input_key_dwell_stddev_ms` — Standard deviation of the keyDown-to-keyUp hold (milliseconds). Default: `18`
- `input_type_delay_stddev_ms` — Standard deviation of the delay between characters (milliseconds). Default: `40`
- `input_scroll_tick_stddev_px` — Standard deviation of the distance one wheel tick carries (CSS pixels). Default: `25`
- `input_word_pause_ms` — Mean of the extra pause taken at a word or sentence boundary (milliseconds). Default: `320`
- `input_word_pause_permille` — Chance in a thousand that a word boundary earns a long pause. Default: `120`
- `input_typo_permille` — Chance in a thousand that a character is mistyped, erased with `Backspace` and retyped; the field still ends up holding the requested text. `0` by default because this one changes the CHARACTER STREAM the page reads, not just the timing. Default: `0`


## CDP and Chrome Session
- `cdp_connection_probe_timeout_secs` — CDP Browser.getVersion liveness probe timeout (seconds). Default: `3`
- `cdp_discovery_max_body_bytes` — Max CDP discovery HTTP body bytes (/json/version, /json/list). Default: `1048576`
- `cdp_event_broadcast_capacity` — Process-local CDP event broadcast channel capacity. Default: `4096`
- `cdp_event_drain_poll_ms` — CDP event drain poll slice during navigation wait (milliseconds). Default: `100`
- `cdp_network_idle_settle_ms` — CDP network-idle settle window (milliseconds). Default: `500`
- `cdp_target_event_wait_ms` — CDP target event short wait (milliseconds). Default: `600`
- `cdp_discovery_timeout_secs` — CDP HTTP discovery timeout for /json/version probes (seconds). Default: `2`
- `event_tracker_max_entries` — In-memory console/network tracker ring size per page session. Default: `1000`
- `capture_preserved_rings` — Navigation boundaries kept for console/net --include-preserved. Default: `3`
- `chrome_default_timeout_ms` — Default per-operation timeout for the Chrome engine (milliseconds). Default: `25000`
- `extension_attach_poll_ms` — Extension attach poll slice (milliseconds). Default: `150`
- `extension_attach_poll_iters` — Extension attach poll iterations; slice x iterations is the total wait. Default: `20`


## HTTP and Network Security
- `http_ssrf_mode` — HTTP SSRF policy: strict|allow_loopback|off. Default: `strict`
- `http_timeout_secs` — Shared HTTP client total timeout (seconds). Default: `30`
- `http_connect_timeout_secs` — HTTP connect-phase timeout (seconds). Default: `10`
- `http_redirect_max` — Max HTTP redirects followed by product clients. Default: `10`
- `http_pool_max_idle_per_host` — reqwest pool max idle connections per host. Default: `4`


## Egress Proxy
- `proxy_url` — Egress proxy for Chrome and the HTTP engine. Default: none
- `proxy_bypass` — Hosts bypassing the proxy (Chrome bypass-list syntax). Default: none
- `proxy_username` — Proxy username (XDG only; argv is visible in the process table). Default: none
- `proxy_password` — Proxy password (XDG only; argv is visible in the process table). Default: none
- `cdp_proxy_bypass_loopback` — Always bypass loopback under --proxy so the CDP control channel survives. Default: `true`


## HTTP/2 Fingerprint
- `http2_enabled` — Negotiate HTTP/2 on the shared HTTP client (Chrome always offers h2). Default: `true`
- `http2_initial_stream_window_size` — HTTP/2 SETTINGS_INITIAL_WINDOW_SIZE advertised to the peer. Default: `6291456`
- `http2_initial_connection_window_size` — HTTP/2 connection-level flow-control window. Default: `15663105`
- `http2_max_header_list_size` — HTTP/2 SETTINGS_MAX_HEADER_LIST_SIZE. Default: `262144`
- `http2_max_frame_size` — HTTP/2 SETTINGS_MAX_FRAME_SIZE (16384..=16777215). Default: `16384`
- `http2_adaptive_window` — Allow the HTTP/2 window to resize at runtime (off keeps the fingerprint constant). Default: `false`


## Robots
- `robots_loopback_exempt` — Loopback hosts skip robots.txt (set false to enforce against localhost). Default: `true`
- `robots_user_agent` — User-agent token robots.txt rules are matched against. Default: none
- `robots_probe_timeout_secs` — robots.txt request timeout (seconds). Default: `5`
- `robots_max_body_bytes` — Max robots.txt body bytes (anti-OOM). Default: `524288`


## Image and SVG
- `image_max_input_bytes` — Max bytes for local image decode/convert/resize input. Default: `32000000`
- `image_max_pixels` — Max width*height for image decode (anti-bomb). Default: `64000000`
- `image_default_format` — Default image convert format: png|jpeg|webp|gif. Default: `png`
- `image_default_quality` — Default lossy quality 1..=100 for image convert/resize. Default: `85`
- `image_download_max_bytes` — Max HTTP body bytes for image download. Default: `32000000`
- `image_avif_speed` — AVIF encoder speed 1..=10 (1 slowest/best); needs the image-avif feature. Default: `6`
- `svg_max_bytes` — Max SVG source bytes accepted before rasterisation. Default: `4000000`
- `svg_max_depth` — Max XML nesting depth accepted in an SVG source. Default: `128`
- `svg_max_entities` — Max <!ENTITY> declarations tolerated in an SVG DTD (0 = reject any). Default: `0`
- `gif_max_frames` — Max animation frames decoded from a GIF. Default: `2000`


## Video and Manifests
- `video_max_input_bytes` — Max bytes for video stdin materialization / path pre-check. Default: `512000000`
- `video_download_max_bytes` — Max HTTP body bytes for video download. Default: `512000000`
- `video_default_container` — Default video convert container: mp4|webm|mkv|mov|avi|m4v. Default: `mp4`
- `video_default_crf` — Default CRF 1..=51 for lossy video re-encode. Default: `23`
- `video_default_audio_bitrate` — Default bitrate for video to-mp3 (e.g. 192k). Default: `192k`
- `manifest_max_bytes` — Max bytes accepted for an HLS or DASH manifest body. Default: `8000000`
- `manifest_max_variants` — Max variant/representation entries emitted per manifest envelope. Default: `500`


## Audio
- `audio_max_input_bytes` — Max bytes for audio stdin materialization / path pre-check. Default: `256000000`
- `audio_download_max_bytes` — Max HTTP body bytes for audio download. Default: `256000000`
- `audio_default_format` — Default audio convert format: mp3|m4a|ogg|opus|flac|wav|aac. Default: `mp3`
- `audio_default_bitrate` — Default bitrate for lossy audio encode (e.g. 192k). Default: `192k`


## Scrape Crawl and Map
- `scrape_max_body_bytes` — Max HTTP scrape body bytes. Default: `5000000`
- `browser_scrape_max_body_bytes` — Max body bytes for browser-engine scrape helpers. Default: `2000000`
- `scrape_max_text_chars` — Max text/markdown chars in scrape envelopes (0=no cap). Default: `32768`
- `scrape_min_delay_ms` — Floor delay between same-origin GETs (ms). Default: `0`
- `scrape_honor_meta_robots` — Honor meta robots / X-Robots-Tag noindex. Default: `true`
- `scrape_honor_nofollow` — Skip rel=nofollow links in crawl discovery. Default: `true`
- `scrape_use_sitemap` — Prefer sitemap.xml when mapping a site. Default: `true`
- `scrape_default_engine` — Default scrape engine when CLI omits --engine (http|browser). Default: `http`
- `scrape_delay_jitter_ratio` — Politeness delay jitter ratio 0.0..=1.0 (0=off). Default: `0.2`
- `scrape_summary_chars` — Max chars for scrape format summary. Default: `400`
- `scrape_feed_max_entries` — Max entries kept by scrape format feed (RSS/Atom/JSON Feed). Default: `50`
- `scrape_follow_rel_next` — Follow rel=next pagination links during crawl. Default: `false`
- `scrape_dedup_similar` — Collapse near-duplicate pages by content similarity in crawl/batch-scrape. Default: `false`
- `scrape_dedup_similar_distance` — SimHash Hamming distance (0..=64) under which pages are near-duplicates. Default: `3`
- `scrape_sitemap_max_bytes` — Max sitemap body bytes. Default: `2000000`
- `scrape_charset_peek_bytes` — Charset sniffing peek window (bytes). Default: `4096`
- `scrape_crawl_limit_max` — Max crawl page budget (anti-DoS clamp for --limit). Default: `500`
- `scrape_crawl_max_depth` — Max BFS depth for crawl/map. Default: `10`
- `scrape_search_limit_max` — Max search result budget (anti-DoS clamp). Default: `50`
- `scrape_max_parse_bytes` — Max local file parse size before reject (bytes). Default: `50000000`
- `scrape_no_cache` — Ignore the response cache on read and always fetch from origin. Default: `false`
- `monitor_diff_max_bytes` — Byte ceiling for the `monitor check --diff-mode` payload. Default: `65536`


## Operator Webhook
- `webhook_post_timeout_secs` — Operator webhook POST timeout (seconds). Default: `15`
- `webhook_retry_base_delay_ms` — Webhook retry base delay (milliseconds; doubles each attempt). Default: `50`
- `webhook_max_attempts` — Webhook max attempts (inclusive of first try). Default: `3`


## Heap
- `heap_snapshot_max_bytes` — Offline heap snapshot file size ceiling (bytes). Default: `536870912`
- `heap_max_retainers` — Heap node-op max retainers returned. Default: `200`
- `heap_max_edges` — Heap node-op max edges returned. Default: `200`
- `heap_max_paths` — Heap paths enumeration max paths. Default: `32`
- `heap_max_path_depth` — Heap paths max depth. Default: `8`
- `heap_max_class_nodes` — Heap class_nodes list cap. Default: `500`
- `heap_dominator_max_states` — Dominator visited-state ceiling (anti-pathological graphs). Default: `50000`
- `heap_outer_iters` — Heap snapshot outer poll max iterations. Default: `200`
- `heap_inner_iters` — Heap snapshot inner drain iterations after finished. Default: `10`
- `heap_final_iters` — Heap snapshot final drain iterations. Default: `20`


## Lifecycle and Residual
- `browser_close_wait_secs` — Browser.close / process wait budget during FINALIZE (seconds). Default: `5`
- `chrome_startup_timeout_secs` — Chrome self-spawn CDP readiness wait (seconds). Default: `20`
- `residual_orphan_min_age_secs` — Age floor before a dead-owner marker profile is collectable (seconds). Default: `60`
- `platform_child_wait_secs` — Platform child wait deadline (seconds). Default: `5`
- `platform_child_poll_ms` — Child-process exit poll interval during FINALIZE (milliseconds). Default: `50`
- `shutdown_deadline_secs` — Shutdown hard deadline waiting for browser exit (seconds). Default: `30`
- `chrome_legacy_oxide_launch` — Launch Chrome via chromiumoxide instead of the self-spawn path (stabilization fallback; loses the residual kill target). Default: `false`
- `default_viewport_width` — Default headless Chrome window width (`--window-size`) when launch options omit viewport. Default: `1920`
- `default_viewport_height` — Default headless Chrome window height (`--window-size`) when launch options omit viewport. Default: `1080`


## Lightpanda
- `lightpanda_startup_timeout_secs` — Lightpanda process startup wait (seconds). Default: `10`
- `lightpanda_session_timeout_secs` — Lightpanda --timeout session max (seconds, 1..=604800). Default: `604800`
- `lightpanda_poll_interval_ms` — Lightpanda CDP readiness poll interval (milliseconds). Default: `100`
- `lightpanda_discovery_timeout_ms` — Per-probe CDP discovery timeout while waiting for Lightpanda (milliseconds). Default: `500`
- `lightpanda_max_log_lines` — Bounded Lightpanda launch log ring (lines per stream). Default: `40`
- `lightpanda_ready_slice_ms` — Drain slice after Lightpanda child exit before snapshotting logs (milliseconds). Default: `25`
- `lightpanda_cdp_connect_timeout_secs` — Lightpanda CDP connect attempt timeout (seconds). Default: `5`
- `lightpanda_target_init_timeout_secs` — Lightpanda target init wait after connect (seconds). Default: `10`


## MITM
- `mitm_list_limit_max` — MITM list/query max items clamp. Default: `10000`
- `mitm_proxy_seconds_max` — MITM proxy one-shot max window (seconds). Default: `600`
- `mitm_chrome_settle_ms` — MITM Chrome launch settle before navigation (milliseconds). Default: `150`
- `mitm_capture_wait_min_ms` — MITM capture wait floor after navigate (milliseconds). Default: `800`
- `mitm_capture_wait_max_ms` — MITM capture wait ceiling after navigate (milliseconds). Default: `8000`
- `mitm_ws_frames_cap` — Cap on in-memory WebSocket frames per capture process. Default: `500`
- `mitm_ws_preview_chars` — WebSocket text preview truncation (Unicode chars). Default: `256`
- `mitm_ca_cache_size` — MITM dynamic certificate cache size (hosts). Default: `1000`
- `mitm_rebind_attempts` — MITM proxy bind retries when the port is transiently in use. Default: `3`


## Perf
- `perf_autostop_settle_ms` — Perf auto-stop settle after load/reload (milliseconds). Default: `500`
- `perf_trace_inner_slice_ms` — Perf trace poll inner slice (milliseconds). Default: `20`
- `perf_trace_outer_slice_ms` — Perf trace outer poll interval (milliseconds). Default: `50`
- `perf_trace_outer_iters` — Perf trace outer poll max iterations. Default: `100`
- `perf_trace_inner_iters` — Perf trace inner drain iterations after complete. Default: `5`


## Storage State
- `state_collect_deadline_secs` — CDP storage collect outer deadline (seconds). Default: `5`
- `state_event_recv_secs` — CDP storage event recv slice (seconds). Default: `2`
- `state_load_settle_ms` — Settle delay after load_state navigation (milliseconds). Default: `500`


## Retry
- `retry_default_max_attempts` — Default retry max attempts (inclusive of first try). Default: `3`
- `retry_base_delay_ms` — Default retry base delay (milliseconds). Default: `50`
- `retry_max_delay_secs` — Default retry max delay (seconds). Default: `2`
- `retry_budget_secs` — Default retry wall budget (seconds). Default: `10`
- `retry_cdp_max_attempts` — CDP retry max attempts. Default: `4`
- `retry_cdp_base_delay_ms` — CDP retry base delay (milliseconds). Default: `100`
- `retry_cdp_max_delay_secs` — CDP retry max delay (seconds). Default: `3`
- `retry_cdp_budget_secs` — CDP retry wall budget (seconds). Default: `15`
- `retry_http_max_attempts` — HTTP scrape retry max attempts. Default: `3`
- `retry_http_base_delay_ms` — HTTP scrape retry base delay (milliseconds). Default: `75`
- `retry_http_max_delay_secs` — HTTP scrape retry max delay (seconds). Default: `2`
- `retry_http_budget_secs` — HTTP scrape retry wall budget (seconds). Default: `12`
- `retry_llm_max_attempts` — LLM HTTP retry max attempts. Default: `2`
- `retry_llm_base_delay_ms` — LLM HTTP retry base delay (milliseconds). Default: `200`
- `retry_llm_max_delay_secs` — LLM HTTP retry max delay (seconds). Default: `4`
- `retry_llm_budget_secs` — LLM HTTP retry wall budget (seconds). Default: `20`


## Canonical Reference
- MUST treat `docs/CONFIGURATION.md` in the repository as the canonical product reference for these keys
- MUST use this file as the operational index of the skill and the canonical document for normative detail
- MUST re-check `docs/CONFIGURATION.md` and `config list-keys --json` when a key here disagrees with the live binary
