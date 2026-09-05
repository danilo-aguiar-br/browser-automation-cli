// SPDX-License-Identifier: MIT OR Apache-2.0
//! The one spelling of the fetched URL, decided before any branch runs.
//!
//! # Why this is not inside the fetch
//!
//! [`super::scrape_http`] is an async function that talks to the network, the
//! cache and the robots policy, and it cannot be exercised without all three.
//! The rule below is pure string-to-string and is the part that was WRONG, so
//! it is the part that has to be testable on its own. Keeping it next to the
//! I/O is how a normalisation defect survives a suite that only tests the
//! fetch: nothing in this file needs a socket to prove it.

/// The URL in the one spelling every branch of `scrape_http` reports.
///
/// # The defect this closes
///
/// `source_url` had two producers. The network path reported `final_url` from
/// the response, which `Url` had already serialised, so `https://example.com`
/// came back as `https://example.com/`. Both cache branches reported the raw
/// argv string instead, which came back without the trailing slash. Measured:
/// the first collection answered `change_status: fresh` with the slash and
/// every later one answered `unchanged` without it, from the same field of the
/// same envelope, with nothing marking the change. A consumer deduplicating on
/// `source_url` counted one page as two.
///
/// Normalising here rather than at each producer also gives the cache key one
/// spelling, so the two grafias stop occupying two entries for one page.
///
/// # Not the crawl normaliser
///
/// [`super::super::path_filter::normalize_url_for_dedup_ex`] exists and
/// is NOT used here on purpose: it strips the trailing slash from non-root
/// paths to make crawl dedup aggressive. That is the right rule for deciding
/// "did I visit this already" and the wrong one for reporting which URL was
/// fetched, since it would report an address the request never used. An
/// unparseable URL is returned untouched: this function reports, it never
/// rejects.
pub(super) fn canonical_request_url(url: &str) -> String {
    url::Url::parse(url).map_or_else(|_| url.to_string(), |u| u.to_string())
}

#[cfg(test)]
mod tests {
    use super::canonical_request_url;

    /// The measured NC-03: one page, two spellings, depending on the branch.
    #[test]
    fn a_bare_origin_gains_the_slash_the_response_would_have_reported() {
        assert_eq!(
            canonical_request_url("https://example.com"),
            "https://example.com/"
        );
        assert_eq!(
            canonical_request_url("https://example.com/"),
            "https://example.com/"
        );
    }

    /// Both spellings must converge, or the cache branch and the network branch
    /// keep reporting different `source_url` for the same page.
    #[test]
    fn both_spellings_of_one_origin_converge() {
        assert_eq!(
            canonical_request_url("https://example.com"),
            canonical_request_url("https://example.com/")
        );
    }

    /// NOT the crawl dedup rule: a non-root path keeps the slash it was asked
    /// with, because this value reports which URL was fetched.
    #[test]
    fn a_non_root_path_is_not_stripped_like_crawl_dedup_would() {
        assert_eq!(
            canonical_request_url("https://example.com/docs/"),
            "https://example.com/docs/"
        );
        assert_eq!(
            canonical_request_url("https://example.com/docs"),
            "https://example.com/docs"
        );
    }

    #[test]
    fn query_and_fragment_survive() {
        assert_eq!(
            canonical_request_url("https://example.com/a?b=1&c=2#frag"),
            "https://example.com/a?b=1&c=2#frag"
        );
    }

    /// This function reports, it never rejects: rejection is `scheme`'s job.
    #[test]
    fn an_unparseable_url_is_returned_untouched() {
        assert_eq!(canonical_request_url("not a url"), "not a url");
        assert_eq!(canonical_request_url(""), "");
    }
}
