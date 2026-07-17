//! Clap derive surface for browser-automation-cli (PRD Layer L).
//!
//! Help text on flags is the primary documentation for this module.
#![allow(missing_docs)]

use clap::{Parser, Subcommand, ValueEnum};

/// One-shot browser automation CLI for AI agents.
#[derive(Debug, Parser)]
#[command(
    name = "browser-automation-cli",
    version,
    about = "One-shot browser automation CLI (Chrome CDP). BORN, EXECUTE, FINALIZE, DIE.",
    long_about = None,
    propagate_version = true
)]
pub struct Cli {
    #[command(flatten)]
    pub globals: GlobalOpts,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Parser)]
pub struct GlobalOpts {
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress non-error human logs on stderr
    #[arg(
        short = 'q',
        long = "quiet",
        global = true,
    )]
    pub quiet: bool,

    /// Increase stderr verbosity (`--verbose` once = info; use RUST_LOG for finer control)
    #[arg(
        long = "verbose",
        global = true,
    )]
    pub verbose: bool,

    /// Maximum tracing detail on stderr (debug/trace)
    #[arg(long = "debug", global = true)]
    pub debug: bool,

    #[arg(
        long,
        global = true,
        default_value_t = 0
    )]
    pub timeout: u64,

    /// Per-step timeout in seconds for `run` scripts (0 = inherit global timeout)
    #[arg(
        long,
        global = true,
        default_value_t = 0
    )]
    pub step_timeout: u64,

    /// Launch Chrome with a visible window (debug; default headless=new)
    #[arg(long, global = true)]
    pub headed: bool,

    #[arg(long, global = true)]
    pub artifacts_dir: Option<std::path::PathBuf>,

    #[arg(long, global = true)]
    pub lang: Option<String>,

    #[arg(long, global = true)]
    pub capture_console: bool,

    #[arg(long, global = true)]
    pub capture_network: bool,

    #[arg(long, global = true)]
    pub ignore_robots: bool,

    #[arg(
        long,
        global = true,
    )]
    pub i_accept_robots_risk: bool,

    /// Enable deep heap analysis tools (PRD category-memory)
    #[arg(long, global = true)]
    pub category_memory: bool,

    /// Enable extension management tools
    #[arg(
        long,
        global = true,
    )]
    pub category_extensions: bool,

    /// Enable third-party developer tool surface
    #[arg(
        long,
        global = true,
    )]
    pub category_third_party: bool,

    /// Enable WebMCP-compatible tool surface
    #[arg(long, global = true)]
    pub category_webmcp: bool,

    /// Enable experimental screencast (may require ffmpeg for file export)
    #[arg(
        long,
        global = true,
    )]
    pub experimental_screencast: bool,

    /// Enable coordinate click-at (vision) tools
    #[arg(
        long,
        global = true,
    )]
    pub experimental_vision: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Diagnose Chrome install and one-shot readiness
    Doctor {
        #[arg(long)]
        offline: bool,
        #[arg(long)]
        quick: bool,
        #[arg(long)]
        fix: bool,
        #[arg(long)]
        json: bool,
    },
    /// List available commands
    Commands {
        #[arg(long)]
        json: bool,
    },
    /// JSON Schema fragment for a command (agent discovery)
    Schema {
        #[arg(long = "cmd", value_name = "CMD")]
        cmd: String,
    },
    /// Print CLI version
    Version,
    /// Navigate to a URL (one-shot)
    Goto {
        url: String,
        /// JS to evaluate before navigation (tool-ref initScript)
        #[arg(long)]
        init_script: Option<String>,
        /// Accept beforeunload dialogs automatically
        #[arg(long)]
        handle_before_unload: bool,
        /// Navigation timeout override in milliseconds
        #[arg(long)]
        navigation_timeout_ms: Option<u64>,
    },
    /// Accessibility snapshot with @eN refs
    View {
        #[arg(long)]
        verbose: bool,
        #[arg(long)]
        path: Option<std::path::PathBuf>,
    },
    /// Click an element (selector or @eN)
    Press {
        target: String,
        #[arg(long)]
        dblclick: bool,
        /// Attach slim a11y snapshot in the same process after the action
        #[arg(long)]
        include_snapshot: bool,
    },
    /// Click at page CSS coordinates (requires --experimental-vision)
    ClickAt {
        #[arg(long)]
        x: f64,
        #[arg(long)]
        y: f64,
        #[arg(long)]
        dblclick: bool,
        /// Attach slim a11y snapshot after the click
        #[arg(long)]
        include_snapshot: bool,
    },
    /// Fill an input value (select/checkbox/radio/text smart fill)
    Write {
        target: String,
        value: String,
        /// Attach slim a11y snapshot after fill
        #[arg(long)]
        include_snapshot: bool,
    },
    /// Press a keyboard key
    Keys {
        key: String,
        /// Attach slim a11y snapshot after the key press
        #[arg(long)]
        include_snapshot: bool,
    },
    /// Type text (tool-ref type_text). Use --target or --focus-only.
    Type {
        /// Text to type (required positional)
        text: String,
        /// CSS selector or @eN (optional; use --focus-only for focused element)
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        clear: bool,
        /// Optional key to press after typing (e.g. Enter)
        #[arg(long)]
        submit: Option<String>,
        /// Type into currently focused element without resolving a target
        #[arg(long)]
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
        #[arg(long)]
        include_snapshot: bool,
    },
    /// Hover an element
    Hover {
        target: String,
        /// Attach slim a11y snapshot after hover
        #[arg(long)]
        include_snapshot: bool,
    },
    /// Drag from one target to another
    Drag {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        /// Attach slim a11y snapshot after drag
        #[arg(long)]
        include_snapshot: bool,
    },
    /// Fill multiple form fields from JSON `[{target|uid,value},...]`
    FillForm {
        #[arg(long)]
        json: String,
        /// Attach slim a11y snapshot after fill-form
        #[arg(long)]
        include_snapshot: bool,
    },
    /// Upload a file to a file input
    Upload {
        target: String,
        path: std::path::PathBuf,
        /// Attach slim a11y snapshot after upload
        #[arg(long)]
        include_snapshot: bool,
    },
    /// History back
    Back,
    /// History forward
    Forward,
    /// Reload current page
    Reload {
        #[arg(long)]
        ignore_cache: bool,
        /// JS to run before navigation/reload (tool-ref initScript)
        #[arg(long)]
        init_script: Option<String>,
        /// Accept beforeunload dialogs automatically
        #[arg(long)]
        handle_before_unload: bool,
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
        #[arg(long)]
        file_path: Option<std::path::PathBuf>,
    },
    /// Capture a screenshot
    Grab {
        #[arg(long)]
        path: Option<std::path::PathBuf>,
        #[arg(long, value_enum, default_value_t = GrabFormat::Png)]
        format: GrabFormat,
        #[arg(long)]
        full_page: bool,
        #[arg(long)]
        quality: Option<i32>,
        /// CSS selector or @eN element to capture
        #[arg(long)]
        element: Option<String>,
    },
    /// Run multi-step NDJSON script in one process
    Run {
        #[arg(long)]
        script: std::path::PathBuf,
    },
    /// Limited inline subcommand (goto)
    Exec {
        // Do NOT set allow_hyphen_values: global flags like --json after `exec`
        // must stay on GlobalOpts, not be swallowed into trailing args.
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Extract text or attribute from a target
    Extract {
        target: String,
        #[arg(long)]
        attr: Option<String>,
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
    /// Navigate and return body text / formats (local Firecrawl-parity)
    Scrape {
        url: String,
        /// text | markdown | html | links | metadata
        #[arg(long, default_value = "text")]
        format: String,
        /// http (reqwest+scraper) or browser (CDP)
        #[arg(long, default_value = "browser")]
        engine: String,
        /// Prefer main/article content heuristics
        #[arg(long)]
        only_main_content: bool,
    },
    /// Scrape many URLs from a file (HTTP engine, one-shot)
    BatchScrape {
        #[arg(long)]
        urls_file: std::path::PathBuf,
        #[arg(long, default_value = "text")]
        format: String,
        #[arg(long, default_value_t = 2)]
        concurrency: usize,
    },
    /// Crawl from a seed URL (HTTP BFS, one-shot)
    Crawl {
        url: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = 2)]
        max_depth: usize,
        #[arg(long, default_value = "text")]
        format: String,
        /// Stay on seed host
        #[arg(long, default_value_t = true)]
        same_host: bool,
    },
    /// Map site URLs from a seed (HTTP)
    Map {
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
    /// Parse a local file (html/md/txt/pdf text extract)
    Parse {
        path: std::path::PathBuf,
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
        #[arg(long)]
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
        #[arg(long)]
        mobile: bool,
    },
    /// Performance trace / metrics
    Perf {
        #[command(subcommand)]
        action: PerfAction,
    },
    /// Run Lighthouse audit (external binary)
    Lighthouse {
        url: String,
        #[arg(long)]
        out_dir: Option<std::path::PathBuf>,
        #[arg(long, default_value = "desktop")]
        device: String,
        /// navigation (default) or snapshot (maps to navigation in one-shot CLI)
        #[arg(long, default_value = "navigation")]
        mode: String,
        #[arg(long)]
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
    /// Generate shell completions (path leve, no Chrome)
    Completions {
        #[arg(value_enum)]
        shell: CompletionShell,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum PerfAction {
    Start {
        #[arg(long)]
        path: Option<std::path::PathBuf>,
        #[arg(long)]
        reload: bool,
        /// Auto-stop after page load/reload (tool-ref autoStop)
        #[arg(long)]
        auto_stop: bool,
    },
    Stop {
        #[arg(long)]
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
        #[arg(long)]
        path: Option<std::path::PathBuf>,
    },
    Stop {
        /// Output path (.webm/.mp4 encodes via ffmpeg; otherwise PNG frames dir)
        #[arg(long)]
        path: Option<std::path::PathBuf>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum HeapAction {
    Take {
        #[arg(long)]
        path: std::path::PathBuf,
    },
    Close {
        #[arg(long)]
        path: std::path::PathBuf,
    },
    Compare {
        #[arg(long)]
        base: std::path::PathBuf,
        #[arg(long)]
        current: std::path::PathBuf,
        /// Optional class index filter (tool-ref classIndex)
        #[arg(long)]
        class_index: Option<u64>,
    },
    Summary {
        #[arg(long)]
        path: std::path::PathBuf,
    },
    Details {
        #[arg(long)]
        path: std::path::PathBuf,
        #[arg(long)]
        filter_name: Option<String>,
        #[arg(long)]
        page_idx: Option<usize>,
        #[arg(long)]
        page_size: Option<usize>,
    },
    ClassNodes {
        #[arg(long)]
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
        #[arg(long)]
        path: std::path::PathBuf,
        #[arg(long)]
        node: u64,
    },
    DupStrings {
        #[arg(long)]
        path: std::path::PathBuf,
        #[arg(long)]
        page_idx: Option<usize>,
        #[arg(long)]
        page_size: Option<usize>,
    },
    Edges {
        #[arg(long)]
        path: std::path::PathBuf,
        #[arg(long)]
        node: u64,
        #[arg(long)]
        page_idx: Option<usize>,
        #[arg(long)]
        page_size: Option<usize>,
    },
    Retainers {
        #[arg(long)]
        path: std::path::PathBuf,
        #[arg(long)]
        node: u64,
        #[arg(long)]
        page_idx: Option<usize>,
        #[arg(long)]
        page_size: Option<usize>,
    },
    Paths {
        #[arg(long)]
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
        #[arg(long)]
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
        #[arg(long)]
        path: Option<std::path::PathBuf>,
    },
    Trigger {
        id: String,
        /// Unpacked extension dir so one-shot can --load-extension before trigger
        #[arg(long)]
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
        #[arg(long)]
        background: bool,
        /// Create isolated browser context when supported
        #[arg(long)]
        isolated_context: bool,
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
        #[arg(long)]
        json: String,
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

#[derive(Debug, Clone, Subcommand)]
pub enum AssertKind {
    Url {
        value: String,
        #[arg(long)]
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
        #[arg(long)]
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
        #[arg(long)]
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
        #[arg(long)]
        include_preserved: bool,
    },
    Get {
        /// 0-based index in net list, or CDP requestId string
        id: String,
        #[arg(long)]
        request_path: Option<std::path::PathBuf>,
        #[arg(long)]
        response_path: Option<std::path::PathBuf>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum DialogAction {
    Accept {
        #[arg(long)]
        text: Option<String>,
    },
    Dismiss,
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
        #[arg(long)]
        out: std::path::PathBuf,
    },
    /// Export capture as JSON/NDJSON
    Export {
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
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
}

#[derive(Debug, Clone, Subcommand)]
pub enum WorkflowAction {
    /// Validate DAG and execute offline steps; journal under XDG state
    Run {
        #[arg(long)]
        manifest: std::path::PathBuf,
        #[arg(long)]
        journal: Option<std::path::PathBuf>,
    },
    /// Resume / re-run from journal + manifest
    Resume {
        #[arg(long)]
        manifest: std::path::PathBuf,
        #[arg(long)]
        journal: Option<std::path::PathBuf>,
    },
    /// Show journal step statuses
    Status {
        #[arg(long)]
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
    /// Set a config key (lang|timeout|artifacts_dir|ignore_robots|namespace|encryption_key|color)
    Set {
        key: String,
        value: String,
    },
    /// Get one config key
    Get {
        key: Option<String>,
    },
}
