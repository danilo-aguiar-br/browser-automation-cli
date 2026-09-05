// SPDX-License-Identifier: MIT OR Apache-2.0
//! Structured-discovery verbs: `sitemap` and `feed`.
//!
//! Both are thin routes over machinery that already ships and already runs in
//! production, so neither adds capability. They exist for DISCOVERABILITY: on
//! an agent-facing CLI the inventory in `commands` is how a capability is
//! found, and a capability reachable only by knowing that a flag on a
//! differently-named verb carries it is, in practice, not reachable.
//!
//! They live in their own module because the two of them pushed
//! `dispatch/scrape.rs` past the 300-line production ceiling that
//! `scripts/filesize-check.sh` enforces. Splitting on the seam that already
//! existed — discovery by declared document, versus fetch-and-render — keeps
//! the boundary meaningful rather than merely arithmetic.

use super::{result_code, DispatchCtx};
use crate::commands::scrape::handle_map;

/// `feed` is `scrape --formats feed --engine http`, delegated not reimplemented.
///
/// # Why the engine and format are fixed rather than exposed
///
/// A feed is an XML or JSON document, not a rendered page. `ScrapeFormat::Feed`
/// deliberately parses the RAW body, because selector and main-content
/// reduction are HTML notions that would destroy the document — so every flag
/// that shapes an HTML scrape is not merely unnecessary here, it is a way to
/// ask this verb to break itself. They are passed as their empty values below
/// and are absent from `FeedArgs`.
///
/// Chrome is not offered for the same reason: rendering a feed produces a
/// browser's XML viewer, not the feed.
///
/// # Why the verb exists when the capability already did
///
/// `scrape --formats feed` has parsed feeds all along. What did not exist was a
/// way to FIND that: `commands` listed no `feed`, so an agent asked to read a
/// feed had to already know that a format of a general-purpose verb carried it.
/// The PRD recorded `feed` as missing with "no substitute", which was wrong
/// about the capability and right about the discoverability.
pub(crate) fn feed(ctx: &DispatchCtx<'_>, a: &crate::cli::FeedArgs) -> i32 {
    super::scrape::scrape(
        ctx,
        &a.url,
        std::slice::from_ref(&a.format_feed),
        "http",
        false,
        None,
        a.select.as_deref(),
        None,
        &[],
        &[],
        false,
        false,
        None,
        None,
        &a.header,
        0,
        &[],
        &[],
        &[],
        a.no_cache,
    )
}

/// `sitemap` is `map` with `--sitemap-only` fixed on, and nothing else.
///
/// # Why this delegates instead of implementing
///
/// The sitemap walk — the `robots.txt` `Sitemap:` hints, the nested
/// `sitemapindex` descent, the path filters, the dedupe — already exists and is
/// exercised in production by `map` and `crawl`. A second implementation would
/// be a second thing to keep correct, and the two would drift; the one that
/// drifts silently is always the one fewer people run.
///
/// So this verb adds no capability. It adds DISCOVERABILITY, which for an
/// agent-facing CLI is not cosmetic: the capability was reachable only by
/// knowing that a flag on a differently-named verb carried it, and the PRD
/// itself had recorded the belief that `map` did link discovery only.
pub(crate) fn sitemap(ctx: &DispatchCtx<'_>, a: &crate::cli::SitemapArgs) -> i32 {
    result_code(
        handle_map(
            &a.url,
            ctx.robots,
            a.limit,
            // Depth is meaningless for a sitemap: it is a DECLARED list, not a
            // frontier to walk, so there is no link graph to bound. The value
            // is passed because the shared handler takes one, and is unused on
            // the sitemap-only path.
            crate::constants::FRONTIER_DEFAULT_MAX_DEPTH,
            ctx.json,
            a.select.as_deref(),
            &a.include_path,
            &a.exclude_path,
            // Asking for the sitemap and then not using it is not a state this
            // verb can be in, so neither flag is exposed to contradict it.
            Some(true),
            a.search.as_deref(),
            a.sort.as_deref(),
            a.dedup_key.as_deref(),
            true,
            a.include_subdomains,
            a.ignore_query_params,
        ),
        ctx.json,
    )
}
