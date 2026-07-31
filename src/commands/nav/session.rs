// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared session helpers for nav handlers.

use crate::browser::{block_on_browser_timeout, CaptureOpts, OneShotSession};
use crate::error::{CliError, ErrorKind};
use crate::lifecycle::Lifecycle;

pub(crate) fn with_session_blank<F, Fut>(
    life: &Lifecycle,
    capture: CaptureOpts,
    timeout_secs: u64,
    f: F,
) -> Result<serde_json::Value, CliError>
where
    F: FnOnce(OneShotSession) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(OneShotSession, serde_json::Value), CliError>> + Send,
{
    block_on_browser_timeout(
        async move {
            let mut session = OneShotSession::launch_headless_with_capture(capture).await?;
            life.record_chrome(session.chrome_pid());
            let _ = session
                .goto("about:blank", crate::robots::RobotsPolicy::Honor)
                .await?;
            let (session, value) = f(session).await?;
            let close = session.shutdown().await;
            life.clear_chrome();
            close?;
            Ok(value)
        },
        timeout_secs,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn post_webhook(webhook_url: &str, data: &serde_json::Value) -> Result<(), CliError> {
    // One-shot operator webhook; no product telemetry. Reuse process-wide client.
    // SSRF: strict public http(s) only (product law).
    crate::net::assert_safe_http_url(webhook_url)?;
    let client = crate::llm_local::shared_blocking_http_client()?;
    let mut last_err = String::new();
    for attempt in 0..crate::xdg::policy::policy_u32(crate::xdg::policy::key::WEBHOOK_MAX_ATTEMPTS)
    {
        match client
            .post(webhook_url)
            .timeout(std::time::Duration::from_secs(
                crate::xdg::policy::policy_u64(crate::xdg::policy::key::WEBHOOK_POST_TIMEOUT_SECS),
            ))
            .json(data)
            .send()
        {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            Ok(resp) => {
                last_err = format!("webhook HTTP {}", resp.status());
            }
            Err(e) => last_err = format!("webhook: {e}"),
        }
        if attempt + 1
            < crate::xdg::policy::policy_u32(crate::xdg::policy::key::WEBHOOK_MAX_ATTEMPTS)
        {
            std::thread::sleep(std::time::Duration::from_millis(
                crate::xdg::policy::policy_u64(
                    crate::xdg::policy::key::WEBHOOK_RETRY_BASE_DELAY_MS,
                ) * (1 << attempt),
            ));
        }
    }
    Err(CliError::with_suggestion(
        ErrorKind::Unavailable,
        last_err,
        crate::i18n::suggestion_key("webhook_unreachable", None),
    ))
}
