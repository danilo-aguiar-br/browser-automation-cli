// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local scrape/crawl/search/parse/sg/sheet handlers.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | formats | Shared multi-format payload derivation |
//! | page | Single-URL scrape (HTTP and browser engines) |
//! | batch | Batch scrape and crawl |
//! | discover | Site map and local search |
//! | files | Document parse, QR, spreadsheet write |
//! | local_fs | Path discovery and structural lint/rewrite |
//!
//! Handlers are re-exported flat so existing paths keep working.

mod batch;
mod discover;
mod files;
mod formats;
mod local_fs;
mod page;

pub(crate) use batch::*;
pub(crate) use discover::*;
pub(crate) use files::*;
pub(crate) use local_fs::*;
pub(crate) use page::*;
