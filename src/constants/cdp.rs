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

/// Hosts that must never be routed through `--proxy-server`.
///
/// # Why an egress proxy breaks the control channel
///
/// The CLI talks to Chrome over a WebSocket on loopback. `--proxy-server`
/// applies to every request Chrome makes, and Chrome does not carve out its
/// own debugging endpoint — so the proxy swallows the control channel and the
/// browser never becomes reachable.
///
/// Measured before this constant existed: `--proxy http://127.0.0.1:1 goto`
/// failed with "Timed out after 20000ms waiting for Chrome CDP endpoint". The
/// message blamed Chrome for a failure the proxy caused, which sent the caller
/// looking in the wrong place. With loopback bypassed the same run fails with
/// `net::ERR_PROXY_CONNECTION_FAILED` — the true cause, on the first try.
///
/// The literal hosts are listed instead of Chrome's `<-loopback>` wildcard:
/// the wildcard's exact membership is a Chrome implementation detail, and this
/// list has to be predictable enough to assert in a test.
pub const CDP_PROXY_LOOPBACK_BYPASS: &str = "127.0.0.1,localhost,[::1]";

/// Default CDP HTTP discovery timeout (seconds) for `/json/version` probes.
pub const DEFAULT_CDP_DISCOVERY_TIMEOUT_SECS: u64 = 2;

/// CDP connection liveness probe timeout (`Browser.getVersion`) in seconds.
pub const CDP_CONNECTION_PROBE_TIMEOUT_SECS: u64 = 3;

/// In-memory console/error tracker ring size (per page session).
pub const EVENT_TRACKER_MAX_ENTRIES: usize = 1000;

/// Navigation boundaries kept for `--include-preserved`.
///
/// Distinct from [`EVENT_TRACKER_MAX_ENTRIES`], which caps how many entries one
/// ring holds; this caps how many RINGS are kept. Reusing that key here would
/// have tied two independent quantities to one number, so a caller widening
/// history would also widen every buffer.
///
/// The value stood as a bare `3` in four places inside a single function, where
/// it governed memory without a name to search for or a key to change.
pub const CAPTURE_PRESERVED_RINGS: usize = 3;

/// Extension attach poll slice (milliseconds).
pub const EXTENSION_ATTACH_POLL_MS: u64 = 150;

/// Polls spent waiting for an extension service worker to register.
///
/// Paired with [`EXTENSION_ATTACH_POLL_MS`]: the two together are the wait
/// budget. Configuring only the interval, as the product did until 0.1.9, moves
/// the cadence while leaving the total pinned, so an operator who doubled the
/// interval doubled the wait without being able to say so.
pub const EXTENSION_ATTACH_POLL_ITERS: u32 = 20;

/// CDP `Network.ResourceType` vocabulary, in the protocol's own spelling.
///
/// This is the set `Network.requestWillBeSent` draws its `type` from, and the
/// set `net list --resource-types` validates against. It is protocol grammar
/// rather than a configurable default, so it carries no XDG key: an operator
/// able to widen it could only widen it into values Chrome never sends.
///
/// Stored in protocol casing so a captured record holds what the CDP sent.
/// Comparison lowercases; storage never does.
pub const CDP_RESOURCE_TYPES: [&str; 19] = [
    "Document",
    "Stylesheet",
    "Image",
    "Media",
    "Font",
    "Script",
    "TextTrack",
    "XHR",
    "Fetch",
    "Prefetch",
    "EventSource",
    "WebSocket",
    "Manifest",
    "SignedExchange",
    "Ping",
    "CSPViolationReport",
    "Preflight",
    "FedCM",
    "Other",
];

/// Resource type recorded when `Network.requestWillBeSent` omits `type`.
///
/// The protocol types that field `Option`, so absence is ordinary rather than
/// exceptional. `Other` is a real member of [`CDP_RESOURCE_TYPES`], which keeps
/// every captured record filterable. Omitting the key instead would let any
/// filter drop the record with no signal, and that silent drop is the defect
/// this constant exists to prevent.
pub const DEFAULT_RESOURCE_TYPE: &str = "Other";
