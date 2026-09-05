// SPDX-License-Identifier: MIT OR Apache-2.0
//! Universal data operations applied to the success envelope (agent CLEAN STDOUT).
//!
//! # Why this lives at the envelope and not in each command
//!
//! The product law is that the binary does the heavy work and the model receives
//! the minimum useful payload. Measured before this module existed:
//! `doctor --offline --quick` emitted 26_277 bytes with no way to narrow it, and
//! the product's own COOKBOOK told agents to pipe it through `jaq`. An agent that
//! needs a JSON processor in its prompt is an agent the CLI failed.
//!
//! Only 8 of 69 commands offered any of these operations, and they disagreed:
//! `crawl` grew eight of them locally, `scrape` grew one, and `doctor` — the most
//! invoked diagnostic — grew none. Implementing them once over `data` covers
//! every command, including the ones nobody thought to wire.
//!
//! # Order of operations
//!
//! `select` → resolve rows → `filter` → `sort` → `dedupe-by` → `limit` →
//! `truncate-content` → `count-only` → `max-output-bytes`.
//!
//! `select` runs FIRST on purpose. It is also the disambiguator: a `data` object
//! holding two arrays has no single obvious list to filter, and narrowing with
//! `--fields checks` leaves exactly one.
//!
//! # Module map
//!
//! This file is a facade: it declares the parts and re-exports the surface the
//! rest of the crate already imports as `crate::agent_ops::…`, so the split is
//! invisible to every caller.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `types` | `AgentOps`, `AgentOpsReport`, `UnresolvedPath` — what was asked, what was done |
//! | `pipeline` | the fixed order above, plus the process-wide install |
//! | `path` | dotted-path projection |
//! | `filter` | predicates, sorting, deduplication |
//! | `rows` | locating the one row list inside `data` |
//! | `budget` | character and byte ceilings |

pub mod budget;
pub mod filter;
pub mod path;
mod pipeline;
mod rows;
mod types;

pub use pipeline::{apply, apply_process_ops, expectation_unmet, set_agent_ops};
pub use types::{AgentOps, AgentOpsReport, UnresolvedPath};

#[cfg(test)]
mod tests;
