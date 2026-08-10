// SPDX-License-Identifier: MIT OR Apache-2.0
//! High-level native browser session (tabs, navigation, CDP attach).
//!
//! # Workload
//!
//! **I/O-bound (CDP):** multi-target attach uses
//! [`crate::concurrency::join_bounded_ordered`] under the process budget.
//! Single-tab navigation/interaction stays sequential (one Page session).
//! Profile cleanup uses `spawn_blocking` so Drop/async paths do not pin workers.
//!
//! # Module map (Pass 31 SRP-01)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | types | PageInfo, TabRef, WaitUntil, pure helpers |
//! | validate | launch option validation, AI-friendly errors |
//! | launch | launch/connect, domains, lightpanda attach |
//! | navigate | navigate, wait, evaluate, content getters |
//! | tabs | tab/page registry management |
//! | emulate | viewport, UA, geo, dialog, upload, scripts |
//! | lifecycle | close, pid, download behavior |

use rustc_hash::FxHashSet;
use std::sync::Arc;

use crate::native::cdp::chrome::ChromeProcess;
use crate::native::cdp::client::CdpClient;
use crate::native::cdp::lightpanda::LightpandaProcess;

mod emulate;
mod identity_override;
mod launch;
mod lifecycle;
mod navigate;
mod tabs;
mod types;
mod validate;

#[cfg(test)]
mod tests;

pub use types::{format_tab_id, is_valid_label, PageInfo, TabRef, WaitUntil};
pub use validate::{to_ai_friendly_error, validate_launch_options};

/// A browser child this invocation spawned and is therefore responsible for.
///
/// The Chrome variant exists so `chrome_pid()` can answer with a real pid. While
/// Chrome was forked inside `chromiumoxide::Browser::launch` the product owned
/// no `Child`, so FINALIZE had nothing to signal and a hard-killed CLI orphaned
/// the whole browser tree.
pub enum BrowserProcess {
    /// A Lightpanda process this invocation spawned and must reap.
    Lightpanda(LightpandaProcess),
    /// A Chrome process this invocation spawned and must reap.
    Chrome(ChromeProcess),
}

impl BrowserProcess {
    /// Kill and reap the child. Idempotent, so it is safe from `Drop`.
    pub fn kill(&mut self) {
        match self {
            BrowserProcess::Lightpanda(p) => p.kill(),
            BrowserProcess::Chrome(p) => p.kill(),
        }
    }

    /// Wait up to `timeout` for a cooperative exit, then kill+reap.
    pub fn wait_or_kill(&mut self, timeout: std::time::Duration) {
        match self {
            BrowserProcess::Lightpanda(p) => p.wait_or_kill(timeout),
            BrowserProcess::Chrome(p) => p.wait_or_kill(timeout),
        }
    }

    /// Non-blocking check whether the browser process has exited (or was reaped).
    pub fn has_exited(&mut self) -> bool {
        match self {
            BrowserProcess::Lightpanda(p) => p.has_exited(),
            BrowserProcess::Chrome(p) => p.has_exited(),
        }
    }

    /// OS process id while still owned; `None` after kill/reap.
    pub fn id(&self) -> Option<u32> {
        match self {
            BrowserProcess::Lightpanda(p) => p.id(),
            BrowserProcess::Chrome(p) => p.id(),
        }
    }

    /// POSIX process group of the child, when the host models one.
    ///
    /// `Some` lets FINALIZE signal the whole tree with `kill(-pgid, …)` instead
    /// of only the launcher pid. Lightpanda is spawned without a dedicated group
    /// and therefore reports `None`.
    pub fn pgid(&self) -> Option<i32> {
        match self {
            BrowserProcess::Lightpanda(_) => None,
            BrowserProcess::Chrome(p) => p.pgid(),
        }
    }
}

/// Owns the CDP client, optional child browser process, and page registry.
///
/// Drop / explicit FINALIZE must reap Chrome; treat as a resource owner.
#[must_use = "BrowserManager owns Chrome/CDP resources; shut down before discard"]
pub struct BrowserManager {
    /// Shared CDP connection. `Arc` because page-scoped helpers hold it too.
    pub client: Arc<CdpClient>,
    browser_process: Option<BrowserProcess>,
    /// True when this manager owns a chromiumoxide-launched Chrome (FINALIZE close/wait/kill).
    owns_oxide_browser: bool,
    ws_url: String,
    pages: Vec<PageInfo>,
    active_page_index: usize,
    default_timeout_ms: u64,
    /// Stored download path from launch options, re-applied to new contexts (e.g., recording)
    pub download_path: Option<String>,
    /// Whether to ignore HTTPS certificate errors, re-applied to new contexts (e.g., recording)
    pub ignore_https_errors: bool,
    /// Origins visited during this session, used by save_state to collect cross-origin localStorage.
    visited_origins: FxHashSet<String>,
    next_tab_id: u32,
    /// True when the CDP WebSocket is already scoped to a page target and
    /// browser-level Target.* commands are not available.
    direct_page: bool,
    /// One-shot temp Chrome user-data-dir to remove after FINALIZE.
    temp_user_data_dir: Option<std::path::PathBuf>,
}
