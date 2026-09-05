// SPDX-License-Identifier: MIT OR Apache-2.0
//! Named retry policy with budget, backoff, and jitter (rules_rust_retry_com_backoff / GAP-013).

use std::time::Duration;

use crate::error::CliError;

/// Explicit retry configuration for transient network/CDP failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryConfig {
    /// Maximum attempts including the first try.
    pub max_attempts: u32,
    /// Base delay before the first retry.
    pub base_delay: Duration,
    /// Cap on exponential backoff delay.
    pub max_delay: Duration,
    /// Total wall-clock budget for all retries.
    pub budget: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: crate::xdg::policy::policy_u32(
                crate::xdg::policy::key::RETRY_DEFAULT_MAX_ATTEMPTS,
            ),
            base_delay: Duration::from_millis(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::RETRY_BASE_DELAY_MS,
            )),
            max_delay: Duration::from_secs(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::RETRY_MAX_DELAY_SECS,
            )),
            budget: Duration::from_secs(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::RETRY_BUDGET_SECS,
            )),
        }
    }
}

impl RetryConfig {
    /// Conservative policy for CDP attach / discovery.
    pub fn cdp() -> Self {
        Self {
            max_attempts: crate::xdg::policy::policy_u32(
                crate::xdg::policy::key::RETRY_CDP_MAX_ATTEMPTS,
            ),
            base_delay: Duration::from_millis(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::RETRY_CDP_BASE_DELAY_MS,
            )),
            max_delay: Duration::from_secs(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::RETRY_CDP_MAX_DELAY_SECS,
            )),
            budget: Duration::from_secs(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::RETRY_CDP_BUDGET_SECS,
            )),
        }
    }

    /// Policy for outbound HTTP scrape.
    pub fn http() -> Self {
        Self {
            max_attempts: crate::xdg::policy::policy_u32(
                crate::xdg::policy::key::RETRY_HTTP_MAX_ATTEMPTS,
            ),
            base_delay: Duration::from_millis(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::RETRY_HTTP_BASE_DELAY_MS,
            )),
            max_delay: Duration::from_secs(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::RETRY_HTTP_MAX_DELAY_SECS,
            )),
            budget: Duration::from_secs(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::RETRY_HTTP_BUDGET_SECS,
            )),
        }
    }

    /// Policy for optional operator LLM HTTP.
    pub fn llm() -> Self {
        Self {
            max_attempts: crate::xdg::policy::policy_u32(
                crate::xdg::policy::key::RETRY_LLM_MAX_ATTEMPTS,
            ),
            base_delay: Duration::from_millis(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::RETRY_LLM_BASE_DELAY_MS,
            )),
            max_delay: Duration::from_secs(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::RETRY_LLM_MAX_DELAY_SECS,
            )),
            budget: Duration::from_secs(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::RETRY_LLM_BUDGET_SECS,
            )),
        }
    }

    /// Compute sleep duration for attempt index `0..` with full jitter.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exp = self.base_delay.saturating_mul(1u32 << attempt.min(8));
        let capped = exp.min(self.max_delay);
        let millis = capped.as_millis() as u64;
        if millis == 0 {
            return Duration::ZERO;
        }
        let mut buf = [0u8; 8];
        let _ = getrandom::getrandom(&mut buf);
        let r = u64::from_le_bytes(buf) % (millis + 1);
        Duration::from_millis(r)
    }
}

/// Classify whether an error message looks transient (retryable).
pub fn is_retryable_message(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    m.contains("timeout")
        || m.contains("timed out")
        || m.contains("connection reset")
        || m.contains("connection refused")
        || m.contains("temporarily")
        || m.contains("try again")
        || m.contains("broken pipe")
        || m.contains("websocket")
        || m.contains("eof")
        || m.contains("503")
        || m.contains("502")
        || m.contains("429")
}

/// Run a fallible operation with the given retry policy (blocking).
///
/// # Panics
///
/// Never, and the `expect` at the tail is what makes that worth stating.
/// `max_attempts` is clamped to at least one below, so the loop always runs the
/// operation once and `last_err` is `Some` on every path that reaches the tail.
/// Without the clamp, `max_attempts = 0` skipped the loop entirely and panicked
/// with exit 101, outside the sysexits table the product promises.
///
/// Zero is reachable because [`RetryConfig`] is public and so is its
/// `max_attempts` field: any caller can write the struct literal, and the test
/// at the bottom of this file does exactly that. It does NOT arrive from the
/// XDG key `retry_default_max_attempts`. That key is declared in
/// `policy_knobs!` and read through `policy_u32` → `policy_u64`, which drops a
/// stored zero (`.filter(|&n| n > 0)`) and falls back to the named default —
/// measured on 2026-08-25 at `src/xdg/policy/access.rs`. The clamp defends the
/// public API surface, not the config surface, and a doc comment that claimed
/// otherwise passed four green gates because none of them reads prose.
///
/// The signature offers no other way out: `E` is opaque, so there is no error
/// value to return when the caller asked for zero attempts.
pub fn retry_blocking<T, E, F>(cfg: RetryConfig, mut f: F) -> Result<T, E>
where
    E: std::fmt::Display,
    F: FnMut() -> Result<T, E>,
{
    let start = std::time::Instant::now();
    let mut last_err = None;
    for attempt in 0..cfg.max_attempts.max(1) {
        if start.elapsed() > cfg.budget {
            break;
        }
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => {
                let retryable = is_retryable_message(&e.to_string());
                last_err = Some(e);
                if !retryable || attempt + 1 >= cfg.max_attempts {
                    break;
                }
                std::thread::sleep(cfg.delay_for_attempt(attempt));
            }
        }
    }
    Err(last_err.expect("retry_blocking: at least one attempt"))
}

/// Outcome of one attempt inside [`retry_http_async`].
///
/// A plain `Result` cannot express the middle case: an HTTP download has errors
/// that must end the run immediately (an SSRF-unsafe redirect, a body over the
/// ceiling) sitting next to errors that are worth another round trip. Folding
/// both into `Err` is what made the three media downloaders each grow their own
/// copy of the same loop.
pub enum Attempt<T> {
    /// The attempt produced the value; stop and return it.
    Done(T),
    /// The attempt failed in a way no retry can fix; surface this error as is.
    Fatal(CliError),
    /// The attempt failed transiently; retry when the budget and
    /// [`is_retryable_message`] allow, and surface this error when they do not.
    Failed(CliError),
}

/// Run one HTTP-shaped operation under `cfg`, retrying only what looks transient.
///
/// Shared by the image, video and audio downloaders, which ran three copies of
/// this loop with the same limits, the same backoff and the same classification.
/// The attempt counter starts at one, so `max_attempts` counts the first try —
/// the behaviour the three copies had.
///
/// # Errors
///
/// The [`Attempt::Fatal`] error unchanged, or the last [`Attempt::Failed`] error
/// once the attempt budget is spent or the message is not classified retryable.
pub async fn retry_http_async<T, F, Fut>(cfg: RetryConfig, mut f: F) -> Result<T, CliError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Attempt<T>>,
{
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match f().await {
            Attempt::Done(v) => return Ok(v),
            Attempt::Fatal(e) => return Err(e),
            Attempt::Failed(e) => {
                if attempt >= cfg.max_attempts || !is_retryable_message(e.message()) {
                    return Err(e);
                }
            }
        }
        tokio::time::sleep(cfg.delay_for_attempt(attempt.saturating_sub(1))).await;
    }
}

/// Async retry with the same classification rules (CDP discovery / attach).
///
/// # Panics
///
/// Never, for the same reason as [`retry_blocking`]: `max_attempts` is clamped
/// to at least one, so the operation always runs once and the `expect` at the
/// tail is unreachable.
pub async fn retry_async<T, E, F, Fut>(cfg: RetryConfig, mut f: F) -> Result<T, E>
where
    E: std::fmt::Display,
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let start = std::time::Instant::now();
    let mut last_err = None;
    for attempt in 0..cfg.max_attempts.max(1) {
        if start.elapsed() > cfg.budget {
            break;
        }
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                let retryable = is_retryable_message(&e.to_string());
                last_err = Some(e);
                if !retryable || attempt + 1 >= cfg.max_attempts {
                    break;
                }
                tokio::time::sleep(cfg.delay_for_attempt(attempt)).await;
            }
        }
    }
    Err(last_err.expect("retry_async: at least one attempt"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A zero-attempt policy runs once and returns, instead of panicking.
    ///
    /// # Why this can regress
    ///
    /// Zero reaches this function through the public API, not through config:
    /// `RetryConfig` is public and so is `max_attempts`, so a zero policy is one
    /// struct literal away, and the fixture below is that literal. With a bare
    /// `0..cfg.max_attempts` the loop never ran, `last_err` stayed `None`, and
    /// the `expect` at the tail turned a caller's zero into exit 101 — outside
    /// the sysexits table. Deleting the `.max(1)` clamp must fail here rather
    /// than in a user's terminal.
    ///
    /// The XDG key `retry_default_max_attempts` is NOT that path: `policy_u64`
    /// filters a stored zero out before `policy_u32` ever sees it. Asserting
    /// against the config surface here would test the wrong layer and pass for
    /// the wrong reason.
    #[test]
    fn a_zero_attempt_policy_still_runs_the_operation_once() {
        let calls = AtomicU32::new(0);
        let cfg = RetryConfig {
            max_attempts: 0,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1),
            budget: Duration::from_secs(1),
        };
        let r: Result<u32, &str> = retry_blocking(cfg, || {
            calls.fetch_add(1, Ordering::SeqCst);
            Err("connection reset by peer")
        });
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "the operation must run exactly once, never zero times"
        );
        assert_eq!(
            r,
            Err("connection reset by peer"),
            "the caller must receive the error, not a panic"
        );
    }

    #[test]
    fn succeeds_after_transient_failures() {
        let n = AtomicU32::new(0);
        let cfg = RetryConfig {
            max_attempts: 5,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(5),
            budget: Duration::from_secs(2),
        };
        let r = retry_blocking(cfg, || {
            let c = n.fetch_add(1, Ordering::SeqCst);
            if c < 2 {
                Err("connection reset by peer")
            } else {
                Ok(42)
            }
        });
        assert_eq!(r.unwrap(), 42);
        assert_eq!(n.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn permanent_errors_do_not_retry() {
        let n = AtomicU32::new(0);
        let cfg = RetryConfig {
            max_attempts: 5,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(5),
            budget: Duration::from_secs(2),
        };
        let r: Result<(), &str> = retry_blocking(cfg, || {
            n.fetch_add(1, Ordering::SeqCst);
            Err("invalid argument permanent")
        });
        assert!(r.is_err());
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn classifies_retryable() {
        assert!(is_retryable_message("HTTP 503"));
        assert!(!is_retryable_message("parse error in robots"));
    }
}
