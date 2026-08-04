// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared multi-format payload derivation for scrape engines.

use crate::error::CliError;
use crate::robots::RobotsPolicy;

use super::types::ScrapeOpts;

/// Derive one payload per requested format from a single fetched HTML body.
///
/// Single source of truth for the HTTP and browser engine multi-format paths;
/// keys are the format names with `-` normalised to `_`.
///
/// `base_opts` propagates include/exclude selectors, redact, content_hash,
/// only_main, honor flags (agent CLEAN multi-format parity).
pub fn build_formats_map(
    source: &str,
    status: u16,
    html: &str,
    formats: &[&str],
    base_opts: &ScrapeOpts,
    engine: &str,
    robots: RobotsPolicy,
) -> Result<serde_json::Map<String, serde_json::Value>, CliError> {
    let mut formats_out = serde_json::Map::new();
    for f in formats {
        let fmt = crate::scrape_local::ScrapeFormat::parse(f)?;
        let opts = ScrapeOpts {
            format: fmt,
            only_main_content: base_opts.only_main_content,
            engine: engine.into(),
            max_body_bytes: base_opts.max_body_bytes,
            max_text_chars: base_opts.max_text_chars,
            honor_meta_robots: base_opts.honor_meta_robots,
            honor_nofollow: base_opts.honor_nofollow,
            follow_rel_next: base_opts.follow_rel_next,
            include_selectors: base_opts.include_selectors.clone(),
            exclude_selectors: base_opts.exclude_selectors.clone(),
            redact_pii: base_opts.redact_pii,
            with_content_hash: base_opts.with_content_hash,
            extra_headers: base_opts.extra_headers.clone(),
        };
        let part = crate::scrape_local::build_scrape_payload(source, status, html, &opts, robots);
        formats_out.insert(f.replace('-', "_"), part);
    }
    Ok(formats_out)
}
