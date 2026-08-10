// SPDX-License-Identifier: MIT OR Apache-2.0
//! Visit the origin root before the target URL.
//!
//! # What a cold deep hit looks like from the server
//!
//! A browser almost never asks for `/deep/article/42` first. It lands on the
//! origin root, collects whatever cookies the edge sets, and follows a link —
//! so by the time the deep URL is requested there is a session cookie and a
//! `Referer`. A one-shot client that opens on the deep URL has neither, and
//! several challenge systems treat "no session, no referrer, straight to the
//! interior" as a distinct pattern rather than as a missing detail.
//!
//! # Why this is opt-in
//!
//! It doubles the request count and adds a round trip per target. That is a
//! real cost the caller has to accept, so it happens under `--warmup` or XDG
//! `warmup` and never by default.
//!
//! # Why failure here is not failure of the run
//!
//! The warm-up is a preparation, not a result. If the root is 404, redirects
//! elsewhere, or times out, the target request is still the request the caller
//! asked for. Failing the run because a preparatory GET failed would break
//! working scrapes on sites whose root simply is not interesting.
//!
//! # What carries over, and what does not
//!
//! The shared client keeps a cookie jar for the length of this process, so a
//! cookie set on the root does reach the target. The jar does NOT survive the
//! process — this is a one-shot CLI, and DIE takes the session with it. A
//! caller that needs a session across invocations uses `storage export` and
//! `storage import`; the scrape envelope says so through
//! `cookie_jar_persistent: false`.
//!
//! Until the `cookies` feature was enabled on the HTTP client, the paragraph
//! above was aspiration rather than description: this module documented a
//! cookie side effect the client had no jar to produce, so the warm-up was
//! paying for a round trip and buying only a warm connection pool.

use url::Url;

/// Fetch the origin root once so the client carries cookies into the target.
///
/// No-op when `--warmup` is off, when the URL has no parseable origin, or when
/// the target IS the root — warming a URL with itself would double the load on
/// the exact page being measured and change nothing.
///
/// `--warmup-url` replaces the computed root. The self-warm guard still
/// applies: an explicit URL equal to the target is the same wasted round trip
/// as an implicit one.
pub(crate) async fn warm_origin(target: &str) {
    if !crate::browser_policy::warmup_enabled() {
        return;
    }
    let Some(root) = crate::browser_policy::warmup_url()
        .map(str::to_string)
        .or_else(|| origin_root(target))
    else {
        return;
    };
    if root == target {
        return;
    }
    let Ok(client) = crate::robots::shared_http_client() else {
        return;
    };

    tracing::debug!(
        target: "browser_automation_cli::scrape",
        root = %root,
        "warming origin before target"
    );

    // The response body is dropped on purpose. What this call is for is the
    // side effect on the client's cookie store and connection pool, not the
    // content; buffering a homepage that nobody reads would be pure cost.
    match client.get(&root).send().await {
        Ok(resp) => {
            tracing::debug!(
                target: "browser_automation_cli::scrape",
                root = %root,
                status = resp.status().as_u16(),
                "warm-up complete"
            );
        }
        Err(e) => {
            // stderr-only: a failed preparation must not appear in the envelope
            // as if it were a property of the target.
            tracing::warn!(
                target: "browser_automation_cli::scrape",
                root = %root,
                error = %e,
                "warm-up request failed; continuing to the target"
            );
        }
    }
}

/// `https://host:port/` for a URL, or `None` when it has no usable origin.
///
/// Exposed to the crate so the browser engine computes the warm-up target the
/// same way the HTTP engine does. Two implementations of "the origin root"
/// would drift, and the drift would show up as one engine warming a URL the
/// other does not.
pub(crate) fn origin_root_of(target: &str) -> Option<String> {
    origin_root(target)
}

/// `https://host:port/` for a URL, or `None` when it has no usable origin.
fn origin_root(target: &str) -> Option<String> {
    let parsed = Url::parse(target).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    parsed.host_str()?;
    let mut root = parsed;
    root.set_path("/");
    root.set_query(None);
    root.set_fragment(None);
    Some(root.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_strips_path_query_and_fragment() {
        assert_eq!(
            origin_root("https://example.com/a/b?c=1#d").as_deref(),
            Some("https://example.com/")
        );
    }

    #[test]
    fn port_is_part_of_the_origin() {
        assert_eq!(
            origin_root("http://example.com:8080/x").as_deref(),
            Some("http://example.com:8080/")
        );
    }

    #[test]
    fn non_http_schemes_have_no_origin_to_warm() {
        assert!(origin_root("file:///etc/hosts").is_none());
        assert!(origin_root("data:text/plain,x").is_none());
        assert!(origin_root("not a url").is_none());
    }

    #[test]
    fn a_root_url_warms_to_itself_and_is_skipped_by_the_caller() {
        // Documents the equality `warm_origin` relies on to avoid doubling the
        // load on the very page being fetched.
        assert_eq!(
            origin_root("https://example.com/").as_deref(),
            Some("https://example.com/")
        );
    }
}
