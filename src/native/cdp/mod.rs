// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chrome DevTools Protocol transport: launch, discovery, client and wire types.

pub mod chrome;
pub mod client;
/// Finding a usable CDP endpoint before a client can exist.
pub mod discovery;
pub mod lightpanda;
pub mod oxide;
pub mod spawn;
pub mod types;
pub mod xvfb;
