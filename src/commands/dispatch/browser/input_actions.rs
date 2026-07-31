// SPDX-License-Identifier: MIT OR Apache-2.0
//! Browser input / pointer arms.

use std::path::Path;

use super::super::gates;
use super::super::{result_code, DispatchCtx};
use crate::commands::nav::*;

pub(crate) fn press(
    ctx: &DispatchCtx<'_>,
    target: &str,
    dblclick: bool,
    include_snapshot: bool,
) -> i32 {
    result_code(
        handle_press(
            ctx.life,
            target,
            dblclick,
            include_snapshot,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn click_at(
    ctx: &DispatchCtx<'_>,
    x: f64,
    y: f64,
    dblclick: bool,
    include_snapshot: bool,
) -> i32 {
    result_code(
        gates::require_experimental_vision(ctx).and_then(|()| {
            handle_click_at(
                ctx.life,
                x,
                y,
                dblclick,
                include_snapshot,
                ctx.capture,
                ctx.timeout_secs,
                ctx.json,
            )
        }),
        ctx.json,
    )
}

pub(crate) fn write(
    ctx: &DispatchCtx<'_>,
    target: &str,
    value: &str,
    include_snapshot: bool,
) -> i32 {
    result_code(
        handle_write(
            ctx.life,
            target,
            value,
            include_snapshot,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn keys(ctx: &DispatchCtx<'_>, key: &str, include_snapshot: bool) -> i32 {
    result_code(
        handle_keys(
            ctx.life,
            key,
            include_snapshot,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn type_text(
    ctx: &DispatchCtx<'_>,
    target: Option<&str>,
    text: &str,
    clear: bool,
    submit: Option<&str>,
    focus_only: bool,
    include_snapshot: bool,
) -> i32 {
    result_code(
        handle_type(
            ctx.life,
            target,
            text,
            clear,
            submit,
            focus_only,
            include_snapshot,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn hover(ctx: &DispatchCtx<'_>, target: &str, include_snapshot: bool) -> i32 {
    result_code(
        handle_hover(
            ctx.life,
            target,
            include_snapshot,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn drag(
    ctx: &DispatchCtx<'_>,
    from: &str,
    to: Option<&str>,
    to_x: Option<f64>,
    to_y: Option<f64>,
    anchor: &str,
    synthetic_payload: Option<&str>,
    include_snapshot: bool,
) -> i32 {
    result_code(
        handle_drag(
            ctx.life,
            from,
            to,
            to_x,
            to_y,
            anchor,
            synthetic_payload,
            include_snapshot,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn submit(
    ctx: &DispatchCtx<'_>,
    target: &str,
    timeout_ms: Option<u64>,
    include_snapshot: bool,
) -> i32 {
    result_code(
        handle_submit(
            ctx.life,
            target,
            timeout_ms,
            include_snapshot,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn fill_form(ctx: &DispatchCtx<'_>, fields_json: &str, include_snapshot: bool) -> i32 {
    result_code(
        handle_fill_form(
            ctx.life,
            fields_json,
            include_snapshot,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn upload(
    ctx: &DispatchCtx<'_>,
    target: &str,
    path: &Path,
    include_snapshot: bool,
) -> i32 {
    result_code(
        handle_upload(
            ctx.life,
            target,
            path,
            include_snapshot,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}
