// SPDX-License-Identifier: MIT OR Apache-2.0
//! Category / experimental feature gates for dispatch.
//!
//! Every check resolves through [`crate::capability`] so the rule lives in one
//! auditable table (GAP-010) and fails as `capability-disabled` / exit 64 instead
//! of `usage` / exit 2 (GAP-011). Encoding the rule inline per command is what let
//! `heap summary` and `heap close` escape `--category-memory`.

use crate::capability::{self, Capability};
use crate::error::CliError;

use super::DispatchCtx;

/// True when `ctx` has the flag for `cap` enabled.
fn enabled(ctx: &DispatchCtx<'_>, cap: Capability) -> bool {
    match cap {
        Capability::Memory => ctx.category_memory,
        Capability::Extensions => ctx.category_extensions,
        Capability::ThirdParty => ctx.category_third_party,
        Capability::Webmcp => ctx.category_webmcp,
        Capability::Vision => ctx.experimental_vision,
        Capability::Screencast => ctx.experimental_screencast,
        // Capture buffers are launch options rather than policy gates.
        Capability::CaptureConsole => ctx.capture.console,
        Capability::CaptureNetwork => ctx.capture.network,
    }
}

/// Enforce every capability the table declares for `cmd` (+ optional `action`).
pub(crate) fn require_capabilities(
    ctx: &DispatchCtx<'_>,
    cmd: &str,
    action: Option<&str>,
) -> Result<(), CliError> {
    let label = match action {
        Some(a) => format!("{cmd} {a}"),
        None => cmd.to_string(),
    };
    for cap in capability::required_capabilities(cmd, action) {
        if !enabled(ctx, *cap) {
            return Err(cap.disabled_error(&label));
        }
    }
    Ok(())
}

pub(crate) fn require_experimental_vision(ctx: &DispatchCtx<'_>) -> Result<(), CliError> {
    require_capabilities(ctx, "click-at", None)
}

pub(crate) fn require_experimental_screencast(ctx: &DispatchCtx<'_>) -> Result<(), CliError> {
    require_capabilities(ctx, "screencast", None)
}

pub(crate) fn require_category_extensions(ctx: &DispatchCtx<'_>) -> Result<(), CliError> {
    require_capabilities(ctx, "extension", None)
}

pub(crate) fn require_category_third_party(ctx: &DispatchCtx<'_>) -> Result<(), CliError> {
    require_capabilities(ctx, "devtools3p", None)
}

pub(crate) fn require_category_webmcp(ctx: &DispatchCtx<'_>) -> Result<(), CliError> {
    require_capabilities(ctx, "webmcp", None)
}
