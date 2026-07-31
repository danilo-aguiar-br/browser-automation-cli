// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local diagnostics for one-shot installs (no multi-process daemon).
//!
//! # Workload
//!
//! **I/O-light sequential (N-144 / PAR-57 / PAR-73 honesty):** each check is an
//! independent path/binary probe assembled in stable report order. Probes are
//! cheap (stat/which); Rayon would rarely beat sequential assembly and the
//! matrix **must not** claim `map_cpu` for doctor. Concurrency budget is still
//! exported for agents (`budget_report` / `by_command.doctor`).

use serde_json::json;

use crate::envelope::print_success_json;
use crate::install;
use crate::native::cdp::chrome;

/// Options for local install diagnostics.
#[derive(Default, Clone, Copy)]
pub struct DoctorOptions {
    /// Skip network checks.
    pub offline: bool,
    /// Run a reduced check set.
    pub quick: bool,
    /// Attempt automatic remediations when supported.
    pub fix: bool,
    /// Emit JSON envelope on stdout.
    pub json: bool,
}

/// Run doctor checks and return a process exit code (`0` = all pass).
mod probes;
mod run;

pub use run::run_doctor;
