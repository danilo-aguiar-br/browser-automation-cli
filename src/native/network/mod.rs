// SPDX-License-Identifier: MIT OR Apache-2.0
//! Network interception, request logs, and console error tracking.
//!
//! # Workload
//!
//! **I/O-bound** CDP. Multi-page `about:blank` sanitize fans out with
//! [`crate::concurrency::join_bounded`]. Domain allow-lists are small and
//! sequential (cost ≪ overhead).
//!
//! ## Module map (componentization)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `emulate` | headers / offline / conditions / CPU / setContent |
//! | `domain_filter` | DomainFilter + install/sanitize |
//! | `domain_script` | injected allow-list JS |
//! | `console` | console arg formatters |
//! | `tracker` | EventTracker ring |

mod console;
mod domain_filter;
mod domain_script;
mod emulate;
mod tracker;

#[cfg(test)]
mod tests;

pub use console::{format_console_arg, format_console_args};
pub use domain_filter::{
    install_domain_filter, install_domain_filter_fetch, install_domain_filter_script,
    sanitize_existing_pages, DomainFilter,
};
pub use emulate::{
    set_content, set_cpu_throttling_rate, set_extra_headers, set_network_conditions, set_offline,
};
pub use tracker::{ConsoleEntry, ErrorEntry, EventTracker};
