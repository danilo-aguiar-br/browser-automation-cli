// SPDX-License-Identifier: MIT OR Apache-2.0
//! Named constants for `browser-automation-cli` (anti-hardcode).
//!
//! Product endpoints that operators may override via XDG live here as **named**
//! compile-time defaults only (never scattered string literals in business logic).
//!
//! # Compile-time identity (`rules_rust_macros`)
//!
//! `env!("CARGO_PKG_*")` / `concat!` below are **build-time** Cargo macros, not
//! product runtime environment knobs (those remain XDG-only via `config set`).
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | identity | User agent, temp prefixes, endpoints, wire version, loopback host |
//! | network_presets | DevTools network throttling table and lookups |
//! | viewport | Viewport defaults and `WxHxDPR` spec parser |
//! | http | HTTP client, robots, and webhook budgets |
//! | cdp | CDP discovery, event pump, attach budgets |
//! | chrome_paths | Built-in Chrome discovery layout per OS (GAP-049) |
//! | lifecycle | BORN/FINALIZE/DIE reap budgets |
//! | lightpanda | Lightpanda engine startup and connect budgets |
//! | external_tools | Lighthouse / ffmpeg budgets and argv values |
//! | logging | Tracing level and log rotation defaults |
//! | cache | Redis/RESP budgets and L2 cache TTLs |
//! | payload_limits | Input-size ceilings and anti-DoS clamps |
//! | record | Interaction-recorder budgets and binding name |
//! | media | Screenshot / screencast quality defaults |
//! | timing | Event-pump slices, settle delays, perf polling |
//! | heap | Heap snapshot ceilings and node-op caps |
//! | retry | Retry policy budgets per transport family |
//! | mitm | MITM clamps, capture windows, redaction |
//!
//! All items are re-exported flat from `crate::constants` so existing paths keep
//! working unchanged.

mod cache;
mod cdp;
mod chrome_paths;
mod external_tools;
mod heap;
mod http;
mod identity;
mod lifecycle;
mod lightpanda;
mod logging;
mod media;
mod mitm;
mod network_presets;
mod payload_limits;
mod record;
mod retry;
mod stealth;
mod timing;
mod viewport;

pub use cache::*;
pub use cdp::*;
pub use chrome_paths::*;
pub use external_tools::*;
pub use heap::*;
pub use http::*;
pub use identity::*;
pub use lifecycle::*;
pub use lightpanda::*;
pub use logging::*;
pub use media::*;
pub use mitm::*;
pub use network_presets::*;
pub use payload_limits::*;
pub use record::*;
pub use retry::*;
pub use stealth::*;
pub use timing::*;
pub use viewport::*;

// Build-time invariants spanning the whole constant surface.
const _: () = assert!(MITM_WS_FRAMES_CAP > 0);
const _: () = assert!(MITM_WS_PREVIEW_CHARS > 0);
const _: () = assert!(MITM_CA_CACHE_SIZE > 0);
const _: () = assert!(!MITM_REDACTED_PLACEHOLDER.is_empty());
const _: () = assert!(!MITM_BIND_HOST.is_empty());
const _: () = assert!(MAX_SG_FILE_BYTES > 0);
const _: () = assert!(DEFAULT_VIEWPORT_WIDTH > 0);
const _: () = assert!(DEFAULT_VIEWPORT_HEIGHT > 0);
const _: () = assert!(REDIS_IO_TIMEOUT_SECS > 0);
const _: () = assert!(REDIS_CONNECT_TIMEOUT_SECS > 0);
const _: () = assert!(ROBOTS_FETCH_TIMEOUT_SECS > 0);
const _: () = assert!(DEFAULT_HTTP_CONNECT_TIMEOUT_SECS > 0);
const _: () = assert!(ROBOTS_PROBE_TIMEOUT_SECS > 0);
const _: () = assert!(ROBOTS_MAX_BODY_BYTES > 0);
const _: () = assert!(CDP_DISCOVERY_MAX_BODY_BYTES > 0);
const _: () = assert!(DEFAULT_SCRAPE_MAX_BODY_BYTES > 0);
const _: () = assert!(DEFAULT_BROWSER_SCRAPE_MAX_BODY_BYTES > 0);
const _: () = assert!(HTTP_REDIRECT_MAX > 0);
const _: () = assert!(HTTP_POOL_MAX_IDLE_PER_HOST > 0);
// HTTP/2 SETTINGS are a fingerprint, so the bounds are RFC 9113 bounds, not
// taste. A value outside them is rejected by the peer and the connection dies
// before the first request, which would turn a stealth knob into an outage.
const _: () = assert!(HTTP2_INITIAL_STREAM_WINDOW_SIZE <= i32::MAX as u32);
const _: () = assert!(HTTP2_INITIAL_CONNECTION_WINDOW_SIZE <= i32::MAX as u32);
const _: () = assert!(HTTP2_INITIAL_CONNECTION_WINDOW_SIZE >= HTTP2_INITIAL_STREAM_WINDOW_SIZE);
const _: () = assert!(HTTP2_MAX_FRAME_SIZE >= 16_384 && HTTP2_MAX_FRAME_SIZE <= 16_777_215);
const _: () = assert!(HTTP2_MAX_HEADER_LIST_SIZE > 0);
const _: () = assert!(DEFAULT_LLM_HTTP_TIMEOUT_SECS > 0);
const _: () = assert!(WEBHOOK_POST_TIMEOUT_SECS > 0);
const _: () = assert!(WEBHOOK_MAX_ATTEMPTS > 0);
const _: () = assert!(RETRY_DEFAULT_MAX_ATTEMPTS > 0);
const _: () = assert!(RETRY_BASE_DELAY_MS > 0);
const _: () = assert!(RETRY_MAX_DELAY_SECS > 0);
const _: () = assert!(RETRY_BUDGET_SECS > 0);
const _: () = assert!(RETRY_CDP_MAX_ATTEMPTS > 0);
const _: () = assert!(RETRY_HTTP_MAX_ATTEMPTS > 0);
const _: () = assert!(RETRY_LLM_MAX_ATTEMPTS > 0);
const _: () = assert!(SCRAPE_HTTP_CACHE_TTL_SECS > 0);
const _: () = assert!(FILE_PARSE_CACHE_TTL_SECS > 0);
const _: () = assert!(!DEFAULT_SEARCH_BASE_URL.is_empty());
const _: () = assert!(!HTTP_USER_AGENT.is_empty());
const _: () = assert!(!XLSX_TMP_NAME_PREFIX.is_empty());
const _: () = assert!(FINALIZE_CHILD_GRACE_SECS > 0);
const _: () = assert!(FINALIZE_CHILD_GRACE_SECS <= 5);
const _: () = assert!(CDP_EVENT_BROADCAST_CAPACITY > 0);
const _: () = assert!(BROWSER_CLOSE_WAIT_SECS > 0);
const _: () = assert!(PLATFORM_CHILD_POLL_MS > 0);
const _: () = assert!(DEFAULT_LIGHTHOUSE_TIMEOUT_SECS > 0);
const _: () = assert!(DEFAULT_LIGHTHOUSE_TIMEOUT_SECS <= EXTERNAL_PROCESS_TIMEOUT_CAP_SECS);
const _: () = assert!(DEFAULT_FFMPEG_ENCODE_TIMEOUT_SECS > 0);
const _: () = assert!(DEFAULT_FFMPEG_ENCODE_TIMEOUT_SECS <= EXTERNAL_PROCESS_TIMEOUT_CAP_SECS);
const _: () = assert!(EXTERNAL_PROCESS_TIMEOUT_CAP_SECS >= 60);
const _: () = assert!(SCREENCAST_FFMPEG_FRAMERATE > 0);
const _: () = assert!(!SCREENCAST_FFMPEG_VCODEC_MP4.is_empty());
const _: () = assert!(!SCREENCAST_FFMPEG_VCODEC_WEBM.is_empty());
const _: () = assert!(!SCREENCAST_FFMPEG_PIX_FMT.is_empty());
const _: () = assert!(!LIGHTHOUSE_CHROME_FLAGS.is_empty());
const _: () = assert!(!LIGHTHOUSE_ONLY_CATEGORIES.is_empty());
const _: () = assert!(!LOOPBACK_HOST.is_empty());
const _: () = assert!(LIGHTPANDA_STARTUP_TIMEOUT_SECS > 0);
const _: () = assert!(LIGHTPANDA_POLL_INTERVAL_MS > 0);
const _: () = assert!(LIGHTPANDA_DISCOVERY_TIMEOUT_MS > 0);
const _: () = assert!(LIGHTPANDA_SESSION_TIMEOUT_SECS > 0);
const _: () = assert!(LIGHTPANDA_MAX_LOG_LINES > 0);
const _: () = assert!(!DEFAULT_LOG_LEVEL.is_empty());
const _: () = assert!(DEFAULT_MAX_LOG_FILES >= MAX_LOG_FILES_MIN);
const _: () = assert!(DEFAULT_MAX_LOG_FILES <= MAX_LOG_FILES_CAP);
const _: () = assert!(MAX_LOG_FILES_MIN >= 1);
const _: () = assert!(MAX_LOG_FILES_CAP >= DEFAULT_MAX_LOG_FILES);
const _: () = assert!(!DEFAULT_LOG_ROTATION.is_empty());
const _: () = assert!(LIGHTPANDA_READY_SLICE_MS > 0);
const _: () = assert!(DEFAULT_CDP_DISCOVERY_TIMEOUT_SECS > 0);
const _: () = assert!(CDP_CONNECTION_PROBE_TIMEOUT_SECS > 0);
const _: () = assert!(LIGHTPANDA_CDP_CONNECT_TIMEOUT_SECS > 0);
const _: () = assert!(LIGHTPANDA_TARGET_INIT_TIMEOUT_SECS > 0);
const _: () = assert!(EVENT_TRACKER_MAX_ENTRIES > 0);
const _: () = assert!(CACHE_MAX_RESP_BULK_BYTES > 0);
const _: () = assert!(CACHE_MAX_RESP_LINE_BYTES > 0);
const _: () = assert!(ENVELOPE_SCHEMA_VERSION >= 1);
const _: () = assert!(DEFAULT_MAX_JSON_FILE_BYTES > 0);
const _: () = assert!(DEFAULT_MAX_NDJSON_LINE_BYTES > 0);
const _: () = assert!(DEFAULT_MAX_CLI_JSON_PAYLOAD_BYTES > 0);
const _: () = assert!(DEFAULT_JPEG_QUALITY >= 1 && DEFAULT_JPEG_QUALITY <= 100);
const _: () = assert!(DEFAULT_EVENT_PUMP_SLICE_MS > 0);
const _: () = assert!(DEFAULT_EVAL_DRAIN_SLICE_MS > 0);
const _: () = assert!(DRAG_INTERCEPT_BUDGET_MS > 0);
const _: () = assert!(DEFAULT_NETWORK_IDLE_WINDOW_MS > 0);
const _: () = assert!(DEFAULT_DOM_STABLE_WINDOW_MS > 0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_user_agent_matches_cargo_package_identity() {
        assert!(
            HTTP_USER_AGENT.starts_with(env!("CARGO_PKG_NAME")),
            "UA must start with package name: {HTTP_USER_AGENT}"
        );
        assert!(
            HTTP_USER_AGENT.contains(env!("CARGO_PKG_VERSION")),
            "UA must embed package version: {HTTP_USER_AGENT}"
        );
        assert!(
            HTTP_USER_AGENT.contains(env!("CARGO_PKG_HOMEPAGE")),
            "UA must embed package homepage: {HTTP_USER_AGENT}"
        );
        assert!(
            HTTP_USER_AGENT.contains("local-scrape"),
            "UA must keep politeness suffix: {HTTP_USER_AGENT}"
        );
    }

    #[test]
    fn xlsx_tmp_prefix_uses_package_name() {
        assert!(XLSX_TMP_NAME_PREFIX.contains(env!("CARGO_PKG_NAME")));
        assert!(XLSX_TMP_NAME_PREFIX.starts_with('.'));
    }

    #[test]
    fn preset_lookup() {
        assert!(network_preset_by_name("Slow 3G").is_some());
        assert!(network_preset_by_name("offline").is_some());
        assert!(network_preset_by_name("nope").is_none());
    }

    #[test]
    fn viewport_parse() {
        let v = parse_viewport_spec("412x823x1.75,mobile,touch").unwrap();
        assert_eq!(v.width, 412);
        assert_eq!(v.height, 823);
        assert!((v.device_scale_factor - 1.75).abs() < 0.001);
        assert!(v.mobile && v.has_touch);
    }
}
