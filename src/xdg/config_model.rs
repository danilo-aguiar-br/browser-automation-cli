// SPDX-License-Identifier: MIT OR Apache-2.0
//! On-disk product configuration model (TOML via CLI `config set`).

use serde::{Deserialize, Serialize};

/// On-disk product configuration (TOML). Flags override these at parse time.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductConfig {
    /// Default language override (`en` / `pt-BR`).
    #[serde(default)]
    pub lang: Option<String>,
    /// Default global timeout seconds (0 = none).
    #[serde(default)]
    pub timeout: Option<u64>,
    /// Default artifacts directory.
    #[serde(default)]
    pub artifacts_dir: Option<String>,
    /// Default ignore-robots (requires explicit risk acceptance in flags still).
    #[serde(default)]
    pub ignore_robots: Option<bool>,
    /// Namespace for isolated state trees (optional).
    #[serde(default)]
    pub namespace: Option<String>,
    /// Optional AES key material for encrypted session state (stored in XDG config, mode 0600).
    #[serde(default)]
    pub encryption_key: Option<String>,
    /// Enable ANSI colors on human stderr paths when true.
    #[serde(default)]
    pub color: Option<bool>,
    /// Tracing filter level when flags are quiet/default (`error`/`info`/`debug`).
    #[serde(default)]
    pub log_level: Option<String>,
    /// Default `--input-profile` (`human` | `direct`).
    ///
    /// The flag still wins; this is what an operator sets once instead of
    /// repeating `--input-profile direct` on every invocation.
    pub input_profile: Option<String>,
    /// Shape of the dispersion around every input delay: `lognormal` |
    /// `normal` | `uniform`.
    ///
    /// A string key and not a `policy_knobs!` row because that macro is
    /// `u64`-only. It earns the seven-file cost because the SHAPE is what a
    /// detector reads: measured 2026-08-31 in the browser, the product's old
    /// uniform noise produced a skewness of 0.036, and human inter-key
    /// intervals sit between 1 and 3. Freezing the shape in a constant would be
    /// the hardcode this key exists to remove.
    ///
    /// # `uniform` does NOT mean zero skewness
    ///
    /// This key governs the shape of the FAST rhythm only. The observable
    /// asymmetry has a second source that this key does not touch: the long
    /// pause at a word or sentence boundary, which is its own knob,
    /// `input_word_pause_permille`.
    ///
    /// MEASURED 2026-08-31 on the final browser event, same seed, 56 inter-key
    /// intervals: `lognormal` gave a skewness of 2.534 and `uniform` gave
    /// 1.076 -- not the ~0.02 the same sampler produces in isolation. Setting
    /// `uniform` reproduces the pre-0.1.9 SAMPLER, not the pre-0.1.9 TRACE.
    /// To reproduce the old trace, set `input_word_pause_permille` to 0 as
    /// well.
    pub input_timing_distribution: Option<String>,
    /// Window mode when no flag decides: `auto` | `headed` | `headless`.
    ///
    /// `auto` is headed inside a private Xvfb on Linux and headed directly on
    /// macOS and Windows. Deliberately not an environment variable: window mode
    /// changes the fingerprint, so it belongs in a file an operator can audit.
    #[serde(default)]
    pub browser_mode: Option<String>,
    /// Whether anti-detection patches run. Default on; `--no-stealth` opts out.
    #[serde(default)]
    pub stealth: Option<bool>,
    /// Impersonated identity: `auto` | `chrome-linux` | `chrome-win` | `chrome-mac`.
    ///
    /// `auto` follows the host, which is the only value that cannot contradict
    /// the Canvas and WebGL hashes the real GPU produces.
    #[serde(default)]
    pub stealth_profile: Option<String>,
    /// Egress proxy URL for both Chrome and the HTTP engine.
    #[serde(default)]
    pub proxy_url: Option<String>,
    /// Hosts bypassing the proxy, in Chrome's bypass-list syntax.
    #[serde(default)]
    pub proxy_bypass: Option<String>,
    /// Always bypass loopback when `--proxy` is set, so the CDP control
    /// channel is not routed through the proxy (default true).
    #[serde(default)]
    pub cdp_proxy_bypass_loopback: Option<bool>,
    /// Proxy username. XDG only: argv is visible in the host process table.
    #[serde(default)]
    pub proxy_username: Option<String>,
    /// Proxy password. XDG only, for the same reason as [`Self::proxy_username`].
    #[serde(default)]
    pub proxy_password: Option<String>,
    /// Seed pinning the stealth identity across processes.
    ///
    /// Absent means the identity is redrawn per process, which is the historical
    /// behaviour. Present means the generated patch script is cached under XDG
    /// state so N one-shot runs present one machine instead of N.
    #[serde(default)]
    pub stealth_seed: Option<String>,
    /// Default screen size `WxH` for `Emulation.setDeviceMetricsOverride`.
    ///
    /// Absent means the screen mirrors the viewport. Never an environment
    /// variable: screen is a fingerprint signal and belongs in the audited file.
    #[serde(default)]
    pub screen: Option<String>,
    /// Negotiate HTTP/2 on the shared HTTP client.
    ///
    /// On by default: Chrome always offers `h2` in ALPN, so an HTTP/1.1-only
    /// client announces itself as a non-browser before any header is sent.
    #[serde(default)]
    pub http2_enabled: Option<bool>,
    /// HTTP/2 `SETTINGS_INITIAL_WINDOW_SIZE` advertised to the peer.
    #[serde(default)]
    pub http2_initial_stream_window_size: Option<u32>,
    /// HTTP/2 connection-level flow-control window.
    #[serde(default)]
    pub http2_initial_connection_window_size: Option<u32>,
    /// HTTP/2 `SETTINGS_MAX_HEADER_LIST_SIZE`.
    #[serde(default)]
    pub http2_max_header_list_size: Option<u32>,
    /// HTTP/2 `SETTINGS_MAX_FRAME_SIZE`.
    #[serde(default)]
    pub http2_max_frame_size: Option<u32>,
    /// Allow the HTTP/2 flow-control window to resize at runtime.
    #[serde(default)]
    pub http2_adaptive_window: Option<bool>,
    /// User-agent token `robots.txt` rules are matched against.
    ///
    /// Exists because stealth sends a browser User-Agent while the product's own
    /// identity string stays honest. Matching robots against the wrong token
    /// would read the wrong rules, so the token is explicit rather than implied.
    #[serde(default)]
    pub robots_user_agent: Option<String>,
    /// Absolute path to Chrome/Chromium binary (XDG only; never product env).
    #[serde(default)]
    pub chrome_path: Option<String>,
    /// Absolute path to lighthouse CLI (optional).
    #[serde(default)]
    pub lighthouse_path: Option<String>,
    /// Absolute path to ffmpeg (optional; screencast encode). Never product env.
    #[serde(default)]
    pub ffmpeg_path: Option<String>,
    /// Wall-clock timeout for lighthouse CLI (seconds). Never product env.
    #[serde(default)]
    pub lighthouse_timeout_secs: Option<u64>,
    /// Wall-clock timeout for ffmpeg screencast encode (seconds). Never product env.
    #[serde(default)]
    pub ffmpeg_timeout_secs: Option<u64>,
    /// Optional LLM provider API key for extract --llm (stored in XDG config mode 0600).
    #[serde(default)]
    pub openrouter_api_key: Option<String>,
    /// OpenAI-compatible API base URL (no trailing slash).
    #[serde(default)]
    pub llm_base_url: Option<String>,
    /// Default model id for extract --llm.
    #[serde(default)]
    pub llm_model: Option<String>,
    /// When true, also write rotated local logs under XDG state (never remote telemetry).
    #[serde(default)]
    pub log_to_file: Option<bool>,
    /// Max retained rotated log files under XDG state (1..=90; default 14).
    #[serde(default)]
    pub max_log_files: Option<u32>,
    /// Rolling policy: `daily` | `hourly` | `never` (default `daily`).
    #[serde(default)]
    pub log_rotation: Option<String>,
    /// Cache backend: `sqlite` (default layered) | `memory` | `redis`.
    #[serde(default)]
    pub cache_backend: Option<String>,
    /// Redis URL when cache_backend=redis (XDG only; never env).
    #[serde(default)]
    pub cache_redis_url: Option<String>,
    /// HTML search endpoint base URL (query appended as `?q=`). XDG only.
    #[serde(default)]
    pub search_base_url: Option<String>,
    /// Persistent Chrome profile directory. XDG only, absent by default.
    ///
    /// # Why this key trades away a product invariant
    ///
    /// Residual-zero is the default contract: a run leaves nothing on disk.
    /// Setting this key deliberately breaks it, because a detector that attests
    /// SESSION cannot be satisfied by fifty one-shot processes that each
    /// present as a different machine. The absent default is what keeps the
    /// contract true for everyone who did not ask.
    #[serde(default)]
    pub user_data_dir: Option<String>,
    /// Lightpanda process startup wait (seconds).
    #[serde(default)]
    pub lightpanda_startup_timeout_secs: Option<u64>,
    /// Lightpanda binary `--timeout` session max (seconds).
    #[serde(default)]
    pub lightpanda_session_timeout_secs: Option<u64>,
    /// Max bytes for a JSON/NDJSON script or manifest file.
    #[serde(default)]
    pub max_json_file_bytes: Option<u64>,
    /// Max bytes for one NDJSON line.
    #[serde(default)]
    pub max_ndjson_line_bytes: Option<u64>,
    /// Max bytes for CLI flag JSON payloads.
    #[serde(default)]
    pub max_cli_json_payload_bytes: Option<u64>,
    /// Default JPEG quality (1..=100) when grab omits `--quality`.
    #[serde(default)]
    pub default_jpeg_quality: Option<u8>,
    /// Event pump / wait_ms slice (milliseconds).
    #[serde(default)]
    pub event_pump_slice_ms: Option<u64>,
    /// Screencast CDP JPEG quality (1..=100).
    #[serde(default)]
    pub screencast_jpeg_quality: Option<u8>,
    /// UI interact settle delay (milliseconds) after click/type/extension load.
    #[serde(default)]
    pub interact_settle_ms: Option<u64>,
    /// Max wait after answering a JS dialog for `javascriptDialogClosed` (ms).
    #[serde(default)]
    pub dialog_settle_ms: Option<u64>,
    /// CDP connection liveness probe timeout (seconds) for `Browser.getVersion`.
    #[serde(default)]
    pub cdp_connection_probe_timeout_secs: Option<u64>,
    /// HTTP SSRF policy: `strict` | `allow_loopback` | `off` (default strict).
    #[serde(default)]
    pub http_ssrf_mode: Option<String>,
    /// Total HTTP client timeout seconds (async shared client).
    #[serde(default)]
    pub http_timeout_secs: Option<u64>,
    /// HTTP connect-phase timeout seconds.
    #[serde(default)]
    pub http_connect_timeout_secs: Option<u64>,
    /// Max HTTP scrape body bytes.
    #[serde(default)]
    pub scrape_max_body_bytes: Option<u64>,
    /// Max text/markdown chars in scrape envelopes (`0` = no cap).
    #[serde(default)]
    pub scrape_max_text_chars: Option<u64>,
    /// Floor delay between same-origin GETs (milliseconds).
    #[serde(default)]
    pub scrape_min_delay_ms: Option<u64>,
    /// Honor meta robots / X-Robots-Tag noindex.
    #[serde(default)]
    pub scrape_honor_meta_robots: Option<bool>,
    /// Skip nofollow links in crawl link discovery.
    #[serde(default)]
    pub scrape_honor_nofollow: Option<bool>,
    /// Prefer sitemap.xml when mapping a site.
    #[serde(default)]
    pub scrape_use_sitemap: Option<bool>,
    /// Default scrape engine (`http` | `browser`) when CLI omits `--engine`.
    #[serde(default)]
    pub scrape_default_engine: Option<String>,
    /// Politeness delay jitter ratio (0.0..=1.0).
    #[serde(default)]
    pub scrape_delay_jitter_ratio: Option<f64>,
    /// Max chars for format `summary`.
    #[serde(default)]
    pub scrape_summary_chars: Option<u64>,
    /// Max entries kept by scrape format `feed`.
    #[serde(default)]
    pub scrape_feed_max_entries: Option<u64>,
    /// Follow `rel=next` pagination links during crawl.
    #[serde(default)]
    pub scrape_follow_rel_next: Option<bool>,
    /// Collapse near-duplicate pages by content similarity.
    #[serde(default)]
    pub scrape_dedup_similar: Option<bool>,
    /// Ignore the response cache on read and always go to origin.
    #[serde(default)]
    pub scrape_no_cache: Option<bool>,
    /// SimHash Hamming distance under which pages count as near-duplicates.
    #[serde(default)]
    pub scrape_dedup_similar_distance: Option<u64>,
    /// Max sitemap body bytes.
    #[serde(default)]
    pub scrape_sitemap_max_bytes: Option<u64>,
    /// Charset sniffing peek window (bytes).
    #[serde(default)]
    pub scrape_charset_peek_bytes: Option<u64>,
    /// LLM/webhook blocking HTTP client timeout seconds.
    #[serde(default)]
    pub llm_http_timeout_secs: Option<u64>,
    /// When true, allow non-loopback Redis hosts (default false).
    #[serde(default)]
    pub redis_allow_remote: Option<bool>,
    /// When true, launch Chrome through `chromiumoxide::Browser::launch` again.
    ///
    /// Stabilization escape hatch for the self-spawn path. The legacy path hands
    /// the child to chromiumoxide, so the product never learns the pid and
    /// FINALIZE has no residual kill target — that is the defect the self-spawn
    /// exists to fix. Only set this when the self-spawn path fails on a host,
    /// and expect residue after a hard kill while it is on.
    #[serde(default)]
    pub chrome_legacy_oxide_launch: Option<bool>,
    /// When true, loopback hosts skip robots.txt (default true; GAP-033).
    ///
    /// Set to `false` to enforce robots.txt even against `127.0.0.1` /
    /// `localhost`, which is what makes the block path reachable for a hermetic
    /// fixture (a local test server is necessarily loopback).
    #[serde(default)]
    pub robots_loopback_exempt: Option<bool>,
    /// Redis TCP connect timeout seconds.
    #[serde(default)]
    pub redis_connect_timeout_secs: Option<u64>,
    /// Ordered Chrome/Chromium discovery search paths (GAP-049).
    ///
    /// Empty means "use the built-in per-OS install layout". `chrome_path`
    /// still wins as an exact override.
    #[serde(default)]
    pub chrome_search_paths: Option<Vec<String>>,
    /// Extra allowed roots for local reads and artifact writes (GAP-026).
    #[serde(default)]
    pub allowed_roots: Option<Vec<String>>,
    /// Max bytes for local image decode / convert / resize input.
    #[serde(default)]
    pub image_max_input_bytes: Option<u64>,
    /// Max decoded pixel count (`width * height`) for image ops.
    #[serde(default)]
    pub image_max_pixels: Option<u64>,
    /// Default `image convert` output format (`png`|`jpeg`|`webp`|`gif`).
    #[serde(default)]
    pub image_default_format: Option<String>,
    /// Default lossy quality for image convert/resize (1..=100).
    #[serde(default)]
    pub image_default_quality: Option<u8>,
    /// Max HTTP body bytes for `image download`.
    #[serde(default)]
    pub image_download_max_bytes: Option<u64>,
    /// AVIF encoder speed 1..=10 (1 slowest/best) for `image convert --format avif`.
    #[serde(default)]
    pub image_avif_speed: Option<u8>,
    /// Max bytes accepted for an SVG source before rasterisation.
    #[serde(default)]
    pub svg_max_bytes: Option<u64>,
    /// Max XML nesting depth accepted in an SVG source.
    #[serde(default)]
    pub svg_max_depth: Option<u32>,
    /// Max `<!ENTITY>` declarations tolerated in an SVG DTD.
    #[serde(default)]
    pub svg_max_entities: Option<u32>,
    /// Max animation frames decoded from a GIF.
    #[serde(default)]
    pub gif_max_frames: Option<u32>,
    /// Max bytes accepted for an HLS / DASH manifest body.
    #[serde(default)]
    pub manifest_max_bytes: Option<u64>,
    /// Max variant / representation entries emitted per manifest envelope.
    #[serde(default)]
    pub manifest_max_variants: Option<u32>,
    /// Max bytes for local video stdin materialization / path pre-check.
    #[serde(default)]
    pub video_max_input_bytes: Option<u64>,
    /// Max HTTP body bytes for `video download`.
    #[serde(default)]
    pub video_download_max_bytes: Option<u64>,
    /// Default `video convert` container (`mp4`|`webm`|`mkv`|`mov`|`avi`|`m4v`).
    #[serde(default)]
    pub video_default_container: Option<String>,
    /// Default CRF for lossy video re-encode (1..=51).
    #[serde(default)]
    pub video_default_crf: Option<u8>,
    /// Default audio bitrate for `video to-mp3` (e.g. `192k`).
    #[serde(default)]
    pub video_default_audio_bitrate: Option<String>,
    /// Max bytes for audio stdin / path pre-check.
    #[serde(default)]
    pub audio_max_input_bytes: Option<u64>,
    /// Max HTTP body for `audio download`.
    #[serde(default)]
    pub audio_download_max_bytes: Option<u64>,
    /// Default `audio convert` format.
    #[serde(default)]
    pub audio_default_format: Option<String>,
    /// Default bitrate for lossy audio encode.
    #[serde(default)]
    pub audio_default_bitrate: Option<String>,
    /// Promoted operation-policy knobs (GAP-048), flattened into the same table.
    #[serde(default, flatten)]
    pub policy: super::policy::PolicyConfig,
}
