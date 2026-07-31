// SPDX-License-Identifier: MIT OR Apache-2.0
//! Retry policy budgets per transport family (default, CDP, HTTP, LLM).

/// Retry policy: default max attempts (inclusive of first try).
pub const RETRY_DEFAULT_MAX_ATTEMPTS: u32 = 3;
/// Retry policy: default base delay (milliseconds).
pub const RETRY_BASE_DELAY_MS: u64 = 50;
/// Retry policy: default max delay (seconds).
pub const RETRY_MAX_DELAY_SECS: u64 = 2;
/// Retry policy: default budget (seconds).
pub const RETRY_BUDGET_SECS: u64 = 10;

/// Retry policy (CDP attach/discovery): max attempts.
pub const RETRY_CDP_MAX_ATTEMPTS: u32 = 4;
/// Retry policy (CDP): base delay (milliseconds).
pub const RETRY_CDP_BASE_DELAY_MS: u64 = 100;
/// Retry policy (CDP): max delay (seconds).
pub const RETRY_CDP_MAX_DELAY_SECS: u64 = 3;
/// Retry policy (CDP): wall budget (seconds).
pub const RETRY_CDP_BUDGET_SECS: u64 = 15;

/// Retry policy (HTTP scrape): max attempts.
pub const RETRY_HTTP_MAX_ATTEMPTS: u32 = 3;
/// Retry policy (HTTP scrape): base delay (milliseconds).
pub const RETRY_HTTP_BASE_DELAY_MS: u64 = 75;
/// Retry policy (HTTP scrape): max delay (seconds).
pub const RETRY_HTTP_MAX_DELAY_SECS: u64 = 2;
/// Retry policy (HTTP scrape): wall budget (seconds).
pub const RETRY_HTTP_BUDGET_SECS: u64 = 12;

/// Retry policy (operator LLM HTTP): max attempts.
pub const RETRY_LLM_MAX_ATTEMPTS: u32 = 2;
/// Retry policy (LLM): base delay (milliseconds).
pub const RETRY_LLM_BASE_DELAY_MS: u64 = 200;
/// Retry policy (LLM): max delay (seconds).
pub const RETRY_LLM_MAX_DELAY_SECS: u64 = 4;
/// Retry policy (LLM): wall budget (seconds).
pub const RETRY_LLM_BUDGET_SECS: u64 = 20;
