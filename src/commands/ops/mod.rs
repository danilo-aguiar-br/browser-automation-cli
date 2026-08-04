// SPDX-License-Identifier: MIT OR Apache-2.0

//! PRD ops handlers (MITM, workflow, config, perf, heap, extensions, …).
//!
//! Split by responsibility (SRP); re-exported for `commands::ops::`.

mod completions;
mod config;
mod devtools3p;
mod emulate;
mod extension;
mod heap;
mod lighthouse;
mod mitm;
mod perf;
mod record;
mod screencast;
mod webmcp;
mod workflow;

pub(crate) use completions::*;
pub(crate) use config::*;
pub(crate) use devtools3p::*;
pub(crate) use emulate::*;
pub(crate) use extension::*;
pub(crate) use heap::*;
pub(crate) use lighthouse::*;
pub(crate) use mitm::*;
pub(crate) use perf::*;
pub(crate) use record::*;
pub(crate) use screencast::*;
pub(crate) use webmcp::*;
pub(crate) use workflow::*;
