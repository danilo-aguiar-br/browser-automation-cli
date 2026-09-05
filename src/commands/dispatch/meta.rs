// SPDX-License-Identifier: MIT OR Apache-2.0
//! Meta / agent inventory command family.

use crate::error::{CliError, ErrorKind};

use super::{result_code, DispatchCtx};
use crate::commands::common::{emit_err, handle_locale, handle_version};
use crate::commands::meta as agent_meta;

pub(crate) fn doctor(
    ctx: &DispatchCtx<'_>,
    offline: bool,
    quick: bool,
    fix: bool,
    fingerprint: bool,
) -> i32 {
    if fingerprint {
        return crate::doctor::run_fingerprint(ctx.json, !quick && !offline);
    }
    crate::doctor::run_doctor(crate::doctor::DoctorOptions {
        offline,
        quick,
        fix,
        json: ctx.json,
    })
}

pub(crate) fn commands(ctx: &DispatchCtx<'_>, detail: bool) -> i32 {
    result_code(agent_meta::list_commands(detail, ctx.json), ctx.json)
}

pub(crate) fn schema(
    ctx: &DispatchCtx<'_>,
    cmd: Option<&str>,
    cmd_positional: Option<&str>,
) -> i32 {
    // GAP-022: positional `schema run` or `schema --cmd run`.
    let resolved = cmd_positional.or(cmd).filter(|s| !s.trim().is_empty());
    match resolved {
        Some(c) => result_code(agent_meta::schema_for_cmd(c, ctx.json), ctx.json),
        None => emit_err(
            &CliError::with_suggestion(
                ErrorKind::Usage,
                "schema requires a command name",
                crate::i18n::suggestion_key("schema_command_required", None),
            ),
            ctx.json,
        ),
    }
}

pub(crate) fn version(ctx: &DispatchCtx<'_>) -> i32 {
    result_code(handle_version(ctx.json), ctx.json)
}

pub(crate) fn locale(ctx: &DispatchCtx<'_>) -> i32 {
    result_code(handle_locale(ctx.json), ctx.json)
}
