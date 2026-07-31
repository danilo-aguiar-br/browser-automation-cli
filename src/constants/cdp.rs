// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP discovery, event pump, and attach infra constants.

/// Max CDP discovery HTTP body bytes (`/json/version`, `/json/list`).
pub const CDP_DISCOVERY_MAX_BODY_BYTES: usize = 1024 * 1024;

/// Capacity of the process-local CDP event `broadcast` channel.
///
/// Sized for short-lived one-shot sessions (not a long-running daemon ring).
/// Lagged receivers drop oldest (tokio broadcast semantics).
pub const CDP_EVENT_BROADCAST_CAPACITY: usize = 4096;

/// CDP event drain poll slice (milliseconds) during navigation wait.
pub const CDP_EVENT_DRAIN_POLL_MS: u64 = 100;

/// CDP network-idle settle window (milliseconds).
pub const CDP_NETWORK_IDLE_SETTLE_MS: u64 = 500;

/// CDP target event short wait (milliseconds).
pub const CDP_TARGET_EVENT_WAIT_MS: u64 = 600;

/// Default CDP HTTP discovery timeout (seconds) for `/json/version` probes.
pub const DEFAULT_CDP_DISCOVERY_TIMEOUT_SECS: u64 = 2;

/// CDP connection liveness probe timeout (`Browser.getVersion`) in seconds.
pub const CDP_CONNECTION_PROBE_TIMEOUT_SECS: u64 = 3;

/// In-memory console/error tracker ring size (per page session).
pub const EVENT_TRACKER_MAX_ENTRIES: usize = 1000;

/// Extension attach poll slice (milliseconds).
pub const EXTENSION_ATTACH_POLL_MS: u64 = 150;
