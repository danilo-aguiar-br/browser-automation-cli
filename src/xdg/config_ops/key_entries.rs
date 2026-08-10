// SPDX-License-Identifier: MIT OR Apache-2.0
//! The described key catalog behind `config list-keys`.
//!
//! Split out of `keys.rs` because the two halves answer different questions and
//! grow at different rates. `keys.rs` answers "is this key real", which every
//! `config set` needs and which is a flat list of names. This file answers
//! "what is this key for", which only the discovery surface needs and which
//! carries a default and a sentence per row.
//!
//! Keeping them together pushed the file past the 300-code-line ceiling the
//! moment six keys were added at once, and the halves have no shared state.

use serde_json::{json, Value};

/// Append the pure-Rust codec and manifest knobs.
///
/// These live in a helper rather than in the `json!([…])` literal above on
/// purpose: that literal already sits near `serde_json`'s macro recursion
/// ceiling, and growing it further fails the build with "recursion limit
/// reached". Pushing shallow objects keeps expansion depth flat.
pub(super) fn push_wave6_media_entries(out: &mut Vec<Value>) {
    out.push(json!({
        "key": "image_avif_speed",
        "default": crate::constants::DEFAULT_IMAGE_AVIF_SPEED,
        "description": "AVIF encoder speed 1..=10 (1 slowest/best); needs the image-avif feature"
    }));
    out.push(json!({
        "key": "svg_max_bytes",
        "default": crate::constants::DEFAULT_SVG_MAX_BYTES,
        "description": "Max SVG source bytes accepted before rasterisation"
    }));
    out.push(json!({
        "key": "svg_max_depth",
        "default": crate::constants::DEFAULT_SVG_MAX_DEPTH,
        "description": "Max XML nesting depth accepted in an SVG source"
    }));
    out.push(json!({
        "key": "svg_max_entities",
        "default": crate::constants::DEFAULT_SVG_MAX_ENTITIES,
        "description": "Max <!ENTITY> declarations tolerated in an SVG DTD (0 = reject any)"
    }));
    out.push(json!({
        "key": "gif_max_frames",
        "default": crate::constants::DEFAULT_GIF_MAX_FRAMES,
        "description": "Max animation frames decoded from a GIF"
    }));
    out.push(json!({
        "key": "manifest_max_bytes",
        "default": crate::constants::DEFAULT_MANIFEST_MAX_BYTES,
        "description": "Max bytes accepted for an HLS or DASH manifest body"
    }));
    out.push(json!({
        "key": "manifest_max_variants",
        "default": crate::constants::DEFAULT_MANIFEST_MAX_VARIANTS,
        "description": "Max variant/representation entries emitted per manifest envelope"
    }));
}

/// Hand-written key catalog (non-policy knobs and secrets).
pub(super) fn base_key_entries() -> Vec<Value> {
    let entries = json!([
            {"key": "lang", "default": null, "description": "Message locale override (en|pt-BR; bare pt rejected)"},
            {"key": "timeout", "default": 0, "description": "Global timeout seconds"},
            {"key": "artifacts_dir", "default": null, "description": "Artifacts output directory"},
            {"key": "ignore_robots", "default": false, "description": "Default robots ignore (flags still required)"},
            {"key": "namespace", "default": null, "description": "Isolated state namespace"},
            {"key": "encryption_key", "default": null, "description": "Session encryption key material"},
            {"key": "color", "default": null, "description": "ANSI colors on human stderr"},
            {"key": "log_level", "default": crate::constants::DEFAULT_LOG_LEVEL, "description": "Tracing EnvFilter when argv flags quiet (no RUST_LOG)"},
            {"key": "input_profile", "default": "human", "description": "Default input shaping: human|direct"},
            {"key": "browser_mode", "default": crate::constants::DEFAULT_BROWSER_MODE, "description": "Window mode: auto|headed|headless (auto resolves to headless; doctor reports it)"},
            {"key": "stealth", "default": true, "description": "Anti-detection patches before first navigation (--no-stealth opts out)"},
            {"key": "stealth_profile", "default": crate::constants::DEFAULT_STEALTH_PROFILE, "description": "Impersonated identity: auto|chrome-linux|chrome-win|chrome-mac"},
            {"key": "proxy_url", "default": null, "description": "Egress proxy for Chrome and the HTTP engine"},
            {"key": "proxy_bypass", "default": null, "description": "Hosts bypassing the proxy (Chrome bypass-list syntax)"},
            {"key": "cdp_proxy_bypass_loopback", "default": true, "description": "Always bypass loopback under --proxy so the CDP control channel survives"},
            {"key": "proxy_username", "default": null, "description": "Proxy username (XDG only; argv is visible in the process table)"},
            {"key": "proxy_password", "default": null, "description": "Proxy password (XDG only; argv is visible in the process table)"},
            {"key": "stealth_seed", "default": null, "description": "Pin the stealth identity across processes (absent = redrawn per process)"},
            {"key": "http2_enabled", "default": crate::constants::DEFAULT_HTTP2_ENABLED, "description": "Negotiate HTTP/2 on the shared HTTP client (Chrome always offers h2)"},
            {"key": "http2_initial_stream_window_size", "default": crate::constants::HTTP2_INITIAL_STREAM_WINDOW_SIZE, "description": "HTTP/2 SETTINGS_INITIAL_WINDOW_SIZE advertised to the peer"},
            {"key": "http2_initial_connection_window_size", "default": crate::constants::HTTP2_INITIAL_CONNECTION_WINDOW_SIZE, "description": "HTTP/2 connection-level flow-control window"},
            {"key": "http2_max_header_list_size", "default": crate::constants::HTTP2_MAX_HEADER_LIST_SIZE, "description": "HTTP/2 SETTINGS_MAX_HEADER_LIST_SIZE"},
            {"key": "http2_max_frame_size", "default": crate::constants::HTTP2_MAX_FRAME_SIZE, "description": "HTTP/2 SETTINGS_MAX_FRAME_SIZE (16384..=16777215)"},
            {"key": "http2_adaptive_window", "default": crate::constants::HTTP2_ADAPTIVE_WINDOW, "description": "Allow the HTTP/2 window to resize at runtime (off keeps the fingerprint constant)"},
            {"key": "robots_user_agent", "default": null, "description": "User-agent token robots.txt rules are matched against"},
            {"key": "log_to_file", "default": false, "description": "Rotated local JSON logs under XDG state (never remote)"},
            {"key": "max_log_files", "default": crate::constants::DEFAULT_MAX_LOG_FILES, "description": "Retained rotated log files (1..=90)"},
            {"key": "log_rotation", "default": crate::constants::DEFAULT_LOG_ROTATION, "description": "Rolling policy: daily|hourly|never"},
            {"key": "chrome_path", "default": null, "description": "Absolute Chrome/Chromium path"},
            {"key": "lighthouse_path", "default": null, "description": "Absolute lighthouse CLI path"},
            {"key": "ffmpeg_path", "default": null, "description": "Absolute ffmpeg path (optional screencast encode + video convert/to-mp3)"},
            {"key": "lighthouse_timeout_secs", "default": crate::constants::DEFAULT_LIGHTHOUSE_TIMEOUT_SECS, "description": "Wall-clock lighthouse CLI timeout (seconds, 1..=3600)"},
            {"key": "ffmpeg_timeout_secs", "default": crate::constants::DEFAULT_FFMPEG_ENCODE_TIMEOUT_SECS, "description": "Wall-clock ffmpeg encode timeout (seconds, 1..=3600)"},
            {"key": "openrouter_api_key", "default": null, "description": "LLM API key (stored 0600)"},
            {"key": "llm_base_url", "default": null, "description": "OpenAI-compatible base URL"},
            {"key": "llm_model", "default": null, "description": "Default LLM model id"},
            {"key": "cache_backend", "default": "sqlite", "description": "sqlite|memory|redis"},
            {"key": "cache_redis_url", "default": null, "description": "Redis URL when backend=redis"},
            {"key": "search_base_url", "default": crate::constants::DEFAULT_SEARCH_BASE_URL, "description": "HTML search endpoint base (?q= appended)"},
            {"key": "lightpanda_startup_timeout_secs", "default": crate::constants::LIGHTPANDA_STARTUP_TIMEOUT_SECS, "description": "Lightpanda process startup wait (seconds)"},
            {"key": "lightpanda_session_timeout_secs", "default": crate::constants::LIGHTPANDA_SESSION_TIMEOUT_SECS, "description": "Lightpanda --timeout session max (seconds, 1..=604800)"},
            {"key": "max_json_file_bytes", "default": crate::constants::DEFAULT_MAX_JSON_FILE_BYTES, "description": "Max bytes for JSON/NDJSON script or manifest files"},
            {"key": "max_ndjson_line_bytes", "default": crate::constants::DEFAULT_MAX_NDJSON_LINE_BYTES, "description": "Max bytes for one NDJSON line (run scripts / traces)"},
            {"key": "max_cli_json_payload_bytes", "default": crate::constants::DEFAULT_MAX_CLI_JSON_PAYLOAD_BYTES, "description": "Max bytes for CLI flag JSON payloads"},
            {"key": "default_jpeg_quality", "default": crate::constants::DEFAULT_JPEG_QUALITY, "description": "JPEG quality 1..=100 when grab omits --quality"},
            {"key": "event_pump_slice_ms", "default": crate::constants::DEFAULT_EVENT_PUMP_SLICE_MS, "description": "Wait/eval event pump slice (milliseconds)"},
            {"key": "screencast_jpeg_quality", "default": crate::constants::DEFAULT_SCREENCAST_JPEG_QUALITY, "description": "Screencast CDP JPEG quality 1..=100"},
            {"key": "interact_settle_ms", "default": crate::constants::DEFAULT_INTERACT_SETTLE_MS, "description": "UI settle delay after click/type/extension (ms)"},
            {"key": "dialog_settle_ms", "default": crate::constants::DEFAULT_DIALOG_SETTLE_MS, "description": "Max wait after JS dialog answer for javascriptDialogClosed (ms, GAP-054)"},
            {"key": "cdp_connection_probe_timeout_secs", "default": crate::constants::CDP_CONNECTION_PROBE_TIMEOUT_SECS, "description": "CDP Browser.getVersion liveness probe timeout (seconds)"},
            {"key": "http_ssrf_mode", "default": "strict", "description": "HTTP SSRF policy: strict|allow_loopback|off"},
            {"key": "http_timeout_secs", "default": crate::constants::DEFAULT_HTTP_TIMEOUT_SECS, "description": "Shared HTTP client total timeout (seconds)"},
            {"key": "http_connect_timeout_secs", "default": crate::constants::DEFAULT_HTTP_CONNECT_TIMEOUT_SECS, "description": "HTTP connect-phase timeout (seconds)"},
            {"key": "scrape_max_body_bytes", "default": crate::constants::DEFAULT_SCRAPE_MAX_BODY_BYTES, "description": "Max HTTP scrape body bytes"},
            {"key": "llm_http_timeout_secs", "default": crate::constants::DEFAULT_LLM_HTTP_TIMEOUT_SECS, "description": "LLM/webhook blocking HTTP timeout (seconds)"},
            {"key": "redis_allow_remote", "default": false, "description": "Allow non-loopback Redis hosts (default false)"},
            {"key": "redis_connect_timeout_secs", "default": crate::constants::REDIS_CONNECT_TIMEOUT_SECS, "description": "Redis TCP connect timeout (seconds)"},
            {"key": "robots_loopback_exempt", "default": true, "description": "Loopback hosts skip robots.txt (set false to enforce against localhost)"},
            {"key": "allowed_roots", "default": null, "description": "Extra allowed roots for local reads and artifact writes (platform-separated); defaults cover cwd, XDG dirs and temp"},
            {"key": "chrome_search_paths", "default": null, "description": "Ordered Chrome/Chromium discovery paths (platform-separated); empty uses the built-in per-OS layout"},
            {"key": "image_max_input_bytes", "default": crate::constants::DEFAULT_IMAGE_MAX_INPUT_BYTES, "description": "Max bytes for local image decode/convert/resize input"},
            {"key": "image_max_pixels", "default": crate::constants::DEFAULT_IMAGE_MAX_PIXELS, "description": "Max width*height for image decode (anti-bomb)"},
            {"key": "image_default_format", "default": crate::constants::DEFAULT_IMAGE_FORMAT, "description": "Default image convert format: png|jpeg|webp|gif"},
            {"key": "image_default_quality", "default": crate::constants::DEFAULT_IMAGE_QUALITY, "description": "Default lossy quality 1..=100 for image convert/resize"},
            {"key": "image_download_max_bytes", "default": crate::constants::DEFAULT_IMAGE_DOWNLOAD_MAX_BYTES, "description": "Max HTTP body bytes for image download"},
            {"key": "video_max_input_bytes", "default": crate::constants::DEFAULT_VIDEO_MAX_INPUT_BYTES, "description": "Max bytes for video stdin materialization / path pre-check"},
            {"key": "video_download_max_bytes", "default": crate::constants::DEFAULT_VIDEO_DOWNLOAD_MAX_BYTES, "description": "Max HTTP body bytes for video download"},
            {"key": "video_default_container", "default": crate::constants::DEFAULT_VIDEO_CONTAINER, "description": "Default video convert container: mp4|webm|mkv|mov|avi|m4v"},
            {"key": "video_default_crf", "default": crate::constants::DEFAULT_VIDEO_CRF, "description": "Default CRF 1..=51 for lossy video re-encode"},
            {"key": "video_default_audio_bitrate", "default": crate::constants::DEFAULT_VIDEO_AUDIO_BITRATE, "description": "Default bitrate for video to-mp3 (e.g. 192k)"},
    ]);
    // Append after the macro array: large json! arrays hit the recursion limit (see full_dump Map pattern).
    let mut out = entries.as_array().cloned().unwrap_or_default();
    push_wave6_media_entries(&mut out);
    out.push(json!({
        "key": "chrome_legacy_oxide_launch",
        "default": false,
        "description": "Launch Chrome via chromiumoxide instead of the self-spawn path (stabilization fallback; loses the residual kill target)"
    }));
    out.push(json!({
        "key": "audio_max_input_bytes",
        "default": crate::constants::DEFAULT_AUDIO_MAX_INPUT_BYTES,
        "description": "Max bytes for audio stdin materialization / path pre-check"
    }));
    out.push(json!({
        "key": "audio_download_max_bytes",
        "default": crate::constants::DEFAULT_AUDIO_DOWNLOAD_MAX_BYTES,
        "description": "Max HTTP body bytes for audio download"
    }));
    out.push(json!({
        "key": "audio_default_format",
        "default": crate::constants::DEFAULT_AUDIO_FORMAT,
        "description": "Default audio convert format: mp3|m4a|ogg|opus|flac|wav|aac"
    }));
    out.push(json!({
        "key": "audio_default_bitrate",
        "default": crate::constants::DEFAULT_AUDIO_BITRATE,
        "description": "Default bitrate for lossy audio encode (e.g. 192k)"
    }));
    out.push(json!({
        "key": "scrape_max_text_chars",
        "default": crate::constants::DEFAULT_SCRAPE_MAX_TEXT_CHARS,
        "description": "Max text/markdown chars in scrape envelopes (0=no cap)"
    }));
    out.push(json!({
        "key": "scrape_min_delay_ms",
        "default": crate::constants::DEFAULT_SCRAPE_MIN_DELAY_MS,
        "description": "Floor delay between same-origin GETs (ms)"
    }));
    out.push(json!({
        "key": "scrape_honor_meta_robots",
        "default": true,
        "description": "Honor meta robots / X-Robots-Tag noindex"
    }));
    out.push(json!({
        "key": "scrape_honor_nofollow",
        "default": true,
        "description": "Skip rel=nofollow links in crawl discovery"
    }));
    out.push(json!({
        "key": "scrape_use_sitemap",
        "default": true,
        "description": "Prefer sitemap.xml when mapping a site"
    }));
    out.push(json!({
        "key": "scrape_default_engine",
        "default": crate::constants::DEFAULT_SCRAPE_ENGINE,
        "description": "Default scrape engine when CLI omits --engine (http|browser)"
    }));
    out.push(json!({
        "key": "scrape_delay_jitter_ratio",
        "default": crate::constants::DEFAULT_SCRAPE_DELAY_JITTER_RATIO,
        "description": "Politeness delay jitter ratio 0.0..=1.0 (0=off)"
    }));
    out.push(json!({
        "key": "scrape_summary_chars",
        "default": crate::constants::DEFAULT_SCRAPE_SUMMARY_CHARS,
        "description": "Max chars for scrape format summary"
    }));
    out.push(json!({
        "key": "scrape_feed_max_entries",
        "default": crate::constants::DEFAULT_SCRAPE_FEED_MAX_ENTRIES,
        "description": "Max entries kept by scrape format feed (RSS/Atom/JSON Feed)"
    }));
    out.push(json!({
        "key": "scrape_follow_rel_next",
        "default": crate::constants::DEFAULT_SCRAPE_FOLLOW_REL_NEXT,
        "description": "Follow rel=next pagination links during crawl"
    }));
    out.push(json!({
        "key": "scrape_dedup_similar",
        "default": crate::constants::DEFAULT_SCRAPE_DEDUP_SIMILAR,
        "description": "Collapse near-duplicate pages by content similarity in crawl/batch-scrape"
    }));
    out.push(json!({
        "key": "scrape_no_cache",
        "default": crate::constants::DEFAULT_SCRAPE_NO_CACHE,
        "description": "Ignore the response cache on read and always fetch from origin"
    }));
    out.push(json!({
        "key": "scrape_dedup_similar_distance",
        "default": crate::constants::DEFAULT_SCRAPE_DEDUP_SIMILAR_DISTANCE,
        "description": "SimHash Hamming distance (0..=64) under which pages are near-duplicates"
    }));
    out.push(json!({
        "key": "scrape_sitemap_max_bytes",
        "default": crate::constants::DEFAULT_SCRAPE_SITEMAP_MAX_BYTES,
        "description": "Max sitemap body bytes"
    }));
    out.push(json!({
        "key": "scrape_charset_peek_bytes",
        "default": crate::constants::DEFAULT_SCRAPE_CHARSET_PEEK_BYTES,
        "description": "Charset sniffing peek window (bytes)"
    }));
    out
}
