// SPDX-License-Identifier: MIT OR Apache-2.0
//! CDP client over chromiumoxide (single connection — no dual WebSocket).
//!
//! Chrome one-shot: `Browser::launch` only.
//! Lightpanda / attach path: `Browser::connect` only.
//! FORBIDDEN: second `tokio-tungstenite` attach to the same browser.
//!
//! # Workload
//!
//! **I/O-bound** CDP WebSocket. Multi-page listener attach fans out with
//! [`crate::concurrency::join_bounded`] after releasing `browser.lock`.
//!
//! # Module map (Pass G SRP)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `raw` | dynamic CDP command for execute |
//! | `types` | [`CdpClient`] ownership shell |
//! | `connect` | connect / from_browser |
//! | `send` | send_command* helpers |
//! | `forwarders` | browser-level event forwarders |
//! | `page_attach` | page-scoped console/network/session attach |

mod connect;
mod forwarders;
mod page_attach;
mod raw;
mod send;
mod types;

#[cfg(test)]
mod tests;

pub use types::CdpClient;
