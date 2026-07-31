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
    /// LLM/webhook blocking HTTP client timeout seconds.
    #[serde(default)]
    pub llm_http_timeout_secs: Option<u64>,
    /// When true, allow non-loopback Redis hosts (default false).
    #[serde(default)]
    pub redis_allow_remote: Option<bool>,
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
    /// Promoted operation-policy knobs (GAP-048), flattened into the same table.
    #[serde(default, flatten)]
    pub policy: super::policy::PolicyConfig,
}
