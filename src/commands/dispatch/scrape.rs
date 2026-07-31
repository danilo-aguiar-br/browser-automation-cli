// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scrape / crawl / search / sg / sheet command family.

use std::path::Path;

use super::{result_code, DispatchCtx};
use crate::cli::QrAction;
use crate::commands::scrape::*;

pub(crate) fn scrape(
    ctx: &DispatchCtx<'_>,
    url: &str,
    format: &[String],
    engine: &str,
    only_main_content: bool,
    webhook_url: Option<&str>,
) -> i32 {
    result_code(
        handle_scrape(
            ctx.life,
            url,
            ctx.robots,
            ctx.capture,
            ctx.timeout_secs,
            ctx.json,
            format,
            engine,
            only_main_content,
            webhook_url,
        ),
        ctx.json,
    )
}

pub(crate) fn batch_scrape(
    ctx: &DispatchCtx<'_>,
    urls_file: &Path,
    format: &str,
    concurrency: usize,
    engine: &str,
) -> i32 {
    result_code(
        handle_batch_scrape(
            ctx.life,
            urls_file,
            ctx.robots,
            ctx.capture,
            ctx.timeout_secs,
            format,
            concurrency,
            engine,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn crawl(
    ctx: &DispatchCtx<'_>,
    url: &str,
    limit: usize,
    max_depth: usize,
    format: &str,
    same_host: bool,
    engine: &str,
) -> i32 {
    result_code(
        handle_crawl(
            ctx.life,
            url,
            ctx.robots,
            ctx.capture,
            ctx.timeout_secs,
            limit,
            max_depth,
            format,
            same_host,
            engine,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn map(ctx: &DispatchCtx<'_>, url: &str, limit: usize, max_depth: usize) -> i32 {
    result_code(
        handle_map(url, ctx.robots, limit, max_depth, ctx.json),
        ctx.json,
    )
}

pub(crate) fn search(ctx: &DispatchCtx<'_>, query: &str, limit: usize) -> i32 {
    result_code(handle_search(query, ctx.robots, limit, ctx.json), ctx.json)
}

pub(crate) fn parse(ctx: &DispatchCtx<'_>, path: &Path, redact_pii: bool) -> i32 {
    result_code(handle_parse(path, redact_pii, ctx.json), ctx.json)
}

pub(crate) fn qr(ctx: &DispatchCtx<'_>, action: QrAction) -> i32 {
    result_code(handle_qr(action, ctx.json), ctx.json)
}

// Mirrors the clap argument surface 1:1; grouping into a struct would add an
// indirection that has to be kept in sync with argv by hand.
#[allow(clippy::too_many_arguments)]
pub(crate) fn find_paths(
    ctx: &DispatchCtx<'_>,
    pattern: Option<&str>,
    paths: &[String],
    extension: Option<&str>,
    hidden: bool,
    no_ignore: bool,
    max_depth: Option<usize>,
    entry_type: Option<&str>,
    limit: usize,
    glob: Option<&str>,
) -> i32 {
    result_code(
        handle_find_paths(
            pattern, paths, extension, hidden, no_ignore, max_depth, entry_type, limit, glob,
            ctx.json,
        ),
        ctx.json,
    )
}

pub(crate) fn sg_scan(ctx: &DispatchCtx<'_>, paths: &[String], limit: usize) -> i32 {
    result_code(handle_sg_scan(paths, limit, ctx.json), ctx.json)
}

pub(crate) fn sg_rewrite(ctx: &DispatchCtx<'_>, paths: &[String], apply: bool) -> i32 {
    result_code(handle_sg_rewrite(paths, apply, ctx.json), ctx.json)
}

pub(crate) fn sheet_write(ctx: &DispatchCtx<'_>, input: &Path, out: &Path, sheet: &str) -> i32 {
    result_code(handle_sheet_write(input, out, sheet, ctx.json), ctx.json)
}
