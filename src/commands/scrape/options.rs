// SPDX-License-Identifier: MIT OR Apache-2.0
//! Option defaults and collection emission shared by batch and crawl handlers.

use crate::error::CliError;
use crate::scrape_local::emit_scrape_collection;

/// Resolve the text cap: `None`/`Some(0)` fall back to the XDG policy value.
pub(super) fn resolve_max_text(cli: Option<usize>) -> usize {
    match cli {
        Some(0) | None => crate::xdg::resolve_scrape_max_text_chars(),
        Some(n) => n,
    }
}

/// Resolve sitemap seeding: an absent flag falls back to the XDG policy value.
pub(super) fn resolve_use_sitemap(cli: Option<bool>) -> bool {
    cli.unwrap_or_else(crate::xdg::resolve_scrape_use_sitemap)
}

/// Resolve `rel=next` following: an absent flag falls back to XDG.
pub(super) fn resolve_follow_rel_next(cli: Option<bool>) -> bool {
    cli.unwrap_or_else(crate::xdg::resolve_scrape_follow_rel_next)
}

/// Resolve near-duplicate collapsing: an absent flag falls back to XDG.
///
/// `--dedup-similar` on the CLI overrides the XDG default; `None` means the
/// flag was omitted, so the persisted preference wins.
pub(super) fn resolve_dedup_similar(cli: Option<bool>) -> bool {
    cli.unwrap_or_else(crate::xdg::resolve_scrape_dedup_similar)
}

/// Resolve the near-duplicate Hamming threshold from XDG.
pub(super) fn resolve_dedup_similar_distance() -> u32 {
    crate::xdg::resolve_scrape_dedup_similar_distance()
}

/// Returns true when output_mode consumed stdout; false = use emit_ok.
///
/// `seed` is the crawl origin, needed only by `llms-txt` to title the document.
/// Batch callers have no single origin and pass an empty string.
pub(super) fn emit_collection(
    data: &serde_json::Value,
    arr_key: &str,
    output_mode: &str,
    json: bool,
    seed: &str,
) -> Result<bool, CliError> {
    emit_scrape_collection(data, arr_key, output_mode, json, seed)
}
