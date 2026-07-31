// SPDX-License-Identifier: MIT OR Apache-2.0
//! Top-level clap Commands subcommand enum.

use clap::{ArgAction, Subcommand, ValueHint};

use super::{
    AssertKind, BeforeUnloadAction, CompletionShell, ConfigAction, ConsoleAction, CookieAction,
    Devtools3pAction, DialogAction, ExtensionAction, GrabFormat, HeapAction, MitmAction,
    MonitorAction, NetAction, PageAction, PerfAction, QrAction, ScreencastAction, StorageAction,
    WebmcpAction, WorkflowAction,
};

/// One-shot subcommand selected from argv.
///
/// Each variant maps 1:1 to a CLI verb; variant docs are the `--help` text and
/// the man page body, so they are written for an agent reading `--help`.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Diagnose Chrome install and one-shot readiness
    ///
    /// Use global `--json` for machine-readable envelopes (no local `--json` —
    /// avoids silent shadowing of the global flag).
    Doctor {
        /// Skip network probes and record `offline` mode in the report
        #[arg(long, action = ArgAction::SetTrue)]
        offline: bool,
        /// Skip the Chrome launch probe (fastest readiness signal)
        #[arg(long, action = ArgAction::SetTrue)]
        quick: bool,
        /// Attach a suggested repair command to each failing check
        #[arg(long, action = ArgAction::SetTrue)]
        fix: bool,
    },
    /// List available commands
    ///
    /// Use global `--json` for the machine inventory payload. Default output is
    /// a flat name list (cheapest for agents); `--detail` replaces it with
    /// objects carrying description, category and surfaces.
    Commands {
        /// Emit command objects (description, category, surfaces) instead of names
        #[arg(long, action = ArgAction::SetTrue)]
        detail: bool,
    },
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
        /// Absolute URL to navigate to (robots.txt is honoured unless overridden)
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
        /// Write the snapshot to this path instead of stdout
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Allow empty about:blank snapshots (GAP-012)
        #[arg(long, action = ArgAction::SetTrue)]
        allow_empty: bool,
    },
    /// Click an element (selector or @eN)
    Press {
        /// CSS selector or `@eN` snapshot ref to click
        target: String,
        /// Send a double click instead of a single click
        #[arg(long, action = ArgAction::SetTrue)]
        dblclick: bool,
        /// Attach slim a11y snapshot in the same process after the action
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Click at page CSS coordinates (requires --experimental-vision)
    ClickAt {
        /// Horizontal page CSS pixel coordinate
        #[arg(long)]
        x: f64,
        /// Vertical page CSS pixel coordinate
        #[arg(long)]
        y: f64,
        /// Send a double click instead of a single click
        #[arg(long, action = ArgAction::SetTrue)]
        dblclick: bool,
        /// Attach slim a11y snapshot after the click
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Fill an input value (select/checkbox/radio/text smart fill)
    Write {
        /// CSS selector or `@eN` snapshot ref of the field to fill
        target: String,
        /// Value to set (select option, checkbox state, or text)
        value: String,
        /// Attach slim a11y snapshot after fill
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Press a keyboard key
    Keys {
        /// Key name to press (for example `Enter`, `Escape`, `Control+a`)
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
        /// Clear the existing value before typing
        #[arg(long, action = ArgAction::SetTrue)]
        clear: bool,
        /// Optional key to press after typing (e.g. Enter)
        #[arg(long)]
        submit: Option<String>,
        /// Type into currently focused element without resolving a target
        #[arg(long, action = ArgAction::SetTrue)]
        focus_only: bool,
        /// Attach slim a11y snapshot after typing
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Wait for ms and/or text and/or selector and/or load state
    Wait {
        /// Unconditional sleep in milliseconds before evaluating other conditions
        #[arg(long, default_value_t = 0)]
        ms: u64,
        /// Text to wait for (repeatable; resolves when any value appears — tool-ref OR)
        #[arg(long = "text", action = clap::ArgAction::Append)]
        text: Vec<String>,
        /// CSS selector that must match before the wait resolves
        #[arg(long)]
        selector: Option<String>,
        /// Page lifecycle: load | domcontentloaded | networkidle | none
        #[arg(long)]
        state: Option<String>,
        /// Max wait time in milliseconds for text/selector/state (0 = default)
        #[arg(long)]
        wait_timeout_ms: Option<u64>,
        /// Resolve when no request has been in flight for this many ms.
        /// Bare flag (or 0) uses the built-in window (GAP-032).
        #[arg(
            long = "network-idle",
            value_name = "MS",
            num_args = 0..=1,
            default_missing_value = "0"
        )]
        network_idle_ms: Option<u64>,
        /// Minimum number of nodes --selector must match (default 1; GAP-032)
        #[arg(long)]
        min_count: Option<u64>,
        /// Resolve when the serialized DOM has not changed for this many ms.
        /// Bare flag (or 0) uses the built-in window (GAP-032).
        #[arg(
            long = "dom-stable",
            value_name = "MS",
            num_args = 0..=1,
            default_missing_value = "0"
        )]
        dom_stable_ms: Option<u64>,
        /// Attach slim a11y snapshot after the wait succeeds
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Hover an element
    Hover {
        /// CSS selector or `@eN` snapshot ref to hover
        target: String,
        /// Attach slim a11y snapshot after hover
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Drag from one target to another (HTML5 drag-and-drop; GAP-030)
    Drag {
        /// Source CSS selector or `@eN` snapshot ref to drag
        #[arg(long)]
        from: String,
        /// Destination CSS selector or @eN (omit only when using --to-x/--to-y)
        #[arg(long)]
        to: Option<String>,
        /// Absolute drop X in page CSS pixels (overrides the destination rect)
        #[arg(long)]
        to_x: Option<f64>,
        /// Absolute drop Y in page CSS pixels (overrides the destination rect)
        #[arg(long)]
        to_y: Option<f64>,
        /// Where in the destination rect to drop: center | before | after.
        /// Edge anchors disambiguate insertion order in a sorted list.
        #[arg(long, default_value = "center")]
        anchor: String,
        /// Inject this CDP DragData instead of the DataTransfer the page builds.
        /// Opt-in: it bypasses the page's own dragstart handler.
        #[arg(long = "synthetic-payload", value_name = "JSON")]
        synthetic_payload: Option<String>,
        /// Attach slim a11y snapshot after drag
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Submit a form, or the form owning a field, and wait for its outcome (GAP-036)
    Submit {
        /// CSS selector or @eN of the `<form>` or of any field inside it
        target: String,
        /// Max time to wait for navigation or a completed request
        #[arg(long)]
        timeout_ms: Option<u64>,
        /// Attach slim a11y snapshot after the outcome
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
        /// CSS selector or `@eN` snapshot ref of the file input
        target: String,
        /// Local file to attach to the input
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
        /// Bypass the HTTP cache (hard reload)
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
        /// Return the deserialized structure under `data.value` plus the
        /// page-reported `data.value_type` instead of the legacy `data.result` (GAP-035)
        #[arg(long, action = ArgAction::SetTrue)]
        typed: bool,
    },
    /// Capture a screenshot
    Grab {
        /// Output path; defaults to a stamped file under the artifacts directory
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Image encoding of the capture
        #[arg(long, value_enum, default_value_t = GrabFormat::Png)]
        format: GrabFormat,
        /// Capture the whole scrollable page instead of the viewport
        #[arg(long, action = ArgAction::SetTrue)]
        full_page: bool,
        /// Encoder quality 1..=100 (jpeg/webp only)
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
        /// Baseline operation to run
        #[command(subcommand)]
        action: MonitorAction,
    },
    /// Run a multi-step script in one process (NDJSON or JSON array of steps)
    Run {
        /// Path to the script file: NDJSON (one step object per line) or a top-level JSON array of step objects
        #[arg(long, value_hint = ValueHint::FilePath)]
        script: std::path::PathBuf,
    },
    /// Single-step inline command (same surface as `run` steps: goto, wait, view, press, …)
    Exec {
        /// Step name followed by its arguments (for example `goto https://example.com`)
        // Do NOT set allow_hyphen_values: global flags like --json after `exec`
        // must stay on GlobalOpts, not be swallowed into trailing args.
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Extract text/attribute from a target, or LLM extract with --llm
    Extract {
        /// Selector, @eN ref, about:blank target, or http(s) URL for LLM/text path
        target: String,
        /// Read this attribute instead of the element text
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
    Text {
        /// CSS selector or `@eN` snapshot ref to read text from
        target: String,
    },
    /// Scroll page or element by delta pixels (PRD §7 `scroll`)
    Scroll {
        /// CSS selector or @eN (optional; omit for window scroll)
        #[arg(long)]
        target: Option<String>,
        /// Horizontal scroll delta in page CSS pixels
        #[arg(long, default_value_t = 0.0)]
        delta_x: f64,
        /// Vertical scroll delta in page CSS pixels
        #[arg(long, default_value_t = 0.0)]
        delta_y: f64,
        /// Absolute horizontal offset; wins over --delta-x on that axis (GAP-031)
        #[arg(long)]
        to_x: Option<f64>,
        /// Absolute vertical offset; wins over --delta-y on that axis (GAP-031)
        #[arg(long)]
        to_y: Option<f64>,
        /// Attach slim a11y snapshot after the scroll
        #[arg(long, action = ArgAction::SetTrue)]
        include_snapshot: bool,
    },
    /// Export or import portable auth state (cookies + localStorage + sessionStorage)
    Storage {
        /// Storage operation to run (save or load)
        #[command(subcommand)]
        action: StorageAction,
    },
    /// Cookie jar helpers for the active page (Network domain)
    Cookie {
        /// Cookie jar operation to run
        #[command(subcommand)]
        action: CookieAction,
    },
    /// Read one attribute from a target
    Attr {
        /// CSS selector or `@eN` snapshot ref to read from
        target: String,
        /// Attribute name to read (falls back to the DOM property)
        name: String,
    },
    /// Assertions (url / text / console)
    Assert {
        /// Assertion to evaluate; a failed assertion sets a non-zero exit code
        #[command(subcommand)]
        kind: AssertKind,
    },
    /// Captured console messages (--capture-console)
    Console {
        /// Console buffer operation to run
        #[command(subcommand)]
        action: ConsoleAction,
    },
    /// Captured network requests (--capture-network)
    Net {
        /// Network capture operation to run
        #[command(subcommand)]
        action: NetAction,
    },
    /// Page info or multi-tab management
    Page {
        /// Tab operation to run; omit for info about the active page
        #[command(subcommand)]
        action: Option<PageAction>,
    },
    /// Accept or dismiss dialogs
    Dialog {
        /// How to answer the next JavaScript dialog
        #[command(subcommand)]
        action: DialogAction,
    },
    /// Navigate and return body text / formats (local HTTP or CDP scrape)
    Scrape {
        /// Absolute URL to fetch
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
        /// File with one absolute URL per line
        #[arg(long, value_hint = ValueHint::FilePath)]
        urls_file: std::path::PathBuf,
        /// Output format applied to every URL
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
        /// Seed URL the breadth-first crawl starts from
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        /// Maximum number of pages to fetch
        #[arg(long, alias = "max-pages", default_value_t = 20)]
        limit: usize,
        /// Maximum link depth from the seed
        #[arg(long, default_value_t = 2)]
        max_depth: usize,
        /// Output format applied to every page
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
        /// Seed URL to expand into a URL list
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        /// Maximum number of URLs to return
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Maximum link depth from the seed
        #[arg(long, default_value_t = 2)]
        max_depth: usize,
    },
    /// Local search (HTTP SERP links or URL map)
    Search {
        /// Search terms sent to the configured HTML endpoint
        query: String,
        /// Maximum number of results to return
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Parse a local file (html/md/txt/pdf/docx/xlsx text extract)
    Parse {
        /// Local file to extract text from
        #[arg(value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Mask email/phone/card-like patterns in text output
        #[arg(long, action = ArgAction::SetTrue)]
        redact_pii: bool,
    },
    /// QR encode/decode one-shot (no Chrome)
    Qr {
        /// QR operation to run (encode or decode)
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
        /// MITM operation to run
        #[command(subcommand)]
        action: MitmAction,
    },
    /// Workflow journal DAG (petgraph + SQLite)
    Workflow {
        /// Workflow operation to run
        #[command(subcommand)]
        action: WorkflowAction,
    },
    /// XDG config and path management (no .env at runtime)
    Config {
        /// XDG config operation to run
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Emulate device / network / UA / geo / CPU
    Emulate {
        /// Override the browser User-Agent string
        #[arg(long)]
        user_agent: Option<String>,
        /// BCP47 locale reported to the page (for example `pt-BR`)
        #[arg(long)]
        locale: Option<String>,
        /// IANA timezone reported to the page (for example `America/Sao_Paulo`)
        #[arg(long)]
        timezone: Option<String>,
        /// Force the page offline (no network)
        #[arg(long, action = ArgAction::SetTrue)]
        offline: bool,
        /// Geolocation latitude in degrees; pair with --longitude
        #[arg(long)]
        latitude: Option<f64>,
        /// Geolocation longitude in degrees; pair with --latitude
        #[arg(long)]
        longitude: Option<f64>,
        /// CSS media type to emulate (for example `print`)
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
        /// Viewport width in CSS pixels
        #[arg(long)]
        width: i32,
        /// Viewport height in CSS pixels
        #[arg(long)]
        height: i32,
        /// Device pixel ratio
        #[arg(long, default_value_t = 1.0)]
        scale: f64,
        /// Enable mobile metrics emulation
        #[arg(long, action = ArgAction::SetTrue)]
        mobile: bool,
    },
    /// Performance trace / metrics
    Perf {
        /// Performance trace operation to run
        #[command(subcommand)]
        action: PerfAction,
    },
    /// Run Lighthouse audit (external binary)
    Lighthouse {
        /// Absolute URL to audit
        #[arg(value_hint = ValueHint::Url)]
        url: String,
        /// Directory for the generated report files
        #[arg(long, value_hint = ValueHint::DirPath)]
        out_dir: Option<std::path::PathBuf>,
        /// Form factor preset: desktop | mobile
        #[arg(long, default_value = "desktop")]
        device: String,
        /// navigation (default) or snapshot (maps to navigation in one-shot CLI)
        #[arg(long, default_value = "navigation")]
        mode: String,
        /// Absolute path to the lighthouse binary; overrides PATH and XDG
        #[arg(long, value_hint = ValueHint::FilePath)]
        lighthouse_path: Option<std::path::PathBuf>,
    },
    /// Screencast start/stop (experimental)
    Screencast {
        /// Screencast operation to run
        #[command(subcommand)]
        action: ScreencastAction,
    },
    /// Heap snapshot tools (requires --category-memory for deep analysis)
    Heap {
        /// Heap snapshot operation to run
        #[command(subcommand)]
        action: HeapAction,
    },
    /// Chrome extension tools (requires --category-extensions)
    Extension {
        /// Extension operation to run
        #[command(subcommand)]
        action: ExtensionAction,
    },
    /// Third-party developer tools surface (requires --category-third-party)
    Devtools3p {
        /// Third-party tool operation to run
        #[command(subcommand)]
        action: Devtools3pAction,
    },
    /// Web surface tools (requires --category-webmcp)
    Webmcp {
        /// Web surface tool operation to run
        #[command(subcommand)]
        action: WebmcpAction,
    },
    /// Generate shell completions (path-level, no Chrome)
    Completions {
        /// Shell dialect to generate completions for
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
