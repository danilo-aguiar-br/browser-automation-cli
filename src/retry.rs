// SPDX-License-Identifier: MIT OR Apache-2.0
//! Named retry policy with budget, backoff, and jitter (rules_rust_retry_com_backoff / GAP-013).

use std::time::Duration;

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
            max_attempts: 3,
            base_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(2),
            budget: Duration::from_secs(10),
        }
    }
}

impl RetryConfig {
    /// Conservative policy for CDP attach / discovery.
    pub fn cdp() -> Self {
        Self {
            max_attempts: 4,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(3),
            budget: Duration::from_secs(15),
        }
    }

    /// Policy for outbound HTTP scrape.
    pub fn http() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(75),
            max_delay: Duration::from_secs(2),
            budget: Duration::from_secs(12),
        }
    }

    /// Policy for optional operator LLM HTTP.
    pub fn llm() -> Self {
        Self {
            max_attempts: 2,
            base_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(4),
            budget: Duration::from_secs(20),
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
pub fn retry_blocking<T, E, F>(cfg: RetryConfig, mut f: F) -> Result<T, E>
where
    E: std::fmt::Display,
    F: FnMut() -> Result<T, E>,
{
    let start = std::time::Instant::now();
    let mut last_err = None;
    for attempt in 0..cfg.max_attempts {
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

/// Async retry with the same classification rules (CDP discovery / attach).
pub async fn retry_async<T, E, F, Fut>(cfg: RetryConfig, mut f: F) -> Result<T, E>
where
    E: std::fmt::Display,
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let start = std::time::Instant::now();
    let mut last_err = None;
    for attempt in 0..cfg.max_attempts {
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
