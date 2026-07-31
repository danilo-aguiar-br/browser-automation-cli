// SPDX-License-Identifier: MIT OR Apache-2.0
//! BORN/FINALIZE/DIE process reap budgets (infra, not operator `--timeout`).

/// Browser.close / process wait budget during FINALIZE (seconds).
///
/// Primary reap deadline for chromiumoxide `wait()` and Lightpanda
/// `wait_or_kill` before escalating to kill. Infra constant (not operator
/// `--timeout`); residual Unix SIGTERM grace is [`FINALIZE_CHILD_GRACE_SECS`].
pub const BROWSER_CLOSE_WAIT_SECS: u64 = 5;

/// Residual Unix SIGTERM→SIGKILL grace during FINALIZE (seconds).
///
/// Infra budget for one-shot DIE (not operator `--timeout`). Exposed as
/// [`crate::lifecycle::FINALIZE_CHILD_GRACE`] (`Duration`).
pub const FINALIZE_CHILD_GRACE_SECS: u64 = 2;

/// Platform child wait poll interval (milliseconds).
pub const PLATFORM_CHILD_POLL_MS: u64 = 50;

/// Platform child wait deadline (seconds).
pub const PLATFORM_CHILD_WAIT_SECS: u64 = 5;

/// Shutdown cooperative poll interval (milliseconds).
pub const DEFAULT_SHUTDOWN_POLL_MS: u64 = 5;

/// Shutdown hard deadline (seconds) waiting for browser exit.
pub const DEFAULT_SHUTDOWN_DEADLINE_SECS: u64 = 30;
