// SPDX-License-Identifier: MIT OR Apache-2.0
//! Top-level clap Commands subcommand enum.

use clap::{ArgAction, Subcommand, ValueHint};

use super::args_interact::{DragArgs, EmulateArgs, ScrollArgs, WaitArgs};
use super::args_page::{
    ClickAtArgs, EvalArgs, ExtractArgs, GotoArgs, GrabArgs, RecordArgs, TypeArgs, ViewArgs,
};
use super::args_scrape::{
    BatchScrapeArgs, CrawlArgs, FeedArgs, MapArgs, ScrapeArgs, SearchArgs, SitemapArgs,
};
use super::args_small::{
    DoctorArgs, FillFormArgs, HoverArgs, KeysArgs, PressArgs, ReloadArgs, SchemaArgs, SubmitArgs,
    UploadArgs, WriteArgs,
};
use super::args_tools::{FindPathsArgs, LighthouseArgs, ResizeArgs, SheetWriteArgs};

use super::{
    AssertKind, AudioAction, CompletionShell, ConfigAction, ConsoleAction, CookieAction,
    Devtools3pAction, DialogAction, ExtensionAction, HeapAction, ImageAction, MitmAction,
    MonitorAction, NetAction, PageAction, PerfAction, QrAction, ScreencastAction, StorageAction,
    VideoAction, WebmcpAction, WorkflowAction,
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
    Doctor(DoctorArgs),
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
    Schema(SchemaArgs),
    /// Print CLI version
    Version,
    /// Show resolved UI locale and detection diagnostics (human suggestions only)
    Locale,
    /// Navigate to a URL (one-shot)
    Goto(GotoArgs),
    /// Accessibility snapshot with @eN refs
    View(ViewArgs),
    /// Click an element (selector or @eN)
    Press(PressArgs),
    /// Click at page CSS coordinates (requires --experimental-vision)
    ClickAt(ClickAtArgs),
    /// Fill an input value (select/checkbox/radio/text smart fill)
    Write(WriteArgs),
    /// Press a keyboard key
    Keys(KeysArgs),
    /// Type text (tool-ref type_text). Use --target or --focus-only.
    Type(TypeArgs),
    /// Wait for ms and/or text and/or selector and/or load state
    Wait(WaitArgs),
    /// Hover an element
    Hover(HoverArgs),
    /// Drag from one target to another (HTML5 drag-and-drop; GAP-030)
    Drag(DragArgs),
    /// Submit a form, or the form owning a field, and wait for its outcome (GAP-036)
    Submit(SubmitArgs),
    /// Fill multiple form fields from JSON `[{target|uid,value},...]`
    FillForm(FillFormArgs),
    /// Upload a file to a file input
    Upload(UploadArgs),
    /// History back
    Back,
    /// History forward
    Forward,
    /// Reload current page
    Reload(ReloadArgs),
    /// Evaluate JavaScript (expression or function declaration)
    Eval(EvalArgs),
    /// Capture a screenshot
    Grab(GrabArgs),
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
    /// Record page interactions as a replayable `run --script` NDJSON file
    Record(RecordArgs),
    /// Extract text/attribute from a target, or LLM extract with --llm
    Extract(ExtractArgs),
    /// Extract visible text from a target (PRD §7 `text`)
    Text {
        /// CSS selector or `@eN` snapshot ref to read text from
        target: String,
    },
    /// Scroll page or element by delta pixels (PRD §7 `scroll`)
    Scroll(ScrollArgs),
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
    Scrape(ScrapeArgs),
    /// Scrape many URLs from a file (HTTP or browser engine, one-shot)
    BatchScrape(BatchScrapeArgs),
    /// Crawl from a seed URL (HTTP BFS or browser, one-shot)
    Crawl(CrawlArgs),
    /// Map site URLs from a seed (HTTP)
    Map(MapArgs),
    /// List URLs declared by a site's sitemap.xml (HTTP)
    Sitemap(SitemapArgs),
    /// Read an RSS / Atom / JSON Feed document (HTTP)
    Feed(FeedArgs),
    /// Local search (HTTP SERP links or URL map)
    Search(SearchArgs),
    /// Parse a local file (html/md/txt/pdf/docx/xlsx text extract)
    Parse {
        /// Local file to extract text from
        #[arg(value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Mask email/phone/card-like patterns in text output
        #[arg(long, action = ArgAction::SetTrue)]
        redact_pii: bool,
        /// Scrape formats to derive from the parsed file (CSV or repeatable).
        ///
        /// HTML input accepts every `scrape` format. Non-HTML input (pdf, docx,
        /// spreadsheets, csv, txt) has no DOM, so it accepts only the
        /// text-derived ones: text, markdown, summary.
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        format: Vec<String>,
    },
    /// QR encode/decode one-shot (no Chrome)
    Qr {
        /// QR operation to run (encode or decode)
        #[command(subcommand)]
        action: QrAction,
    },
    /// Local image pipeline one-shot (no Chrome): info/convert/resize/download/exif
    Image {
        /// Image operation to run
        #[command(subcommand)]
        action: ImageAction,
    },
    /// Local video pipeline one-shot (no Chrome): info/download/convert/to-mp3/trim/thumbnail/manifest
    Video {
        /// Video operation to run
        #[command(subcommand)]
        action: VideoAction,
    },
    /// Local audio pipeline one-shot (no Chrome): info/download/convert/trim
    Audio {
        /// Audio operation to run
        #[command(subcommand)]
        action: AudioAction,
    },
    /// Discover filesystem paths (fd-like UX; binary remains browser-automation-cli)
    FindPaths(FindPathsArgs),
    /// Structural lint scan for forbidden product patterns (one-shot; §5AC / GAP-A011)
    SgScan {
        /// Roots to scan (default: `.`)
        #[arg(num_args = 0..)]
        paths: Vec<String>,
        /// Max findings (0 = unlimited)
        #[arg(long, default_value_t = crate::constants::SG_SCAN_FINDINGS_LIMIT)]
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
    SheetWrite(SheetWriteArgs),
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
    Emulate(EmulateArgs),
    /// Resize page viewport
    Resize(ResizeArgs),
    /// Performance trace / metrics
    Perf {
        /// Performance trace operation to run
        #[command(subcommand)]
        action: PerfAction,
    },
    /// Run Lighthouse audit (external binary)
    Lighthouse(LighthouseArgs),
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
