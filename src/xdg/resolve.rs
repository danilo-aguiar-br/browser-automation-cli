// SPDX-License-Identifier: MIT OR Apache-2.0
//! Resolve XDG knobs with named constant defaults (no product env).

use super::config_io::load_config;

/// Resolve Lightpanda startup wait: XDG override when `> 0`, else constant default.
pub fn resolve_lightpanda_startup_timeout_secs() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.lightpanda_startup_timeout_secs)
        .filter(|&t| t > 0)
        .unwrap_or(crate::constants::LIGHTPANDA_STARTUP_TIMEOUT_SECS)
}

/// Resolve Lightpanda session `--timeout`: XDG when in `1..=max`, else constant default.
pub fn resolve_lightpanda_session_timeout_secs() -> u64 {
    let max = crate::constants::LIGHTPANDA_SESSION_TIMEOUT_SECS;
    load_config()
        .ok()
        .and_then(|c| c.lightpanda_session_timeout_secs)
        .filter(|&t| t > 0 && t <= max)
        .unwrap_or(max)
}

/// Resolve max JSON/NDJSON file size: XDG when `> 0`, else named default.
pub fn resolve_max_json_file_bytes() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.max_json_file_bytes)
        .filter(|&n| n > 0)
        .unwrap_or(crate::constants::DEFAULT_MAX_JSON_FILE_BYTES)
}

/// Resolve max NDJSON line size: XDG when `> 0`, else named default.
pub fn resolve_max_ndjson_line_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.max_ndjson_line_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::constants::DEFAULT_MAX_NDJSON_LINE_BYTES)
}

/// Resolve max CLI JSON payload size: XDG when `> 0`, else named default.
pub fn resolve_max_cli_json_payload_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.max_cli_json_payload_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::constants::DEFAULT_MAX_CLI_JSON_PAYLOAD_BYTES)
}

/// Resolve default JPEG quality: XDG when in `1..=100`, else named default.
pub fn resolve_default_jpeg_quality() -> i32 {
    load_config()
        .ok()
        .and_then(|c| c.default_jpeg_quality)
        .filter(|&n| (1..=100).contains(&n))
        .map(i32::from)
        .unwrap_or(i32::from(crate::constants::DEFAULT_JPEG_QUALITY))
}

/// Resolve event pump / wait slice (ms): XDG when `> 0`, else named default.
pub fn resolve_event_pump_slice_ms() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.event_pump_slice_ms)
        .filter(|&n| n > 0)
        .unwrap_or(crate::constants::DEFAULT_EVENT_PUMP_SLICE_MS)
}

/// Resolve eval drain slice (ms): min of pump slice and named eval default.
pub fn resolve_eval_drain_slice_ms() -> u64 {
    resolve_event_pump_slice_ms().min(crate::xdg::policy::policy_u64(
        crate::xdg::policy::key::DEFAULT_EVAL_DRAIN_SLICE_MS,
    ))
}

/// Resolve screencast CDP JPEG quality: XDG when in `1..=100`, else named default.
pub fn resolve_screencast_jpeg_quality() -> i32 {
    load_config()
        .ok()
        .and_then(|c| c.screencast_jpeg_quality)
        .filter(|&n| (1..=100).contains(&n))
        .map(i32::from)
        .unwrap_or(i32::from(crate::constants::DEFAULT_SCREENCAST_JPEG_QUALITY))
}

/// Resolve UI interact settle delay (ms): XDG when `> 0`, else named default.
pub fn resolve_interact_settle_ms() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.interact_settle_ms)
        .filter(|&n| n > 0)
        .unwrap_or(crate::constants::DEFAULT_INTERACT_SETTLE_MS)
}

/// Resolve dialog settle budget (ms) after handleJavaScriptDialog (GAP-054).
pub fn resolve_dialog_settle_ms() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.dialog_settle_ms)
        .filter(|&n| n > 0)
        .unwrap_or(crate::constants::DEFAULT_DIALOG_SETTLE_MS)
}

/// Resolve CDP connection probe timeout (seconds): XDG when `> 0`, else named default.
pub fn resolve_cdp_connection_probe_timeout_secs() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.cdp_connection_probe_timeout_secs)
        .filter(|&n| n > 0)
        .unwrap_or(crate::constants::CDP_CONNECTION_PROBE_TIMEOUT_SECS)
}

/// Chrome/Chromium binary from XDG config only.
pub fn chrome_path_from_config() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.chrome_path)
        .filter(|s| !s.is_empty())
}

/// Ordered Chrome/Chromium discovery search paths from XDG config (GAP-049).
///
/// Empty vector means "use the built-in per-OS install layout". The exact
/// override [`chrome_path_from_config`] still wins over this list.
pub fn resolve_chrome_search_paths() -> Vec<String> {
    load_config()
        .ok()
        .and_then(|c| c.chrome_search_paths)
        .unwrap_or_default()
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect()
}

/// Extra allowed roots for local reads and artifact writes (GAP-026).
pub fn resolve_allowed_roots() -> Vec<String> {
    load_config()
        .ok()
        .and_then(|c| c.allowed_roots)
        .unwrap_or_default()
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect()
}

/// Lighthouse binary path from XDG config only.
pub fn lighthouse_path_from_config() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.lighthouse_path)
        .filter(|s| !s.is_empty())
}

/// ffmpeg binary path from XDG config only (optional screencast encode).
pub fn ffmpeg_path_from_config() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.ffmpeg_path)
        .filter(|s| !s.is_empty())
}

/// Lighthouse CLI wall-clock timeout (seconds): XDG or named default.
pub fn resolve_lighthouse_timeout_secs() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.lighthouse_timeout_secs)
        .filter(|&n| n > 0 && n <= crate::constants::EXTERNAL_PROCESS_TIMEOUT_CAP_SECS)
        .unwrap_or(crate::constants::DEFAULT_LIGHTHOUSE_TIMEOUT_SECS)
}

/// ffmpeg encode wall-clock timeout (seconds): XDG or named default.
pub fn resolve_ffmpeg_timeout_secs() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.ffmpeg_timeout_secs)
        .filter(|&n| n > 0 && n <= crate::constants::EXTERNAL_PROCESS_TIMEOUT_CAP_SECS)
        .unwrap_or(crate::constants::DEFAULT_FFMPEG_ENCODE_TIMEOUT_SECS)
}

/// Optional LLM base URL from XDG config only.
pub fn llm_base_url() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.llm_base_url)
        .filter(|s| !s.is_empty())
}

/// Optional LLM model id from XDG config only.
pub fn llm_model() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.llm_model)
        .filter(|s| !s.is_empty())
}

/// HTML search base URL from XDG, or named compile-time default.
pub fn search_base_url() -> String {
    load_config()
        .ok()
        .and_then(|c| c.search_base_url)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| crate::constants::DEFAULT_SEARCH_BASE_URL.to_string())
}

/// Persistent Chrome profile directory from XDG, or `None`.
///
/// # Why `None` is the answer that matters
///
/// Absent is the default and absent means residual-zero: the launch gets a
/// throwaway profile and the run leaves nothing behind. A caller only reaches
/// the other branch by writing the key, which is the point — the trade is
/// visible in the config file rather than buried in a launch heuristic.
///
/// Whitespace-only is treated as absent, matching `search_base_url` above, so
/// `config set user_data_dir ""` clears the opt-in instead of asking Chrome to
/// use a profile directory named the empty string.
pub fn user_data_dir() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.user_data_dir)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// HTTP SSRF mode from XDG (`strict` | `allow_loopback` | `off`); default `strict`.
pub fn resolve_http_ssrf_mode() -> String {
    load_config()
        .ok()
        .and_then(|c| c.http_ssrf_mode)
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| matches!(s.as_str(), "strict" | "allow_loopback" | "off"))
        .unwrap_or_else(|| "strict".to_string())
}

/// Shared HTTP client total timeout (seconds).
pub fn resolve_http_timeout_secs() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.http_timeout_secs)
        .filter(|&n| n > 0 && n <= crate::constants::EXTERNAL_PROCESS_TIMEOUT_CAP_SECS)
        .unwrap_or(crate::constants::DEFAULT_HTTP_TIMEOUT_SECS)
}

/// HTTP connect-phase timeout (seconds).
pub fn resolve_http_connect_timeout_secs() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.http_connect_timeout_secs)
        .filter(|&n| n > 0 && n <= crate::constants::EXTERNAL_PROCESS_TIMEOUT_CAP_SECS)
        .unwrap_or(crate::constants::DEFAULT_HTTP_CONNECT_TIMEOUT_SECS)
}

/// Byte ceiling for the `monitor check --diff-mode` payload.
///
/// No `.filter(|&n| n > 0)` here because there is nothing to filter: the value
/// comes from `policy_u64`, which already drops a stored zero and falls back to
/// the named default. The guard lives one layer down, which is why this function
/// is allowed to be a single expression.
///
/// The narrowing goes through `usize::try_from`, not `as usize`, for the reason
/// spelled out in `resolve_scrape.rs`: on a 32-bit target the cast truncates in
/// silence, and a byte ceiling that silently becomes tiny is the worst of the
/// available failures.
///
/// It saturates instead of falling back to a default, which is where it departs
/// from its siblings, and the departure is deliberate: there is no named default
/// in scope here — `policy_u64` already applied it — so the only choice left is
/// what to do when the operator asked for more bytes than the machine can
/// address. Saturating grants the largest ceiling that exists, which is the
/// closest honest reading of "more than `usize::MAX`". The value still comes
/// from a validated layer, so this is not a way for a zero to slip through.
pub fn resolve_monitor_diff_max_bytes() -> usize {
    usize::try_from(crate::xdg::policy::policy_u64(
        crate::xdg::policy::key::MONITOR_DIFF_MAX_BYTES,
    ))
    .unwrap_or(usize::MAX)
}

/// Whether loopback is bypassed when Chrome is launched behind `--proxy`.
///
/// Defaults to true because the CDP control channel is loopback, and routing
/// it through an egress proxy produces a browser that never answers — a
/// failure that surfaces as a Chrome startup timeout and blames the wrong
/// component. Switch it off only when the proxy is deliberately expected to
/// carry the control channel too.
pub fn resolve_cdp_proxy_bypass_loopback() -> bool {
    load_config()
        .ok()
        .and_then(|c| c.cdp_proxy_bypass_loopback)
        .unwrap_or(true)
}

/// Whether the shared HTTP client negotiates HTTP/2.
pub fn resolve_http2_enabled() -> bool {
    load_config()
        .ok()
        .and_then(|c| c.http2_enabled)
        .unwrap_or(crate::constants::DEFAULT_HTTP2_ENABLED)
}

/// HTTP/2 `SETTINGS_INITIAL_WINDOW_SIZE` advertised to the peer.
///
/// Values above `i32::MAX` are dropped rather than clamped: a clamp would ship
/// a fingerprint the operator did not choose, and silently presenting the wrong
/// identity is the failure this whole surface exists to avoid.
pub fn resolve_http2_initial_stream_window_size() -> u32 {
    load_config()
        .ok()
        .and_then(|c| c.http2_initial_stream_window_size)
        .filter(|&n| n <= i32::MAX as u32)
        .unwrap_or(crate::constants::HTTP2_INITIAL_STREAM_WINDOW_SIZE)
}

/// HTTP/2 connection-level flow-control window.
pub fn resolve_http2_initial_connection_window_size() -> u32 {
    load_config()
        .ok()
        .and_then(|c| c.http2_initial_connection_window_size)
        .filter(|&n| n <= i32::MAX as u32)
        .unwrap_or(crate::constants::HTTP2_INITIAL_CONNECTION_WINDOW_SIZE)
}

/// HTTP/2 `SETTINGS_MAX_HEADER_LIST_SIZE`.
pub fn resolve_http2_max_header_list_size() -> u32 {
    load_config()
        .ok()
        .and_then(|c| c.http2_max_header_list_size)
        .filter(|&n| n > 0)
        .unwrap_or(crate::constants::HTTP2_MAX_HEADER_LIST_SIZE)
}

/// HTTP/2 `SETTINGS_MAX_FRAME_SIZE`, bounded by RFC 9113 §6.5.2.
pub fn resolve_http2_max_frame_size() -> u32 {
    load_config()
        .ok()
        .and_then(|c| c.http2_max_frame_size)
        .filter(|&n| (16_384..=16_777_215).contains(&n))
        .unwrap_or(crate::constants::HTTP2_MAX_FRAME_SIZE)
}

/// Whether the HTTP/2 flow-control window may resize at runtime.
pub fn resolve_http2_adaptive_window() -> bool {
    load_config()
        .ok()
        .and_then(|c| c.http2_adaptive_window)
        .unwrap_or(crate::constants::HTTP2_ADAPTIVE_WINDOW)
}

/// Proxy credentials from XDG, as a pair, or `None` when either half is absent.
///
/// Both halves are required together: reqwest's `Proxy::basic` takes a pair, and
/// sending a username with an empty password is an authentication attempt that
/// fails differently from no authentication at all.
pub fn resolve_proxy_credentials() -> Option<(String, String)> {
    let cfg = load_config().ok()?;
    let user = cfg.proxy_username.filter(|s| !s.is_empty())?;
    let pass = cfg.proxy_password?;
    Some((user, pass))
}

/// Explicit robots.txt user-agent token, when the operator pinned one.
pub fn resolve_robots_user_agent() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.robots_user_agent)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// The stealth identity seed, or `None` when the identity is redrawn per process.
pub fn resolve_stealth_seed() -> Option<String> {
    load_config()
        .ok()
        .and_then(|c| c.stealth_seed)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Explicit screen `WxH` from XDG, or `None` to mirror the viewport.
#[must_use]
pub fn resolve_screen_spec() -> Option<(i32, i32)> {
    load_config()
        .ok()
        .and_then(|c| c.screen)
        .and_then(|raw| crate::native::stealth::parse_screen_spec(raw.trim()).ok())
}

/// LLM/webhook blocking HTTP timeout (seconds).
pub fn resolve_llm_http_timeout_secs() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.llm_http_timeout_secs)
        .filter(|&n| n > 0 && n <= crate::constants::EXTERNAL_PROCESS_TIMEOUT_CAP_SECS)
        .unwrap_or(crate::constants::DEFAULT_LLM_HTTP_TIMEOUT_SECS)
}

/// Whether loopback hosts skip robots.txt (GAP-033; default `true`).
///
/// Defaults to `true` so the local test loop keeps working without the dual
/// risk flags. Setting `robots_loopback_exempt = false` enforces robots.txt
/// against loopback too, which is what makes the block path exercisable by a
/// hermetic fixture.
pub fn resolve_robots_loopback_exempt() -> bool {
    load_config()
        .ok()
        .and_then(|c| c.robots_loopback_exempt)
        .unwrap_or(true)
}

/// Whether the Chrome launch path falls back to `chromiumoxide::Browser::launch`.
///
/// Defaults to `false`: the product self-spawns Chrome so it owns the pid and the
/// process group. See [`crate::xdg::ProductConfig::chrome_legacy_oxide_launch`].
pub fn resolve_chrome_legacy_oxide_launch() -> bool {
    load_config()
        .ok()
        .and_then(|c| c.chrome_legacy_oxide_launch)
        .unwrap_or(false)
}

/// Allow non-loopback Redis hosts.
pub fn resolve_redis_allow_remote() -> bool {
    load_config()
        .ok()
        .and_then(|c| c.redis_allow_remote)
        .unwrap_or(false)
}

/// Redis TCP connect timeout (seconds).
pub fn resolve_redis_connect_timeout_secs() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.redis_connect_timeout_secs)
        .filter(|&n| n > 0 && n <= crate::constants::EXTERNAL_PROCESS_TIMEOUT_CAP_SECS)
        .unwrap_or(crate::constants::REDIS_CONNECT_TIMEOUT_SECS)
}

// Media (image/video) resolvers live in resolve_media for file-size hygiene.
pub use super::resolve_media::*;
