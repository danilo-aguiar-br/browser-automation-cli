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

/// Default Chrome self-spawn startup wait (seconds).
///
/// Matches `chromiumoxide::browser::LAUNCH_TIMEOUT` (20_000 ms) so migrating the
/// Chrome path from `Browser::launch` to a self-spawn plus
/// `Browser::connect_with_config` cannot change how long a slow host is given.
/// Operator override: XDG `config set chrome_startup_timeout_secs`.
pub const CHROME_STARTUP_TIMEOUT_SECS: u64 = 20;

/// Minimum age before a dead-owner marker profile becomes collectable (seconds).
///
/// The floor is what keeps the window between `create_dir_all` and Chrome
/// appearing in the process table uncollectable, so a concurrent invocation is
/// never robbed of its in-flight profile. Operator override: XDG
/// `config set residual_orphan_min_age_secs`.
pub const RESIDUAL_ORPHAN_MIN_AGE_SECS: u64 = 60;

/// Platform child wait poll interval (milliseconds).
///
/// Default for `config set platform_child_poll_ms`. This is the interval that
/// actually runs during FINALIZE, while the process tree is reaped: a slow host
/// wants a longer slice, a fast one a shorter. Read it through
/// `policy_u64(key::PLATFORM_CHILD_POLL_MS)`, never as a bare constant.
pub const PLATFORM_CHILD_POLL_MS: u64 = 50;

/// Platform child wait deadline (seconds).
pub const PLATFORM_CHILD_WAIT_SECS: u64 = 5;

/// Shutdown hard deadline (seconds) waiting for browser exit.
pub const DEFAULT_SHUTDOWN_DEADLINE_SECS: u64 = 30;
