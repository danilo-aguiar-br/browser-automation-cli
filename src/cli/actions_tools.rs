// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tools / config / MITM / completion clap action enums.

use clap::{Subcommand, ValueEnum, ValueHint};

/// Unpacked Chrome extension operations for this one-shot process.
#[derive(Debug, Clone, Subcommand)]
pub enum ExtensionAction {
    /// List extensions loaded in this one-shot process
    List,
    /// Launch Chrome with an unpacked extension directory loaded
    Install {
        /// Unpacked extension directory (the one holding `manifest.json`)
        path: std::path::PathBuf,
    },
    /// Reload a loaded extension by id
    Reload {
        /// Extension id reported by `extension list`
        id: String,
        /// Unpacked extension dir so one-shot can --load-extension before reload
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
    },
    /// Trigger an extension action via its service worker
    Trigger {
        /// Extension id reported by `extension list`
        id: String,
        /// Unpacked extension dir so one-shot can --load-extension before trigger
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
    },
    /// Uninstall an extension by id
    Uninstall {
        /// Extension id reported by `extension list`
        id: String,
    },
}

/// Third-party developer tools a page exposes to DevTools.
#[derive(Debug, Clone, Subcommand)]
pub enum Devtools3pAction {
    /// List third-party developer tools exposed by the page
    List {
        /// Optional page URL to open before discovery
        #[arg(long)]
        url: Option<String>,
    },
    /// Execute one third-party developer tool by name
    Exec {
        /// Tool name from `devtools3p list`
        name: String,
        /// JSON object passed to the tool
        #[arg(long)]
        params: Option<String>,
        /// Page URL to open before executing
        #[arg(long)]
        url: Option<String>,
    },
}

/// WebMCP tools a page advertises to agents.
#[derive(Debug, Clone, Subcommand)]
pub enum WebmcpAction {
    /// List WebMCP tools exposed by the page
    List {
        /// Page URL to open before discovery
        #[arg(long)]
        url: Option<String>,
    },
    /// Execute one WebMCP tool by name
    Exec {
        /// Tool name from `webmcp list`
        name: String,
        /// JSON input passed to the tool
        #[arg(long)]
        input: Option<String>,
        /// Page URL to open before executing
        #[arg(long)]
        url: Option<String>,
    },
}

/// Shell dialects `completions` can emit.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CompletionShell {
    /// GNU Bash.
    Bash,
    /// Z shell.
    Zsh,
    /// fish shell.
    Fish,
    /// Elvish shell.
    Elvish,
    /// Windows PowerShell.
    Powershell,
}
/// One-shot QR encode and decode (no Chrome).
#[derive(Debug, Clone, Subcommand)]
pub enum QrAction {
    /// Encode text to PNG, SVG, or terminal matrix
    Encode {
        /// Payload encoded into the QR code
        #[arg(long)]
        text: String,
        /// png | svg | terminal
        #[arg(long, default_value = "png")]
        format: String,
        /// Output file; omit to write the terminal matrix to stdout
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
    },
    /// Decode QR payload from an image file
    Decode {
        /// Image file containing the QR code
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
    },
}
/// Loopback MITM proxy capture, export, and policy.
#[derive(Debug, Clone, Subcommand)]
pub enum MitmAction {
    /// CA paths, capture count, bind policy
    Status {
        /// Read a capture written by another invocation (GAP-009 explicit path)
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        capture_path: Option<String>,
    },
    /// List captured exchanges
    List {
        /// Only list exchanges with this host
        #[arg(long)]
        host: Option<String>,
        /// Maximum exchanges to return
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// Read a capture written by another invocation (GAP-009 explicit path)
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        capture_path: Option<String>,
    },
    /// Get one exchange by id
    Get {
        /// Exchange id from `mitm list`
        id: u64,
        /// Read a capture written by another invocation (GAP-009 explicit path)
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        capture_path: Option<String>,
    },
    /// Export HAR 1.2 JSON
    Har {
        /// Destination `.har` file
        #[arg(long, value_hint = ValueHint::FilePath)]
        out: std::path::PathBuf,
        /// Read a capture written by another invocation (GAP-009 explicit path)
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        capture_path: Option<String>,
    },
    /// Export capture as JSON/NDJSON
    Export {
        /// Output encoding: json | ndjson
        #[arg(long, default_value = "json")]
        format: String,
        /// Destination file
        #[arg(long, value_hint = ValueHint::FilePath)]
        out: std::path::PathBuf,
        /// Read a capture written by another invocation (GAP-009 explicit path)
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        capture_path: Option<String>,
    },
    /// Unique hosts seen
    Domains {
        /// Read a capture written by another invocation (GAP-009 explicit path)
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        capture_path: Option<String>,
    },
    /// REST/GraphQL endpoint discovery
    Apis {
        /// Only report endpoints of this kind (for example `rest`, `graphql`)
        #[arg(long)]
        kind: Option<String>,
        /// Read a capture written by another invocation (GAP-009 explicit path)
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        capture_path: Option<String>,
    },
    /// Ensure local CA under XDG data
    InitCa,
    /// Start one-shot MITM proxy on 127.0.0.1 (ephemeral port); captures until timeout
    Start {
        /// Seconds to keep the proxy alive (one-shot; default 30)
        #[arg(long, default_value_t = 30)]
        seconds: u64,
    },
    /// One-shot: proxy + Chrome + navigate URL + capture (GAP-011 / GAP-019)
    CaptureUrl {
        /// Target URL to open through the MITM proxy
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        /// Max seconds for the whole one-shot (default 30)
        #[arg(long, default_value_t = 30)]
        seconds: u64,
        /// Optional HAR output path
        #[arg(long, value_hint = ValueHint::FilePath)]
        har: Option<std::path::PathBuf>,
        /// Optional host allowlist for TLS intercept
        #[arg(long)]
        hosts: Option<String>,
    },
    /// GraphQL operations discovered in capture
    Graphql {
        /// Maximum operations to return
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// Read a capture written by another invocation (GAP-009 explicit path)
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        capture_path: Option<String>,
    },
    /// WebSocket frames from capture
    Ws {
        /// WebSocket frame operation to run
        #[command(subcommand)]
        action: MitmWsAction,
    },
    /// Short-circuit block host/path (persists for next start/capture-url in same process config note)
    Block {
        /// Host to short-circuit
        #[arg(long)]
        host: Option<String>,
        /// Restrict the block to this path prefix
        #[arg(long)]
        path: Option<String>,
    },
    /// Allowlist host for TLS intercept
    Allow {
        /// Host added to the TLS intercept allowlist
        #[arg(long)]
        host: String,
    },
    /// Show or set redact-secrets policy for exports
    Redact {
        /// When true, redact Authorization/Cookie (default true)
        #[arg(long, default_value_t = true)]
        secrets: bool,
    },
}

/// WebSocket frame views over a MITM capture.
#[derive(Debug, Clone, Subcommand)]
pub enum MitmWsAction {
    /// List captured WebSocket frames
    List {
        /// Maximum frames to return
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// Read a capture written by another invocation (GAP-009 explicit path)
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        capture_path: Option<String>,
    },
    /// Get one frame by id
    Get {
        /// Frame id from `mitm ws list`
        id: u64,
        /// Read a capture written by another invocation (GAP-009 explicit path)
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        capture_path: Option<String>,
    },
}

/// Workflow DAG execution backed by a SQLite journal under XDG state.
#[derive(Debug, Clone, Subcommand)]
pub enum WorkflowAction {
    /// Validate DAG and execute offline steps; journal under XDG state
    Run {
        /// Workflow manifest JSON describing the DAG
        #[arg(long, value_hint = ValueHint::FilePath)]
        manifest: std::path::PathBuf,
        /// Journal database path; defaults to XDG state
        #[arg(long, value_hint = ValueHint::FilePath)]
        journal: Option<std::path::PathBuf>,
    },
    /// Resume / re-run from journal + manifest
    Resume {
        /// Workflow manifest JSON describing the DAG
        #[arg(long, value_hint = ValueHint::FilePath)]
        manifest: std::path::PathBuf,
        /// Journal database path; defaults to XDG state
        #[arg(long, value_hint = ValueHint::FilePath)]
        journal: Option<std::path::PathBuf>,
    },
    /// Show journal step statuses
    Status {
        /// Journal database path; defaults to XDG state
        #[arg(long, value_hint = ValueHint::FilePath)]
        journal: Option<std::path::PathBuf>,
        /// Workflow name to report on; omit for all
        #[arg(long)]
        name: Option<String>,
    },
}

/// XDG configuration management (the only runtime config mechanism).
#[derive(Debug, Clone, Subcommand)]
pub enum ConfigAction {
    /// Print resolved XDG paths
    Path,
    /// Create XDG layout + default config.toml
    Init,
    /// Show config values
    Show,
    /// Set a config key (lang|timeout|artifacts_dir|ignore_robots|namespace|encryption_key|color|log_level|log_to_file|chrome_path|lighthouse_path|ffmpeg_path|lighthouse_timeout_secs|ffmpeg_timeout_secs|openrouter_api_key|llm_base_url|llm_model|cache_backend|cache_redis_url|search_base_url)
    Set {
        /// Config key from `config list-keys`
        key: String,
        /// New value; validated per key before it is persisted
        value: String,
    },
    /// Get one config key
    Get {
        /// Config key to read; omit to dump every key
        key: Option<String>,
    },
    /// List supported config keys and defaults (GAP-018)
    ListKeys,
}
