// SPDX-License-Identifier: MIT OR Apache-2.0
//! Clap derive surface for browser-automation-cli (PRD Layer L).
//!
//! Help text on flags is the primary documentation for this module.
//! Item-level rustdoc is intentionally light: clap `///` strings power `--help`
//! and man pages; agent skills cover recipes (audit D-02/D-11).
#![allow(missing_docs)]

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum, ValueHint};

/// One-shot browser automation CLI for AI agents.
#[derive(Debug, Parser)]
#[command(
    name = "browser-automation-cli",
    version,
    author,
    about = "One-shot browser automation CLI (Chrome CDP). BORN, EXECUTE, FINALIZE, DIE.",
    long_about = None,
    propagate_version = true,
    after_help = "Examples:\n  \
browser-automation-cli doctor --json\n  \
browser-automation-cli goto https://example.com --json\n  \
browser-automation-cli schema run\n  \
browser-automation-cli run --script steps.ndjson --json-steps\n  \
browser-automation-cli config path\n\n\
Exit codes follow sysexits-style mapping (2 usage, 69 unavailable, 70 software, 124 timeout).\n\
Config is XDG-only (config set); product settings do not read process environment variables."
)]
pub struct Cli {
    /// Global flags shared by all subcommands
    #[command(flatten)]
    pub globals: GlobalOpts,

    /// Subcommand to execute (one-shot)
    #[command(subcommand)]
    pub command: Commands,
}

/// Global options applied to every subcommand.
///
/// Flattened into the root [`Cli`] via `#[command(flatten)]`.
#[derive(Debug, Clone, Args)]
pub struct GlobalOpts {
    /// Emit machine-readable JSON success/error envelopes on stdout
    #[arg(long, global = true, action = ArgAction::SetTrue, help_heading = "Output")]
    pub json: bool,

    /// GAP-020: stream one NDJSON object per `run` step on stdout (step,cmd,ok,result)
    #[arg(
        long = "json-steps",
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Output"
    )]
    pub json_steps: bool,

    /// Suppress non-error human logs on stderr
    #[arg(
        short = 'q',
        long = "quiet",
        global = true,
        action = ArgAction::SetTrue,
        conflicts_with_all = ["verbose", "debug"],
        help_heading = "Output"
    )]
    pub quiet: bool,

    /// Increase stderr verbosity (`-v` / `--verbose` = info; or `config set log_level debug`)
    #[arg(
        short = 'v',
        long = "verbose",
        global = true,
        action = ArgAction::SetTrue,
        conflicts_with = "quiet",
        help_heading = "Output"
    )]
    pub verbose: bool,

    /// Maximum tracing detail on stderr (debug/trace)
    #[arg(
        long = "debug",
        global = true,
        action = ArgAction::SetTrue,
        conflicts_with = "quiet",
        help_heading = "Output"
    )]
    pub debug: bool,

    /// Force plain stderr (no ANSI colors; accessibility / agent-friendly)
    #[arg(
        long = "plain",
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Output"
    )]
    pub plain: bool,

    /// Global wall-clock timeout in seconds (0 = no override)
    #[arg(
        long,
        global = true,
        default_value_t = 0,
        value_name = "SECS",
        help_heading = "Timeouts"
    )]
    pub timeout: u64,

    /// Max concurrent I/O tasks (batch/crawl/CDP fan-out) and Rayon CPU pool hint
    ///
    /// `0` = auto: `min(cpus, (free_ram_mb×50%)/64, 64)`. Every fan-out is
    /// hard-capped (no unbounded `join_all` / spawn loops).
    #[arg(
        long = "max-concurrency",
        global = true,
        default_value_t = 0,
        value_name = "N",
        help_heading = "Parallelism"
    )]
    pub max_concurrency: usize,

    /// Per-step timeout in seconds for `run` scripts (0 = inherit global timeout)
    #[arg(
        long,
        global = true,
        default_value_t = 0,
        value_name = "SECS",
        help_heading = "Timeouts"
    )]
    pub step_timeout: u64,

    /// Launch Chrome with a visible window (debug; default headless=new)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Browser"
    )]
    pub headed: bool,

    /// Directory for screenshots, PDFs, and other one-shot artifacts
    #[arg(
        long,
        global = true,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help_heading = "Browser"
    )]
    pub artifacts_dir: Option<std::path::PathBuf>,

    /// Force UI language (`en` / `pt-BR`); default: flag → env → XDG → OS → en
    ///
    /// Accepts BCP 47 (`pt-BR`, `en`) or legacy `pt`. Env override:
    /// `BROWSER_AUTOMATION_CLI_LANG`. Machine JSON stays English.
    #[arg(long, global = true, value_name = "LANG", help_heading = "Output")]
    pub lang: Option<String>,

    /// Capture console messages during browser commands
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Browser"
    )]
    pub capture_console: bool,

    /// Capture network requests during browser commands
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Browser"
    )]
    pub capture_network: bool,

    /// Skip robots.txt policy checks (requires risk acceptance for blocked hosts)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Robots"
    )]
    pub ignore_robots: bool,

    /// Explicitly accept robots.txt override risk when using --ignore-robots
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Robots"
    )]
    pub i_accept_robots_risk: bool,

    /// Enable deep heap analysis tools (PRD category-memory)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Categories"
    )]
    pub category_memory: bool,

    /// Enable extension management tools
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Categories"
    )]
    pub category_extensions: bool,

    /// Enable third-party developer tool surface
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Categories"
    )]
    pub category_third_party: bool,

    /// Enable WebMCP-compatible tool surface
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Categories"
    )]
    pub category_webmcp: bool,

    /// Enable experimental screencast (may require ffmpeg for file export)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Categories"
    )]
    pub experimental_screencast: bool,

    /// Enable coordinate click-at (vision) tools
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "Categories"
    )]
    pub experimental_vision: bool,

    /// Enable one-shot local MITM proxy and route Chrome through it (PRD §5E / GAP-019)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm: bool,

    /// Directory for MITM CA key+cert PEM (default: XDG data)
    #[arg(
        long,
        global = true,
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        help_heading = "MITM"
    )]
    pub mitm_ca_dir: Option<std::path::PathBuf>,

    /// Write HAR 1.2 to this path on FINALIZE when --mitm is active
    #[arg(
        long,
        global = true,
        value_name = "FILE",
        value_hint = ValueHint::FilePath,
        help_heading = "MITM"
    )]
    pub mitm_har: Option<std::path::PathBuf>,

    /// Comma-separated hosts to decrypt (empty = all via proxy)
    #[arg(long, global = true, value_name = "HOSTS", help_heading = "MITM")]
    pub mitm_hosts: Option<String>,

    /// Capture WebSocket frames in MITM handler
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm_ws: bool,

    /// Max body bytes retained per exchange
    #[arg(long, global = true, value_name = "BYTES", help_heading = "MITM")]
    pub mitm_max_body_bytes: Option<usize>,

    /// Drop image/video/audio bodies from MITM capture
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm_no_media_bodies: bool,

    /// Redact Authorization/Cookie secrets in MITM exports (default on when set)
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help_heading = "MITM"
    )]
    pub mitm_redact_secrets: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Diagnose Chrome install and one-shot readiness
    ///
    /// Use global `--json` for machine-readable envelopes (no local `--json` —
    /// avoids silent shadowing of the global flag).
    Doctor {
        #[arg(long, action = ArgAction::SetTrue)]
        offline: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        quick: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        fix: bool,
    },
    /// List available commands
    ///
    /// Use global `--json` for the machine inventory payload.
    Commands,
    /// JSON Schema fragment for a command (agent discovery)
    /// GAP-022: accepts `schema run` or `schema --cmd run`.
    Schema {
        /// Command name via --cmd
        #[arg(long = "cmd", value_name = "CMD")]
        cmd: Option<String>,
        /// Command name as positional (preferred agent UX)
        #[arg(value_name = "CMD")]
        cmd_positional: Option<String>,
    },
    /// Print CLI version
    Version,
    /// Show resolved UI locale and detection diagnostics (human suggestions only)
    Locale,
    /// Navigate to a URL (one-shot)
    Goto {
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        /// JS to evaluate before navigation (tool-ref initScript)
        #[arg(long)]
        init_script: Option<String>,
        /// Auto-handle beforeunload: accept | dismiss (GAP-003; flag alone = accept)
        #[arg(long, value_enum, num_args = 0..=1, default_missing_value = "accept")]
        handle_before_unload: Option<BeforeUnloadAction>,
        /// Navigation timeout override in milliseconds
        #[arg(long)]
        navigation_timeout_ms: Option<u64>,
    },
    /// Accessibility snapshot with @eN refs
    View {
        /// Full a11y tree (tool-ref take_snapshot.verbose / run JSON `"verbose":true`).
        ///
        /// CLI long name is `--detailed` so it does not silently shadow global
        /// `--verbose` (product logging). Multi-step `run` scripts still use the
        /// JSON key `verbose` for DevTools tool-ref parity.
        #[arg(long = "detailed", action = ArgAction::SetTrue)]
        verbose: bool,
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Allow empty about:blank snapshots (GAP-012)
        #[arg(long, action = ArgAction::SetTrue)]
        allow_empty: bool,
    },
    /// Click an element (selector or @eN)
    Press {
        target: String,
        #[arg(long, action = ArgAction::SetTrue)]
        dblclick: bool,
        /// Attach slim a11y snapshot in the same process after the action
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Click at page CSS coordinates (requires --experimental-vision)
    ClickAt {
        #[arg(long)]
        x: f64,
        #[arg(long)]
        y: f64,
        #[arg(long, action = ArgAction::SetTrue)]
        dblclick: bool,
        /// Attach slim a11y snapshot after the click
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Fill an input value (select/checkbox/radio/text smart fill)
    Write {
        target: String,
        value: String,
        /// Attach slim a11y snapshot after fill
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Press a keyboard key
    Keys {
        key: String,
        /// Attach slim a11y snapshot after the key press
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Type text (tool-ref type_text). Use --target or --focus-only.
    Type {
        /// Text to type (required positional)
        text: String,
        /// CSS selector or @eN (optional; use --focus-only for focused element)
        #[arg(long)]
        target: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        clear: bool,
        /// Optional key to press after typing (e.g. Enter)
        #[arg(long)]
        submit: Option<String>,
        /// Type into currently focused element without resolving a target
        #[arg(long, action = ArgAction::SetTrue)]
        focus_only: bool,
    },
    /// Wait for ms and/or text and/or selector and/or load state
    Wait {
        #[arg(long, default_value_t = 0)]
        ms: u64,
        /// Text to wait for (repeatable; resolves when any value appears — tool-ref OR)
        #[arg(long = "text", action = clap::ArgAction::Append)]
        text: Vec<String>,
        #[arg(long)]
        selector: Option<String>,
        /// Page lifecycle: load | domcontentloaded | networkidle | none
        #[arg(long)]
        state: Option<String>,
        /// Max wait time in milliseconds for text/selector/state (0 = default)
        #[arg(long)]
        wait_timeout_ms: Option<u64>,
        /// Attach slim a11y snapshot after the wait succeeds
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Hover an element
    Hover {
        target: String,
        /// Attach slim a11y snapshot after hover
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Drag from one target to another
    Drag {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        /// Attach slim a11y snapshot after drag
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Fill multiple form fields from JSON `[{target|uid,value},...]`
    FillForm {
        /// JSON array payload (not the global envelope flag `--json`)
        #[arg(long = "fields-json", value_name = "JSON")]
        fields_json: String,
        /// Attach slim a11y snapshot after fill-form
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Upload a file to a file input
    Upload {
        target: String,
        #[arg(value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Attach slim a11y snapshot after upload
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// History back
    Back,
    /// History forward
    Forward,
    /// Reload current page
    Reload {
        #[arg(long, action = ArgAction::SetTrue)]
        ignore_cache: bool,
        /// JS to run before navigation/reload (tool-ref initScript)
        #[arg(long)]
        init_script: Option<String>,
        /// Auto-handle beforeunload: accept | dismiss (GAP-003; flag alone = accept)
        #[arg(long, value_enum, num_args = 0..=1, default_missing_value = "accept")]
        handle_before_unload: Option<BeforeUnloadAction>,
    },
    /// Evaluate JavaScript (expression or function declaration)
    Eval {
        /// JS expression or function declaration `() => ...`
        expression: String,
        /// Snapshot uids (@eN) passed as function args (JSON array of strings)
        #[arg(long)]
        args: Option<String>,
        /// accept | dismiss | prompt response text (default accept)
        #[arg(long)]
        dialog_action: Option<String>,
        /// Write evaluate result JSON to this path
        #[arg(long, value_hint = ValueHint::FilePath)]
        file_path: Option<std::path::PathBuf>,
        /// Evaluate inside an extension service worker target (tool-ref serviceWorkerId)
        #[arg(long)]
        service_worker_id: Option<String>,
    },
    /// Capture a screenshot
    Grab {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        #[arg(long, value_enum, default_value_t = GrabFormat::Png)]
        format: GrabFormat,
        #[arg(long, action = ArgAction::SetTrue)]
        full_page: bool,
        #[arg(long)]
        quality: Option<i32>,
        /// CSS selector or @eN element to capture
        #[arg(long)]
        element: Option<String>,
    },
    /// Print current page to PDF via CDP Page.printToPDF (one-shot)
    PrintPdf {
        /// Output path for the PDF artifact
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Optional URL to navigate before printing (one-shot)
        #[arg(long)]
        url: Option<String>,
    },
    /// One-shot change check against a baseline file (hash/text)
    Monitor {
        #[command(subcommand)]
        action: MonitorAction,
    },
    /// Run multi-step NDJSON script in one process
    Run {
        #[arg(long, value_hint = ValueHint::FilePath)]
        script: std::path::PathBuf,
    },
    /// Single-step inline command (same surface as `run` steps: goto, wait, view, press, …)
    Exec {
        // Do NOT set allow_hyphen_values: global flags like --json after `exec`
        // must stay on GlobalOpts, not be swallowed into trailing args.
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Extract text/attribute from a target, or LLM extract with --llm
    Extract {
        /// Selector, @eN ref, about:blank target, or http(s) URL for LLM/text path
        target: String,
        #[arg(long)]
        attr: Option<String>,
        /// Opt-in LLM HTTP extract (requires XDG openrouter_api_key)
        #[arg(long, action = ArgAction::SetTrue)]
        llm: bool,
        /// Question for LLM extract
        #[arg(long)]
        question: Option<String>,
        /// Path to JSON schema file for structured LLM extract
        #[arg(long, value_hint = ValueHint::FilePath)]
        schema_json: Option<std::path::PathBuf>,
    },
    /// Extract visible text from a target (PRD §7 `text`)
    Text { target: String },
    /// Scroll page or element by delta pixels (PRD §7 `scroll`)
    Scroll {
        /// CSS selector or @eN (optional; omit for window scroll)
        #[arg(long)]
        target: Option<String>,
        #[arg(long, default_value_t = 0.0)]
        delta_x: f64,
        #[arg(long, default_value_t = 0.0)]
        delta_y: f64,
    },
    /// Cookie jar helpers for the active page (Network domain)
    Cookie {
        #[command(subcommand)]
        action: CookieAction,
    },
    /// Read one attribute from a target
    Attr { target: String, name: String },
    /// Assertions (url / text / console)
    Assert {
        #[command(subcommand)]
        kind: AssertKind,
    },
    /// Captured console messages (--capture-console)
    Console {
        #[command(subcommand)]
        action: ConsoleAction,
    },
    /// Captured network requests (--capture-network)
    Net {
        #[command(subcommand)]
        action: NetAction,
    },
    /// Page info or multi-tab management
    Page {
        #[command(subcommand)]
        action: Option<PageAction>,
    },
    /// Accept or dismiss dialogs
    Dialog {
        #[command(subcommand)]
        action: DialogAction,
    },
    /// Navigate and return body text / formats (local HTTP or CDP scrape)
    Scrape {
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        /// text | markdown | html | links | metadata | … (CSV or repeatable; alias --formats)
        #[arg(long = "format", alias = "formats", value_delimiter = ',', num_args = 1.., default_value = "text")]
        format: Vec<String>,
        /// http (reqwest+scraper) or browser (CDP)
        #[arg(long, default_value = "browser")]
        engine: String,
        /// Prefer main/article content heuristics
        #[arg(long, action = ArgAction::SetTrue)]
        only_main_content: bool,
        /// Optional one-shot webhook POST of the result envelope data (127.0.0.1/operator URL)
        #[arg(long)]
        webhook_url: Option<String>,
    },
    /// Scrape many URLs from a file (HTTP or browser engine, one-shot)
    BatchScrape {
        #[arg(long, value_hint = ValueHint::FilePath)]
        urls_file: std::path::PathBuf,
        #[arg(long = "format", alias = "formats", default_value = "text")]
        format: String,
        /// Concurrent HTTP fetches (`0` = use global `--max-concurrency` / auto)
        #[arg(long, default_value_t = 0)]
        concurrency: usize,
        /// http (default) or browser (CDP per URL; GAP-010)
        #[arg(long, default_value = "http")]
        engine: String,
    },
    /// Crawl from a seed URL (HTTP BFS or browser, one-shot)
    Crawl {
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        #[arg(long, alias = "max-pages", default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = 2)]
        max_depth: usize,
        #[arg(long = "format", alias = "formats", default_value = "text")]
        format: String,
        /// Stay on seed host
        #[arg(long, default_value_t = true)]
        same_host: bool,
        /// http (default) or browser (GAP-010)
        #[arg(long, default_value = "http")]
        engine: String,
    },
    /// Map site URLs from a seed (HTTP)
    Map {
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long, default_value_t = 2)]
        max_depth: usize,
    },
    /// Local search (HTTP SERP links or URL map)
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Parse a local file (html/md/txt/pdf/docx/xlsx text extract)
    Parse {
        #[arg(value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Mask email/phone/card-like patterns in text output
        #[arg(long, action = ArgAction::SetTrue)]
        redact_pii: bool,
    },
    /// QR encode/decode one-shot (no Chrome)
    Qr {
        #[command(subcommand)]
        action: QrAction,
    },
    /// Discover filesystem paths (fd-like UX; binary remains browser-automation-cli)
    FindPaths {
        /// Regex pattern on name/path (optional)
        pattern: Option<String>,
        /// Root paths to search
        #[arg(num_args = 0..)]
        paths: Vec<String>,
        /// Filter by extension (e.g. rs, html)
        #[arg(long)]
        extension: Option<String>,
        /// Include hidden files
        #[arg(long, action = ArgAction::SetTrue)]
        hidden: bool,
        /// Do not respect .gitignore
        #[arg(long, action = ArgAction::SetTrue)]
        no_ignore: bool,
        /// Max directory depth
        #[arg(long)]
        max_depth: Option<usize>,
        /// Entry type: f|d
        #[arg(long = "type")]
        entry_type: Option<String>,
        /// Max results
        #[arg(long, default_value_t = 10000)]
        limit: usize,
        /// Shell-style glob filter (e.g. `**/*.rs`) — GAP-A011 / §5AE
        #[arg(long)]
        glob: Option<String>,
    },
    /// Structural lint scan for forbidden product patterns (one-shot; §5AC / GAP-A011)
    SgScan {
        /// Roots to scan (default: `.`)
        #[arg(num_args = 0..)]
        paths: Vec<String>,
        /// Max findings (0 = unlimited)
        #[arg(long, default_value_t = 500)]
        limit: usize,
    },
    /// Structural rewrite for known-safe fixes (dry-run default; `--apply` writes)
    SgRewrite {
        /// Roots to rewrite (default: `.`)
        #[arg(num_args = 0..)]
        paths: Vec<String>,
        /// Apply changes (default is dry-run report only)
        #[arg(long, action = ArgAction::SetTrue)]
        apply: bool,
    },
    /// Write a simple XLSX workbook from CSV/JSON (one-shot; §5Z / GAP-A011)
    SheetWrite {
        /// Input path (.csv or .json array-of-objects)
        #[arg(value_hint = ValueHint::FilePath)]
        input: std::path::PathBuf,
        /// Output .xlsx path
        #[arg(long, short = 'o', value_hint = ValueHint::FilePath)]
        out: std::path::PathBuf,
        /// Worksheet name
        #[arg(long, default_value = "Sheet1")]
        sheet: String,
    },
    /// MITM capture / CA / HAR (one-shot local)
    Mitm {
        #[command(subcommand)]
        action: MitmAction,
    },
    /// Workflow journal DAG (petgraph + SQLite)
    Workflow {
        #[command(subcommand)]
        action: WorkflowAction,
    },
    /// XDG config and path management (no .env at runtime)
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Emulate device / network / UA / geo / CPU
    Emulate {
        #[arg(long)]
        user_agent: Option<String>,
        #[arg(long)]
        locale: Option<String>,
        #[arg(long)]
        timezone: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        offline: bool,
        #[arg(long)]
        latitude: Option<f64>,
        #[arg(long)]
        longitude: Option<f64>,
        #[arg(long)]
        media: Option<String>,
        /// Network preset: Offline, No throttling, Slow 3G, Fast 3G, Slow 4G, Fast 4G
        #[arg(long)]
        network_conditions: Option<String>,
        /// CPU slowdown factor 1..=20 (1 disables)
        #[arg(long)]
        cpu_throttling_rate: Option<f64>,
        /// prefers-color-scheme: dark | light | auto
        #[arg(long)]
        color_scheme: Option<String>,
        /// Extra HTTP headers as JSON object string
        #[arg(long)]
        extra_headers: Option<String>,
        /// Viewport `WxHxDPR` with optional `,mobile`, `,touch`, `,landscape` flags
        #[arg(long)]
        viewport: Option<String>,
    },
    /// Resize page viewport
    Resize {
        #[arg(long)]
        width: i32,
        #[arg(long)]
        height: i32,
        #[arg(long, default_value_t = 1.0)]
        scale: f64,
        #[arg(long, action = ArgAction::SetTrue)]
        mobile: bool,
    },
    /// Performance trace / metrics
    Perf {
        #[command(subcommand)]
        action: PerfAction,
    },
    /// Run Lighthouse audit (external binary)
    Lighthouse {
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        #[arg(long, value_hint = ValueHint::DirPath)]
        out_dir: Option<std::path::PathBuf>,
        #[arg(long, default_value = "desktop")]
        device: String,
        /// navigation (default) or snapshot (maps to navigation in one-shot CLI)
        #[arg(long, default_value = "navigation")]
        mode: String,
        #[arg(long, value_hint = ValueHint::FilePath)]
        lighthouse_path: Option<std::path::PathBuf>,
    },
    /// Screencast start/stop (experimental)
    Screencast {
        #[command(subcommand)]
        action: ScreencastAction,
    },
    /// Heap snapshot tools (requires --category-memory for deep analysis)
    Heap {
        #[command(subcommand)]
        action: HeapAction,
    },
    /// Chrome extension tools (requires --category-extensions)
    Extension {
        #[command(subcommand)]
        action: ExtensionAction,
    },
    /// Third-party developer tools surface (requires --category-third-party)
    Devtools3p {
        #[command(subcommand)]
        action: Devtools3pAction,
    },
    /// Web surface tools (requires --category-webmcp)
    Webmcp {
        #[command(subcommand)]
        action: WebmcpAction,
    },
    /// Generate shell completions (path-level, no Chrome)
    Completions {
        #[arg(value_enum)]
        shell: CompletionShell,
    },
    /// Generate man page (roff) via clap_mangen (path-level, no Chrome)
    Man {
        /// Write man page to PATH instead of stdout
        #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
        out: Option<std::path::PathBuf>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum PerfAction {
    Start {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        #[arg(long, action = ArgAction::SetTrue)]
        reload: bool,
        /// Auto-stop after page load/reload (tool-ref autoStop)
        #[arg(long, action = ArgAction::SetTrue)]
        auto_stop: bool,
    },
    Stop {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
    },
    Insight {
        /// Insight name (e.g. DocumentLatency, LCPBreakdown)
        #[arg(long)]
        name: Option<String>,
        /// Insight set id from perf stop "available_insight_sets"
        #[arg(long)]
        insight_set_id: Option<String>,
        /// Alias for --name (tool-ref insightName)
        #[arg(long)]
        insight_name: Option<String>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ScreencastAction {
    Start {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
    },
    Stop {
        /// Output path (.webm/.mp4 encodes via ffmpeg; otherwise PNG frames dir)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum HeapAction {
    Take {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
    },
    Close {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
    },
    Compare {
        #[arg(long, value_hint = ValueHint::FilePath)]
        base: std::path::PathBuf,
        #[arg(long, value_hint = ValueHint::FilePath)]
        current: std::path::PathBuf,
        /// Optional class index filter (tool-ref classIndex)
        #[arg(long)]
        class_index: Option<u64>,
    },
    Summary {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
    },
    Details {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        #[arg(long)]
        filter_name: Option<String>,
        #[arg(long)]
        page_idx: Option<usize>,
        #[arg(long)]
        page_size: Option<usize>,
    },
    ClassNodes {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        #[arg(long)]
        id: u64,
        #[arg(long)]
        filter_name: Option<String>,
        #[arg(long)]
        page_idx: Option<usize>,
        #[arg(long)]
        page_size: Option<usize>,
    },
    Dominators {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        #[arg(long)]
        node: u64,
    },
    DupStrings {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        #[arg(long)]
        page_idx: Option<usize>,
        #[arg(long)]
        page_size: Option<usize>,
    },
    Edges {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        #[arg(long)]
        node: u64,
        #[arg(long)]
        page_idx: Option<usize>,
        #[arg(long)]
        page_size: Option<usize>,
    },
    Retainers {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        #[arg(long)]
        node: u64,
        #[arg(long)]
        page_idx: Option<usize>,
        #[arg(long)]
        page_size: Option<usize>,
    },
    Paths {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        #[arg(long)]
        node: u64,
        #[arg(long, default_value_t = 8)]
        max_depth: u64,
        #[arg(long)]
        max_nodes: Option<u64>,
        #[arg(long)]
        max_siblings: Option<u64>,
    },
    /// Detailed info for one heap object (size, distance, retained size, detachedness)
    ObjectDetails {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        #[arg(long)]
        node: u64,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ExtensionAction {
    List,
    Install {
        path: std::path::PathBuf,
    },
    Reload {
        id: String,
        /// Unpacked extension dir so one-shot can --load-extension before reload
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
    },
    Trigger {
        id: String,
        /// Unpacked extension dir so one-shot can --load-extension before trigger
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
    },
    Uninstall {
        id: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum Devtools3pAction {
    List {
        /// Optional page URL to open before discovery
        #[arg(long)]
        url: Option<String>,
    },
    Exec {
        name: String,
        #[arg(long)]
        params: Option<String>,
        #[arg(long)]
        url: Option<String>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum WebmcpAction {
    List {
        #[arg(long)]
        url: Option<String>,
    },
    Exec {
        name: String,
        #[arg(long)]
        input: Option<String>,
        #[arg(long)]
        url: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    Elvish,
    Powershell,
}

#[derive(Debug, Clone, Subcommand)]
pub enum QrAction {
    /// Encode text to PNG, SVG, or terminal matrix
    Encode {
        #[arg(long)]
        text: String,
        /// png | svg | terminal
        #[arg(long, default_value = "png")]
        format: String,
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
    },
    /// Decode QR payload from an image file
    Decode {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum MonitorAction {
    /// Compare URL body hash/text to a baseline file and exit
    Check {
        /// URL to fetch/scrape one-shot
        #[arg(long)]
        url: String,
        /// Baseline file path (created on first run if missing when --write-baseline)
        #[arg(long, value_hint = ValueHint::FilePath)]
        baseline: std::path::PathBuf,
        /// Write/update baseline after check
        #[arg(long, action = ArgAction::SetTrue)]
        write_baseline: bool,
        /// Use browser engine instead of HTTP
        #[arg(long, default_value = "http")]
        engine: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum PageAction {
    /// Current page url and title (default when bare `page`)
    Info,
    /// List tabs in this one-shot process
    List,
    /// Open a new tab
    New {
        #[arg(long)]
        url: Option<String>,
        /// Open without focusing (tool-ref background)
        #[arg(long, action = ArgAction::SetTrue)]
        background: bool,
        /// Named isolated browser context (tool-ref isolatedContext string; GAP-004)
        #[arg(long, num_args = 0..=1, default_missing_value = "default-isolated")]
        isolated_context: Option<String>,
    },
    /// Select tab by zero-based index (alias: --page-id)
    Select {
        #[arg(value_name = "INDEX")]
        index: Option<usize>,
        /// Tool-ref pageId alias for index
        #[arg(long = "page-id")]
        page_id: Option<usize>,
        /// Bring selected tab to front (tool-ref bringToFront)
        #[arg(long, default_value_t = true)]
        bring_to_front: bool,
    },
    /// Close a tab (default: active)
    Close {
        #[arg(long)]
        index: Option<usize>,
        /// Tool-ref pageId alias for index
        #[arg(long = "page-id")]
        page_id: Option<usize>,
    },
    /// Return the stable tab id of the active page (tool-ref get_tab_id)
    TabId,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CookieAction {
    /// List cookies (optional URL filter)
    List {
        #[arg(long)]
        url: Option<String>,
    },
    /// Set cookies from a JSON array of cookie objects
    Set {
        /// JSON array: [{"name":"a","value":"b","url":"https://..."}]
        /// (long name avoids shadowing global envelope `--json`)
        #[arg(long = "cookies-json", value_name = "JSON")]
        cookies_json: String,
    },
    /// Clear all browser cookies in this one-shot process
    Clear,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum GrabFormat {
    #[default]
    Png,
    Jpeg,
    Webp,
}

/// GAP-003: tool-ref handleBeforeUnload accept | dismiss.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BeforeUnloadAction {
    Accept,
    Dismiss,
}

impl BeforeUnloadAction {
    /// CDP dialog action token.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Dismiss => "dismiss",
        }
    }
}


#[derive(Debug, Clone, Subcommand)]
pub enum AssertKind {
    Url {
        value: String,
        #[arg(long, action = ArgAction::SetTrue)]
        contains: bool,
    },
    Text {
        value: String,
        #[arg(long)]
        target: Option<String>,
    },
    Console {
        #[arg(long, default_value = "error")]
        level: String,
        #[arg(long, default_value_t = 0)]
        max: u64,
    },
    /// GAP-025: require zero console messages (any level)
    ConsoleEmpty,
    /// GAP-025: require no message text matching pattern
    ConsoleNoMatch {
        #[arg(long)]
        pattern: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConsoleAction {
    List {
        /// 0-based page index for pagination
        #[arg(long)]
        page_idx: Option<usize>,
        /// Max messages per page
        #[arg(long)]
        page_size: Option<usize>,
        /// Filter by types (comma-separated: log,warning,error,info,debug)
        #[arg(long)]
        types: Option<String>,
        /// Include messages preserved across navigations in this process
        #[arg(long, action = ArgAction::SetTrue)]
        include_preserved: bool,
        /// Optional service worker id filter
        #[arg(long)]
        service_worker_id: Option<String>,
    },
    Get {
        id: usize,
    },
    Clear,
    Dump {
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum NetAction {
    List {
        /// 0-based page index for pagination
        #[arg(long)]
        page_idx: Option<usize>,
        /// Max requests per page
        #[arg(long)]
        page_size: Option<usize>,
        /// Filter resource types (comma-separated: Document,Script,XHR,Fetch,...)
        #[arg(long)]
        resource_types: Option<String>,
        /// Include requests preserved over recent navigations in this process
        #[arg(long, action = ArgAction::SetTrue)]
        include_preserved: bool,
    },
    Get {
        /// 0-based index in net list, or CDP requestId string
        id: String,
        #[arg(long, value_hint = ValueHint::FilePath)]
        request_path: Option<std::path::PathBuf>,
        #[arg(long, value_hint = ValueHint::FilePath)]
        response_path: Option<std::path::PathBuf>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum DialogAction {
    Accept {
        #[arg(long)]
        text: Option<String>,
        /// Soft-ok when no dialog is showing (GAP-006)
        #[arg(long, action = ArgAction::SetTrue)]
        if_present: bool,
    },
    Dismiss {
        /// Soft-ok when no dialog is showing (GAP-006)
        #[arg(long, action = ArgAction::SetTrue)]
        if_present: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum MitmAction {
    /// CA paths, capture count, bind policy
    Status,
    /// List captured exchanges
    List {
        #[arg(long)]
        host: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Get one exchange by id
    Get { id: u64 },
    /// Export HAR 1.2 JSON
    Har {
        #[arg(long, value_hint = ValueHint::FilePath)]
        out: std::path::PathBuf,
    },
    /// Export capture as JSON/NDJSON
    Export {
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long, value_hint = ValueHint::FilePath)]
        out: std::path::PathBuf,
    },
    /// Unique hosts seen
    Domains,
    /// REST/GraphQL endpoint discovery
    Apis {
        #[arg(long)]
        kind: Option<String>,
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
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// WebSocket frames from capture
    Ws {
        #[command(subcommand)]
        action: MitmWsAction,
    },
    /// Short-circuit block host/path (persists for next start/capture-url in same process config note)
    Block {
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        path: Option<String>,
    },
    /// Allowlist host for TLS intercept
    Allow {
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

#[derive(Debug, Clone, Subcommand)]
pub enum MitmWsAction {
    /// List captured WebSocket frames
    List {
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Get one frame by id
    Get { id: u64 },
}

#[derive(Debug, Clone, Subcommand)]
pub enum WorkflowAction {
    /// Validate DAG and execute offline steps; journal under XDG state
    Run {
        #[arg(long, value_hint = ValueHint::FilePath)]
        manifest: std::path::PathBuf,
        #[arg(long, value_hint = ValueHint::FilePath)]
        journal: Option<std::path::PathBuf>,
    },
    /// Resume / re-run from journal + manifest
    Resume {
        #[arg(long, value_hint = ValueHint::FilePath)]
        manifest: std::path::PathBuf,
        #[arg(long, value_hint = ValueHint::FilePath)]
        journal: Option<std::path::PathBuf>,
    },
    /// Show journal step statuses
    Status {
        #[arg(long, value_hint = ValueHint::FilePath)]
        journal: Option<std::path::PathBuf>,
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigAction {
    /// Print resolved XDG paths
    Path,
    /// Create XDG layout + default config.toml
    Init,
    /// Show config values
    Show,
    /// Set a config key (lang|timeout|artifacts_dir|ignore_robots|namespace|encryption_key|color|log_level|log_to_file|chrome_path|lighthouse_path|openrouter_api_key|llm_base_url|llm_model|cache_backend|cache_redis_url)
    Set { key: String, value: String },
    /// Get one config key
    Get { key: Option<String> },
    /// List supported config keys and defaults (GAP-018)
    ListKeys,
}
