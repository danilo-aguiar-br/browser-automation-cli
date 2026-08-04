// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scrape / crawl / search / sg / sheet command family.

use std::path::Path;

use super::{result_code, DispatchCtx};
use crate::cli::{AudioAction, ImageAction, QrAction, VideoAction};
use crate::commands::scrape::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn scrape(
    ctx: &DispatchCtx<'_>,
    url: &str,
    format: &[String],
    engine: &str,
    only_main_content: bool,
    webhook_url: Option<&str>,
    select: Option<&str>,
    max_text_chars: Option<usize>,
    include_selector: &[String],
    exclude_selector: &[String],
    redact_pii: bool,
    with_content_hash: bool,
    schema_json: Option<&Path>,
    question: Option<&str>,
    header: &[String],
    wait_ms: u64,
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
            select,
            max_text_chars,
            include_selector,
            exclude_selector,
            redact_pii,
            with_content_hash,
            schema_json,
            question,
            header,
            wait_ms,
        ),
        ctx.json,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn batch_scrape(
    ctx: &DispatchCtx<'_>,
    urls_file: &Path,
    format: &[String],
    concurrency: usize,
    engine: &str,
    select: Option<&str>,
    max_text_chars: Option<usize>,
    filter: Option<&str>,
    output_mode: &str,
    sort: Option<&str>,
    dedup_key: Option<&str>,
    dedup_similar: Option<bool>,
    include_selector: &[String],
    exclude_selector: &[String],
    redact_pii: bool,
    with_content_hash: bool,
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
            select,
            max_text_chars,
            filter,
            output_mode,
            sort,
            dedup_key,
            dedup_similar,
            include_selector,
            exclude_selector,
            redact_pii,
            with_content_hash,
        ),
        ctx.json,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn crawl(
    ctx: &DispatchCtx<'_>,
    url: &str,
    limit: usize,
    max_depth: usize,
    format: &[String],
    same_host: bool,
    engine: &str,
    select: Option<&str>,
    max_text_chars: Option<usize>,
    filter: Option<&str>,
    output_mode: &str,
    include_path: &[String],
    exclude_path: &[String],
    use_sitemap: Option<bool>,
    ignore_query_params: bool,
    follow_rel_next: Option<bool>,
    dedup_similar: Option<bool>,
    sort: Option<&str>,
    dedup_key: Option<&str>,
    redact_pii: bool,
    with_content_hash: bool,
    include_selector: &[String],
    exclude_selector: &[String],
    dry_run: bool,
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
            select,
            max_text_chars,
            filter,
            output_mode,
            include_path,
            exclude_path,
            use_sitemap,
            ignore_query_params,
            follow_rel_next,
            dedup_similar,
            sort,
            dedup_key,
            redact_pii,
            with_content_hash,
            include_selector,
            exclude_selector,
            dry_run,
        ),
        ctx.json,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn map(
    ctx: &DispatchCtx<'_>,
    url: &str,
    limit: usize,
    max_depth: usize,
    select: Option<&str>,
    include_path: &[String],
    exclude_path: &[String],
    use_sitemap: Option<bool>,
    search: Option<&str>,
    sort: Option<&str>,
    dedup_key: Option<&str>,
) -> i32 {
    result_code(
        handle_map(
            url,
            ctx.robots,
            limit,
            max_depth,
            ctx.json,
            select,
            include_path,
            exclude_path,
            use_sitemap,
            search,
            sort,
            dedup_key,
        ),
        ctx.json,
    )
}

pub(crate) fn search(
    ctx: &DispatchCtx<'_>,
    query: &str,
    limit: usize,
    select: Option<&str>,
    sort: Option<&str>,
    dedup_key: Option<&str>,
) -> i32 {
    result_code(
        handle_search(query, ctx.robots, limit, ctx.json, select, sort, dedup_key),
        ctx.json,
    )
}

pub(crate) fn parse(ctx: &DispatchCtx<'_>, path: &Path, redact_pii: bool) -> i32 {
    result_code(handle_parse(path, redact_pii, ctx.json), ctx.json)
}

pub(crate) fn qr(ctx: &DispatchCtx<'_>, action: QrAction) -> i32 {
    result_code(handle_qr(action, ctx.json), ctx.json)
}

pub(crate) fn image(ctx: &DispatchCtx<'_>, action: ImageAction) -> i32 {
    result_code(handle_image(action, ctx.json), ctx.json)
}

pub(crate) fn video(ctx: &DispatchCtx<'_>, action: VideoAction) -> i32 {
    result_code(
        crate::commands::local_video::handle_video(action, ctx.json),
        ctx.json,
    )
}

pub(crate) fn audio(ctx: &DispatchCtx<'_>, action: AudioAction) -> i32 {
    result_code(
        crate::commands::local_audio::handle_audio(action, ctx.json),
        ctx.json,
    )
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
