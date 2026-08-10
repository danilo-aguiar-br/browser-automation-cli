// SPDX-License-Identifier: MIT OR Apache-2.0
//! `config set` mutation path for the scrape/crawl knob family.
//!
//! Split out of `set.rs` (mirroring `set_media`) so the scrape knobs — body and
//! text caps, politeness, robots directives, sitemap, feed, `rel=next` and
//! near-duplicate collapsing — validate in one place and the parent file stays
//! under the project file-size gate.

use super::super::config_model::ProductConfig;
use super::validate::{parse_boolish, parse_positive_u64};
use crate::error::{CliError, ErrorKind};

/// Apply a scrape-family `config set` mutation.
///
/// Returns `Ok(false)` when `key` belongs to another family, so the caller
/// falls through to its own match without treating it as an unknown key.
pub(super) fn apply_scrape_set(
    cfg: &mut ProductConfig,
    key: &str,
    value: &str,
) -> Result<bool, CliError> {
    match key {
        "scrape_max_body_bytes" => {
            cfg.scrape_max_body_bytes = Some(parse_positive_u64(value, "scrape_max_body_bytes")?);
        }
        "scrape_max_text_chars" => {
            // 0 = no cap (agent opt-in)
            let n: u64 = value.parse().map_err(|_| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("invalid scrape_max_text_chars: {value}"),
                    crate::i18n::suggestion_key("use_listed_value", None),
                )
            })?;
            cfg.scrape_max_text_chars = Some(n);
        }
        "scrape_min_delay_ms" => {
            let n: u64 = value.parse().map_err(|_| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("invalid scrape_min_delay_ms: {value}"),
                    crate::i18n::suggestion_key("use_listed_value", None),
                )
            })?;
            cfg.scrape_min_delay_ms = Some(n);
        }
        "scrape_honor_meta_robots" => {
            cfg.scrape_honor_meta_robots = Some(parse_boolish(value, key)?)
        }
        "scrape_honor_nofollow" => cfg.scrape_honor_nofollow = Some(parse_boolish(value, key)?),
        "scrape_use_sitemap" => cfg.scrape_use_sitemap = Some(parse_boolish(value, key)?),
        "scrape_default_engine" => {
            let eng = value.trim().to_ascii_lowercase();
            if eng != "http" && eng != "browser" {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("invalid scrape_default_engine: {value} (use http|browser)"),
                    crate::i18n::suggestion_key("use_listed_value", None),
                ));
            }
            cfg.scrape_default_engine = Some(eng);
        }
        "scrape_delay_jitter_ratio" => {
            let r: f64 = value.parse().map_err(|_| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("invalid scrape_delay_jitter_ratio: {value}"),
                    crate::i18n::suggestion_key("use_listed_value", None),
                )
            })?;
            if !(0.0..=1.0).contains(&r) {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "scrape_delay_jitter_ratio must be 0.0..=1.0",
                    crate::i18n::suggestion_key("use_listed_value", None),
                ));
            }
            cfg.scrape_delay_jitter_ratio = Some(r);
        }
        "scrape_summary_chars" => {
            cfg.scrape_summary_chars = Some(parse_positive_u64(value, "scrape_summary_chars")?);
        }
        "scrape_feed_max_entries" => {
            cfg.scrape_feed_max_entries =
                Some(parse_positive_u64(value, "scrape_feed_max_entries")?);
        }
        "scrape_follow_rel_next" => cfg.scrape_follow_rel_next = Some(parse_boolish(value, key)?),
        "scrape_dedup_similar" => cfg.scrape_dedup_similar = Some(parse_boolish(value, key)?),
        "scrape_no_cache" => cfg.scrape_no_cache = Some(parse_boolish(value, key)?),
        "scrape_dedup_similar_distance" => {
            // 0 is meaningful (identical fingerprints only), so this cannot use
            // parse_positive_u64; the ceiling is the fingerprint width.
            let n: u64 = value.trim().parse().map_err(|_| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    "scrape_dedup_similar_distance must be an integer 0..=64",
                    crate::i18n::suggestion_key("use_listed_value", None),
                )
            })?;
            if n > u64::BITS as u64 {
                return Err(CliError::with_suggestion(
                    ErrorKind::Usage,
                    "scrape_dedup_similar_distance must be an integer 0..=64",
                    crate::i18n::suggestion_key("use_listed_value", None),
                ));
            }
            cfg.scrape_dedup_similar_distance = Some(n);
        }
        "scrape_sitemap_max_bytes" => {
            cfg.scrape_sitemap_max_bytes =
                Some(parse_positive_u64(value, "scrape_sitemap_max_bytes")?);
        }
        "scrape_charset_peek_bytes" => {
            cfg.scrape_charset_peek_bytes =
                Some(parse_positive_u64(value, "scrape_charset_peek_bytes")?);
        }
        _ => return Ok(false),
    }
    Ok(true)
}
