// SPDX-License-Identifier: MIT OR Apache-2.0
//! Crawl frontier admission: normalize, dedup and host/path gating of links.

use std::collections::{BTreeSet, VecDeque};

use url::Url;

use super::path_filter::{normalize_url_for_dedup_ex, PathFilter};

/// Enqueue a discovered link when host/path filters allow it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn crawl_enqueue_link(
    href: &str,
    depth: usize,
    same_host: bool,
    seed_host: Option<&str>,
    path_filter: &PathFilter,
    seen: &mut BTreeSet<String>,
    queue: &mut VecDeque<(String, usize)>,
    ignore_query_params: bool,
) {
    let href = normalize_url_for_dedup_ex(href, ignore_query_params);
    if !path_filter.allows_url(&href) {
        return;
    }
    if !seen.insert(href.clone()) {
        return;
    }
    if same_host {
        if let (Some(sh), Ok(u)) = (seed_host, Url::parse(&href)) {
            if u.host_str() != Some(sh) {
                return;
            }
        }
    }
    queue.push_back((href, depth + 1));
}
