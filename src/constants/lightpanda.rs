// SPDX-License-Identifier: MIT OR Apache-2.0
//! Lightpanda engine startup, connect, and log-ring constants.

/// Default Lightpanda process startup wait (seconds).
///
/// Operator override: XDG `config set lightpanda_startup_timeout_secs`.
pub const LIGHTPANDA_STARTUP_TIMEOUT_SECS: u64 = 10;

/// Lightpanda CDP readiness poll interval (milliseconds).
pub const LIGHTPANDA_POLL_INTERVAL_MS: u64 = 100;

/// Per-probe CDP discovery timeout while waiting for Lightpanda (milliseconds).
pub const LIGHTPANDA_DISCOVERY_TIMEOUT_MS: u64 = 500;

/// Default Lightpanda `--timeout` session max (seconds).
///
/// Documented Lightpanda maximum (1 week). Operator override:
/// XDG `config set lightpanda_session_timeout_secs` (clamped 1..=this).
pub const LIGHTPANDA_SESSION_TIMEOUT_SECS: u64 = 604_800;

/// Bounded Lightpanda launch log ring (lines per stream).
pub const LIGHTPANDA_MAX_LOG_LINES: usize = 40;

/// Brief drain slice after Lightpanda child exit before snapshotting logs (ms).
pub const LIGHTPANDA_READY_SLICE_MS: u64 = 25;

/// Lightpanda CDP connect attempt timeout (seconds).
pub const LIGHTPANDA_CDP_CONNECT_TIMEOUT_SECS: u64 = 5;

/// Lightpanda target init wait after connect (seconds).
pub const LIGHTPANDA_TARGET_INIT_TIMEOUT_SECS: u64 = 10;
