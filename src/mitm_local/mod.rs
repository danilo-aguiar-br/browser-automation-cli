// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot local MITM capture helpers (PRD §5E).
//!
//! This module:
//! - Generates/loads a local CA under XDG data (`mitm/ca`)
//! - Stores invocation captures under XDG state (`mitm/`)
//! - Exports HAR JSON without Python mitmproxy
//!
//! Full TLS intercept proxy (hudsucker) can attach to the same capture store.
//! CDP Network remains complementary and can feed the same HAR exporter.
//!
//! # Workload
//!
//! **Mista:** proxy accept loop is one awaited JoinHandle (not multi-URL fan-out).
//! Domain/API classification over large captures uses [`crate::concurrency::map_cpu`]
//! (PAR-56). Start/capture is sequential one-shot by design.
//!
//! **PAR-91:** CA PEM load in async oneshot paths uses
//! [`crate::concurrency::read_to_string_blocking`] via `ca::load_ca_pems_blocking`
//! (never `std::fs::read_to_string` on a Tokio worker).
//!
//! ## Module map (componentization)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `types` | exchange / WS frame / `MitmCapture` store |
//! | `util` | redact, atomic write, lock, clocks |
//! | `ca` | XDG CA ensure + blocking PEM load |
//! | `store` | status/list/get/import/ws/rules |
//! | `har` | HAR 1.2 export |
//! | `analyze` | domains + API classification |
//! | `handler` | single DRY hudsucker CaptureHandler |
//! | `proxy` | oneshot start + capture_url windows |

mod analyze;
mod body;
mod ca;
mod handler;
mod har;
pub mod policy;
mod proxy;
mod redact;
mod store;
mod types;
mod util;

#[cfg(test)]
mod tests;

pub use analyze::{apis, domains};
pub use ca::ensure_ca;
pub use har::export_har;
pub use proxy::{capture_url_oneshot, start_proxy_oneshot};
pub use store::{
    allow_host, block_rule, default_capture_path, get, graphql, import_cdp_network, list,
    redact_policy, resolve_capture_path, status, ws_get, ws_list,
};
pub use types::{BTreeMapString, CapturedExchange, CapturedWsFrame, MitmCapture};
pub use util::{shared_capture, SharedCapture};
