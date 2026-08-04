// SPDX-License-Identifier: MIT OR Apache-2.0
//! `config set|get|list-keys` operators (single key catalog).
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | keys | Key catalog and `list-keys` payload |
//! | validate | Value parsers, range checks, secret redaction |
//! | set | `config set` mutation and persist |
//! | get | `config get` single key and full dump |

mod get;
mod keys;
mod set;
mod set_media;
mod set_scrape;
mod validate;

pub use get::config_get;
pub use keys::{all_config_keys, config_keys_description, config_list_keys, CONFIG_KEYS};
pub use set::config_set;
