[Português](CONFIGURATION.pt-BR.md)


# Configuration Reference


- Canonical XDG reference for every durable `browser-automation-cli` configuration key


## How Configuration Resolves
- The product reads no product environment variables at all
- Every durable setting lives in the XDG `config.toml` file
- Precedence is CLI flag first, XDG key second, built-in default last
- A CLI flag overrides the XDG key only for the invocation that carries it
- An XDG key overrides the built-in default for every invocation on that host
- A key absent from `config.toml` falls back to the built-in default documented below
- A key with no built-in default stays unset until you write it or pass the matching flag
- Secrets such as `openrouter_api_key` and `encryption_key` are written with permission `0600`
- Secrets never appear in logs, in JSON envelopes or in human stderr
- Robots bypass still requires both `--ignore-robots` and `--i-accept-robots-risk` on the command line
- Find the file that won with `config path`


## Configuration Commands
- `config init` creates the XDG configuration file when it is missing
- `browser-automation-cli --json config init`
- `config path` prints the resolved configuration and state paths
- `browser-automation-cli --json config path`
- `config show` prints the values stored in `config.toml`
- `browser-automation-cli --json config show`
- `config get <key>` reads the value stored for one key
- `browser-automation-cli --json config get timeout`
- `config set <key> <value>` writes one durable value
- `browser-automation-cli --json config set dialog_settle_ms 3000`
- `config unset <key>` restores one key to its built-in default
- `browser-automation-cli --json config unset dialog_settle_ms`
- `config unset` on a key that is already absent still succeeds
- `config set <key> ""` is not an undo, because it stores an empty string or fails to parse a number
- `config list-keys` enumerates every accepted key on the running binary
- `browser-automation-cli --json config list-keys`

### Write and undo one key
- Write the value with `config set dialog_settle_ms 3000`
- Confirm the stored value with `config get dialog_settle_ms`
- Undo the write with `config unset dialog_settle_ms`
- The `unset` envelope reports `was_set: true` when the key was present in the file
- Read the built-in default from `config list-keys`, and never from `config get`
- Measured on 0.2.0 with a fresh XDG file: `config get http_timeout_secs` returned `0` while `config list-keys` reports the default `30`

```bash
browser-automation-cli --json config set dialog_settle_ms 3000
browser-automation-cli --json config get dialog_settle_ms
browser-automation-cli --json config unset dialog_settle_ms
browser-automation-cli --json config list-keys
```


## Core and Locale
- `lang` — Message locale override accepting `en` or `pt-BR` with bare `pt` rejected, default none
- `lang` selects the locale of the `suggestion` field only
- `message` is the technical diagnostic and stays English in every locale, the same split `rustc` and `git` ship — see `docs/AGENTS.md`
- `timeout` — Global timeout in seconds, default `0`
- `artifacts_dir` — Artifacts output directory, default none
- `namespace` — Isolated state namespace, default none
- `encryption_key` — Session encryption key material, default none
- `color` — ANSI colors on human stderr, default none


## Logging
- `log_level` — Tracing `EnvFilter` used when argv flags stay quiet, default `error`
- `log_to_file` — Rotated local JSON logs under XDG state, never remote, default `false`
- `mitm/redact_policy.json` — persisted redact-secrets default under XDG state, written by `mitm redact --secrets true|false`
- Read only when neither `--mitm-redact-secrets` nor `--mitm-no-redact-secrets` is on argv
- Argv always wins and the built-in default is to redact
- `max_log_files` — Retained rotated log files in range `1..=90`, default `14`
- `log_rotation` — Rolling policy `daily`, `hourly` or `never`, default `daily`


## External Binaries
- `chrome_path` — Absolute Chrome or Chromium path, default none
- `lighthouse_path` — Absolute `lighthouse` CLI path, default none
- `ffmpeg_path` — Absolute `ffmpeg` path for screencast encode and video convert or `to-mp3`, default none
- `lighthouse_timeout_secs` — Wall-clock `lighthouse` CLI timeout in seconds, range `1..=3600`, default `300`
- `ffmpeg_timeout_secs` — Wall-clock `ffmpeg` encode timeout in seconds, range `1..=3600`, default `120`


## LLM and Webhooks
- `openrouter_api_key` — LLM API key stored with permission `0600`, default none
- `llm_base_url` — OpenAI-compatible base URL, default none
- `llm_model` — Default LLM model id, default none
- `llm_http_timeout_secs` — LLM and webhook blocking HTTP timeout in seconds, default `60`
- `webhook_post_timeout_secs` — Operator webhook POST timeout in seconds, default `15`
- `webhook_retry_base_delay_ms` — Webhook retry base delay in milliseconds, doubled each attempt, default `50`
- `webhook_max_attempts` — Webhook max attempts including the first try, default `3`


## Cache and Redis
- `cache_backend` — Cache backend `sqlite`, `memory` or `redis`, default `sqlite`
- `cache_redis_url` — Redis URL required when the backend is `redis`, default none
- `redis_allow_remote` — Allow non-loopback Redis hosts, default `false`
- `redis_connect_timeout_secs` — Redis TCP connect timeout in seconds, default `2`
- `redis_io_timeout_secs` — Redis RESP stream I/O timeout in seconds, default `3`
- `cache_max_resp_bulk_bytes` — Redis RESP bulk string size ceiling in bytes, default `16777216`
- `cache_max_resp_line_bytes` — Redis RESP line size ceiling in bytes, default `16777216`
- `scrape_http_cache_ttl_secs` — HTTP scrape response L2 cache TTL in seconds, default `3600`
- `file_parse_cache_ttl_secs` — Local file-parse L2 cache TTL in seconds, default `86400`


## HTTP and Network Safety
- `http_ssrf_mode` — HTTP SSRF policy `strict`, `allow_loopback` or `off`, default `strict`
- `http_timeout_secs` — Shared HTTP client total timeout in seconds, default `30`
- `http_connect_timeout_secs` — HTTP connect-phase timeout in seconds, default `10`
- `http_redirect_max` — Max HTTP redirects followed by product clients, default `10`
- `http_pool_max_idle_per_host` — HTTP pool max idle connections per host, default `4`
- `scrape_max_body_bytes` — Max HTTP scrape body bytes, default `5000000`
- `browser_scrape_max_body_bytes` — Max body bytes for browser-engine scrape helpers, default `2000000`
- `search_base_url` — HTML search endpoint base with `?q=` appended, default `https://html.duckduckgo.com/html/`
- `user_data_dir` — Persistent Chrome profile directory, opt-in, default none
- Unset by default, and unset is what keeps residual-zero: the launch gets a throwaway profile and the run leaves nothing on disk
- Set it only when a detector attests session across invocations, because a persistent profile is a directory this CLI will never delete for you
- Created with mode 0700 on Unix
- `--profile` on argv wins over this key


## Robots and Politeness
- `ignore_robots` — Default robots ignore, with both CLI risk flags still required, default `false`
- `robots_loopback_exempt` — Loopback hosts skip `robots.txt`, and `false` enforces it against `localhost`, default `true`
- `robots_probe_timeout_secs` — `robots.txt` request timeout in seconds, default `5`
- `robots_max_body_bytes` — Max `robots.txt` body bytes as anti-OOM guard, default `524288`
- `scrape_min_delay_ms` — Floor delay between same-origin GETs in milliseconds, default `0`
- `scrape_honor_meta_robots` — Honor meta robots and `X-Robots-Tag` noindex, default `true`
- `scrape_honor_nofollow` — Skip `rel=nofollow` links in crawl discovery, default `true`
- `scrape_delay_jitter_ratio` — Politeness delay jitter ratio in range `0.0..=1.0`, zero disables, default `0.2`


## Scrape and Crawl
- `scrape_default_engine` — Default scrape engine when the CLI omits `--engine`, `http` or `browser`, default `http`
- `scrape_use_sitemap` — Prefer `sitemap.xml` when mapping a site, default `true`
- `scrape_max_text_chars` — Max text or markdown chars in scrape envelopes, zero means no cap, default `32768`
- `scrape_summary_chars` — Max chars for scrape format `summary`, default `400`
- `scrape_feed_max_entries` — Max entries kept by scrape format `feed` for RSS, Atom and JSON Feed, default `50`
- `scrape_follow_rel_next` — Follow `rel=next` pagination links during crawl, default `false`
- `scrape_dedup_similar` — Collapse near-duplicate pages by content similarity in `crawl` and `batch-scrape`, default `false`
- `scrape_no_cache` — Ignore the response cache on READ and always fetch from origin, default `false`
- The fresh response is still written, so a bypassing call refreshes the entry for later callers instead of leaving a stale one
- `--no-cache` on `scrape` overrides this per invocation
- There is no way to express the same thing with `scrape_http_cache_ttl_secs`: a TTL of `0` already means "never expires", which is the opposite, and the key rejects it
- `monitor check` bypasses unconditionally and ignores this key, because a cached body made it compare a stored page with itself and report `changed: false`
- `scrape_dedup_similar_distance` — SimHash Hamming distance in range `0..=64` under which pages are near-duplicates, default `3`
- `scrape_sitemap_max_bytes` — Max sitemap body bytes, default `2000000`
- `scrape_charset_peek_bytes` — Charset sniffing peek window in bytes, default `4096`
- `scrape_crawl_limit_max` — Max crawl page budget as anti-DoS clamp for `--limit`, default `500`
- `scrape_crawl_max_depth` — Max BFS depth for `crawl` and `map`, default `10`
- `scrape_search_limit_max` — Max search result budget as anti-DoS clamp, default `50`
- `scrape_max_parse_bytes` — Max local file parse size in bytes before reject, default `50000000`
- `max_urls_file_bytes` — Max bytes for the `batch-scrape --urls-file` list, default `8388608`


## Image
- `image_max_input_bytes` — Max bytes for local image decode, convert or resize input, default `32000000`
- `image_max_pixels` — Max width times height for image decode as anti-bomb guard, default `64000000`
- `image_default_format` — Default image convert format `png`, `jpeg`, `webp` or `gif`, default `png`
- `image_default_quality` — Default lossy quality in range `1..=100` for image convert and resize, default `85`
- `image_download_max_bytes` — Max HTTP body bytes for image download, default `32000000`
- `image_avif_speed` — AVIF encoder speed in range `1..=10` where one is slowest and best, needs the `image-avif` feature, default `6`
- `default_jpeg_quality` — JPEG quality in range `1..=100` when `grab` omits `--quality`, default `80`


## Video and Audio
- `video_max_input_bytes` — Max bytes for video stdin materialization or path pre-check, default `512000000`
- `video_download_max_bytes` — Max HTTP body bytes for video download, default `512000000`
- `video_default_container` — Default video convert container `mp4`, `webm`, `mkv`, `mov`, `avi` or `m4v`, default `mp4`
- `video_default_crf` — Default CRF in range `1..=51` for lossy video re-encode, default `23`
- `video_default_audio_bitrate` — Default bitrate for video `to-mp3`, default `192k`
- `audio_max_input_bytes` — Max bytes for audio stdin materialization or path pre-check, default `256000000`
- `audio_download_max_bytes` — Max HTTP body bytes for audio download, default `256000000`
- `audio_default_format` — Default audio convert format `mp3`, `m4a`, `ogg`, `opus`, `flac`, `wav` or `aac`, default `mp3`
- `audio_default_bitrate` — Default bitrate for lossy audio encode, default `192k`


## SVG, GIF and Manifests
- `svg_max_bytes` — Max SVG source bytes accepted before rasterisation, default `4000000`
- `svg_max_depth` — Max XML nesting depth accepted in an SVG source, default `128`
- `svg_max_entities` — Max `<!ENTITY>` declarations tolerated in an SVG DTD, zero rejects any, default `0`
- `gif_max_frames` — Max animation frames decoded from a GIF, default `2000`
- `manifest_max_bytes` — Max bytes accepted for an HLS or DASH manifest body, default `8000000`
- `manifest_max_variants` — Max variant or representation entries emitted per manifest envelope, default `500`


## Chrome Engine and Lifecycle
- `chrome_search_paths` — Ordered Chrome or Chromium discovery paths, platform-separated, with an empty value using the built-in per-OS layout, default none
- `chrome_legacy_oxide_launch` — Launch Chrome through the legacy `chromiumoxide` path instead of the self-spawn path as a stabilization fallback that loses the residual kill target, default `false`
- Setting `chrome_legacy_oxide_launch` to `true` reopens an unauthenticated loopback DevTools port and never starts the private Xvfb, which undoes the two headed and security fixes of `0.2.0`
- `chrome_startup_timeout_secs` — Chrome self-spawn CDP readiness wait in seconds, measured on the DevTools pipe bridge, default `20`
- The bridge gives the CDP client twice this budget to connect
- `chrome_default_timeout_ms` — Default per-operation timeout for the Chrome engine in milliseconds, default `25000`
- `browser_close_wait_secs` — `Browser.close` and process wait budget during FINALIZE in seconds, default `5`
- `residual_orphan_min_age_secs` — Age floor in seconds before a dead-owner marker profile is collectable, default `60`
- `platform_child_wait_secs` — Platform child wait deadline in seconds, default `5`
- `platform_child_poll_ms` — Child-process exit poll interval during FINALIZE in milliseconds, default `50`
- `shutdown_deadline_secs` — Shutdown hard deadline waiting for browser exit in seconds, default `30`


## CDP and Events
- `cdp_connection_probe_timeout_secs` — CDP `Browser.getVersion` liveness probe timeout in seconds, default `3`
- `cdp_discovery_timeout_secs` — CDP HTTP discovery timeout for `/json/version` probes in seconds, default `2`
- No launch path reads it: the self-spawned Chrome uses the DevTools pipe and exposes no `/json/version`, and Lightpanda readiness uses `lightpanda_discovery_timeout_ms`
- `cdp_discovery_max_body_bytes` — Max CDP discovery HTTP body bytes for `/json/version` and `/json/list`, default `1048576`
- Only the Lightpanda readiness probe reads it, because the self-spawned Chrome uses the DevTools pipe
- `cdp_event_broadcast_capacity` — Process-local CDP event broadcast channel capacity, default `4096`
- `cdp_event_drain_poll_ms` — CDP event drain poll slice during navigation wait in milliseconds, default `100`
- `cdp_network_idle_settle_ms` — CDP network-idle settle window in milliseconds, default `500`
- `cdp_target_event_wait_ms` — CDP target event short wait in milliseconds, default `600`
- `event_tracker_max_entries` — In-memory console and network tracker ring size per page session, default `1000`
- `capture_preserved_rings` — Navigation boundaries kept for console and net `--include-preserved`, default `3`
- `event_pump_slice_ms` — `wait` and `eval` event pump slice in milliseconds, default `50`
- `eval_drain_slice_ms` — Eval drain slice while waiting for `Runtime.evaluate` results in milliseconds, default `40`
- `extension_attach_poll_ms` — Extension attach poll slice in milliseconds, default `150`
- `extension_attach_poll_iters` — Extension attach poll iterations, default `20`
- Slice x iterations is the total wait


## Lightpanda Engine
- `lightpanda_startup_timeout_secs` — Lightpanda process startup wait in seconds, default `10`
- `lightpanda_session_timeout_secs` — Lightpanda `--timeout` session max in seconds, range `1..=604800`, default `604800`
- `lightpanda_poll_interval_ms` — Lightpanda CDP readiness poll interval in milliseconds, default `100`
- `lightpanda_discovery_timeout_ms` — Per-probe CDP discovery timeout while waiting for Lightpanda in milliseconds, default `500`
- `lightpanda_max_log_lines` — Bounded Lightpanda launch log ring in lines per stream, default `40`
- `lightpanda_ready_slice_ms` — Drain slice after Lightpanda child exit before snapshotting logs in milliseconds, default `25`
- `lightpanda_cdp_connect_timeout_secs` — Lightpanda CDP connect attempt timeout in seconds, default `5`
- `lightpanda_target_init_timeout_secs` — Lightpanda target init wait after connect in seconds, default `10`


## Interaction and Waits
- `interact_settle_ms` — UI settle delay after click, type or extension action in milliseconds, default `200`
- `dialog_settle_ms` — Max wait after a JS dialog answer for `javascriptDialogClosed` in milliseconds, default `2000`
- `network_idle_window_ms` — Quiet window for `wait --network-idle` in milliseconds, default `500`
- `dom_stable_window_ms` — Quiet window for `wait --dom-stable-ms` in milliseconds, default `500`
- `drag_move_steps` — Intermediate mouse positions synthesized for one HTML5 drag, default `6`
- `drag_move_gap_ms` — Delay between synthesized drag positions in milliseconds, default `16`
- `input_profile` — Default input shaping when `--input-profile` is absent: `human` synthesizes a trajectory, wheel ticks and key events, default `human`
- `direct` keeps the pre-0.1.8 dispatch
- The flag still wins
- `browser_mode` — Window mode: `auto` resolves to `headed` inside a private virtual display on Linux with Xvfb on PATH and without `--no-xvfb`, and to `headless` in every other case, default `auto`
- `headed` puts a real window on your display, and on Linux that window is rendered into a private virtual display when Xvfb is available
- `headless` is cheapest and most detectable
- `--headed` still wins
- Inverting the `auto` default carries a latency bill and is a separate decision
- `doctor` reports what `auto` resolves to on this host under the `virtual_display` check, so the answer never drifts from the binary
- `stealth` — Anti-detection patches applied before the first navigation, default `true`
- `--no-stealth` turns them off for one run
- `stealth_profile` — Impersonated identity: `auto`, `chrome-linux`, `chrome-win`, `chrome-mac`, default `auto`
- `auto` follows the host, which is the only value that cannot contradict the Canvas and WebGL hashes the real GPU produces
- `doctor --fingerprint` (not an XDG key) — envelope fields `measurement_scope` (`linux-headless-xvfb`), `unmeasured_os` (`macos`, `windows`), `measurement_note`
- Live Canvas/WebGL/audio have been scored only on Linux headless + Xvfb
- The same types compile on macOS and Windows
- `proxy_url` — Egress proxy for both Chrome and the HTTP engine (`http`, `https`, `socks5`, `socks5h`), default none
- Put credentials here rather than in `--proxy`, where the process table exposes them
- `proxy_bypass` — Hosts bypassing the proxy, in Chrome's bypass-list syntax, default none
- `proxy_username` — Proxy account name, sent as basic auth, default none
- Kept here rather than in argv, where the process table would expose it
- `proxy_password` — Proxy password, sent as basic auth, default none
- Never echoed by `config get` or `config show`
- `cdp_proxy_bypass_loopback` — Always bypass loopback when Chrome runs behind `--proxy`, default `true`
- The CDP control channel is loopback, so a proxy that captures it produces a browser that never answers — reported as a Chrome startup timeout, which blames the wrong component
- `stealth_seed` — Pins the stealth identity so the same fingerprint is reproduced across processes, default none
- Absent means a fresh identity each run, which is the default precisely because caching an identity writes it to disk
- `http2_enabled` — Offer `h2` in ALPN, default `true`
- ALPN is visible in the clear during the TLS handshake and Chrome always lists `h2`, so a client that offers only `http/1.1` has answered "not a browser" before sending a byte
- `http2_initial_stream_window_size` — `SETTINGS_INITIAL_WINDOW_SIZE` advertised to the peer, default `6291456`
- Library defaults are three orders of magnitude away from Chrome's
- `http2_initial_connection_window_size` — Connection-level flow-control window advertised to the peer, default `15663105`
- `http2_max_header_list_size` — `SETTINGS_MAX_HEADER_LIST_SIZE` advertised to the peer, default `262144`
- `http2_max_frame_size` — `SETTINGS_MAX_FRAME_SIZE` advertised to the peer, range `16384..=16777215`, default `16384`
- `http2_adaptive_window` — Let the HTTP/2 stack resize windows dynamically, default `false`
- Off keeps the advertised values fixed, which is what makes the fingerprint reproducible
- `robots_user_agent` — User-agent token that `robots.txt` rules are matched against, default none
- Set it when stealth sends a browser User-Agent, so the rules evaluated are the ones that apply to the request actually sent
- `input_move_steps` — Intermediate pointer positions synthesized for one move (human profile), default `24`
- `input_move_gap_ms` — Delay between synthesized pointer positions in milliseconds, default `12`
- `input_click_dwell_ms` — Hold time between `mousePressed` and `mouseReleased` in milliseconds, default `65`
- `input_key_dwell_ms` — Hold time between `keyDown` and `keyUp` in milliseconds, default `45`
- `input_type_delay_ms` — Delay between characters while typing in milliseconds, default `95`
- `input_scroll_tick_px` — Scroll distance carried by one synthesized wheel tick in CSS pixels, default `100`
- `input_scroll_max_ticks` — Ceiling on the number of wheel ticks one scroll gesture synthesizes, default `40`
- Each tick is a CDP round trip, so without a ceiling the cost of a scroll grows linearly with the distance requested and a large `--delta-y` exhausts the command timeout
- Past the ceiling the ticks carry more pixels each
- Total travel is unchanged and only the granularity degrades
- `input_target_jitter_px` — Radius of the random offset applied to a click target in CSS pixels, default `3`
- `input_scroll_settle_rounds` — Extra rounds allowed to deliver a wheel delta the renderer dropped, default `3`
- `input_timing_distribution` — Shape of the dispersion drawn around every input delay: `lognormal`, `normal` or `uniform`, default `lognormal`
- `lognormal` is the default because human inter-key intervals are right-skewed, and a symmetric draw reproduces the width of the human distribution without its asymmetry
- It governs the fast rhythm only
- The long-pause tail is `input_word_pause_permille`
- Every mean is floored at 5% dispersion and truncated between a quarter and four times itself, so setting a standard deviation to `0` does not buy the zero variance a detector reads as machine
- `input_move_steps_stddev` — Standard deviation of the per-gesture pointer sample budget, so two moves over the same distance do not carry the same number of intermediate positions, default `6`
- A drag rescales this to its own smaller budget instead of carrying the absolute value across
- `input_move_gap_stddev_ms` — Standard deviation of the delay between synthesized pointer positions in milliseconds, default `5`
- `input_click_dwell_stddev_ms` — Standard deviation of the press-to-release hold in milliseconds, default `26`
- `input_key_dwell_stddev_ms` — Standard deviation of the `keyDown`-to-`keyUp` hold in milliseconds, default `18`
- `input_type_delay_stddev_ms` — Standard deviation of the delay between characters in milliseconds, default `40`
- A caller that asks for its own typing rhythm gets this dispersion rescaled by the same ratio, so halving the mean halves the spread instead of leaving an absolute value that no longer fits
- `input_scroll_tick_stddev_px` — Standard deviation of the distance one synthesized wheel tick carries in CSS pixels, default `25`
- `input_word_pause_ms` — Mean of the extra pause taken at a word or sentence boundary in milliseconds, itself dispersed by half of its value, default `320`
- This pause is what produces the long right tail of a typing trace, which no amount of jitter around the per-character mean can create
- `input_word_pause_permille` — Chance in a thousand that a word or sentence boundary earns that long pause, default `120`
- `0` removes the tail and leaves only the fast rhythm
- `input_typo_permille` — Chance in a thousand that a character is typed wrong, erased with `Backspace` and retyped, default `0`
- The field always ends up holding exactly the requested text
- `0` by default, and the only humanisation key that is: every other one disperses TIMING, which a page cannot read as a different value, while this one changes the CHARACTER STREAM, so an `input` listener sees the wrong prefix and may autocomplete or navigate on it
- The wrong key is always a physical neighbour on the QWERTY row
- `support_settle_ms` — Support-thread settle for sync helpers in milliseconds, default `80`
- `nav_micro_settle_ms` — Navigation micro-settle after page transitions in milliseconds, default `100`


## Screencast and Perf
- `screencast_jpeg_quality` — Screencast CDP JPEG quality in range `1..=100`, default `60`
- `screencast_ffmpeg_framerate` — Screencast `ffmpeg` input framerate in frames per second, default `10`
- `screencast_start_pump_iters` — Immediate pump iterations after `Page.startScreencast`, default `15`
- `screencast_stop_pump_iters` — Drain pump iterations before `Page.stopScreencast`, default `40`
- `perf_autostop_settle_ms` — Perf auto-stop settle after load or reload in milliseconds, default `500`
- `perf_trace_inner_slice_ms` — Perf trace poll inner slice in milliseconds, default `20`
- `perf_trace_outer_slice_ms` — Perf trace outer poll interval in milliseconds, default `50`
- `perf_trace_outer_iters` — Perf trace outer poll max iterations, default `100`
- `perf_trace_inner_iters` — Perf trace inner drain iterations after complete, default `5`


## Heap
- `heap_snapshot_max_bytes` — Offline heap snapshot file size ceiling in bytes, default `536870912`
- `heap_max_retainers` — Heap node-op max retainers returned, default `200`
- `heap_max_edges` — Heap node-op max edges returned, default `200`
- `heap_max_paths` — Heap paths enumeration max paths, default `32`
- `heap_max_path_depth` — Heap paths max depth, default `8`
- `heap_max_class_nodes` — Heap `class_nodes` list cap, default `500`
- `heap_dominator_max_states` — Dominator visited-state ceiling against pathological graphs, default `50000`
- `heap_outer_iters` — Heap snapshot outer poll max iterations, default `200`
- `heap_inner_iters` — Heap snapshot inner drain iterations after finished, default `10`
- `heap_final_iters` — Heap snapshot final drain iterations, default `20`


## MITM
- `monitor_diff_max_bytes` — Byte ceiling for the `monitor check --diff-mode` payload, default `65536`
- A page rewritten wholesale diffs to the whole page twice, and the caller asked what changed, not for everything
- `diff_truncated` says when the ceiling applied, and `added_count` / `removed_count` keep reporting the real size
- `mitm_list_limit_max` — MITM list and query max items clamp, default `10000`
- `mitm_proxy_seconds_max` — MITM proxy one-shot max window in seconds, default `600`
- `mitm_chrome_settle_ms` — MITM Chrome launch settle before navigation in milliseconds, default `150`
- `mitm_capture_wait_min_ms` — MITM capture wait floor after navigate in milliseconds, default `800`
- `mitm_capture_wait_max_ms` — MITM capture wait ceiling after navigate in milliseconds, default `8000`
- `mitm_ws_frames_cap` — Cap on in-memory WebSocket frames per capture process, default `500`
- `mitm_ws_preview_chars` — WebSocket text preview truncation in Unicode chars, default `256`
- `mitm_ca_cache_size` — MITM dynamic certificate cache size in hosts, default `1000`
- `mitm_rebind_attempts` — MITM proxy bind retries when the port is transiently in use, default `3`


## Local Files and Roots
- `allowed_roots` — Extra allowed roots for local reads and artifact writes, platform-separated, with the defaults already covering cwd, XDG dirs and temp, default none
- `max_json_file_bytes` — Max bytes for JSON or NDJSON script and manifest files, default `33554432`
- `max_ndjson_line_bytes` — Max bytes for one NDJSON line in run scripts and traces, default `1048576`
- `max_cli_json_payload_bytes` — Max bytes for CLI flag JSON payloads, default `4194304`
- `max_sg_file_bytes` — Max bytes for one source file read by `sg-scan` and `sg-rewrite`, default `16777216`
- `run_max_include_depth` — Max nesting depth for `run --script` include chains, default `16`


## Retry Budgets
- `retry_default_max_attempts` — Default retry max attempts including the first try, default `3`
- `retry_base_delay_ms` — Default retry base delay in milliseconds, default `50`
- `retry_max_delay_secs` — Default retry max delay in seconds, default `2`
- `retry_budget_secs` — Default retry wall budget in seconds, default `10`
- `retry_cdp_max_attempts` — CDP retry max attempts, default `4`
- `retry_cdp_base_delay_ms` — CDP retry base delay in milliseconds, default `100`
- `retry_cdp_max_delay_secs` — CDP retry max delay in seconds, default `3`
- `retry_cdp_budget_secs` — CDP retry wall budget in seconds, default `15`
- `retry_http_max_attempts` — HTTP scrape retry max attempts, default `3`
- `retry_http_base_delay_ms` — HTTP scrape retry base delay in milliseconds, default `75`
- `retry_http_max_delay_secs` — HTTP scrape retry max delay in seconds, default `2`
- `retry_http_budget_secs` — HTTP scrape retry wall budget in seconds, default `12`
- `retry_llm_max_attempts` — LLM HTTP retry max attempts, default `2`
- `retry_llm_base_delay_ms` — LLM HTTP retry base delay in milliseconds, default `200`
- `retry_llm_max_delay_secs` — LLM HTTP retry max delay in seconds, default `4`
- `retry_llm_budget_secs` — LLM HTTP retry wall budget in seconds, default `20`


## Viewport and State
- `default_viewport_width` — Default headless Chrome window width (`--window-size`) when launch options omit the viewport, default `1920`
- Distinct from `screen`: this is the process window and the screenshot/screencast fallback, not `screen.width`
- `default_viewport_height` — Default headless Chrome window height (`--window-size`) when launch options omit the viewport, default `1080`
- `screen` — Default page screen `WxH` for `Emulation.setDeviceMetricsOverride` (`screen.width`/`screen.height`), default none
- Absent = mirror the viewport
- Argv `--screen` and a `run` emulate/resize `screen` field still win
- Never smaller than the viewport
- `state_collect_deadline_secs` — CDP storage collect outer deadline in seconds, default `5`
- `state_event_recv_secs` — CDP storage event recv slice in seconds, default `2`
- `state_load_settle_ms` — Settle delay after `load_state` navigation in milliseconds, default `500`


## Discovering Keys at Runtime
- Enumerate every accepted key on the running binary with the JSON envelope
- `browser-automation-cli --json config list-keys`
- Each entry returns the key name, the built-in default and the description
- Read the stored value of one key with `browser-automation-cli --json config get <key>`
- Inspect every stored value with `browser-automation-cli --json config show`
- Confirm which file won with `browser-automation-cli --json config path`
- Treat the live output of `config list-keys` as the source of truth when this document and the binary disagree
- Prefer runtime discovery to any memorized list


## See Also
- [README](../README.md) for the product overview
- [How to Use](HOW_TO_USE.md) for the practical usage guide
- [Agents](AGENTS.md) for the agent integration contract
