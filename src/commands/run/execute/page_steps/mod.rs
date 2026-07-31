// SPDX-License-Identifier: MIT OR Apache-2.0
//! Page-state steps for run/exec.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Steps |
//! |--------|-------|
//! | state | cookie, attr |
//! | capture | console, net |
//! | page | page, dialog |
#![allow(missing_docs, unused_imports)]

use std::path::Path;

use serde_json::{json, Value};

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::super::RunFlags;
use super::helpers::step_beforeunload_action;
mod capture;
mod page;
mod state;

pub(super) async fn handle(
    session: &mut OneShotSession,
    cmd: &str,
    step: &Value,
    _robots: RobotsPolicy,
    _flags: RunFlags,
) -> Result<Value, CliError> {
    match cmd {
        "cookie" | "attr" => state::handle(session, cmd, step).await,
        "console" | "net" => capture::handle(session, cmd, step).await,
        "page" | "dialog" => page::handle(session, cmd, step).await,
        other => Err(CliError::new(
            ErrorKind::Usage,
            format!("internal: unexpected cmd in this family: {other}"),
        )),
    }
}
