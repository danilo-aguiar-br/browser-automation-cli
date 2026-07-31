// SPDX-License-Identifier: MIT OR Apache-2.0
//! Navigation and interaction steps for run/exec.
//!
//! # Module map (GAP-051 SRP split)
//!
//! | Module | Steps |
//! |--------|-------|
//! | fields | Shared JSON field accessors |
//! | navigate | goto, back, forward, reload |
//! | wait | wait |
//! | pointer | hover, drag, press/click, click-at |
//! | forms | fill-form, select-option/pick, upload, submit, write, keys, type |
//! | inspect | view, eval |

use serde_json::Value;

use crate::browser::OneShotSession;
use crate::error::{CliError, ErrorKind};
use crate::robots::RobotsPolicy;

use super::super::RunFlags;

mod fields;
mod forms;
mod inspect;
mod navigate;
mod pointer;
mod wait;

pub(super) async fn handle(
    session: &mut OneShotSession,
    cmd: &str,
    step: &Value,
    robots: RobotsPolicy,
    flags: RunFlags,
) -> Result<Value, CliError> {
    match cmd {
        "goto" => navigate::goto(session, step, robots).await,
        "wait" => wait::wait(session, step).await,
        "hover" => pointer::hover(session, step).await,
        "drag" => pointer::drag(session, step).await,
        "fill-form" | "fill_form" => forms::fill_form(session, step).await,
        "select-option" | "select_option" | "pick" => forms::pick_option(session, step).await,
        "upload" => forms::upload(session, step).await,
        "submit" => forms::submit(session, step).await,
        "back" => navigate::back(session).await,
        "forward" => navigate::forward(session).await,
        "reload" => navigate::reload(session, step).await,
        "view" => inspect::view(session, step).await,
        "press" | "click" => pointer::press(session, step).await,
        "write" | "fill" => forms::write(session, step).await,
        "keys" => forms::keys(session, step).await,
        "type" => forms::type_text(session, step).await,
        "click-at" | "click_at" => pointer::click_at(session, step, flags).await,
        "eval" => inspect::eval(session, step).await,
        other => Err(CliError::new(
            ErrorKind::Usage,
            format!("internal: unexpected cmd in this family: {other}"),
        )),
    }
}
