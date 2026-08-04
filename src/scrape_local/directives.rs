// SPDX-License-Identifier: MIT OR Apache-2.0
//! Meta robots / X-Robots-Tag / nofollow helpers for scrape compliance.

use scraper::{Html, Selector};

/// Parsed robots directives for a page (meta + HTTP header).
#[derive(Debug, Clone, Copy, Default)]
pub struct PageRobots {
    /// True when indexing is disallowed.
    pub noindex: bool,
    /// True when following links is disallowed.
    pub nofollow: bool,
    /// Where the directive was observed (`meta` or `x-robots-tag`).
    pub source: &'static str,
}

/// Parse `X-Robots-Tag` header value (comma-separated directives).
pub fn parse_x_robots_tag(header: Option<&str>) -> PageRobots {
    let mut out = PageRobots {
        source: "x-robots-tag",
        ..Default::default()
    };
    let Some(h) = header else {
        return out;
    };
    apply_directives(&mut out, h);
    out
}

/// Parse HTML `<meta name="robots" content="…">`.
pub fn parse_meta_robots(html: &str) -> PageRobots {
    let mut out = PageRobots {
        source: "meta",
        ..Default::default()
    };
    let doc = Html::parse_document(html);
    let Ok(sel) = Selector::parse("meta[name=\"robots\"], meta[name=\"ROBOTS\"]") else {
        return out;
    };
    for el in doc.select(&sel) {
        if let Some(c) = el.value().attr("content") {
            apply_directives(&mut out, c);
        }
    }
    out
}

/// Merge header (precedence) over meta.
pub fn merge_robots(header: PageRobots, meta: PageRobots) -> PageRobots {
    // X-Robots-Tag takes precedence when present (any directive set).
    if header.noindex || header.nofollow {
        return PageRobots {
            noindex: header.noindex || meta.noindex,
            nofollow: header.nofollow || meta.nofollow,
            source: "x-robots-tag",
        };
    }
    meta
}

fn apply_directives(out: &mut PageRobots, raw: &str) {
    for part in raw.split(',') {
        let d = part.trim().to_ascii_lowercase();
        match d.as_str() {
            "noindex" => out.noindex = true,
            "nofollow" => out.nofollow = true,
            "none" => {
                out.noindex = true;
                out.nofollow = true;
            }
            _ => {}
        }
    }
}

/// True when `rel` attribute contains nofollow (case-insensitive tokens).
pub fn rel_has_nofollow(rel: Option<&str>) -> bool {
    let Some(rel) = rel else {
        return false;
    };
    rel.split_whitespace()
        .any(|t| t.eq_ignore_ascii_case("nofollow"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_noindex_nofollow() {
        let html = r#"<html><head><meta name="robots" content="noindex, nofollow"></head></html>"#;
        let p = parse_meta_robots(html);
        assert!(p.noindex && p.nofollow);
    }

    #[test]
    fn x_robots_none() {
        let p = parse_x_robots_tag(Some("none"));
        assert!(p.noindex && p.nofollow);
    }

    #[test]
    fn rel_nofollow() {
        assert!(rel_has_nofollow(Some("noopener nofollow")));
        assert!(!rel_has_nofollow(Some("noopener")));
    }
}
