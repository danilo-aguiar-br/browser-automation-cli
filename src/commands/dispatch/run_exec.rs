// SPDX-License-Identifier: MIT OR Apache-2.0
//! Run / exec / monitor command family.

use std::path::Path;

use super::{result_code, DispatchCtx};
use crate::cli::MonitorAction;
use crate::commands::common::peel_trailing_globals;
use crate::commands::nav::{handle_exec, handle_monitor, handle_run};
use crate::commands::run;

pub(crate) fn monitor(ctx: &DispatchCtx<'_>, action: MonitorAction) -> i32 {
    result_code(
        handle_monitor(action, ctx.robots, ctx.timeout_secs, ctx.json),
        ctx.json,
    )
}

pub(crate) fn run(ctx: &DispatchCtx<'_>, script: &Path) -> i32 {
    let flags = run::RunFlags::from_globals(
        ctx.experimental_vision,
        ctx.experimental_screencast,
        ctx.category_memory,
        ctx.category_extensions,
        ctx.category_third_party,
        ctx.category_webmcp,
        ctx.json_steps,
        ctx.step_timeout_secs,
    );
    result_code(
        handle_run(
            ctx.life,
            script,
            ctx.robots,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
            flags,
        ),
        ctx.json,
    )
}

pub(crate) fn exec(ctx: &DispatchCtx<'_>, args: &[String]) -> i32 {
    // trailing_var_arg can capture global flags placed after `exec`;
    // peel them so agents can write: exec goto URL --json
    let (args, json_from_trail) = peel_trailing_globals(args);
    let json = ctx.json || json_from_trail;
    let flags = run::RunFlags::from_globals(
        ctx.experimental_vision,
        ctx.experimental_screencast,
        ctx.category_memory,
        ctx.category_extensions,
        ctx.category_third_party,
        ctx.category_webmcp,
        ctx.json_steps,
        ctx.step_timeout_secs,
    );
    result_code(
        handle_exec(
            ctx.life,
            &args,
            ctx.robots,
            ctx.capture,
            ctx.timeout_secs,
            json,
            flags,
        ),
        json,
    )
}
