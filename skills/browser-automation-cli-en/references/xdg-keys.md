# browser-automation-cli XDG Keys


## Configuration Contract
- MUST configure the product ONLY through CLI flags and `config`, and NEVER through an environment variable, `export` or `.env`
- MUST apply precedence CLI flag, then value stored in XDG, then built-in default, because the flag ALWAYS beats XDG
- MUST run `browser-automation-cli --json config init` to create the XDG layout and the default `config.toml`
- MUST run `browser-automation-cli --json config path` to resolve the config file and every XDG directory, and NEVER invent a path
- MUST run `browser-automation-cli --json config list-keys` to discover every live key with default and description
- MUST run `browser-automation-cli --json config show` to read the stored values
- MUST run `browser-automation-cli --json config get <KEY>` to read the STORED value of one key, and omit the key to dump them all
- MUST know that `config get` on an unwritten key returns `null`, and NEVER treat that `null` as the value in force
- MUST read the value in force of an unwritten key from the `default` that `config list-keys` reports
- MUST run `browser-automation-cli --json config set <KEY> <VALUE>`, which the binary validates per key before persisting
- MUST run `browser-automation-cli --json config unset <KEY>` to restore the built-in default, which also succeeds when the key is already absent
- NEVER treat `config set <KEY> ""` as a restore, except in `user_data_dir`, where the empty string clears the opt-in
- MUST store proxy credentials ONLY with `config set proxy_username` and `config set proxy_password`, and NEVER pass them in argv
- MUST store secrets with `config set encryption_key` and `config set openrouter_api_key`, and NEVER log the value
- MUST read an `unset` default as a key with no built-in value, where the command logic decides the behavior
- NEVER invent a key outside `config list-keys`, and NEVER refuse a key from this list from memory


## Core and Identity
- `lang` default `unset` — MUST pin the message locale to `en` or `pt-BR`, and NEVER store bare `pt`, which the binary rejects
- `timeout` default `0` — MUST pin the global timeout in seconds
- `artifacts_dir` default `unset` — MUST point the default artifact output directory
- `ignore_robots` default `false` — NEVER treat it as a robots bypass, because BOTH robots flags stay REQUIRED
- `namespace` default `unset` — MUST isolate local state under its own namespace
- `encryption_key` default `unset` — MUST keep the session encryption key material ONLY in XDG, and NEVER log the value
- `color` default `unset` — MUST turn ANSI colors on human stderr on or off


## Anti-Detection and Window
- `stealth` default `true` — MUST stay on to mask automation markers, and ALWAYS use `--no-stealth` to turn it off for one run only
- `stealth_profile` default `auto` — MUST pick `auto`, `chrome-linux`, `chrome-win` or `chrome-mac`, and NEVER declare a platform other than the host
- `stealth_seed` default `unset` — MUST pin the stealth identity across processes, because when unset it is redrawn per process
- `screen` default `unset` — MUST pin the default `WxH` screen for device metrics, because when unset it mirrors the viewport
- `browser_mode` default `auto` — MUST be `auto`, `headed` or `headless`, knowing `auto` resolves to headed inside the private Xvfb ONLY on Linux with `Xvfb` on PATH and without `--no-xvfb`, and to headless everywhere else
- `input_profile` default `human` — MUST be `human` to pace pointer and keyboard, or `direct` for unshaped input


## Local Logging
- `log_level` default `error` — MUST pin the tracing level used when no verbosity flag is passed
- `log_to_file` default `false` — MUST turn on rotated local JSON logs under XDG state, which NEVER leave the machine
- `max_log_files` default `14` — MUST cap retained log files between 1 and 90
- `log_rotation` default `daily` — MUST pick `daily`, `hourly` or `never` rotation


## External Binaries
- `chrome_path` default `unset` — MUST point the absolute path of the system Chrome or Chromium
- `chrome_search_paths` default `unset` — MUST list Chrome discovery paths in order with the platform separator, and when unset the built-in per-OS layout applies
- `lighthouse_path` default `unset` — MUST point the absolute path of the lighthouse CLI
- `lighthouse_timeout_secs` default `300` — MUST cap the lighthouse CLI wall clock between 1 and 3600 seconds
- `ffmpeg_path` default `unset` — MUST point the absolute ffmpeg path used by screencast, video convert and `to-mp3`
- `ffmpeg_timeout_secs` default `120` — MUST cap the ffmpeg encode wall clock between 1 and 3600 seconds


## LLM
- `openrouter_api_key` default `unset` — MUST keep the LLM API key ONLY in XDG, stored with mode 0600, and NEVER log the value
- `llm_base_url` default `unset` — MUST point the OpenAI-compatible base URL
- `llm_model` default `unset` — MUST pin the default LLM model id
- `llm_http_timeout_secs` default `60` — MUST cap the blocking LLM and webhook HTTP call in seconds


## Cache and Redis
- `cache_backend` default `sqlite` — MUST pick `sqlite`, `memory` or `redis`, and NEVER pick `redis` without `cache_redis_url`
- `cache_redis_url` default `unset` — MUST point the `redis://` URL when the backend is `redis`, and NEVER use `rediss://`
- `redis_allow_remote` default `false` — NEVER turn it on without an explicit decision, because it allows Redis hosts outside loopback
- `redis_connect_timeout_secs` default `2` — MUST cap the Redis TCP connect in seconds
- `redis_io_timeout_secs` default `3` — MUST cap Redis stream I/O in seconds
- `cache_max_resp_bulk_bytes` default `16777216` — MUST cap the Redis RESP bulk string in bytes
- `cache_max_resp_line_bytes` default `16777216` — MUST cap the Redis RESP line in bytes
- `scrape_http_cache_ttl_secs` default `3600` — MUST pin in seconds the lifetime of the HTTP scrape response cache
- `file_parse_cache_ttl_secs` default `86400` — MUST pin in seconds the lifetime of the local file-parse cache


## Search Endpoint
- `search_base_url` default `https://html.duckduckgo.com/html/` — MUST point the HTML search endpoint base, to which the binary appends `?q=`


## Payload Limits and Roots
- `max_json_file_bytes` default `33554432` — MUST cap in bytes the JSON or NDJSON script and manifest file
- `max_ndjson_line_bytes` default `1048576` — MUST cap in bytes one NDJSON line of a `run` script or trace
- `max_cli_json_payload_bytes` default `4194304` — MUST cap in bytes the JSON payload passed in a flag
- `max_sg_file_bytes` default `16777216` — MUST cap in bytes the source file read by `sg-scan` and `sg-rewrite`
- `max_urls_file_bytes` default `8388608` — MUST cap in bytes the `batch-scrape` `--urls-file` list
- `run_max_include_depth` default `16` — MUST cap include nesting depth in `run --script`
- `allowed_roots` default `unset` — MUST add allowed read and write roots with the platform separator, and ALWAYS prefer this key to `--allow-outside-roots`


## Visual Capture and Screencast
- `default_jpeg_quality` default `80` — MUST pin JPEG quality between 1 and 100 when `grab` omits `--quality`
- `screencast_jpeg_quality` default `60` — MUST pin screencast JPEG quality between 1 and 100
- `screencast_ffmpeg_framerate` default `10` — MUST pin in frames per second the screencast ffmpeg input
- `screencast_start_pump_iters` default `15` — MUST pin the pump iterations right after screencast start
- `screencast_stop_pump_iters` default `40` — MUST pin the drain iterations before screencast stop


## Interaction and Waiting
- `event_pump_slice_ms` default `50` — MUST pin in milliseconds the event pump slice in `wait` and `eval`
- `interact_settle_ms` default `200` — MUST pin in milliseconds the page settle after click, typing or extension
- `dialog_settle_ms` default `2000` — MUST pin in milliseconds the max wait for the answered JS dialog to close, and NEVER add an artificial wait
- `network_idle_window_ms` default `500` — MUST pin in milliseconds the quiet window of `wait --network-idle`
- `dom_stable_window_ms` default `500` — MUST pin in milliseconds the quiet window of `wait --dom-stable-ms`
- `drag_move_steps` default `6` — MUST pin the intermediate mouse positions in one HTML5 drag
- `drag_move_gap_ms` default `16` — MUST pin in milliseconds the gap between drag positions
- `eval_drain_slice_ms` default `40` — MUST pin in milliseconds the drain slice while `eval` waits for its result
- `support_settle_ms` default `80` — MUST pin in milliseconds the settle of synchronous helpers
- `nav_micro_settle_ms` default `100` — MUST pin in milliseconds the micro-settle after a page transition


## Input Kinematics
- `input_timing_distribution` default `lognormal` — MUST pick `lognormal`, `normal` or `uniform` for the fast rhythm, knowing the long-pause tail is `input_word_pause_permille`
- `input_move_steps` default `24` — MUST pin the intermediate pointer positions per move in the `human` profile
- `input_move_gap_ms` default `12` — MUST pin in milliseconds the gap between pointer positions
- `input_click_dwell_ms` default `65` — MUST pin in milliseconds the hold between button press and release
- `input_key_dwell_ms` default `45` — MUST pin in milliseconds the hold between key down and key up
- `input_type_delay_ms` default `95` — MUST pin in milliseconds the delay between typed characters
- `input_scroll_tick_px` default `100` — MUST pin in CSS pixels the distance of one wheel tick
- `input_scroll_max_ticks` default `40` — MUST cap wheel ticks per scroll gesture
- `input_target_jitter_px` default `3` — MUST pin in CSS pixels the radius of the random click target offset
- `input_scroll_settle_rounds` default `3` — MUST pin the extra rounds to deliver a dropped wheel delta
- `input_move_steps_stddev` default `6` — MUST pin the standard deviation of pointer samples per gesture
- `input_move_gap_stddev_ms` default `5` — MUST pin in milliseconds the standard deviation of the gap between pointer positions
- `input_click_dwell_stddev_ms` default `26` — MUST pin in milliseconds the standard deviation of the click hold
- `input_key_dwell_stddev_ms` default `18` — MUST pin in milliseconds the standard deviation of the key hold
- `input_type_delay_stddev_ms` default `40` — MUST pin in milliseconds the standard deviation of the delay between characters
- `input_scroll_tick_stddev_px` default `25` — MUST pin in CSS pixels the standard deviation of the wheel tick distance
- `input_word_pause_ms` default `320` — MUST pin in milliseconds the mean extra pause at a word or sentence boundary
- `input_word_pause_permille` default `120` — MUST pin the per-thousand chance that a word boundary gets a long pause
- `input_typo_permille` default `0` — NEVER turn it on without an explicit decision, because the typo corrected with `Backspace` changes the character stream the page reads


## CDP and Chrome Session
- `cdp_connection_probe_timeout_secs` default `3` — MUST cap in seconds the CDP liveness probe
- `cdp_discovery_max_body_bytes` default `1048576` — MUST cap in bytes the CDP discovery body, knowing only Lightpanda readiness reads this key
- `cdp_discovery_timeout_secs` default `2` — NEVER expect an effect on Chrome launch, which speaks CDP over a pipe, and Lightpanda uses `lightpanda_discovery_timeout_ms`
- `cdp_event_broadcast_capacity` default `4096` — MUST pin the capacity of the local CDP event channel
- `cdp_event_drain_poll_ms` default `100` — MUST pin in milliseconds the event drain slice during navigation wait
- `cdp_network_idle_settle_ms` default `500` — MUST pin in milliseconds the CDP network-idle settle
- `cdp_target_event_wait_ms` default `600` — MUST pin in milliseconds the short wait for a target event
- `event_tracker_max_entries` default `1000` — MUST move the console and network buffer ceiling ONLY through this key, and ALWAYS read `dropped_oldest`
- `capture_preserved_rings` default `3` — MUST pin how many navigation boundaries `--include-preserved` keeps for console and network
- `chrome_default_timeout_ms` default `25000` — MUST pin in milliseconds the default per-operation Chrome timeout
- `extension_attach_poll_ms` default `150` — MUST pin in milliseconds the extension attach poll slice
- `extension_attach_poll_iters` default `20` — MUST pin the extension attach iterations, knowing slice times iterations is the total wait


## HTTP and Network Security
- `http_ssrf_mode` default `strict` — MUST keep `strict`, use `allow_loopback` only for a local target, and NEVER store `off` without an explicit decision
- `http_timeout_secs` default `30` — MUST cap in seconds the total HTTP client time
- `http_connect_timeout_secs` default `10` — MUST cap in seconds the HTTP connect phase
- `http_redirect_max` default `10` — MUST cap the HTTP redirects followed
- `http_pool_max_idle_per_host` default `4` — MUST cap idle HTTP connections per host


## Egress Proxy
- `proxy_url` default `unset` — MUST point the `http`, `https` or `socks5` egress proxy for Chrome and the HTTP engine
- `proxy_bypass` default `unset` — MUST list the hosts that skip the proxy in Chrome bypass-list syntax
- `proxy_username` default `unset` — MUST keep the proxy username ONLY in XDG, and NEVER in argv, which the process table exposes
- `proxy_password` default `unset` — MUST keep the proxy password ONLY in XDG, NEVER in argv, and NEVER log the value
- `cdp_proxy_bypass_loopback` default `true` — MUST stay on so the loopback CDP control channel survives under `--proxy`


## HTTP/2 Fingerprint
- `http2_enabled` default `true` — MUST stay on so the HTTP engine negotiates HTTP/2 like Chrome
- `http2_initial_stream_window_size` default `6291456` — MUST pin the `SETTINGS_INITIAL_WINDOW_SIZE` advertised to the peer
- `http2_initial_connection_window_size` default `15663105` — MUST pin the HTTP/2 connection flow-control window
- `http2_max_header_list_size` default `262144` — MUST pin the HTTP/2 `SETTINGS_MAX_HEADER_LIST_SIZE`
- `http2_max_frame_size` default `16384` — MUST pin `SETTINGS_MAX_FRAME_SIZE` between 16384 and 16777215
- `http2_adaptive_window` default `false` — MUST stay off so the HTTP/2 fingerprint stays constant


## Robots
- `robots_loopback_exempt` default `true` — MUST store `false` to enforce robots.txt against localhost too
- `robots_user_agent` default `unset` — MUST pin the user-agent token matched against robots.txt rules
- `robots_probe_timeout_secs` default `5` — MUST cap in seconds the robots.txt request
- `robots_max_body_bytes` default `524288` — MUST cap in bytes the robots.txt body


## Image and SVG
- `image_max_input_bytes` default `32000000` — MUST cap in bytes the local image to decode, convert or resize
- `image_max_pixels` default `64000000` — MUST cap width times height on decode against image bombs
- `image_default_format` default `png` — MUST pick `png`, `jpeg`, `webp` or `gif` as the default convert format
- `image_default_quality` default `85` — MUST pin lossy quality between 1 and 100 for convert and resize
- `image_download_max_bytes` default `32000000` — MUST cap in bytes the HTTP body of `image download`
- `image_avif_speed` default `6` — NEVER expect an effect without AVIF encoding in the binary, and NEVER request AVIF output
- `svg_max_bytes` default `4000000` — MUST cap in bytes the accepted SVG source
- `svg_max_depth` default `128` — MUST cap the XML nesting depth of an SVG source
- `svg_max_entities` default `0` — MUST keep `0` to reject every `<!ENTITY>` declaration in SVG
- `gif_max_frames` default `2000` — MUST cap the frames decoded from a GIF


## Video and Manifests
- `video_max_input_bytes` default `512000000` — MUST cap in bytes the video read from stdin or path
- `video_download_max_bytes` default `512000000` — MUST cap in bytes the HTTP body of `video download`
- `video_default_container` default `mp4` — MUST pick `mp4`, `webm`, `mkv`, `mov`, `avi` or `m4v` as the default container
- `video_default_crf` default `23` — MUST pin CRF between 1 and 51 for lossy re-encode
- `video_default_audio_bitrate` default `192k` — MUST pin the default `video to-mp3` bitrate
- `manifest_max_bytes` default `8000000` — MUST cap in bytes the HLS or DASH manifest body
- `manifest_max_variants` default `500` — MUST cap the variants emitted per `video manifest` envelope


## Audio
- `audio_max_input_bytes` default `256000000` — MUST cap in bytes the audio read from stdin or path
- `audio_download_max_bytes` default `256000000` — MUST cap in bytes the HTTP body of `audio download`
- `audio_default_format` default `mp3` — MUST pick `mp3`, `m4a`, `ogg`, `opus`, `flac`, `wav` or `aac` as the default format
- `audio_default_bitrate` default `192k` — MUST pin the default lossy audio encode bitrate


## Scrape Crawl and Map
- `scrape_default_engine` default `http` — MUST keep `http` as the cheap engine, and ALWAYS pass `--engine browser` only for a page that depends on JavaScript
- `scrape_max_body_bytes` default `5000000` — MUST cap in bytes the HTTP scrape body
- `browser_scrape_max_body_bytes` default `2000000` — MUST cap in bytes the body of browser-engine scrape helpers
- `scrape_max_text_chars` default `32768` — MUST cap text and markdown characters in the envelope, knowing `0` removes the ceiling
- `scrape_min_delay_ms` default `0` — MUST pin in milliseconds the politeness floor between same-origin GETs
- `scrape_delay_jitter_ratio` default `0.2` — MUST pin between 0.0 and 1.0 the politeness delay jitter, knowing `0` turns it off
- `scrape_honor_meta_robots` default `true` — MUST stay on to honor meta robots and `X-Robots-Tag` noindex
- `scrape_honor_nofollow` default `true` — MUST stay on to skip `rel=nofollow` links in crawl discovery
- `scrape_use_sitemap` default `true` — MUST stay on to prefer sitemap.xml when mapping a site
- `scrape_follow_rel_next` default `false` — MUST turn on so crawl follows `rel=next` pagination
- `scrape_dedup_similar` default `false` — MUST turn on to collapse near-duplicate pages in `crawl` and `batch-scrape`
- `scrape_dedup_similar_distance` default `3` — MUST pin between 0 and 64 the SimHash distance under which a page is a near-duplicate
- `scrape_summary_chars` default `400` — MUST cap the characters of the `summary` format
- `scrape_feed_max_entries` default `50` — MUST cap the entries of the RSS, Atom and JSON Feed `feed` format
- `scrape_sitemap_max_bytes` default `2000000` — MUST cap in bytes the sitemap body
- `scrape_charset_peek_bytes` default `4096` — MUST pin in bytes the charset detection window
- `scrape_crawl_limit_max` default `500` — MUST cap the page budget that crawl `--limit` accepts
- `scrape_crawl_max_depth` default `10` — MUST cap the depth of `crawl` and `map`
- `scrape_search_limit_max` default `50` — MUST cap the `search` result budget
- `scrape_max_parse_bytes` default `50000000` — MUST cap in bytes the local file accepted by `parse`
- `scrape_no_cache` default `false` — MUST turn on to skip the cache on read and ALWAYS fetch from origin
- `monitor_diff_max_bytes` default `65536` — MUST cap in bytes the `monitor check --diff-mode` payload


## Operator Webhook
- `webhook_post_timeout_secs` default `15` — MUST cap in seconds the operator webhook POST
- `webhook_retry_base_delay_ms` default `50` — MUST pin in milliseconds the retry base delay, which doubles per attempt
- `webhook_max_attempts` default `3` — MUST cap webhook attempts including the first


## Heap
- `heap_snapshot_max_bytes` default `536870912` — MUST cap in bytes the heap snapshot file read offline
- `heap_max_retainers` default `200` — MUST cap the retainers returned per node operation
- `heap_max_edges` default `200` — MUST cap the edges returned per node operation
- `heap_max_paths` default `32` — MUST cap the paths enumerated by `heap paths`
- `heap_max_path_depth` default `8` — MUST cap the depth of `heap paths`
- `heap_max_class_nodes` default `500` — MUST cap the per-class node list
- `heap_dominator_max_states` default `50000` — MUST cap the states visited in the dominator computation
- `heap_outer_iters` default `200` — MUST cap the outer snapshot poll iterations
- `heap_inner_iters` default `10` — MUST pin the inner drain iterations after the snapshot finishes
- `heap_final_iters` default `20` — MUST pin the final snapshot drain iterations


## Lifecycle and Residual
- `user_data_dir` default `unset` — MUST treat it as an EXPLICIT waiver of residual-zero, because the profile is created 0700 on Unix and survives DIE, and ALWAYS revert with `config unset user_data_dir`
- `chrome_legacy_oxide_launch` default `false` — NEVER turn it on, because it reopens an unauthenticated DevTools port, does not start Xvfb and draws a headed window on the operator display
- `chrome_startup_timeout_secs` default `20` — MUST cap in seconds the readiness wait of the Chrome the CLI launches
- `browser_close_wait_secs` default `5` — MUST cap in seconds the browser close wait during FINALIZE
- `shutdown_deadline_secs` default `30` — MUST pin in seconds the hard browser exit deadline
- `platform_child_wait_secs` default `5` — MUST pin in seconds the child process wait deadline
- `platform_child_poll_ms` default `50` — MUST pin in milliseconds the child exit poll during FINALIZE
- `residual_orphan_min_age_secs` default `60` — MUST pin in seconds the minimum age to collect a dead-owner marker profile
- `default_viewport_width` default `1920` — MUST pin the default window width when launch omits a viewport
- `default_viewport_height` default `1080` — MUST pin the default window height when launch omits a viewport


## Lightpanda
- `lightpanda_startup_timeout_secs` default `10` — MUST cap in seconds the Lightpanda process startup
- `lightpanda_session_timeout_secs` default `604800` — MUST cap the Lightpanda session between 1 and 604800 seconds
- `lightpanda_poll_interval_ms` default `100` — MUST pin in milliseconds the Lightpanda CDP readiness poll
- `lightpanda_discovery_timeout_ms` default `500` — MUST cap in milliseconds each Lightpanda CDP discovery probe
- `lightpanda_max_log_lines` default `40` — MUST cap Lightpanda launch log lines per stream
- `lightpanda_ready_slice_ms` default `25` — MUST pin in milliseconds the drain after the Lightpanda child exits
- `lightpanda_cdp_connect_timeout_secs` default `5` — MUST cap in seconds the Lightpanda CDP connect
- `lightpanda_target_init_timeout_secs` default `10` — MUST cap in seconds the Lightpanda target init


## MITM
- `mitm_list_limit_max` default `10000` — MUST cap MITM list and query items
- `mitm_proxy_seconds_max` default `600` — MUST cap in seconds the MITM proxy window
- `mitm_chrome_settle_ms` default `150` — MUST pin in milliseconds the Chrome settle before navigating under MITM
- `mitm_capture_wait_min_ms` default `800` — MUST pin in milliseconds the capture wait floor after navigating
- `mitm_capture_wait_max_ms` default `8000` — MUST pin in milliseconds the capture wait ceiling after navigating
- `mitm_ws_frames_cap` default `500` — MUST cap in-memory WebSocket frames per capture
- `mitm_ws_preview_chars` default `256` — MUST cap the characters of the WebSocket text preview
- `mitm_ca_cache_size` default `1000` — MUST pin in hosts the dynamic certificate cache
- `mitm_rebind_attempts` default `3` — MUST pin the rebind attempts when the port is busy


## Perf
- `perf_autostop_settle_ms` default `500` — MUST pin in milliseconds the perf auto-stop settle after load
- `perf_trace_inner_slice_ms` default `20` — MUST pin in milliseconds the inner trace poll slice
- `perf_trace_outer_slice_ms` default `50` — MUST pin in milliseconds the outer trace poll interval
- `perf_trace_outer_iters` default `100` — MUST cap the outer trace poll iterations
- `perf_trace_inner_iters` default `5` — MUST pin the inner drain iterations after the trace completes


## Storage State
- `state_collect_deadline_secs` default `5` — MUST cap in seconds the storage collection
- `state_event_recv_secs` default `2` — MUST pin in seconds the storage event receive slice
- `state_load_settle_ms` default `500` — MUST pin in milliseconds the settle after the state import navigation


## Retry
- `retry_default_max_attempts` default `3` — MUST cap default retry attempts including the first
- `retry_base_delay_ms` default `50` — MUST pin in milliseconds the default retry base delay
- `retry_max_delay_secs` default `2` — MUST cap in seconds the default retry delay
- `retry_budget_secs` default `10` — MUST cap in seconds the total default retry budget
- `retry_cdp_max_attempts` default `4` — MUST cap CDP retry attempts
- `retry_cdp_base_delay_ms` default `100` — MUST pin in milliseconds the CDP retry base delay
- `retry_cdp_max_delay_secs` default `3` — MUST cap in seconds the CDP retry delay
- `retry_cdp_budget_secs` default `15` — MUST cap in seconds the total CDP retry budget
- `retry_http_max_attempts` default `3` — MUST cap HTTP scrape retry attempts
- `retry_http_base_delay_ms` default `75` — MUST pin in milliseconds the HTTP scrape retry base delay
- `retry_http_max_delay_secs` default `2` — MUST cap in seconds the HTTP scrape retry delay
- `retry_http_budget_secs` default `12` — MUST cap in seconds the total HTTP scrape retry budget
- `retry_llm_max_attempts` default `2` — MUST cap LLM HTTP retry attempts
- `retry_llm_base_delay_ms` default `200` — MUST pin in milliseconds the LLM HTTP retry base delay
- `retry_llm_max_delay_secs` default `4` — MUST cap in seconds the LLM HTTP retry delay
- `retry_llm_budget_secs` default `20` — MUST cap in seconds the total LLM HTTP retry budget
