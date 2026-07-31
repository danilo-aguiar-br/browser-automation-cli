// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession content methods (SRP split).
//!
//! # Module map (Tier-3 SRP)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `scrape_view` | scrape, view, attach_snapshot_if |
//! | `input` | press, write, click_at, keys, type_text |
//! | `eval` | eval, eval_service_worker, wait_ms |
//! | `capture_out` | print_pdf, grab |
//! | `query` | extract, attr, text, scroll |
//! | `submit` | form submission with outcome wait |
//!
//! Interaction methods that live in `interact` (hover/drag/fill_form/…) stay
//! there; this module owns content/read + primary input used by Layer A/B.

mod capture_out;
mod eval;
mod input;
mod query;
mod scrape_view;
mod submit;

use super::OneShotSession;
use crate::error::{CliError, ErrorKind};

impl OneShotSession {
    /// Active CDP session id (DRY helper for content methods).
    pub(crate) fn session_id(&self) -> Result<String, CliError> {
        self.manager
            .active_session_id()
            .map(|s| s.to_string())
            .map_err(|e| CliError::new(ErrorKind::Browser, e))
    }
}
