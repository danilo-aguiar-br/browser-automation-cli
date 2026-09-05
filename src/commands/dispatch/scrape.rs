// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scrape / crawl / search / sg / sheet command family.

use std::path::Path;

use super::{result_code, DispatchCtx};
use crate::cli::{AudioAction, ImageAction, QrAction, VideoAction};
use crate::commands::scrape::*;

use super::scrape_args::{pair_attribute_targets, parse_actions};

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
    attribute_selector: &[String],
    attribute_name: &[String],
    action: &[String],
    no_cache: Option<bool>,
) -> i32 {
    // Paired here, once, before any work happens. Pairing inside the handler
    // would put the check after the browser has already launched, and a
    // mismatched count is an argv mistake, not a page problem.
    let attribute_targets = match pair_attribute_targets(attribute_selector, attribute_name) {
        Ok(pairs) => pairs,
        Err(e) => return result_code(Err(e), ctx.json),
    };
    // Parsed before the browser launches: a malformed action is an argv
    // mistake, and finding it after a page load wastes the navigation.
    let actions = match parse_actions(action) {
        Ok(steps) => steps,
        Err(e) => return result_code(Err(e), ctx.json),
    };
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
            &attribute_targets,
            &actions,
            no_cache,
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
    only_main_content: bool,
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
    webhook_url: Option<&str>,
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
            only_main_content,
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
            webhook_url,
        ),
        ctx.json,
    )
}

/// Route `crawl`, unpacking the parsed args here rather than in the match.
///
/// Takes the struct instead of 28 loose parameters: spelling those out in the
/// match arm AND in this signature was the single largest block in
/// `dispatch/mod.rs`, and it pushed that file past the 300-line ceiling the
/// project enforces on itself.
pub(crate) fn crawl(ctx: &DispatchCtx<'_>, a: &crate::cli::CrawlArgs) -> i32 {
    let (url, format, engine, output_mode) = (&a.url, &a.format, &a.engine, a.output_mode.as_str());
    // `--no-same-host` wins, the same way `--allow-non-image` overrides
    // `--require-image`: the negation is the only value the caller could have
    // typed on purpose, since the positive one is also the default.
    let same_host = a.same_host && !a.no_same_host;
    let (limit, max_depth) = (a.limit, a.max_depth);
    let (select, filter, sort, dedup_key) = (
        a.select.as_deref(),
        a.filter.as_deref(),
        a.sort.as_deref(),
        a.dedup_key.as_deref(),
    );
    let (max_text_chars, use_sitemap, sitemap_only) =
        (a.max_text_chars, a.use_sitemap, a.sitemap_only);
    let (ignore_query_params, follow_rel_next, dedup_similar) =
        (a.ignore_query_params, a.follow_rel_next, a.dedup_similar);
    let (redact_pii, with_content_hash, dry_run) = (a.redact_pii, a.with_content_hash, a.dry_run);
    let (include_path, exclude_path) = (&a.include_path, &a.exclude_path);
    let (include_selector, exclude_selector) = (&a.include_selector, &a.exclude_selector);
    let (include_regex, exclude_regex) = (&a.include_regex, &a.exclude_regex);
    let webhook_url = a.webhook_url.as_deref();
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
            sitemap_only,
            ignore_query_params,
            follow_rel_next,
            dedup_similar,
            sort,
            dedup_key,
            redact_pii,
            with_content_hash,
            include_selector,
            exclude_selector,
            a.only_main_content,
            dry_run,
            include_regex,
            exclude_regex,
            webhook_url,
        ),
        ctx.json,
    )
}

/// Route `map`, unpacking the parsed args here rather than in the match.
///
/// Same reason as [`crawl`]: the field list is long enough that spelling it out
/// twice cost `dispatch/mod.rs` more lines than its 300-line ceiling allowed.
///
/// The sibling verbs `sitemap` and `feed` live in [`super::discovery`]; they
/// were briefly written directly above this function, which silently reassigned
/// THIS doc comment to `feed` and left `map` undocumented. Rust attaches a
/// `///` block to whatever item follows it, and neither the compiler nor
/// `cargo doc -D warnings` can tell that the prose stopped describing its item.
pub(crate) fn map(ctx: &DispatchCtx<'_>, a: &crate::cli::MapArgs) -> i32 {
    result_code(
        handle_map(
            &a.url,
            ctx.robots,
            a.limit,
            a.max_depth,
            ctx.json,
            a.select.as_deref(),
            &a.include_path,
            &a.exclude_path,
            a.use_sitemap,
            a.search.as_deref(),
            a.sort.as_deref(),
            a.dedup_key.as_deref(),
            a.sitemap_only,
            a.include_subdomains,
            a.ignore_query_params,
        ),
        ctx.json,
    )
}

/// Route `search`, including the two domain-scoping dimensions.
pub(crate) fn search(ctx: &DispatchCtx<'_>, a: &crate::cli::SearchArgs) -> i32 {
    result_code(
        handle_search(
            &a.query,
            ctx.robots,
            a.limit,
            ctx.json,
            a.select.as_deref(),
            a.sort.as_deref(),
            a.dedup_key.as_deref(),
            a.include_domains.as_deref(),
            a.exclude_domains.as_deref(),
            a.country.as_deref(),
            a.search_lang.as_deref(),
            a.time_filter.as_deref(),
        ),
        ctx.json,
    )
}

pub(crate) fn parse(
    ctx: &DispatchCtx<'_>,
    path: &Path,
    redact_pii: bool,
    formats: &[String],
) -> i32 {
    result_code(handle_parse(path, redact_pii, formats, ctx.json), ctx.json)
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

pub(crate) fn sheet_write(
    ctx: &DispatchCtx<'_>,
    input: &Path,
    out: &Path,
    sheet: &str,
    force: bool,
) -> i32 {
    result_code(
        handle_sheet_write(input, out, sheet, force, ctx.json),
        ctx.json,
    )
}
