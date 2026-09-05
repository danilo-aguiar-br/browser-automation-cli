// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared batch plumbing for the local media families (`image`, `video`, `audio`).
//!
//! The three handlers each resolve a source from `--path` / `--stdin` and run
//! one operation. `--paths-file` turns that single source into a list without
//! duplicating the resolution rules or the per-item error policy three times.

mod batch;
mod produce;

pub(crate) use batch::{resolve, run, MediaInputs};
pub(crate) use produce::{input_ext, run_producing};
