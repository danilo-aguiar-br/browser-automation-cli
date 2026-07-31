// SPDX-License-Identifier: MIT OR Apache-2.0
//! Machine-readable command list and JSON Schema fragments for agents.
//!
//! ## Module map (componentization)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`inventory`] | `COMMANDS` / parity / DevTools map |
//! | [`schema`] | per-command JSON Schema fragments |
//! | [`handlers`] | `list_commands` / `schema_for_cmd` |

mod handlers;
mod inventory;
mod schema;

#[cfg(test)]
mod tests;

pub use handlers::{list_commands, schema_for_cmd};
// Public inventory for agents/skills; may not be referenced inside this crate.
