// SPDX-License-Identifier: MIT OR Apache-2.0
//! The two Chrome launch strategies behind `chrome_legacy_oxide_launch`.
//!
//! | Strategy | Who owns the child | `chrome_pid()` | Hard-kill outcome |
//! |----------|--------------------|----------------|-------------------|
//! | [`launch_self_spawned`] (default) | this process | real pid | FINALIZE / kernel binding reap the group |
//! | [`launch_via_oxide`] (fallback) | chromiumoxide | `None` | the browser tree is orphaned |
//!
//! The fallback is kept only so an operator can unblock a host where the
//! self-spawn path fails, and it is the reason `chrome_pid()` is still an
//! `Option` rather than a plain `u32`.

use rustc_hash::FxHashSet;
use std::sync::Arc;

use crate::native::browser::{BrowserManager, BrowserProcess};
use crate::native::cdp::chrome::LaunchOptions;
use crate::native::cdp::client::CdpClient;

/// Post-CDP manager fields that both strategies carry through unchanged.
pub(super) struct ChromeSeed {
    /// Download directory re-applied to new browser contexts.
    pub download_path: Option<String>,
    /// Whether certificate errors are ignored for this session.
    pub ignore_https_errors: bool,
}

/// Default per-operation timeout for a Chrome-backed manager (milliseconds).
///
/// Operator override: XDG `config set chrome_default_timeout_ms <n>`.
fn chrome_default_timeout_ms() -> u64 {
    crate::xdg::policy::policy_u64(crate::xdg::policy::key::CHROME_DEFAULT_TIMEOUT_MS)
}

/// Assemble a manager around an established Chrome connection.
fn assemble(
    client: Arc<CdpClient>,
    browser_process: Option<BrowserProcess>,
    owns_oxide_browser: bool,
    ws_url: String,
    seed: ChromeSeed,
    temp_user_data_dir: Option<std::path::PathBuf>,
) -> BrowserManager {
    BrowserManager {
        client,
        browser_process,
        owns_oxide_browser,
        ws_url,
        pages: Vec::new(),
        active_page_index: 0,
        default_timeout_ms: chrome_default_timeout_ms(),
        download_path: seed.download_path,
        ignore_https_errors: seed.ignore_https_errors,
        visited_origins: FxHashSet::default(),
        next_tab_id: 1,
        direct_page: false,
        temp_user_data_dir,
    }
}

/// Default strategy: fork Chrome here, then connect over CDP.
///
/// The resulting manager owns a [`BrowserProcess::Chrome`], so `chrome_pid()`
/// answers with a real pid and the lifecycle ledger gets a residual kill target.
pub(super) async fn launch_self_spawned(
    options: &LaunchOptions,
    seed: ChromeSeed,
) -> Result<BrowserManager, String> {
    let launched = crate::native::cdp::chrome::launch_self_spawned(options).await?;
    let crate::native::cdp::chrome::ChromeLaunch {
        browser,
        handler,
        process,
        ws_url,
        temp_user_data_dir,
    } = launched;
    let client = Arc::new(CdpClient::from_browser(browser, handler).await?);
    Ok(assemble(
        client,
        Some(BrowserProcess::Chrome(process)),
        // The browser handle came from `connect`, so chromiumoxide holds no
        // `Child`: FINALIZE must reap through `browser_process`, not through
        // the oxide close/wait/kill path.
        false,
        ws_url,
        seed,
        temp_user_data_dir,
    ))
}

/// Fallback strategy: let chromiumoxide fork and own Chrome.
///
/// Records no pid, by construction: `Browser::launch` keeps the `Child` private.
pub(super) async fn launch_via_oxide(
    options: &LaunchOptions,
    seed: ChromeSeed,
) -> Result<BrowserManager, String> {
    tracing::warn!(
        "chrome_legacy_oxide_launch is on: Chrome runs without a residual kill \
         target, so a hard kill of this process orphans the browser tree"
    );
    let launched = crate::native::cdp::oxide::launch_with_oxide(options).await?;
    let crate::native::cdp::oxide::OxideLaunch {
        browser,
        handler,
        ws_url,
        temp_user_data_dir,
        ..
    } = launched;
    let client = Arc::new(CdpClient::from_browser(browser, handler).await?);
    Ok(assemble(
        client,
        None,
        true,
        ws_url,
        seed,
        temp_user_data_dir,
    ))
}
