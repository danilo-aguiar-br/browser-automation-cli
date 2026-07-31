// SPDX-License-Identifier: MIT OR Apache-2.0
//! Media / performance clap action enums.

use clap::{ArgAction, Subcommand, ValueHint};

/// Performance trace lifecycle and offline insight analysis.
#[derive(Debug, Clone, Subcommand)]
pub enum PerfAction {
    /// Start a performance trace, optionally reloading and auto-stopping
    Start {
        /// Destination for the trace written when it stops
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
        /// Reload the page after tracing starts, to capture load
        #[arg(long, action = ArgAction::SetTrue)]
        reload: bool,
        /// Auto-stop after page load/reload (tool-ref autoStop)
        #[arg(long, action = ArgAction::SetTrue)]
        auto_stop: bool,
    },
    /// Stop the performance trace and write the trace artifact
    Stop {
        /// Destination for the trace artifact
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
    },
    /// Analyze one insight from a stopped trace
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

/// Screencast capture lifecycle (experimental).
#[derive(Debug, Clone, Subcommand)]
pub enum ScreencastAction {
    /// Start capturing screencast frames (requires --experimental-screencast)
    Start {
        /// Directory frames are buffered into
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
    },
    /// Stop the screencast and write frames or an encoded video
    Stop {
        /// Output path (.webm/.mp4 encodes via ffmpeg; otherwise PNG frames dir)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: Option<std::path::PathBuf>,
    },
}
/// Heap snapshot capture and offline graph analysis.
#[derive(Debug, Clone, Subcommand)]
pub enum HeapAction {
    /// Capture a heap snapshot from the live page to a .heapsnapshot file
    Take {
        /// Heap snapshot file (`.heapsnapshot`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
    },
    /// Release an open heap snapshot handle
    Close {
        /// Heap snapshot file (`.heapsnapshot`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
    },
    /// Compare two heap snapshots and report class deltas
    Compare {
        /// Baseline snapshot to compare against
        #[arg(long, value_hint = ValueHint::FilePath)]
        base: std::path::PathBuf,
        /// Newer snapshot whose growth is reported
        #[arg(long, value_hint = ValueHint::FilePath)]
        current: std::path::PathBuf,
        /// Optional class index filter (tool-ref classIndex)
        #[arg(long)]
        class_index: Option<u64>,
    },
    /// Summarize a heap snapshot by class totals
    Summary {
        /// Heap snapshot file (`.heapsnapshot`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
    },
    /// Page through heap snapshot class details with an optional name filter
    Details {
        /// Heap snapshot file (`.heapsnapshot`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Only include entries whose class name contains this substring
        #[arg(long)]
        filter_name: Option<String>,
        /// 0-based page index for pagination
        #[arg(long)]
        page_idx: Option<usize>,
        /// Maximum entries per page
        #[arg(long)]
        page_size: Option<usize>,
    },
    /// List nodes of one heap snapshot class id
    ClassNodes {
        /// Heap snapshot file (`.heapsnapshot`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Class id from `heap summary`
        #[arg(long)]
        id: u64,
        /// Only include entries whose class name contains this substring
        #[arg(long)]
        filter_name: Option<String>,
        /// 0-based page index for pagination
        #[arg(long)]
        page_idx: Option<usize>,
        /// Maximum entries per page
        #[arg(long)]
        page_size: Option<usize>,
    },
    /// Report the dominator tree entry for one heap node
    Dominators {
        /// Heap snapshot file (`.heapsnapshot`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Heap node id from a previous heap listing
        #[arg(long)]
        node: u64,
    },
    /// List duplicated strings in a heap snapshot
    DupStrings {
        /// Heap snapshot file (`.heapsnapshot`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// 0-based page index for pagination
        #[arg(long)]
        page_idx: Option<usize>,
        /// Maximum entries per page
        #[arg(long)]
        page_size: Option<usize>,
    },
    /// List outgoing edges of one heap node
    Edges {
        /// Heap snapshot file (`.heapsnapshot`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Heap node id from a previous heap listing
        #[arg(long)]
        node: u64,
        /// 0-based page index for pagination
        #[arg(long)]
        page_idx: Option<usize>,
        /// Maximum entries per page
        #[arg(long)]
        page_size: Option<usize>,
    },
    /// List retainers holding one heap node alive
    Retainers {
        /// Heap snapshot file (`.heapsnapshot`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Heap node id from a previous heap listing
        #[arg(long)]
        node: u64,
        /// 0-based page index for pagination
        #[arg(long)]
        page_idx: Option<usize>,
        /// Maximum entries per page
        #[arg(long)]
        page_size: Option<usize>,
    },
    /// Enumerate retaining paths from GC roots to one heap node
    Paths {
        /// Heap snapshot file (`.heapsnapshot`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Heap node id from a previous heap listing
        #[arg(long)]
        node: u64,
        /// Maximum path length walked back towards a GC root
        #[arg(long, default_value_t = 8)]
        max_depth: u64,
        /// Stop after visiting this many nodes (anti-pathological guard)
        #[arg(long)]
        max_nodes: Option<u64>,
        /// Maximum sibling branches explored per level
        #[arg(long)]
        max_siblings: Option<u64>,
    },
    /// Detailed info for one heap object (size, distance, retained size, detachedness)
    ObjectDetails {
        /// Heap snapshot file (`.heapsnapshot`)
        #[arg(long, value_hint = ValueHint::FilePath)]
        path: std::path::PathBuf,
        /// Heap node id from a previous heap listing
        #[arg(long)]
        node: u64,
    },
}
/// Baseline change detection over a URL.
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
