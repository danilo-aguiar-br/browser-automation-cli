// SPDX-License-Identifier: MIT OR Apache-2.0
//! Command workload matrix (PAR honesty) and concurrency budget report.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`rows`] | Static command posture table (data) |
//! | `report` | JSON builders + [`resolve_permits`] |

mod report;
mod rows;

pub use report::{budget_report, command_workload_matrix, resolve_permits};
