// SPDX-License-Identifier: MIT OR Apache-2.0
//! Ops / config / perf / gated experimental command family.

use std::path::Path;

use super::gates;
use super::{result_code, DispatchCtx};
use crate::cli::{
    CompletionShell, ConfigAction, Devtools3pAction, ExtensionAction, HeapAction, MitmAction,
    PerfAction, ScreencastAction, WebmcpAction, WorkflowAction,
};
use crate::commands::ops::*;

pub(crate) fn mitm(ctx: &DispatchCtx<'_>, action: MitmAction) -> i32 {
    result_code(handle_mitm(action, ctx.json), ctx.json)
}

pub(crate) fn workflow(ctx: &DispatchCtx<'_>, action: WorkflowAction) -> i32 {
    result_code(handle_workflow(action, ctx.json), ctx.json)
}

pub(crate) fn config(ctx: &DispatchCtx<'_>, action: ConfigAction) -> i32 {
    result_code(handle_config(action, ctx.json), ctx.json)
}

pub(crate) fn perf(ctx: &DispatchCtx<'_>, action: PerfAction) -> i32 {
    result_code(
        handle_perf(ctx.life, action, ctx.capture, ctx.timeout_secs, ctx.json),
        ctx.json,
    )
}

pub(crate) fn lighthouse(
    ctx: &DispatchCtx<'_>,
    url: &str,
    out_dir: Option<&Path>,
    device: &str,
    mode: &str,
    lighthouse_path: Option<&Path>,
) -> i32 {
    result_code(
        handle_lighthouse(url, out_dir, device, mode, lighthouse_path, ctx.json),
        ctx.json,
    )
}

pub(crate) fn screencast(ctx: &DispatchCtx<'_>, action: ScreencastAction) -> i32 {
    result_code(
        gates::require_experimental_screencast(ctx).and_then(|()| {
            handle_screencast(ctx.life, action, ctx.capture, ctx.timeout_secs, ctx.json)
        }),
        ctx.json,
    )
}

pub(crate) fn heap(ctx: &DispatchCtx<'_>, action: HeapAction) -> i32 {
    // GAP-010: the free/gated split is declared in `crate::capability`, not here.
    // The previous inline `matches!` also exempted `summary` and `close`, which
    // the reference surface gates.
    result_code(
        gates::require_capabilities(ctx, "heap", Some(heap_action_key(&action)))
            .and_then(|()| handle_heap(ctx.life, action, ctx.capture, ctx.timeout_secs, ctx.json)),
        ctx.json,
    )
}

/// Capability-table key for a heap action (kebab-case, matches the CLI subcommand).
fn heap_action_key(action: &HeapAction) -> &'static str {
    match action {
        HeapAction::Take { .. } => "take",
        HeapAction::Close { .. } => "close",
        HeapAction::Compare { .. } => "compare",
        HeapAction::Summary { .. } => "summary",
        HeapAction::Details { .. } => "details",
        HeapAction::ClassNodes { .. } => "class-nodes",
        HeapAction::Dominators { .. } => "dominators",
        HeapAction::DupStrings { .. } => "dup-strings",
        HeapAction::Edges { .. } => "edges",
        HeapAction::Retainers { .. } => "retainers",
        HeapAction::Paths { .. } => "paths",
        HeapAction::ObjectDetails { .. } => "object-details",
    }
}

pub(crate) fn extension(ctx: &DispatchCtx<'_>, action: ExtensionAction) -> i32 {
    result_code(
        gates::require_category_extensions(ctx).and_then(|()| {
            handle_extension(ctx.life, action, ctx.capture, ctx.timeout_secs, ctx.json)
        }),
        ctx.json,
    )
}

pub(crate) fn devtools3p(ctx: &DispatchCtx<'_>, action: Devtools3pAction) -> i32 {
    result_code(
        gates::require_category_third_party(ctx).and_then(|()| {
            handle_devtools3p(ctx.life, action, ctx.capture, ctx.timeout_secs, ctx.json)
        }),
        ctx.json,
    )
}

pub(crate) fn webmcp(ctx: &DispatchCtx<'_>, action: WebmcpAction) -> i32 {
    result_code(
        gates::require_category_webmcp(ctx).and_then(|()| {
            handle_webmcp(ctx.life, action, ctx.capture, ctx.timeout_secs, ctx.json)
        }),
        ctx.json,
    )
}

pub(crate) fn completions(ctx: &DispatchCtx<'_>, shell: CompletionShell) -> i32 {
    result_code(handle_completions(shell), ctx.json)
}

pub(crate) fn man(ctx: &DispatchCtx<'_>, out: Option<&Path>) -> i32 {
    result_code(handle_man(out), ctx.json)
}
