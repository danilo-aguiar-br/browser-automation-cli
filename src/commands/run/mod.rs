// SPDX-License-Identifier: MIT OR Apache-2.0
//! `run --script` — multi-step NDJSON or JSON array, one launch, fail-fast (layers A+B).
//!
//! # Workload
//!
//! **Sequential by design (N-134):** script steps run in order with fail-fast.
//! Internal steps may call parallel callees (scrape/http, filters) under the
//! process concurrency budget. Never parallelize the step loop itself.
//!
//! # Module map (Tier-4 SRP)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `inventory` | RUN_DISPATCHED_CMDS, INTENTIONAL_RUN_EXCLUDE |
//! | `parse` | parse_run_script / normalize |
//! | `preflight` | whole-script validation before BORN (GAP-012/029) |
//! | `flags` | RunFlags |
//! | `engine` | run_script_with_flags, run_one_step |
//! | `argv` | argv_to_step + tests |
//! | `execute` | per-cmd step handlers |

mod argv;
mod engine;
mod execute;
mod flags;
mod inventory;
mod parse;
mod preflight;
mod stream;

pub use argv::argv_to_step;
pub use engine::{run_one_step, run_script_with_flags};
pub(crate) use execute::is_dispatchable_cmd;
pub use flags::RunFlags;
pub use inventory::{run_supported_suggestion, INTENTIONAL_RUN_EXCLUDE, RUN_DISPATCHED_CMDS};
pub use parse::parse_run_script;
pub use preflight::preflight_script;
pub use stream::{is_stdin_marker, run_stream_with_flags};
