// SPDX-License-Identifier: MIT OR Apache-2.0
//! Resolve scrape/crawl XDG knobs with named constant defaults (no product env).
//!
//! Split out of `resolve.rs` so the scrape knob family — body/text caps,
//! politeness, robots directives, sitemap, feed, `rel=next` and near-duplicate
//! collapsing — reads as one surface and the parent file stays under the
//! project file-size gate. Every function is re-exported from `super`, so call
//! sites keep using `crate::xdg::resolve_scrape_*`.
//!
//! # Narrowing config integers to `usize`
//!
//! Config values arrive as `u64` and several knobs here are byte or item counts,
//! which the callers want as `usize`. Every one of them narrows through
//! `usize::try_from(..).ok()`, never through `as usize`.
//!
//! The difference only shows on a 32-bit target, where `as usize` truncates in
//! silence: a `scrape_max_body_bytes` of 4 GiB + 1 would become a 1-byte cap and
//! the scrape would return empty with exit 0. `try_from` fails instead, and the
//! `unwrap_or` below it falls back to the named default — the same fail-safe
//! shape `resolve.rs` already used for `max_ndjson_line_bytes`. Measured on
//! 2026-08-25, six functions in this file still used the truncating form while
//! their siblings next door did not; the cross-platform promise in
//! `docs/CROSS_PLATFORM.md` is what made that divergence a defect rather than a
//! style choice.

use super::config_io::load_config;

/// Max HTTP scrape body bytes.
pub fn resolve_scrape_max_body_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.scrape_max_body_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_MAX_BODY_BYTES)
}

/// Max text/markdown chars in scrape envelopes (`0` = no cap).
pub fn resolve_scrape_max_text_chars() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.scrape_max_text_chars)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_MAX_TEXT_CHARS)
}

/// Floor delay between same-origin GETs (ms); `0` disables the wait.
///
/// The sibling knobs in this file reject a stored `0` and fall back to the named
/// default, because for a byte cap or an entry count zero is a typo that
/// silently disables the guard. Here it is the opposite: zero is the only way an
/// operator can say "do not pace me", and a politeness floor is a courtesy the
/// operator owns, not a safety limit the product enforces. Stated so that the
/// missing `.filter(|&n| n > 0)` reads as a decision rather than the omission it
/// looks like next to its neighbours.
pub fn resolve_scrape_min_delay_ms() -> u64 {
    load_config()
        .ok()
        .and_then(|c| c.scrape_min_delay_ms)
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_MIN_DELAY_MS)
}

/// Honor meta robots / X-Robots-Tag noindex (default true).
pub fn resolve_scrape_honor_meta_robots() -> bool {
    load_config()
        .ok()
        .and_then(|c| c.scrape_honor_meta_robots)
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_HONOR_META_ROBOTS)
}

/// Skip nofollow links when extracting for crawl (default true).
pub fn resolve_scrape_honor_nofollow() -> bool {
    load_config()
        .ok()
        .and_then(|c| c.scrape_honor_nofollow)
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_HONOR_NOFOLLOW)
}

/// Prefer sitemap discovery for `map` (default true).
pub fn resolve_scrape_use_sitemap() -> bool {
    load_config()
        .ok()
        .and_then(|c| c.scrape_use_sitemap)
        .unwrap_or(true)
}

/// Default scrape engine when CLI omits `--engine` (`http` | `browser`).
pub fn resolve_scrape_default_engine() -> String {
    load_config()
        .ok()
        .and_then(|c| c.scrape_default_engine)
        .filter(|s| {
            let t = s.trim().to_ascii_lowercase();
            t == "http" || t == "browser"
        })
        .unwrap_or_else(|| crate::constants::DEFAULT_SCRAPE_ENGINE.to_string())
}

/// Politeness delay jitter ratio (0.0 = off, default 0.2).
pub fn resolve_scrape_delay_jitter_ratio() -> f64 {
    load_config()
        .ok()
        .and_then(|c| c.scrape_delay_jitter_ratio)
        .filter(|r| r.is_finite() && *r >= 0.0 && *r <= 1.0)
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_DELAY_JITTER_RATIO)
}

/// Max chars for scrape `summary` format.
pub fn resolve_scrape_summary_chars() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.scrape_summary_chars)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_SUMMARY_CHARS)
}

/// Max entries kept by scrape format `feed`.
pub fn resolve_scrape_feed_max_entries() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.scrape_feed_max_entries)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_FEED_MAX_ENTRIES)
}

/// Follow `rel=next` pagination links during crawl (default false).
pub fn resolve_scrape_follow_rel_next() -> bool {
    load_config()
        .ok()
        .and_then(|c| c.scrape_follow_rel_next)
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_FOLLOW_REL_NEXT)
}

/// Collapse near-duplicate pages by content similarity (default false).
pub fn resolve_scrape_dedup_similar() -> bool {
    load_config()
        .ok()
        .and_then(|c| c.scrape_dedup_similar)
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_DEDUP_SIMILAR)
}

/// Ignore the response cache on read and always fetch from origin (default false).
///
/// A READ bypass only: the fresh response is still written, so a caller that
/// turns this on refreshes the cache for everyone else rather than leaving a
/// stale entry behind.
pub fn resolve_scrape_no_cache() -> bool {
    load_config()
        .ok()
        .and_then(|c| c.scrape_no_cache)
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_NO_CACHE)
}

/// SimHash Hamming distance under which two pages count as near-duplicates.
///
/// Clamped to the 64-bit fingerprint width; `0` is legal and means the
/// fingerprints must be identical.
///
/// The `.min()` runs BEFORE the cast, which is what makes narrowing the stored
/// `u64` to `u32` lossless no matter what the operator wrote: the value is
/// already bounded by 64 when the cast happens.
pub fn resolve_scrape_dedup_similar_distance() -> u32 {
    load_config()
        .ok()
        .and_then(|c| c.scrape_dedup_similar_distance)
        .map(|n| n.min(u64::BITS as u64) as u32)
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_DEDUP_SIMILAR_DISTANCE)
}

/// Max sitemap body bytes.
pub fn resolve_scrape_sitemap_max_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.scrape_sitemap_max_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_SITEMAP_MAX_BYTES)
}

/// Charset HTML peek window (bytes).
pub fn resolve_scrape_charset_peek_bytes() -> usize {
    load_config()
        .ok()
        .and_then(|c| c.scrape_charset_peek_bytes)
        .filter(|&n| n > 0)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::constants::DEFAULT_SCRAPE_CHARSET_PEEK_BYTES)
}
