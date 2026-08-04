// SPDX-License-Identifier: MIT OR Apache-2.0
//! RSS / Atom / JSON Feed extraction for scrape `--format feed`.
//!
//! Parsing is delegated to `feed-rs`, which covers RSS 0.9/1.0/2.0, Atom and
//! JSON Feed behind one model. This module only projects that model onto the
//! agent envelope: every field is omitted when absent (CLEAN STDOUT — never a
//! dead `null`), and the entry list is capped by XDG `scrape_feed_max_entries`.

use serde_json::{json, Map, Value};

/// Build the `feed` payload from a raw response body.
///
/// `source_url` is only used to resolve nothing — `feed-rs` already yields
/// absolute links — but it is echoed so a projected row stays self-describing.
/// A body that is not a feed yields `{"found": false}` rather than an error, so
/// a mixed-format crawl does not fail a page just because it serves HTML.
pub(crate) fn extract_feed(body: &str, max_entries: usize) -> Value {
    let Ok(parsed) = feed_rs::parser::parse(body.as_bytes()) else {
        return json!({ "found": false });
    };

    let mut out = Map::new();
    out.insert("found".into(), json!(true));
    out.insert("feed_type".into(), json!(feed_type_str(&parsed.feed_type)));
    insert_if_some(&mut out, "title", parsed.title.map(|t| t.content));
    insert_if_some(
        &mut out,
        "description",
        parsed.description.map(|t| t.content),
    );
    insert_if_some(&mut out, "updated", parsed.updated.map(|d| d.to_rfc3339()));
    insert_if_some(&mut out, "language", parsed.language);
    if let Some(link) = first_link(&parsed.links) {
        out.insert("home_page_url".into(), json!(link));
    }

    let total = parsed.entries.len();
    let entries: Vec<Value> = parsed
        .entries
        .into_iter()
        .take(max_entries)
        .map(entry_to_value)
        .collect();
    out.insert("entry_count".into(), json!(entries.len()));
    out.insert("entry_total".into(), json!(total));
    out.insert("truncated".into(), json!(total > entries.len()));
    out.insert("entries".into(), Value::Array(entries));
    Value::Object(out)
}

/// Project one feed entry onto title / link / published / author / summary.
fn entry_to_value(entry: feed_rs::model::Entry) -> Value {
    let mut item = Map::new();
    insert_if_some(&mut item, "title", entry.title.map(|t| t.content));
    if let Some(link) = first_link(&entry.links) {
        item.insert("url".into(), json!(link));
    }
    insert_if_some(
        &mut item,
        "published",
        entry.published.or(entry.updated).map(|d| d.to_rfc3339()),
    );
    let authors: Vec<String> = entry
        .authors
        .into_iter()
        .map(|p| p.name)
        .filter(|n| !n.trim().is_empty())
        .collect();
    if !authors.is_empty() {
        item.insert("authors".into(), json!(authors));
    }
    insert_if_some(&mut item, "summary", entry.summary.map(|t| t.content));
    if !entry.id.trim().is_empty() {
        item.insert("id".into(), json!(entry.id));
    }
    let categories: Vec<String> = entry
        .categories
        .into_iter()
        .map(|c| c.term)
        .filter(|t| !t.trim().is_empty())
        .collect();
    if !categories.is_empty() {
        item.insert("categories".into(), json!(categories));
    }
    Value::Object(item)
}

/// Stable lowercase discriminant for the parsed feed dialect.
fn feed_type_str(t: &feed_rs::model::FeedType) -> &'static str {
    use feed_rs::model::FeedType;
    match t {
        FeedType::Atom => "atom",
        FeedType::JSON => "json",
        FeedType::RSS0 => "rss0",
        FeedType::RSS1 => "rss1",
        FeedType::RSS2 => "rss2",
    }
}

/// First non-empty link href, preferring `rel="alternate"` when one exists.
fn first_link(links: &[feed_rs::model::Link]) -> Option<String> {
    links
        .iter()
        .find(|l| l.rel.as_deref() == Some("alternate") && !l.href.trim().is_empty())
        .or_else(|| links.iter().find(|l| !l.href.trim().is_empty()))
        .map(|l| l.href.clone())
}

/// Insert a key only when the value is present and non-blank (CLEAN STDOUT).
fn insert_if_some(map: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(v) = value {
        if !v.trim().is_empty() {
            map.insert(key.into(), json!(v));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RSS: &str = r#"<?xml version="1.0"?>
<rss version="2.0"><channel>
  <title>Example Blog</title>
  <link>https://example.com/</link>
  <description>Notes</description>
  <item>
    <title>First post</title>
    <link>https://example.com/1</link>
    <pubDate>Mon, 02 Jan 2006 15:04:05 GMT</pubDate>
    <author>ada@example.com (Ada)</author>
    <description>Hello</description>
  </item>
  <item>
    <title>Second post</title>
    <link>https://example.com/2</link>
  </item>
</channel></rss>"#;

    const ATOM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Atom Example</title>
  <updated>2006-01-02T15:04:05Z</updated>
  <id>urn:uuid:1</id>
  <entry>
    <title>Atom entry</title>
    <link href="https://example.com/a"/>
    <id>urn:uuid:2</id>
    <updated>2006-01-02T15:04:05Z</updated>
    <author><name>Grace</name></author>
    <summary>Body</summary>
  </entry>
</feed>"#;

    #[test]
    fn rss2_entries_are_projected() {
        let v = extract_feed(RSS, 50);
        assert_eq!(v["found"], json!(true));
        assert_eq!(v["feed_type"], json!("rss2"));
        assert_eq!(v["title"], json!("Example Blog"));
        assert_eq!(v["entry_count"], json!(2));
        assert_eq!(v["truncated"], json!(false));
        assert_eq!(v["entries"][0]["title"], json!("First post"));
        assert_eq!(v["entries"][0]["url"], json!("https://example.com/1"));
        assert!(v["entries"][0]["published"].is_string());
    }

    #[test]
    fn atom_author_and_summary() {
        let v = extract_feed(ATOM, 50);
        assert_eq!(v["feed_type"], json!("atom"));
        assert_eq!(v["entries"][0]["authors"], json!(["Grace"]));
        assert_eq!(v["entries"][0]["summary"], json!("Body"));
    }

    #[test]
    fn max_entries_caps_and_flags_truncation() {
        let v = extract_feed(RSS, 1);
        assert_eq!(v["entry_count"], json!(1));
        assert_eq!(v["entry_total"], json!(2));
        assert_eq!(v["truncated"], json!(true));
    }

    #[test]
    fn non_feed_body_is_not_an_error() {
        let v = extract_feed("<html><body>not a feed</body></html>", 50);
        assert_eq!(v["found"], json!(false));
        assert!(v.get("entries").is_none());
    }

    #[test]
    fn absent_fields_are_omitted_never_null() {
        let v = extract_feed(RSS, 50);
        // Second item has no pubDate / author / description.
        let second = &v["entries"][1];
        assert!(second.get("published").is_none());
        assert!(second.get("authors").is_none());
        assert!(second.get("summary").is_none());
        assert_eq!(second["title"], json!("Second post"));
    }
}
