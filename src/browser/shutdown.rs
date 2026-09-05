// SPDX-License-Identifier: MIT OR Apache-2.0
//! OS shutdown signal detection for one-shot lifecycle.

use crate::error::{CliError, ErrorKind};

pub(crate) fn cancelled_error() -> CliError {
    CliError::with_suggestion(
        ErrorKind::Cancelled,
        "cancelled by signal (SIGINT/SIGTERM)",
        crate::i18n::suggestion_key("retry_after_cancel", None),
    )
}

/// Which OS event triggered cooperative shutdown (for logs / tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownTrigger {
    /// Portable Ctrl-C / `tokio::signal::ctrl_c`.
    CtrlC,
    /// Unix `SIGINT`.
    SigInt,
    /// Unix `SIGTERM` (systemd, k8s, supervisors).
    SigTerm,
    /// Windows Ctrl-Break.
    CtrlBreak,
    /// Windows console close (user closed the console window).
    CtrlClose,
}

impl ShutdownTrigger {
    /// Stable machine-readable label for tracing.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CtrlC => "ctrl_c",
            Self::SigInt => "sigint",
            Self::SigTerm => "sigterm",
            Self::CtrlBreak => "ctrl_break",
            Self::CtrlClose => "ctrl_close",
        }
    }
}

/// Central OS shutdown detector (rules: single `shutdown_signal` entrypoint).
///
/// - **Unix:** first of Ctrl-C / `SIGINT` / `SIGTERM` via `tokio::select!`.
/// - **Windows:** first of Ctrl-C / Ctrl-Break / Ctrl-Close.
/// - **Other:** Ctrl-C only (`ctrl_c`).
///
/// Does not perform cleanup (async-signal-safe path: only await + return).
/// Callers cancel a `CancellationToken`(tokio_util::sync::CancellationToken) and run FINALIZE outside this future.
///
/// SIGHUP / SIGUSR* / Windows logoff+shutdown service events are intentionally
/// **not** captured: this is a one-shot console CLI (no hot-reload, no Windows
/// Service host). `ctrl_close` is captured so closing the console still runs
/// cooperative cancel → FINALIZE.
pub async fn shutdown_signal() -> ShutdownTrigger {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "failed to register SIGTERM; falling back to ctrl_c");
                let _ = tokio::signal::ctrl_c().await;
                return ShutdownTrigger::CtrlC;
            }
        };
        let mut sigint = match signal(SignalKind::interrupt()) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "failed to register SIGINT; falling back to ctrl_c");
                let _ = tokio::signal::ctrl_c().await;
                return ShutdownTrigger::CtrlC;
            }
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => ShutdownTrigger::CtrlC,
            _ = sigterm.recv() => ShutdownTrigger::SigTerm,
            _ = sigint.recv() => ShutdownTrigger::SigInt,
        }
    }
    #[cfg(windows)]
    {
        let mut break_stream = match tokio::signal::windows::ctrl_break() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "failed to register ctrl_break; falling back to ctrl_c");
                let _ = tokio::signal::ctrl_c().await;
                return ShutdownTrigger::CtrlC;
            }
        };
        let mut close_stream = match tokio::signal::windows::ctrl_close() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "failed to register ctrl_close; continuing without it");
                // Fall through with a stream that never fires: use pending.
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => return ShutdownTrigger::CtrlC,
                    _ = break_stream.recv() => return ShutdownTrigger::CtrlBreak,
                }
            }
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => ShutdownTrigger::CtrlC,
            _ = break_stream.recv() => ShutdownTrigger::CtrlBreak,
            _ = close_stream.recv() => ShutdownTrigger::CtrlClose,
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = tokio::signal::ctrl_c().await;
        ShutdownTrigger::CtrlC
    }
}

#[cfg(test)]
mod tests {
    use crate::browser::block_on_browser_timeout;
    use crate::browser::helpers::tree_to_at_refs;
    use crate::browser::session::{is_internal_browser_url, is_noise_network_url};
    use crate::error::ErrorKind;
    use crate::lifecycle::Lifecycle;
    use crate::native::browser::WaitUntil;
    use serde_json::json;

    #[test]
    fn pre_cancelled_token_returns_exit_130() {
        let lc = Lifecycle::new();
        lc.cancel.cancel();
        let err = block_on_browser_timeout(async { Ok::<(), _>(()) }, 5).expect_err("must cancel");
        assert_eq!(err.kind(), ErrorKind::Cancelled);
        assert_eq!(err.exit_code(), 130);
    }

    #[test]
    fn shutdown_trigger_labels_are_stable() {
        use super::ShutdownTrigger;
        assert_eq!(ShutdownTrigger::CtrlC.as_str(), "ctrl_c");
        assert_eq!(ShutdownTrigger::SigInt.as_str(), "sigint");
        assert_eq!(ShutdownTrigger::SigTerm.as_str(), "sigterm");
        assert_eq!(ShutdownTrigger::CtrlBreak.as_str(), "ctrl_break");
        assert_eq!(ShutdownTrigger::CtrlClose.as_str(), "ctrl_close");
    }

    #[test]
    fn zero_timeout_sleep_can_complete() {
        let _lc = Lifecycle::new();
        let r = block_on_browser_timeout(
            async {
                tokio::time::sleep(std::time::Duration::from_millis(
                    crate::xdg::policy::policy_u64(crate::xdg::policy::key::PLATFORM_CHILD_POLL_MS),
                ))
                .await;
                Ok::<u32, crate::error::CliError>(7)
            },
            0,
        );
        assert_eq!(r.unwrap(), 7);
    }

    #[test]
    fn hard_timeout_returns_exit_124() {
        let _lc = Lifecycle::new();
        let err = block_on_browser_timeout(
            async {
                tokio::time::sleep(std::time::Duration::from_secs(
                    crate::xdg::policy::policy_u64(
                        crate::xdg::policy::key::DEFAULT_SHUTDOWN_DEADLINE_SECS,
                    ),
                ))
                .await;
                Ok::<(), crate::error::CliError>(())
            },
            1,
        )
        .expect_err("must timeout");
        assert_eq!(err.kind(), ErrorKind::Timeout);
        assert_eq!(err.exit_code(), 124);
    }

    #[test]
    fn tree_to_at_refs_rewrites_markers() {
        let raw = r#"- link "Home" [ref=e1]
  - button "Go" [checked=false, ref=e2]
"#;
        let out = tree_to_at_refs(raw);
        assert!(out.contains("[@e1]"), "out={out}");
        assert!(out.contains("@e2"), "out={out}");
    }

    #[test]
    fn internal_browser_urls_filtered() {
        assert!(is_internal_browser_url("chrome://new-tab-page/"));
        assert!(is_internal_browser_url("chrome-extension://abc/x.js"));
        assert!(is_internal_browser_url("devtools://devtools/bundled/"));
        assert!(!is_internal_browser_url("https://example.com/"));
        assert!(!is_internal_browser_url(crate::constants::ABOUT_BLANK));
        assert!(is_noise_network_url(
            "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"
        ));
        assert!(is_noise_network_url("blob:https://example.com/uuid"));
        assert!(is_noise_network_url("chrome://new-tab-page/"));
        assert!(!is_noise_network_url("https://example.com/"));
    }

    #[test]
    fn wait_until_tokens_parse() {
        assert_eq!(
            WaitUntil::parse_token("networkidle"),
            WaitUntil::NetworkIdle
        );
        assert_eq!(
            WaitUntil::parse_token("domcontentloaded"),
            WaitUntil::DomContentLoaded
        );
        assert_eq!(WaitUntil::parse_token("load"), WaitUntil::Load);
        assert_eq!(WaitUntil::parse_token("none"), WaitUntil::None);
    }

    #[test]
    fn net_request_id_resolution_logic() {
        let requests = [
            json!({"requestId": "rid-1", "method": "GET", "url": "https://a.example/"}),
            json!({"requestId": "rid-2", "method": "POST", "url": "https://b.example/"}),
        ];
        let by_index = requests.get(1).unwrap();
        assert_eq!(by_index["requestId"], "rid-2");
        let by_rid = requests.iter().find(|r| r["requestId"] == "rid-1").unwrap();
        assert_eq!(by_rid["url"], "https://a.example/");
        // String id that is numeric index
        let idx: usize = "0".parse().unwrap();
        assert_eq!(requests[idx]["requestId"], "rid-1");
    }
}
