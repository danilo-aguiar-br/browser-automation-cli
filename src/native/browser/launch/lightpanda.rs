// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lightpanda CDP attach retry, startup deadline, and manager bootstrap.

use rustc_hash::FxHashSet;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::native::browser::{BrowserManager, BrowserProcess};
use crate::native::cdp::client::CdpClient;

/// Lightpanda CDP connect attempt budget (XDG `lightpanda_cdp_connect_timeout_secs`).
pub(crate) fn lightpanda_cdp_connect_timeout() -> Duration {
    crate::xdg::policy::policy_secs(crate::xdg::policy::key::LIGHTPANDA_CDP_CONNECT_TIMEOUT_SECS)
}
/// Lightpanda CDP readiness poll interval (XDG `lightpanda_poll_interval_ms`).
pub(crate) fn lightpanda_cdp_connect_poll_interval() -> Duration {
    crate::xdg::policy::policy_millis(crate::xdg::policy::key::LIGHTPANDA_POLL_INTERVAL_MS)
}
/// Lightpanda target init wait after connect (XDG `lightpanda_target_init_timeout_secs`).
pub(crate) fn lightpanda_target_init_budget() -> Duration {
    crate::xdg::policy::policy_secs(crate::xdg::policy::key::LIGHTPANDA_TARGET_INIT_TIMEOUT_SECS)
}

pub(crate) async fn attach_ws_with_retry(
    ws_url: &str,
    total_timeout: Duration,
    poll_interval: Duration,
) -> Result<CdpClient, String> {
    let deadline = Instant::now() + total_timeout;

    loop {
        match CdpClient::connect(ws_url).await {
            Ok(client) => return Ok(client),
            Err(err) => {
                if Instant::now() >= deadline {
                    return Err(err);
                }
            }
        }

        tokio::time::sleep(poll_interval).await;
    }
}

pub(crate) async fn initialize_lightpanda_manager(
    ws_url: String,
    process: BrowserProcess,
) -> Result<BrowserManager, String> {
    let deadline = Instant::now() + lightpanda_target_init_budget();
    let mut process = Some(process);

    loop {
        let client = match attach_ws_with_retry(
            &ws_url,
            lightpanda_cdp_connect_timeout(),
            lightpanda_cdp_connect_poll_interval(),
        )
        .await
        {
            Ok(client) => client,
            Err(err) => {
                if Instant::now() >= deadline {
                    return Err(lightpanda_target_init_timeout(Some(&err)));
                }
                tokio::time::sleep(lightpanda_cdp_connect_poll_interval()).await;
                continue;
            }
        };

        let mut manager = BrowserManager {
            client: Arc::new(client),
            browser_process: None,
            owns_oxide_browser: false,
            ws_url: ws_url.clone(),
            pages: Vec::new(),
            active_page_index: 0,
            // Same knob the Chrome engine reads. This was the literal `25_000`,
            // which happened to equal the Chrome default and silently stopped
            // tracking it: `config set chrome_default_timeout_ms` moved one
            // engine and not the other. The per-operation budget is a property
            // of the CDP round trip, not of which binary is on the far end.
            default_timeout_ms: crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::CHROME_DEFAULT_TIMEOUT_MS,
            ),
            download_path: None,
            ignore_https_errors: false,
            visited_origins: FxHashSet::default(),
            next_tab_id: 1,
            direct_page: false,
            temp_user_data_dir: None,
        };

        match discover_and_attach_lightpanda_targets(&mut manager, deadline).await {
            Ok(()) => {
                manager.browser_process = process.take();
                return Ok(manager);
            }
            Err(err) => {
                if Instant::now() >= deadline {
                    return Err(lightpanda_target_init_timeout(Some(&err)));
                }
                tokio::time::sleep(lightpanda_cdp_connect_poll_interval()).await;
            }
        }
    }
}

pub(crate) async fn discover_and_attach_lightpanda_targets(
    manager: &mut BrowserManager,
    deadline: Instant,
) -> Result<(), String> {
    run_with_lightpanda_deadline(
        deadline,
        manager.discover_and_attach_targets(),
        "Target domain initialization attempt exceeded the remaining startup deadline",
    )
    .await
}

pub(crate) fn remaining_until(deadline: Instant) -> Option<Duration> {
    deadline.checked_duration_since(Instant::now())
}

pub(crate) async fn run_with_lightpanda_deadline<F, T>(
    deadline: Instant,
    operation: F,
    timeout_context: &'static str,
) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    let remaining = remaining_until(deadline)
        .ok_or_else(|| lightpanda_target_init_timeout(Some("deadline expired before retry")))?;

    match tokio::time::timeout(remaining, operation).await {
        Ok(result) => result,
        Err(_) => Err(lightpanda_target_init_timeout(Some(timeout_context))),
    }
}

pub(crate) fn lightpanda_target_init_timeout(last_error: Option<&str>) -> String {
    let mut message = format!(
        "Timed out after {}ms waiting for Lightpanda Target domain to initialize",
        lightpanda_target_init_budget().as_millis(),
    );
    if let Some(last_error) = last_error {
        message.push_str(&format!("\nLast error: {last_error}"));
    }
    message
}
