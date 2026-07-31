// SPDX-License-Identifier: MIT OR Apache-2.0
//! Browser navigation / capture arms.

use std::path::Path;

use super::super::{result_code, DispatchCtx};
use crate::cli::{BeforeUnloadAction, GrabFormat};
use crate::commands::nav::*;

pub(crate) fn goto(
    ctx: &DispatchCtx<'_>,
    url: &str,
    init_script: Option<&str>,
    handle_before_unload: Option<BeforeUnloadAction>,
    navigation_timeout_ms: Option<u64>,
) -> i32 {
    result_code(
        handle_goto(
            ctx.life,
            url,
            ctx.robots,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
            init_script,
            handle_before_unload,
            navigation_timeout_ms,
        ),
        ctx.json,
    )
}

pub(crate) fn view(
    ctx: &DispatchCtx<'_>,
    verbose: bool,
    path: Option<&Path>,
    allow_empty: bool,
) -> i32 {
    result_code(
        handle_view(
            ctx.life,
            verbose,
            path,
            allow_empty,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn wait(
    ctx: &DispatchCtx<'_>,
    ms: u64,
    text: &[String],
    selector: Option<&str>,
    state: Option<&str>,
    wait_timeout_ms: Option<u64>,
    network_idle_ms: Option<u64>,
    min_count: Option<u64>,
    dom_stable_ms: Option<u64>,
    include_snapshot: bool,
) -> i32 {
    result_code(
        handle_wait(
            ctx.life,
            ms,
            text,
            selector,
            state,
            wait_timeout_ms,
            network_idle_ms,
            min_count,
            dom_stable_ms,
            include_snapshot,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn back(ctx: &DispatchCtx<'_>) -> i32 {
    result_code(
        handle_history(ctx.life, "back", ctx.capture, ctx.timeout_secs, ctx.json),
        ctx.json,
    )
}

pub(crate) fn forward(ctx: &DispatchCtx<'_>) -> i32 {
    result_code(
        handle_history(ctx.life, "forward", ctx.capture, ctx.timeout_secs, ctx.json),
        ctx.json,
    )
}

pub(crate) fn reload(
    ctx: &DispatchCtx<'_>,
    ignore_cache: bool,
    init_script: Option<&str>,
    handle_before_unload: Option<BeforeUnloadAction>,
) -> i32 {
    result_code(
        handle_reload(
            ctx.life,
            ignore_cache,
            init_script,
            handle_before_unload,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn eval(
    ctx: &DispatchCtx<'_>,
    expression: &str,
    args: Option<&str>,
    dialog_action: Option<&str>,
    file_path: Option<&Path>,
    service_worker_id: Option<&str>,
    typed: bool,
) -> i32 {
    result_code(
        handle_eval(
            ctx.life,
            expression,
            args,
            dialog_action,
            file_path,
            service_worker_id,
            typed,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn grab(
    ctx: &DispatchCtx<'_>,
    path: Option<&Path>,
    format: GrabFormat,
    full_page: bool,
    quality: Option<i32>,
    element: Option<&str>,
) -> i32 {
    result_code(
        handle_grab(
            ctx.life,
            path,
            format,
            full_page,
            quality,
            element,
            ctx.artifacts.as_deref(),
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn print_pdf(ctx: &DispatchCtx<'_>, path: Option<&Path>, url: Option<&str>) -> i32 {
    result_code(
        handle_print_pdf(
            ctx.life,
            path,
            url,
            ctx.robots,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}
