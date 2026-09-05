// SPDX-License-Identifier: MIT OR Apache-2.0
//! Browser page state / query / emulate arms.

use std::path::Path;

use super::super::{result_code, DispatchCtx};
use crate::cli::{AssertKind, ConsoleAction, CookieAction, DialogAction, NetAction, PageAction};
use crate::commands::nav::*;
use crate::commands::ops::{handle_emulate, handle_resize};

pub(crate) fn extract(
    ctx: &DispatchCtx<'_>,
    target: &str,
    attr: Option<&str>,
    llm: bool,
    question: Option<&str>,
    schema_json: Option<&Path>,
) -> i32 {
    result_code(
        handle_extract(
            ctx.life,
            target,
            attr,
            llm,
            question,
            schema_json,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn text(ctx: &DispatchCtx<'_>, target: &str) -> i32 {
    result_code(
        handle_text(ctx.life, target, ctx.capture, ctx.timeout_secs, ctx.json),
        ctx.json,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn scroll(
    ctx: &DispatchCtx<'_>,
    target: Option<&str>,
    delta_x: f64,
    delta_y: f64,
    to_x: Option<f64>,
    to_y: Option<f64>,
    include_snapshot: bool,
) -> i32 {
    result_code(
        handle_scroll(
            ctx.life,
            target,
            delta_x,
            delta_y,
            to_x,
            to_y,
            include_snapshot,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn cookie(ctx: &DispatchCtx<'_>, action: CookieAction) -> i32 {
    result_code(
        handle_cookie(ctx.life, action, ctx.capture, ctx.timeout_secs, ctx.json),
        ctx.json,
    )
}

pub(crate) fn attr(ctx: &DispatchCtx<'_>, target: &str, name: &str) -> i32 {
    result_code(
        handle_attr(
            ctx.life,
            target,
            name,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn assert_cmd(ctx: &DispatchCtx<'_>, kind: AssertKind) -> i32 {
    result_code(
        handle_assert(ctx.life, kind, ctx.capture, ctx.timeout_secs, ctx.json),
        ctx.json,
    )
}

pub(crate) fn console(ctx: &DispatchCtx<'_>, action: ConsoleAction) -> i32 {
    result_code(
        handle_console(ctx.life, action, ctx.capture, ctx.timeout_secs, ctx.json),
        ctx.json,
    )
}

pub(crate) fn net(ctx: &DispatchCtx<'_>, action: NetAction) -> i32 {
    result_code(
        handle_net(ctx.life, action, ctx.capture, ctx.timeout_secs, ctx.json),
        ctx.json,
    )
}

pub(crate) fn page(ctx: &DispatchCtx<'_>, action: Option<PageAction>) -> i32 {
    result_code(
        handle_page(ctx.life, action, ctx.capture, ctx.timeout_secs, ctx.json),
        ctx.json,
    )
}

pub(crate) fn dialog(ctx: &DispatchCtx<'_>, action: DialogAction) -> i32 {
    result_code(
        handle_dialog(ctx.life, action, ctx.capture, ctx.timeout_secs, ctx.json),
        ctx.json,
    )
}

// Mirrors the clap argument surface 1:1; grouping into a struct would add an
// indirection that has to be kept in sync with argv by hand.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emulate(
    ctx: &DispatchCtx<'_>,
    user_agent: Option<&str>,
    locale: Option<&str>,
    timezone: Option<&str>,
    offline: bool,
    latitude: Option<f64>,
    longitude: Option<f64>,
    media: Option<&str>,
    network_conditions: Option<&str>,
    cpu_throttling_rate: Option<f64>,
    color_scheme: Option<&str>,
    extra_headers: Option<&str>,
    viewport: Option<&str>,
    screen: Option<&str>,
) -> i32 {
    result_code(
        handle_emulate(
            ctx.life,
            user_agent,
            locale,
            timezone,
            offline,
            latitude,
            longitude,
            media,
            network_conditions,
            cpu_throttling_rate,
            color_scheme,
            extra_headers,
            viewport,
            screen,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn resize(
    ctx: &DispatchCtx<'_>,
    width: i32,
    height: i32,
    scale: f64,
    mobile: bool,
    screen: Option<&str>,
) -> i32 {
    result_code(
        handle_resize(
            ctx.life,
            width,
            height,
            scale,
            mobile,
            screen,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
        ),
        ctx.json,
    )
}
