// SPDX-License-Identifier: MIT OR Apache-2.0
//! One-shot workflow journal (PRD §5H): DAG + SQLite, no live Page/@eN across processes.
//!
//! # Workload / parallelism
//!
//! - **DAG validate:** CPU-light (petgraph); sequential is fine.
//! - **Step execution:** sequential topo order with fail-fast.
//! - Parallelism lives inside each step.
//!
//! # Module map (Pass G SRP)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `types` | manifest/step structs |
//! | `journal` | XDG journal path + SQLite open |
//! | `dag` | load manifest + validate topo |
//! | `run` | run / resume / status |
//! | `offline` | offline step executor |

mod dag;
mod journal;
mod offline;
mod run;
mod types;

#[cfg(test)]
mod tests;

pub use dag::{load_manifest, validate_dag};
pub use journal::journal_path;
pub use run::{workflow_resume, workflow_run, workflow_status};
pub use types::{WorkflowManifest, WorkflowStep};
