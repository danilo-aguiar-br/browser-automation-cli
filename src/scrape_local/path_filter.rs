// SPDX-License-Identifier: MIT OR Apache-2.0
//! Include/exclude path filters for crawl/map (scraping-oriented).

use url::Url;

/// Path filter: include prefixes (empty = allow all) and exclude prefixes.
#[derive(Debug, Clone, Default)]
pub struct PathFilter {
    /// Path prefixes that must match (empty = allow all paths not excluded).
    pub include: Vec<String>,
    /// Path prefixes that are always rejected.
    pub exclude: Vec<String>,
    /// Regexes matched against the path or the full URL (include family).
    pub include_regex: Vec<regex::Regex>,
    /// Regexes that reject the path or the full URL.
    pub exclude_regex: Vec<regex::Regex>,
}

impl PathFilter {
    /// Build a filter from raw CLI/XDG path prefix lists.
    pub fn from_lists(include: &[String], exclude: &[String]) -> Self {
        Self::from_lists_with_regex(include, exclude, &[], &[]).unwrap_or_default()
    }

    /// Prefix lists plus compiled include/exclude regular expressions.
    pub fn from_lists_with_regex(
        include: &[String],
        exclude: &[String],
        include_regex: &[String],
        exclude_regex: &[String],
    ) -> Result<Self, String> {
        Ok(Self {
            include: include
                .iter()
                .map(|s| normalize_prefix(s))
                .filter(|s| !s.is_empty())
                .collect(),
            exclude: exclude
                .iter()
                .map(|s| normalize_prefix(s))
                .filter(|s| !s.is_empty())
                .collect(),
            include_regex: compile_regexes(include_regex, "include-regex")?,
            exclude_regex: compile_regexes(exclude_regex, "exclude-regex")?,
        })
    }

    /// Return true when the URL path is allowed by include/exclude rules.
    pub fn allows_url(&self, url: &str) -> bool {
        let path = match Url::parse(url) {
            Ok(u) => {
                let mut p = u.path().to_string();
                if p.is_empty() {
                    p = "/".into();
                }
                p
            }
            Err(_) => return false,
        };
        self.allows_target(&path, url)
    }

    /// Return true when the absolute path is allowed by include/exclude rules.
    pub fn allows_path(&self, path: &str) -> bool {
        let path = if path.is_empty() { "/" } else { path };
        self.allows_target(path, path)
    }

    fn allows_target(&self, path: &str, url: &str) -> bool {
        if self.exclude.iter().any(|ex| path_matches(path, ex)) {
            return false;
        }
        if self
            .exclude_regex
            .iter()
            .any(|re| re.is_match(path) || re.is_match(url))
        {
            return false;
        }
        if self.include.is_empty() && self.include_regex.is_empty() {
            return true;
        }
        self.include.iter().any(|inc| path_matches(path, inc))
            || self
                .include_regex
                .iter()
                .any(|re| re.is_match(path) || re.is_match(url))
    }

    /// True when both include and exclude lists are empty.
    pub fn is_empty(&self) -> bool {
        self.include.is_empty()
            && self.exclude.is_empty()
            && self.include_regex.is_empty()
            && self.exclude_regex.is_empty()
    }
}

fn compile_regexes(raw: &[String], flag: &str) -> Result<Vec<regex::Regex>, String> {
    let mut out = Vec::with_capacity(raw.len());
    for pat in raw {
        let pat = pat.trim();
        if pat.is_empty() {
            continue;
        }
        match regex::Regex::new(pat) {
            Ok(re) => out.push(re),
            Err(e) => return Err(format!("{flag} is not a valid regex: {e}")),
        }
    }
    Ok(out)
}

fn normalize_prefix(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return String::new();
    }
    if s.starts_with('/') {
        s.to_string()
    } else {
        format!("/{s}")
    }
}

fn path_matches(path: &str, pattern: &str) -> bool {
    // Prefix match; trailing * means prefix without requiring rest.
    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.starts_with(prefix);
    }
    path == pattern || path.starts_with(&format!("{pattern}/")) || path.starts_with(pattern)
}

/// Strip URL fragment for crawl dedup.
pub fn normalize_url_for_dedup(url: &str) -> String {
    normalize_url_for_dedup_ex(url, false)
}

/// Strip fragment and optionally query parameters for crawl dedup.
///
/// Also collapses trailing slash on non-root paths so `example.com` and
/// `example.com/` dedup as one agent row.
pub fn normalize_url_for_dedup_ex(url: &str, ignore_query: bool) -> String {
    match Url::parse(url) {
        Ok(mut u) => {
            u.set_fragment(None);
            if ignore_query {
                u.set_query(None);
            }
            // Normalize empty path to /
            if u.path().is_empty() {
                u.set_path("/");
            } else if u.path() != "/" && u.path().ends_with('/') {
                let p = u.path().trim_end_matches('/').to_string();
                u.set_path(&p);
            }
            u.to_string()
        }
        Err(_) => url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn include_docs_only() {
        let f = PathFilter::from_lists(&["/docs".into()], &[]);
        assert!(f.allows_url("https://ex.com/docs/a"));
        assert!(!f.allows_url("https://ex.com/blog"));
    }

    #[test]
    fn exclude_admin() {
        let f = PathFilter::from_lists(&[], &["/admin".into()]);
        assert!(!f.allows_url("https://ex.com/admin/x"));
        assert!(f.allows_url("https://ex.com/public"));
    }

    #[test]
    fn include_regex_matches_path() {
        let f = PathFilter::from_lists_with_regex(&[], &[], &[r"^/docs/.*\.md$".into()], &[])
            .expect("regex");
        assert!(f.allows_url("https://ex.com/docs/a.md"));
        assert!(!f.allows_url("https://ex.com/docs/a.html"));
    }

    #[test]
    fn exclude_regex_wins() {
        let f = PathFilter::from_lists_with_regex(&["/docs".into()], &[], &[], &[r"secret".into()])
            .expect("regex");
        assert!(f.allows_url("https://ex.com/docs/a"));
        assert!(!f.allows_url("https://ex.com/docs/secret"));
    }

    #[test]
    fn bad_regex_is_an_error() {
        assert!(PathFilter::from_lists_with_regex(&[], &[], &["(".into()], &[]).is_err());
    }

    #[test]
    fn dedup_strips_fragment() {
        assert_eq!(
            normalize_url_for_dedup("https://ex.com/a#frag"),
            "https://ex.com/a"
        );
    }
}
