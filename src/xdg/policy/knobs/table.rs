// SPDX-License-Identifier: MIT OR Apache-2.0
//! The promoted-policy rows: data, with no mechanism of its own.
//!
//! One row generates six coupled surfaces (serde field, key constant,
//! default, `config get`, `config set`, `config list-keys`).

use serde::{Deserialize, Serialize};

use crate::error::CliError;

use super::super::validate::parse_policy_value;
use super::expand::policy_knobs;

policy_knobs! {
    // ── cache: Redis transport budgets and L2 TTLs ───────────────────────
    REDIS_IO_TIMEOUT_SECS => redis_io_timeout_secs,
        "Redis/RESP stream I/O timeout (seconds)";
    CACHE_MAX_RESP_BULK_BYTES => cache_max_resp_bulk_bytes,
        "Redis RESP bulk string size ceiling (bytes)";
    CACHE_MAX_RESP_LINE_BYTES => cache_max_resp_line_bytes,
        "Redis RESP line size ceiling (bytes)";
    SCRAPE_HTTP_CACHE_TTL_SECS => scrape_http_cache_ttl_secs,
        "HTTP scrape response L2 cache TTL (seconds)";
    FILE_PARSE_CACHE_TTL_SECS => file_parse_cache_ttl_secs,
        "Local file-parse L2 cache TTL (seconds)";

    // ── cdp: discovery, event pump, attach ───────────────────────────────
    CDP_DISCOVERY_MAX_BODY_BYTES => cdp_discovery_max_body_bytes,
        "Max CDP discovery HTTP body bytes (/json/version, /json/list)";
    CDP_EVENT_BROADCAST_CAPACITY => cdp_event_broadcast_capacity,
        "Process-local CDP event broadcast channel capacity";
    CDP_EVENT_DRAIN_POLL_MS => cdp_event_drain_poll_ms,
        "CDP event drain poll slice during navigation wait (milliseconds)";
    CDP_NETWORK_IDLE_SETTLE_MS => cdp_network_idle_settle_ms,
        "CDP network-idle settle window (milliseconds)";
    CDP_TARGET_EVENT_WAIT_MS => cdp_target_event_wait_ms,
        "CDP target event short wait (milliseconds)";
    DEFAULT_CDP_DISCOVERY_TIMEOUT_SECS => cdp_discovery_timeout_secs,
        "CDP HTTP discovery timeout for /json/version probes (seconds)";
    EVENT_TRACKER_MAX_ENTRIES => event_tracker_max_entries,
        "In-memory console/network tracker ring size per page session";
    EXTENSION_ATTACH_POLL_MS => extension_attach_poll_ms,
        "Extension attach poll slice (milliseconds)";

    // ── external tools ───────────────────────────────────────────────────
    SCREENCAST_FFMPEG_FRAMERATE => screencast_ffmpeg_framerate,
        "Screencast ffmpeg input framerate (frames per second)";

    // ── http: robots, scrape, pool, webhook ──────────────────────────────
    ROBOTS_PROBE_TIMEOUT_SECS => robots_probe_timeout_secs,
        "robots.txt HEAD/probe timeout (seconds)";
    ROBOTS_MAX_BODY_BYTES => robots_max_body_bytes,
        "Max robots.txt body bytes (anti-OOM)";
    DEFAULT_BROWSER_SCRAPE_MAX_BODY_BYTES => browser_scrape_max_body_bytes,
        "Max body bytes for browser-engine scrape helpers";
    HTTP_REDIRECT_MAX => http_redirect_max,
        "Max HTTP redirects followed by product clients";
    HTTP_POOL_MAX_IDLE_PER_HOST => http_pool_max_idle_per_host,
        "reqwest pool max idle connections per host";
    WEBHOOK_POST_TIMEOUT_SECS => webhook_post_timeout_secs,
        "Operator webhook POST timeout (seconds)";
    WEBHOOK_RETRY_BASE_DELAY_MS => webhook_retry_base_delay_ms,
        "Webhook retry base delay (milliseconds; doubles each attempt)";
    WEBHOOK_MAX_ATTEMPTS => webhook_max_attempts,
        "Webhook max attempts (inclusive of first try)";

    // ── heap snapshot ceilings and node-op caps ──────────────────────────
    HEAP_SNAPSHOT_MAX_BYTES => heap_snapshot_max_bytes,
        "Offline heap snapshot file size ceiling (bytes)";
    HEAP_DEFAULT_MAX_RETAINERS => heap_max_retainers,
        "Heap node-op max retainers returned";
    HEAP_DEFAULT_MAX_EDGES => heap_max_edges,
        "Heap node-op max edges returned";
    HEAP_DEFAULT_MAX_PATHS => heap_max_paths,
        "Heap paths enumeration max paths";
    HEAP_DEFAULT_MAX_PATH_DEPTH => heap_max_path_depth,
        "Heap paths max depth";
    HEAP_DEFAULT_MAX_CLASS_NODES => heap_max_class_nodes,
        "Heap class_nodes list cap";
    HEAP_DOMINATOR_MAX_STATES => heap_dominator_max_states,
        "Dominator visited-state ceiling (anti-pathological graphs)";
    DEFAULT_HEAP_OUTER_ITERS => heap_outer_iters,
        "Heap snapshot outer poll max iterations";
    DEFAULT_HEAP_INNER_ITERS => heap_inner_iters,
        "Heap snapshot inner drain iterations after finished";
    DEFAULT_HEAP_FINAL_ITERS => heap_final_iters,
        "Heap snapshot final drain iterations";

    // ── lifecycle reap budgets reachable outside src/lifecycle ───────────
    BROWSER_CLOSE_WAIT_SECS => browser_close_wait_secs,
        "Browser.close / process wait budget during FINALIZE (seconds)";
    CHROME_STARTUP_TIMEOUT_SECS => chrome_startup_timeout_secs,
        "Chrome self-spawn CDP readiness wait (seconds)";
    RESIDUAL_ORPHAN_MIN_AGE_SECS => residual_orphan_min_age_secs,
        "Age floor before a dead-owner marker profile is collectable (seconds)";
    PLATFORM_CHILD_WAIT_SECS => platform_child_wait_secs,
        "Platform child wait deadline (seconds)";
    DEFAULT_SHUTDOWN_POLL_MS => shutdown_poll_ms,
        "Shutdown cooperative poll interval (milliseconds)";
    DEFAULT_SHUTDOWN_DEADLINE_SECS => shutdown_deadline_secs,
        "Shutdown hard deadline waiting for browser exit (seconds)";

    // ── lightpanda engine ────────────────────────────────────────────────
    LIGHTPANDA_POLL_INTERVAL_MS => lightpanda_poll_interval_ms,
        "Lightpanda CDP readiness poll interval (milliseconds)";
    LIGHTPANDA_DISCOVERY_TIMEOUT_MS => lightpanda_discovery_timeout_ms,
        "Per-probe CDP discovery timeout while waiting for Lightpanda (milliseconds)";
    LIGHTPANDA_MAX_LOG_LINES => lightpanda_max_log_lines,
        "Bounded Lightpanda launch log ring (lines per stream)";
    LIGHTPANDA_READY_SLICE_MS => lightpanda_ready_slice_ms,
        "Drain slice after Lightpanda child exit before snapshotting logs (milliseconds)";
    LIGHTPANDA_CDP_CONNECT_TIMEOUT_SECS => lightpanda_cdp_connect_timeout_secs,
        "Lightpanda CDP connect attempt timeout (seconds)";
    LIGHTPANDA_TARGET_INIT_TIMEOUT_SECS => lightpanda_target_init_timeout_secs,
        "Lightpanda target init wait after connect (seconds)";

    // ── media pump iterations ────────────────────────────────────────────
    DEFAULT_SCREENCAST_START_PUMP_ITERS => screencast_start_pump_iters,
        "Screencast start: immediate pump iterations after Page.startScreencast";
    DEFAULT_SCREENCAST_STOP_PUMP_ITERS => screencast_stop_pump_iters,
        "Screencast stop: drain pump iterations before Page.stopScreencast";

    // ── mitm clamps and capture windows ──────────────────────────────────
    MITM_LIST_LIMIT_MAX => mitm_list_limit_max,
        "MITM list/query max items clamp";
    MITM_PROXY_SECONDS_MAX => mitm_proxy_seconds_max,
        "MITM proxy one-shot max window (seconds)";
    MITM_CHROME_SETTLE_MS => mitm_chrome_settle_ms,
        "MITM Chrome launch settle before navigation (milliseconds)";
    MITM_CAPTURE_WAIT_MIN_MS => mitm_capture_wait_min_ms,
        "MITM capture wait floor after navigate (milliseconds)";
    MITM_CAPTURE_WAIT_MAX_MS => mitm_capture_wait_max_ms,
        "MITM capture wait ceiling after navigate (milliseconds)";
    MITM_WS_FRAMES_CAP => mitm_ws_frames_cap,
        "Cap on in-memory WebSocket frames per capture process";
    MITM_WS_PREVIEW_CHARS => mitm_ws_preview_chars,
        "WebSocket text preview truncation (Unicode chars)";
    MITM_CA_CACHE_SIZE => mitm_ca_cache_size,
        "MITM dynamic certificate cache size (hosts)";

    // ── payload ceilings and crawl budgets ───────────────────────────────
    MAX_SG_FILE_BYTES => max_sg_file_bytes,
        "Max bytes for one source file read by sg scan/rewrite";
    MAX_URLS_FILE_BYTES => max_urls_file_bytes,
        "Max bytes for the batch-scrape --urls-file list";
    RUN_MAX_INCLUDE_DEPTH => run_max_include_depth,
        "Max nesting depth for run --script include chains";
    MITM_REBIND_ATTEMPTS => mitm_rebind_attempts,
        "MITM proxy bind retries when the port is transiently in use";
    DEFAULT_NETWORK_IDLE_WINDOW_MS => network_idle_window_ms,
        "Quiet window for wait --network-idle (milliseconds)";
    DEFAULT_DOM_STABLE_WINDOW_MS => dom_stable_window_ms,
        "Quiet window for wait --dom-stable-ms (milliseconds)";
    CHROME_DEFAULT_TIMEOUT_MS => chrome_default_timeout_ms,
        "Default per-operation timeout for the Chrome engine (milliseconds)";
    DRAG_MOVE_STEPS => drag_move_steps,
        "Intermediate mouse positions synthesized for one HTML5 drag";
    DRAG_MOVE_GAP_MS => drag_move_gap_ms,
        "Delay between synthesized drag positions (milliseconds)";
    ROBOTS_FETCH_TIMEOUT_SECS => robots_fetch_timeout_secs,
        "Timeout for fetching robots.txt (seconds)";
    SCRAPE_CRAWL_LIMIT_MAX => scrape_crawl_limit_max,
        "Max crawl page budget (anti-DoS clamp for --limit)";
    SCRAPE_CRAWL_MAX_DEPTH => scrape_crawl_max_depth,
        "Max BFS depth for crawl/map";
    SCRAPE_SEARCH_LIMIT_MAX => scrape_search_limit_max,
        "Max search result budget (anti-DoS clamp)";
    SCRAPE_MAX_PARSE_BYTES => scrape_max_parse_bytes,
        "Max local file parse size before reject (bytes)";

    // ── retry budgets per transport family ───────────────────────────────
    RETRY_DEFAULT_MAX_ATTEMPTS => retry_default_max_attempts,
        "Default retry max attempts (inclusive of first try)";
    RETRY_BASE_DELAY_MS => retry_base_delay_ms,
        "Default retry base delay (milliseconds)";
    RETRY_MAX_DELAY_SECS => retry_max_delay_secs,
        "Default retry max delay (seconds)";
    RETRY_BUDGET_SECS => retry_budget_secs,
        "Default retry wall budget (seconds)";
    RETRY_CDP_MAX_ATTEMPTS => retry_cdp_max_attempts,
        "CDP retry max attempts";
    RETRY_CDP_BASE_DELAY_MS => retry_cdp_base_delay_ms,
        "CDP retry base delay (milliseconds)";
    RETRY_CDP_MAX_DELAY_SECS => retry_cdp_max_delay_secs,
        "CDP retry max delay (seconds)";
    RETRY_CDP_BUDGET_SECS => retry_cdp_budget_secs,
        "CDP retry wall budget (seconds)";
    RETRY_HTTP_MAX_ATTEMPTS => retry_http_max_attempts,
        "HTTP scrape retry max attempts";
    RETRY_HTTP_BASE_DELAY_MS => retry_http_base_delay_ms,
        "HTTP scrape retry base delay (milliseconds)";
    RETRY_HTTP_MAX_DELAY_SECS => retry_http_max_delay_secs,
        "HTTP scrape retry max delay (seconds)";
    RETRY_HTTP_BUDGET_SECS => retry_http_budget_secs,
        "HTTP scrape retry wall budget (seconds)";
    RETRY_LLM_MAX_ATTEMPTS => retry_llm_max_attempts,
        "LLM HTTP retry max attempts";
    RETRY_LLM_BASE_DELAY_MS => retry_llm_base_delay_ms,
        "LLM HTTP retry base delay (milliseconds)";
    RETRY_LLM_MAX_DELAY_SECS => retry_llm_max_delay_secs,
        "LLM HTTP retry max delay (seconds)";
    RETRY_LLM_BUDGET_SECS => retry_llm_budget_secs,
        "LLM HTTP retry wall budget (seconds)";

    // ── timing slices and settle delays ──────────────────────────────────
    DEFAULT_EVAL_DRAIN_SLICE_MS => eval_drain_slice_ms,
        "Eval drain slice while waiting for Runtime.evaluate results (milliseconds)";
    DEFAULT_SUPPORT_SETTLE_MS => support_settle_ms,
        "Support-thread settle for sync helpers (milliseconds)";
    DEFAULT_NAV_MICRO_SETTLE_MS => nav_micro_settle_ms,
        "Navigation micro-settle after page transitions (milliseconds)";
    DEFAULT_PERF_AUTOSTOP_SETTLE_MS => perf_autostop_settle_ms,
        "Perf auto-stop settle after load/reload (milliseconds)";
    DEFAULT_PERF_TRACE_INNER_SLICE_MS => perf_trace_inner_slice_ms,
        "Perf trace poll inner slice (milliseconds)";
    DEFAULT_PERF_TRACE_OUTER_SLICE_MS => perf_trace_outer_slice_ms,
        "Perf trace outer poll interval (milliseconds)";
    DEFAULT_PERF_TRACE_OUTER_ITERS => perf_trace_outer_iters,
        "Perf trace outer poll max iterations";
    DEFAULT_PERF_TRACE_INNER_ITERS => perf_trace_inner_iters,
        "Perf trace inner drain iterations after complete";
    STATE_COLLECT_DEADLINE_SECS => state_collect_deadline_secs,
        "CDP storage collect outer deadline (seconds)";
    STATE_EVENT_RECV_SECS => state_event_recv_secs,
        "CDP storage event recv slice (seconds)";
    STATE_LOAD_SETTLE_MS => state_load_settle_ms,
        "Settle delay after load_state navigation (milliseconds)";

    // ── viewport defaults ────────────────────────────────────────────────
    DEFAULT_VIEWPORT_WIDTH => default_viewport_width,
        "Default headless Chrome window width when launch options omit viewport";
    DEFAULT_VIEWPORT_HEIGHT => default_viewport_height,
        "Default headless Chrome window height when launch options omit viewport";
}
