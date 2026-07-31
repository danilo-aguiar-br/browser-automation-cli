// SPDX-License-Identifier: MIT OR Apache-2.0
//! MITM proxy clamps, capture windows, CA cache, and redaction placeholder.

use super::identity::LOOPBACK_HOST;

/// MITM list/query max items clamp.
pub const MITM_LIST_LIMIT_MAX: usize = 10_000;
/// MITM proxy oneshot max window (seconds).
pub const MITM_PROXY_SECONDS_MAX: u64 = 600;
/// MITM Chrome launch settle before navigation (milliseconds).
pub const MITM_CHROME_SETTLE_MS: u64 = 150;
/// MITM capture wait min (milliseconds) after navigate.
pub const MITM_CAPTURE_WAIT_MIN_MS: u64 = 800;
/// MITM capture wait max (milliseconds) after navigate.
pub const MITM_CAPTURE_WAIT_MAX_MS: u64 = 8_000;
/// Cap on in-memory WebSocket frames per capture process.
pub const MITM_WS_FRAMES_CAP: usize = 500;
/// Truncate WS text preview to this many Unicode chars (agent-facing).
pub const MITM_WS_PREVIEW_CHARS: usize = 256;
/// hudsucker `RcgenAuthority` dynamic cert cache size (hosts).
pub const MITM_CA_CACHE_SIZE: u64 = 1_000;
/// Placeholder substituted for sensitive header values.
pub const MITM_REDACTED_PLACEHOLDER: &str = "[REDACTED]";

/// Loopback-only bind host for MITM (same value as [`LOOPBACK_HOST`]).
pub const MITM_BIND_HOST: &str = LOOPBACK_HOST;
