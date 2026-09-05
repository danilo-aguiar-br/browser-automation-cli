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

/// One-shot local image ops (no Chrome): info, convert, resize, download, exif.
#[derive(Debug, Clone, Subcommand)]
pub enum ImageAction {
    /// Probe format, dimensions, sha256, and optional EXIF (no pixel dump)
    Info {
        /// Image file path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Read image bytes from stdin
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one image path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Emits one envelope holding a per-item result. A failing item is
        /// reported and the run continues, so `ok: true` means the batch
        /// produced SOMETHING, never that every item passed — read
        /// `error_count`. If NO item succeeds the run fails with exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Include GPS EXIF tags (default strips location)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        include_gps: bool,
        /// Project envelope fields (CSV): format,width,height,bytes,path,sha256,exif,iptc,xmp,magic_ok,has_alpha,frame_count,frame_count_exact,animated,engine,action
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
    },
    /// Convert format (re-encode strips EXIF by default)
    Convert {
        /// Input image path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Read image bytes from stdin
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one image path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Each output is derived beside its input as `input.<target format>`;
        /// `--out` is refused here because one destination cannot serve N
        /// inputs. An item whose derived path already exists, or equals its own
        /// input, fails and the batch continues, so `ok: true` means the batch
        /// produced SOMETHING, never that every item passed — read
        /// `error_count`. If NO item succeeds the run fails with exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Output format: png | jpeg | webp | gif | avif (avif needs the `image-avif` feature; heic is rejected — no pure-Rust HEVC encoder)
        #[arg(long)]
        format: String,
        /// Lossy quality 1..=100 (applied for jpeg and avif; local webp encoder is lossless — quality recorded but not applied)
        #[arg(long)]
        quality: Option<u8>,
        /// Output path (default under XDG cache)
        #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
        out: Option<std::path::PathBuf>,
        /// Request EXIF strip (default true). Pixel re-encode always drops EXIF in this build.
        #[arg(long, default_value_t = true, action = clap::ArgAction::SetTrue)]
        strip_exif: bool,
        /// Request keep EXIF (intent only). This build re-encodes pixels and cannot re-attach EXIF; envelope sets keep_exif_honored=false.
        #[arg(long, action = clap::ArgAction::SetTrue)]
        keep_exif: bool,
    },
    /// Resize pixels (not the CDP viewport `resize` command)
    Resize {
        /// Input image path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Read image bytes from stdin
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one image path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Each output is derived beside its input as `input.<target format>`;
        /// `--out` is refused here because one destination cannot serve N
        /// inputs. An item whose derived path already exists, or equals its own
        /// input, fails and the batch continues, so `ok: true` means the batch
        /// produced SOMETHING, never that every item passed — read
        /// `error_count`. If NO item succeeds the run fails with exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Target width in pixels
        #[arg(long)]
        width: u32,
        /// Target height in pixels (optional with `--keep-aspect`)
        #[arg(long)]
        height: Option<u32>,
        /// Preserve aspect ratio when computing the missing dimension
        #[arg(long, action = clap::ArgAction::SetTrue)]
        keep_aspect: bool,
        /// Output path
        #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
        out: Option<std::path::PathBuf>,
        /// Output format (default: XDG image_default_format). Same token set as `image convert`.
        #[arg(long)]
        format: Option<String>,
        /// Lossy quality 1..=100 (jpeg and avif only for local encode; webp is lossless)
        #[arg(long)]
        quality: Option<u8>,
    },
    /// Download an image URL (SSRF + body cap + magic verify)
    Download {
        /// http(s) URL of the image
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        /// Output path (default under XDG cache)
        #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
        out: Option<std::path::PathBuf>,
        /// Max body bytes (default: XDG image_download_max_bytes)
        #[arg(long)]
        max_bytes: Option<usize>,
        /// Fail closed unless body is a supported image magic (default; use `--allow-non-image` to disable)
        #[arg(long, default_value_t = true, action = clap::ArgAction::SetTrue)]
        require_image: bool,
        /// Allow non-image bodies (disables magic fail-closed)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        allow_non_image: bool,
    },
    /// Read EXIF tags (GPS omitted unless `--include-gps`)
    Exif {
        /// Image file path (omit with `--stdin`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Read image bytes from stdin
        #[arg(long, action = clap::ArgAction::SetTrue)]
        stdin: bool,
        /// Batch: one image path per line (`#` comments skipped). Mutually exclusive with `--path` and `--stdin`
        ///
        /// Emits one envelope holding a per-item result. A failing item is
        /// reported and the run continues, so `ok: true` means the batch
        /// produced SOMETHING, never that every item passed — read
        /// `error_count`. If NO item succeeds the run fails with exit 65.
        #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
        paths_file: Option<std::path::PathBuf>,
        /// Include GPS EXIF tags
        #[arg(long, action = clap::ArgAction::SetTrue)]
        include_gps: bool,
        /// Project fields (CSV): path,count,exif,include_gps,engine (aliases: tags=exif, tag_count=count). EXIF only — no IPTC/XMP.
        #[arg(long, value_name = "FIELDS")]
        select: Option<String>,
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
        #[arg(long, default_value_t = crate::constants::MITM_LIST_LIMIT)]
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
        #[arg(long, default_value_t = crate::constants::MITM_DEFAULT_SECONDS)]
        seconds: u64,
    },
    /// One-shot: proxy + Chrome + navigate URL + capture (GAP-011 / GAP-019)
    CaptureUrl {
        /// Target URL to open through the MITM proxy
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        /// Max seconds for the whole one-shot (default 30)
        #[arg(long, default_value_t = crate::constants::MITM_DEFAULT_SECONDS)]
        seconds: u64,
        /// Optional HAR output path
        #[arg(long, value_hint = ValueHint::FilePath)]
        har: Option<std::path::PathBuf>,
        /// Host allowlist for TLS DECRYPTION (CSV). Decides what is intercepted, not what is kept
        #[arg(long)]
        hosts: Option<String>,
        /// Host allowlist for the CAPTURE RECORD (CSV). Exchanges off this list are forwarded but never written
        ///
        /// Distinct from `--hosts`: that one decides which connections are
        /// decrypted, from the SNI; this one decides which exchanges reach the
        /// artifact, from the request host. Plaintext HTTP and the CDP merge
        /// never pass through the decryption decision, so `--hosts` alone still
        /// leaves the browser's own background chatter in the capture. Matches a
        /// host exactly or as a subdomain: `example.com` admits
        /// `api.example.com` and refuses `evil-example.com`.
        #[arg(long, value_name = "CSV")]
        capture_hosts: Option<String>,
    },
    /// GraphQL operations discovered in capture
    Graphql {
        /// Maximum operations to return
        #[arg(long, default_value_t = crate::constants::MITM_GRAPHQL_LIMIT)]
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
    ///
    /// At least one of `--host` or `--path` is required: a rule with neither
    /// names no target, and a block rule that matches everything is not a
    /// narrower version of a block rule — it is a different, far larger act.
    Block {
        /// Host to short-circuit
        #[arg(long, required_unless_present = "path")]
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
    /// Show or set the redact-secrets policy applied to captures
    Redact {
        /// Set the policy: `--secrets false` stops masking, `--secrets true` restores it. Omit to SHOW the effective policy without writing anything
        ///
        /// Was `#[arg(long, default_value_t = true)]` over a bare `bool`, which
        /// clap derives as `SetTrue`. Measured 2026-09-01: omitting the flag
        /// gave `true`, passing `--secrets` gave `true`, and `--secrets false`
        /// exited 2 with `unexpected argument 'false'`. The one value an
        /// operator would ever type was the one the parser refused, while the
        /// help text said "when true" as though a "when false" existed.
        #[arg(long)]
        secrets: Option<bool>,
    },
}

/// WebSocket frame views over a MITM capture.
#[derive(Debug, Clone, Subcommand)]
pub enum MitmWsAction {
    /// List captured WebSocket frames
    List {
        /// Maximum frames to return
        #[arg(long, default_value_t = crate::constants::MITM_WS_FRAMES_LIMIT)]
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
    /// Set a config key from `config list-keys` (includes image_*, video_*, audio_*, dialog_settle_ms, …)
    Set {
        /// Config key from `config list-keys`
        key: String,
        /// New value; validated per key before it is persisted
        value: String,
    },
    /// Restore one config key to its built-in default
    ///
    /// The inverse of `set`. `config set <key> ""` is not an inverse: for a
    /// string key it stores an empty value the normal path never produces, and
    /// for a numeric key it is a parse error. Unsetting a key that is already
    /// absent succeeds, so a script never has to know the prior state.
    Unset {
        /// Config key to restore; must appear in `config list-keys`
        key: String,
    },
    /// Get one config key
    Get {
        /// Config key to read; omit to dump every key
        key: Option<String>,
    },
    /// List supported config keys and defaults (GAP-018)
    ListKeys,
}
