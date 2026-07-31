// SPDX-License-Identifier: MIT OR Apache-2.0
//! Browser page / network / assert clap action enums.

use clap::{ArgAction, Subcommand, ValueEnum, ValueHint};

/// Tab (page) operations inside the one-shot process.
#[derive(Debug, Clone, Subcommand)]
pub enum PageAction {
    /// Current page url and title (default when bare `page`)
    Info,
    /// List tabs in this one-shot process
    List,
    /// Open a new tab
    New {
        /// URL to open in the new tab (defaults to about:blank)
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
        /// Zero-based tab index to activate
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
        /// Zero-based tab index to close (defaults to the active tab)
        #[arg(long)]
        index: Option<usize>,
        /// Tool-ref pageId alias for index
        #[arg(long = "page-id")]
        page_id: Option<usize>,
    },
    /// Return the stable tab id of the active page (tool-ref get_tab_id)
    TabId,
}

/// Cookie jar operations on the active page.
#[derive(Debug, Clone, Subcommand)]
pub enum CookieAction {
    /// List cookies (optional URL filter)
    List {
        /// Only return cookies scoped to this URL
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

/// Image encoding accepted by `grab`.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum GrabFormat {
    /// Lossless PNG (default).
    #[default]
    Png,
    /// Lossy JPEG; honours `--quality`.
    Jpeg,
    /// Lossy WebP; honours `--quality`.
    Webp,
}

/// GAP-003: tool-ref handleBeforeUnload accept | dismiss.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BeforeUnloadAction {
    /// Let the navigation proceed.
    Accept,
    /// Cancel the navigation and stay on the page.
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

/// Assertions that turn a page observation into a process exit code.
#[derive(Debug, Clone, Subcommand)]
pub enum AssertKind {
    /// Assert the current page URL equals or contains a value
    Url {
        /// Expected URL, matched exactly unless --contains is set
        value: String,
        /// Match `value` as a substring instead of the whole URL
        #[arg(long, action = ArgAction::SetTrue)]
        contains: bool,
    },
    /// Assert page or target text contains a value (substring match)
    Text {
        /// Substring that must appear in the searched text
        value: String,
        /// Restrict the search to this CSS selector or `@eN` ref
        #[arg(long)]
        target: Option<String>,
    },
    /// Assert captured console messages of a level stay at or below --max
    Console {
        /// Console level to count (for example `error`, `warning`)
        #[arg(long, default_value = "error")]
        level: String,
        /// Highest count still considered a pass
        #[arg(long, default_value_t = 0)]
        max: u64,
    },
    /// GAP-025: require zero console messages (any level)
    ConsoleEmpty,
    /// GAP-025: require no message text matching pattern
    ConsoleNoMatch {
        /// Regular expression that must not match any console message
        #[arg(long)]
        pattern: String,
    },
}

/// Operations over the console buffer captured with `--capture-console`.
#[derive(Debug, Clone, Subcommand)]
pub enum ConsoleAction {
    /// List captured console messages with pagination and type filters
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
    /// Get one captured console message by 0-based id
    Get {
        /// 0-based index in the captured console list
        id: usize,
    },
    /// Drop all console messages captured in this process
    Clear,
    /// Write captured console messages to a JSON file
    Dump {
        /// Destination file for the JSON dump
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
    },
}

/// Operations over the request buffer captured with `--capture-network`.
#[derive(Debug, Clone, Subcommand)]
pub enum NetAction {
    /// List captured network requests with pagination and resource-type filters
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
    /// Get one captured network request by index or CDP requestId
    Get {
        /// 0-based index in net list, or CDP requestId string
        id: String,
        /// Write the request body to this file
        #[arg(long, value_hint = ValueHint::FilePath)]
        request_path: Option<std::path::PathBuf>,
        /// Write the response body to this file
        #[arg(long, value_hint = ValueHint::FilePath)]
        response_path: Option<std::path::PathBuf>,
    },
}

/// How to answer a JavaScript dialog (`alert`, `confirm`, `prompt`).
#[derive(Debug, Clone, Subcommand)]
pub enum DialogAction {
    /// Accept the open dialog, optionally answering a prompt
    Accept {
        /// Text typed into a `prompt` before accepting
        #[arg(long)]
        text: Option<String>,
        /// Soft-ok when no dialog is showing (GAP-006)
        #[arg(long, action = ArgAction::SetTrue)]
        if_present: bool,
    },
    /// Dismiss the open dialog
    Dismiss {
        /// Soft-ok when no dialog is showing (GAP-006)
        #[arg(long, action = ArgAction::SetTrue)]
        if_present: bool,
    },
}

/// Portable auth state (GAP-034 pillar 1). The path is always explicit.
#[derive(Debug, Clone, Subcommand)]
pub enum StorageAction {
    /// Write cookies, localStorage and sessionStorage to an explicit path (mode 0600)
    Export {
        /// Destination file; never implicit, never under XDG by default
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Navigate here first so the export sees the authenticated origin
        #[arg(long)]
        url: Option<String>,
    },
    /// Restore cookies and per-origin storage from an explicit path
    Import {
        /// Source file produced by `storage export`
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Navigate here after the import so the restored state applies
        #[arg(long)]
        url: Option<String>,
    },
}
