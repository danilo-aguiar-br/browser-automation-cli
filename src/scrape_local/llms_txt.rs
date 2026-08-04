// SPDX-License-Identifier: MIT OR Apache-2.0
//! Render a crawl envelope as an `llms.txt` document.
//!
//! # What this is
//!
//! `llms.txt` is a site-root Markdown file that tells a model what a site
//! contains and where to read more, instead of making it crawl and guess. The
//! shape is fixed:
//!
//! - exactly one H1 — the site or project name, the only required element
//! - an optional blockquote holding the one-paragraph summary
//! - optional prose with no headings, for context that does not fit a list
//! - zero or more H2 sections, each a list of `- [name](url): note` entries
//! - an optional final H2 named `Optional`, whose links may be dropped first
//!   when the reader is short on budget
//!
//! # Why it lives behind `crawl --output-mode`
//!
//! Producing this file is a serialisation of a whole crawl, not a per-page
//! format and not a new capability: the walking, the robots policy, the host
//! scoping and the page budget are all `crawl` already. Adding a top-level
//! command would have duplicated that pipeline to change only the last step.
//!
//! `--format metadata` is what makes the output useful, because that is where
//! titles and descriptions come from. Without it every entry degrades to its
//! URL path, which is still valid `llms.txt` and still honest — the renderer
//! never invents a title it was not given.

use serde_json::Value;

use crate::error::CliError;

/// Entries beyond this depth are filed under `Optional` rather than dropped.
///
/// The spec treats `Optional` as the section a reader discards first under
/// budget pressure. Deep pages are the ones most likely to be incidental, so
/// they land there instead of competing with the seed's immediate neighbours.
const OPTIONAL_DEPTH: u64 = 2;

/// Render `data` (a crawl envelope) as an `llms.txt` document on stdout.
///
/// `seed` names the crawl origin and supplies the H1 when no page offers a
/// better title.
pub fn emit_llms_txt(data: &Value, seed: &str) -> Result<(), CliError> {
    let pages = data
        .get("pages")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);

    let host = host_of(seed);
    let (title, summary) = seed_identity(pages, seed, &host);

    let mut out = String::with_capacity(1024);
    out.push_str(&format!("# {title}\n"));
    if let Some(summary) = summary {
        out.push_str(&format!("\n> {summary}\n"));
    }

    let mut primary = Vec::new();
    let mut optional = Vec::new();
    for page in pages {
        let Some(url) = page_url(page) else {
            continue;
        };
        let depth = page.get("depth").and_then(Value::as_u64).unwrap_or(0);
        let entry = render_entry(page, url);
        if depth > OPTIONAL_DEPTH {
            optional.push(entry);
        } else {
            primary.push(entry);
        }
    }

    if !primary.is_empty() {
        out.push_str("\n## Pages\n\n");
        for line in &primary {
            out.push_str(line);
        }
    }
    if !optional.is_empty() {
        out.push_str("\n## Optional\n\n");
        for line in &optional {
            out.push_str(line);
        }
    }

    crate::output::writeln_stdout(out.trim_end())
}

/// One `- [name](url): note` line, with the note omitted when absent.
///
/// A missing description yields no colon and no filler text: an entry that
/// says nothing is more useful than one that says something invented.
fn render_entry(page: &Value, url: &str) -> String {
    let name = page_title(page).map_or_else(|| path_label(url), |t| t);
    match page_description(page) {
        Some(note) => format!("- [{name}]({url}): {note}\n"),
        None => format!("- [{name}]({url})\n"),
    }
}

/// A crawl page carries its address as `source_url`; `url` is accepted too so
/// the renderer also works on envelopes shaped by `map` or a future producer.
fn page_url(page: &Value) -> Option<&str> {
    page.get("source_url")
        .or_else(|| page.get("url"))
        .and_then(Value::as_str)
}

/// Title from the page, or from its `metadata` block when `--format metadata`
/// is the only format that carried one.
fn page_title(page: &Value) -> Option<String> {
    nested_str(page, "title")
}

/// Description, falling back to `summary` for `--format summary` crawls.
fn page_description(page: &Value) -> Option<String> {
    nested_str(page, "description").or_else(|| nested_str(page, "summary"))
}

/// Read `key` from the page, then from `page.metadata`, collapsing whitespace.
///
/// Formats disagree about where they put things: `metadata` nests, `summary`
/// promotes to the top level. Looking in both is cheaper than teaching the
/// renderer which format produced the envelope.
fn nested_str(page: &Value, key: &str) -> Option<String> {
    page.get(key)
        .or_else(|| page.get("metadata").and_then(|m| m.get(key)))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(collapse_ws)
}

/// H1 text and blockquote summary, taken from the seed page.
///
/// The seed is identified by `depth == 0` rather than by comparing URLs: a
/// seed written as `https://example.com` is normalised to `https://example.com/`
/// before the fetch, and a string comparison would silently miss it.
fn seed_identity(pages: &[Value], _seed: &str, host: &str) -> (String, Option<String>) {
    let seed_page = pages
        .iter()
        .find(|p| p.get("depth").and_then(Value::as_u64) == Some(0))
        .or_else(|| pages.first());

    let title = seed_page
        .and_then(page_title)
        .unwrap_or_else(|| host.to_string());
    let summary = seed_page.and_then(page_description);
    (title, summary)
}

/// Host of `url`, or the raw string when it does not parse.
fn host_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| url.to_string())
}

/// Last path segment as a human label, falling back to the host.
fn path_label(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.path_segments()
                .and_then(|segs| {
                    segs.filter(|s| !s.is_empty())
                        .next_back()
                        .map(str::to_string)
                })
                .or_else(|| u.host_str().map(str::to_string))
        })
        .unwrap_or_else(|| url.to_string())
}

/// Fold newlines and runs of whitespace so one entry stays on one line.
///
/// A description lifted from a page can contain hard newlines, and a newline
/// inside a list item silently ends the item for every Markdown parser.
fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc(data: &Value, seed: &str) -> String {
        let pages = data.get("pages").and_then(Value::as_array).unwrap();
        let host = host_of(seed);
        let (title, summary) = seed_identity(pages, seed, &host);
        let mut out = format!("# {title}\n");
        if let Some(s) = summary {
            out.push_str(&format!("\n> {s}\n"));
        }
        for p in pages {
            let url = page_url(p).unwrap();
            out.push_str(&render_entry(p, url));
        }
        out
    }

    #[test]
    fn the_seed_page_supplies_the_h1_and_the_blockquote() {
        let data = json!({"pages": [
            {"url": "https://example.com/", "depth": 0, "title": "Example", "description": "A demo site"}
        ]});
        let rendered = doc(&data, "https://example.com/");
        assert!(rendered.starts_with("# Example\n"));
        assert!(rendered.contains("\n> A demo site\n"));
    }

    #[test]
    fn a_crawl_without_metadata_falls_back_to_host_and_path() {
        let data = json!({"pages": [{"url": "https://example.com/docs/guide"}]});
        let rendered = doc(&data, "https://example.com/");
        assert!(rendered.starts_with("# example.com\n"));
        assert!(rendered.contains("- [guide](https://example.com/docs/guide)\n"));
    }

    /// A real crawl page uses `source_url` and nests metadata; the renderer
    /// must read the shape the pipeline actually emits, not an idealised one.
    #[test]
    fn a_real_crawl_page_shape_is_understood() {
        let data = json!({"pages": [{
            "source_url": "https://example.com/",
            "depth": 0,
            "title": "Example Domain",
            "metadata": {"title": "Example Domain", "description": "Illustrative use"}
        }]});
        let rendered = doc(&data, "https://example.com");
        assert!(rendered.starts_with("# Example Domain\n"));
        assert!(rendered.contains("\n> Illustrative use\n"));
        assert!(rendered.contains("- [Example Domain](https://example.com/)"));
    }

    /// An empty metadata description must not become an empty blockquote.
    #[test]
    fn an_empty_description_is_treated_as_absent() {
        let data = json!({"pages": [{
            "source_url": "https://example.com/",
            "depth": 0,
            "title": "Example",
            "metadata": {"description": ""}
        }]});
        let rendered = doc(&data, "https://example.com");
        assert!(!rendered.contains('>'), "no blockquote: {rendered}");
    }

    #[test]
    fn an_entry_without_a_description_carries_no_colon() {
        let page = json!({"url": "https://example.com/a", "title": "A"});
        assert_eq!(
            render_entry(&page, "https://example.com/a"),
            "- [A](https://example.com/a)\n"
        );
    }

    #[test]
    fn a_multiline_description_is_folded_onto_one_line() {
        let page = json!({
            "url": "https://example.com/a",
            "title": "A",
            "description": "first line\nsecond   line"
        });
        let line = render_entry(&page, "https://example.com/a");
        assert_eq!(line.matches('\n').count(), 1, "entry must be one line");
        assert!(line.contains("first line second line"));
    }

    #[test]
    fn a_title_with_newlines_cannot_break_the_list_item() {
        let page = json!({"url": "https://example.com/a", "title": "Two\nLines"});
        let line = render_entry(&page, "https://example.com/a");
        assert!(line.contains("[Two Lines]"));
        assert_eq!(line.matches('\n').count(), 1);
    }
}
