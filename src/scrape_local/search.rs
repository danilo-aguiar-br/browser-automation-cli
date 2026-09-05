// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local HTTP search (public HTML SERP; no SaaS key).

use serde_json::{json, Value};
use url::Url;

use crate::error::CliError;
use crate::robots::RobotsPolicy;

use super::crawl_map::map_http;
use super::http::scrape_http;
use super::path_filter::PathFilter;
use super::types::{ScrapeFormat, ScrapeOpts};

/// Region and recency narrowing for a SERP query.
///
/// Grouped rather than passed loose because all three share one fate: they are
/// endpoint parameters, so they either all reach a SERP that understands them or
/// none of them does. Keeping them together makes that check a single test.
///
/// Private to this module: the caller passes the three values and this type is
/// assembled here, so the endpoint's parameter model stays where the endpoint is
/// known rather than leaking into the command layer.
#[derive(Debug, Clone, Copy, Default)]
struct SearchDimensions<'a> {
    /// Country code for the region key (`br`, `us`, ...).
    country: Option<&'a str>,
    /// Language code for the region key (`pt`, `en`, ...).
    lang: Option<&'a str>,
    /// Recency window: `d`, `w`, `m` or `y`.
    time_filter: Option<&'a str>,
}

impl SearchDimensions<'_> {
    /// Did the caller ask to narrow the search at all?
    fn is_empty(&self) -> bool {
        self.country.is_none() && self.lang.is_none() && self.time_filter.is_none()
    }

    /// Is exactly one half of the composed region key present?
    ///
    /// `kl` is a single key holding `country-lang`, so half of it is not a
    /// narrower request — it is a value the endpoint may or may not recognise,
    /// and we cannot tell which from here. A flag that might quietly do nothing
    /// reads, from the caller's side, exactly like one that worked.
    fn has_half_region(&self) -> bool {
        self.country.is_some() != self.lang.is_some()
    }

    /// Query-string pairs for the DuckDuckGo HTML endpoint.
    ///
    /// Region is emitted only when BOTH halves are known; `has_half_region` is
    /// refused before this runs, so the partial arms are unreachable rather than
    /// silently dropped here.
    fn to_query_pairs(self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        if let (Some(c), Some(l)) = (self.country, self.lang) {
            pairs.push((
                "kl",
                format!("{}-{}", c.to_ascii_lowercase(), l.to_ascii_lowercase()),
            ));
        }
        if let Some(df) = self.time_filter {
            pairs.push(("df", df.to_ascii_lowercase()));
        }
        pairs
    }
}

/// The separator that joins `search_base_url` to the first query parameter.
///
/// # Why this is not a plain `/?`
///
/// Until 0.1.9 the caller's base was `trim_end_matches('/')`-ed and then joined
/// with a literal `/?`. That is idempotent for exactly one value — the compiled
/// default `https://html.duckduckgo.com/html/`, whose path already ends in a
/// slash — and wrong for every other endpoint. Measured: a base of
/// `https://www.google.com/search` became `https://www.google.com/search/?q=`
/// and answered 404, and `https://httpbin.org/get` became
/// `https://httpbin.org/get/?q=` and did the same. So the key was documented,
/// configurable, and usable only at its factory value.
///
/// The path belongs to the caller. This function adds a separator and never a
/// path segment: `?` when the base carries no query, `&` when it already does,
/// and nothing at all when the base already ends in the character that would
/// open or continue one.
fn query_separator_for(base: &str) -> &'static str {
    if base.ends_with('?') || base.ends_with('&') {
        // The caller already wrote the separator; adding another makes an
        // empty parameter that some servers reject and others silently keep.
        ""
    } else if base.contains('?') {
        "&"
    } else {
        "?"
    }
}

/// Does this base URL point at the SERP whose parameter names we know?
///
/// Compared against the host of the compiled default rather than a second
/// hard-coded string, so retargeting the product means editing one constant.
/// Host of a URL, lowercased, or `None` when it does not parse.
///
/// Shared by the dimension gate and the SERP-chrome filter on purpose: two
/// separate host resolutions would drift, and the filter would then keep links
/// the gate considers same-host.
fn host_of(u: &str) -> Option<String> {
    Url::parse(u)
        .ok()
        .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
}

fn endpoint_understands_dimensions(base: &str) -> bool {
    match (
        host_of(base),
        host_of(crate::constants::DEFAULT_SEARCH_BASE_URL),
    ) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// Local search: fetch a public HTML search page or treat query as URL list seed.
/// MVP: if query looks like URL, map it; else use DuckDuckGo HTML (optional network).
///
/// # Errors
///
/// Returns [`CliError`] with [`crate::error::ErrorKind::Usage`] in three cases,
/// all of them the same rule: a narrowing that might silently not narrow is
/// worse than a refusal, because the caller cannot tell the two apart.
///
/// 1. The query is a URL, so it is answered by mapping that site and never
///    reaches a search endpoint that could carry these parameters.
/// 2. Only one half of the composed region key was given — `--country` without
///    `--search-lang`, or the reverse.
/// 3. `search_base_url` points at an endpoint whose parameter names are unknown,
///    where `kl` and `df` would be ignored or mean something else entirely.
///
/// Returns [`crate::error::ErrorKind::Data`] when every link the endpoint served
/// was its own navigation chrome, because `count: 0` would read as an answer.
///
/// Also propagates fetch failures from the underlying HTTP scrape.
pub async fn search_http(
    query: &str,
    robots: RobotsPolicy,
    limit: usize,
    country: Option<&str>,
    lang: Option<&str>,
    time_filter: Option<&str>,
) -> Result<Value, CliError> {
    let dims = SearchDimensions {
        country,
        lang,
        time_filter,
    };
    let limit = limit.clamp(
        1,
        crate::xdg::policy::policy_usize(crate::xdg::policy::key::SCRAPE_SEARCH_LIMIT_MAX),
    );
    let q = query.trim();
    if q.starts_with("http://") || q.starts_with("https://") {
        // A URL query is answered by walking that site, not by asking a SERP, so
        // there is no endpoint to carry these parameters. Refusing beats letting
        // them parse and vanish on a path the caller cannot see they took.
        if !dims.is_empty() {
            return Err(CliError::new(
                crate::error::ErrorKind::Usage,
                "--country / --search-lang / --time-filter do not apply when the query is a URL: \
                 that path maps the site directly and never reaches a search endpoint",
            ));
        }
        return map_http(
            q,
            robots,
            limit,
            1,
            &PathFilter::default(),
            crate::xdg::resolve_scrape_use_sitemap(),
            None,
        )
        .await;
    }
    // Endpoint from XDG `search_base_url` or named const DEFAULT_SEARCH_BASE_URL.
    if dims.has_half_region() {
        return Err(CliError::new(
            crate::error::ErrorKind::Usage,
            "--country and --search-lang travel together: the endpoint expresses region as one \
             composed key, so half of it is a value we cannot confirm it honours; pass both",
        ));
    }
    let base = crate::xdg::search_base_url();
    // Classified HERE, where the endpoint is DECIDED, and never where a payload
    // is emitted. The previous placement computed this after the empty-result
    // `return Err`, so the failure envelope was structurally unable to carry it
    // and every later branch of this function would have been born blind for
    // the same reason.
    let endpoint_known = endpoint_understands_dimensions(&base);
    let serp_endpoint = if endpoint_known { "known" } else { "unknown" };
    if !dims.is_empty() && !endpoint_known {
        // Same provenance pair as the two envelopes below. Presence of the
        // field is a property of this FUNCTION, not of one lucky branch: an
        // agent told to read `data.serp_endpoint` must find it on every exit
        // that had already resolved `base`.
        return Err(CliError::new(
            crate::error::ErrorKind::Usage,
            format!(
                "--country / --search-lang / --time-filter need the default search endpoint; \
                 configured search_base_url is {base}, whose query parameters are unknown"
            ),
        )
        .with_data(json!({
            "serp_endpoint": serp_endpoint,
            "search_base_url": base,
        })));
    }
    let mut search_url = format!(
        "{base}{}q={}",
        query_separator_for(&base),
        urlencoding::encode(q)
    );
    for (key, value) in dims.to_query_pairs() {
        search_url.push_str(&format!("&{key}={}", urlencoding::encode(&value)));
    }
    let opts = ScrapeOpts {
        format: ScrapeFormat::Links,
        engine: "http".into(),
        ..ScrapeOpts::default()
    };
    let page = scrape_http(&search_url, robots, &opts).await?;
    let mut results = Vec::new();
    if let Some(links) = page.get("links").and_then(|v| v.as_array()) {
        for l in links {
            let raw = l
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let text = l
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let clean = clean_serp_url(&raw);
            // Drop same-host SERP chrome and empty destinations.
            if clean.is_empty() {
                continue;
            }
            // A link whose host is the configured endpoint's own host is
            // navigation chrome, never an organic hit. Deriving the host from
            // `base` keeps the filter honest when the operator retargets
            // `search_base_url`, where a hard-coded host silently let the new
            // engine's own menu links through as results.
            match (host_of(&clean), host_of(&base)) {
                (Some(hit), Some(serp)) if hit != serp => {}
                _ => continue,
            }
            results.push(json!({ "text": text, "url": clean }));
            if results.len() >= limit {
                break;
            }
        }
    }
    // An empty set here is an endpoint mismatch, not an answer: `ok: true` with
    // `count: 0` reads to the caller as "the web has nothing", which is the one
    // conclusion this path cannot support.
    if results.is_empty() {
        // The SAME `serp_endpoint` the success envelope publishes, attached to
        // the failure so one field answers the question on BOTH branches. Read
        // as prose, an unknown endpoint and a genuinely empty web are the same
        // sentence; read as `data`, they are two different diagnoses.
        return Err(CliError::new(
            crate::error::ErrorKind::Data,
            format!(
                "no organic result survived the same-host SERP filter; configured \
                 search_base_url is {base}, which returned only its own navigation links"
            ),
        )
        .with_data(json!({
            "serp_endpoint": serp_endpoint,
            "search_base_url": base,
        })));
    }
    Ok(json!({
        "query": q,
        "count": results.len(),
        "results": results,
        "serp_endpoint": serp_endpoint,
        "source_url": search_url,
        "robots_policy": robots.as_str(),
        "engine": "http",
        "note": "local HTTP search via public HTML SERP; no SaaS API key",
    }))
}

/// Unwrap SERP redirect wrappers (e.g. uddg=) into destination URLs.
pub(crate) fn clean_serp_url(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return String::new();
    }
    if let Ok(u) = Url::parse(raw) {
        // Redirect wrappers, by PARAMETER rather than by host: `uddg` is
        // DuckDuckGo's spelling, `u` and `url` are what most other SERPs use,
        // and none of the three means anything except "the real destination is
        // in here". Keying on the parameter is what lets an operator point
        // `search_base_url` at another endpoint and still get destinations.
        for (k, v) in u.query_pairs() {
            if k == "uddg" || k == "u" || k == "url" {
                let decoded = urlencoding::decode(&v).unwrap_or_else(|_| v.clone());
                let s = decoded.into_owned();
                if s.starts_with("http://") || s.starts_with("https://") {
                    return s;
                }
            }
        }
    }
    // No wrapper parameter: the URL is already the destination. There used to
    // be a host check for `duckduckgo.com` here whose two branches BOTH
    // returned `raw`, so it decided nothing while looking like it decided
    // something — the same embedded-host defect as the SERP filter above, in
    // its dead form. Removed 2026-09-04.
    raw.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The property the whole Defeito 19 fix rests on.
    ///
    /// The filter used to match two literal `duckduckgo.com` substrings, so an
    /// operator who pointed `search_base_url` anywhere else got NO filter at
    /// all and read the engine's own menu links as organic results. The fix is
    /// only a fix if the comparison is derived from the configured endpoint,
    /// and that is what this states — with a host the product has never heard
    /// of, so a literal cannot pass by accident.
    #[test]
    fn same_host_is_serp_chrome_on_an_endpoint_the_product_never_heard_of() {
        let base = "https://search.example.invalid/results";
        let menu = "https://search.example.invalid/settings?lang=pt";
        let organic = "https://docs.rs/chromiumoxide";

        assert_eq!(host_of(base), host_of(menu), "fixture must share the host");
        assert_ne!(host_of(base), host_of(organic));
    }

    /// A link with no parseable host is dropped, not kept by default.
    ///
    /// `mailto:` and bare fragments appear in real SERP markup. The old filter
    /// let them through because it only asked whether the string CONTAINED a
    /// literal, and a destination the caller cannot fetch is not a result.
    #[test]
    fn an_unparseable_destination_has_no_host_to_compare() {
        assert!(host_of("mailto:someone@example.com").is_none());
        assert!(host_of("#footer").is_none());
        assert!(host_of("").is_none());
    }

    /// Host comparison is case-insensitive, because DNS is.
    #[test]
    fn host_resolution_folds_case() {
        assert_eq!(
            host_of("https://Search.EXAMPLE.invalid/a"),
            host_of("https://search.example.invalid/b")
        );
    }

    /// Redirect unwrapping keys on the PARAMETER, never on the host.
    ///
    /// `uddg` is DuckDuckGo's spelling and `u` / `url` are what most other
    /// engines use. Keying on the host is what made the old code specific to
    /// one engine; this states that the new code is not.
    #[test]
    fn wrappers_unwrap_by_parameter_across_engines() {
        assert_eq!(
            clean_serp_url("https://any.example.invalid/l/?uddg=https%3A%2F%2Fdocs.rs%2Ffoo"),
            "https://docs.rs/foo"
        );
        assert_eq!(
            clean_serp_url("https://other.example.invalid/r?url=https%3A%2F%2Fdocs.rs%2Fbar"),
            "https://docs.rs/bar"
        );
    }

    /// A destination that is already clean survives untouched.
    ///
    /// This is the branch whose two arms both returned the input, so it
    /// asserted nothing for a whole version. Stated once, plainly.
    #[test]
    fn an_unwrapped_destination_is_returned_as_is() {
        let direct = "https://docs.rs/chromiumoxide/latest";
        assert_eq!(clean_serp_url(direct), direct);
    }
}
