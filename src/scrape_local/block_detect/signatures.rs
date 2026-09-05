// SPDX-License-Identifier: MIT OR Apache-2.0
//! The evidence tables, kept apart from the procedure that reads them.
//!
//! # Why the tables are their own file
//!
//! [`super::detect`] is a decision procedure that changes on the order of once
//! a year; these rows change every time a vendor ships a new header, a new
//! cookie or a new challenge phrase. Mixing the two puts a frequently-appended
//! list in front of the logic every reader has to re-read, and it hides the one
//! property that actually distinguishes the four tables from each other.
//!
//! That property is the whole point of the split: [`MITIGATION_HEADERS`] is the
//! only table whose rows are sufficient ON THEIR OWN. Every other row here is
//! ATTRIBUTION — it can name the vendor standing in front of an origin, and it
//! can never, by itself, prove that a wall was raised. Reading a vendor
//! fingerprint as a block flags every site behind a CDN, which is a large share
//! of the web.

/// Headers that assert an **active mitigation**, not merely a vendor.
///
/// # The distinction this table exists to enforce
///
/// Measured against `https://example.com`: it answers HTTP 200 with real content
/// and carries `cf-cache-status`. Treating a vendor header as a block flagged a
/// perfectly good page, which would have broken scraping for every site behind a
/// CDN -- a large share of the web. Presence of a WAF is not action by a WAF.
///
/// Only headers whose *semantics* are "this request was mitigated" belong here.
pub(super) const MITIGATION_HEADERS: &[(&str, &str)] =
    &[("cf-mitigated", "cloudflare"), ("x-px-block", "perimeterx")];

/// Headers that merely name the vendor in front of the origin.
///
/// Never sufficient on their own. Used only to attribute a block that some other
/// signal already proved, so the report says `cloudflare` instead of `generic`.
pub(super) const VENDOR_HEADERS: &[(&str, &str)] = &[
    ("cf-ray", "cloudflare"),
    ("cf-cache-status", "cloudflare"),
    ("x-akamai-request-id", "akamai"),
    ("x-akamai-transformed", "akamai"),
    ("x-datadome", "datadome"),
    ("x-iinfo", "imperva"),
    ("x-kpsdk-ct", "kasada"),
];

/// Cookies that name the vendor in front of the origin.
///
/// Attribution only, for the same reason as [`VENDOR_HEADERS`], and with a
/// sharper trap: `cf_clearance` is issued **after** a challenge is solved, so
/// reading it as a block inverts its meaning entirely. `__cf_bm` and `datadome`
/// ride along with ordinary traffic.
///
/// Matched as *substrings* because a `Set-Cookie` carries attributes after the
/// value.
pub(super) const VENDOR_COOKIES: &[(&str, &str)] = &[
    ("cf_clearance", "cloudflare"),
    ("__cf_bm", "cloudflare"),
    ("__cflb", "cloudflare"),
    ("ak_bmsc", "akamai"),
    ("datadome", "datadome"),
    ("_px3", "perimeterx"),
    ("_pxhd", "perimeterx"),
    ("_pxvid", "perimeterx"),
    ("incap_ses_", "imperva"),
    ("visid_incap_", "imperva"),
    ("___utmvc", "imperva"),
];

/// Challenge markers in the rendered body, lowercased.
///
/// Vendor-neutral on purpose: the phrasing is localized, and matching the vendor
/// name in the body would fire on any page that merely mentions a CDN.
pub(super) const BODY_SIGNATURES: &[(&str, &str)] = &[
    ("captcha-form", "generic"),
    ("g-recaptcha", "generic"),
    ("h-captcha", "generic"),
    ("cf-challenge-running", "cloudflare"),
    ("challenge-platform", "cloudflare"),
    ("checking your browser before accessing", "cloudflare"),
    ("unusual traffic", "generic"),
    ("tráfego incomum", "generic"),
    ("trafego incomum", "generic"),
    ("enable javascript and cookies to continue", "generic"),
    ("ative o javascript", "generic"),
    ("verifying you are human", "generic"),
    ("_imperva_", "imperva"),
    ("/_incapsula_resource", "imperva"),
    // Measured 2026-08-06 against google.com/search with `--engine http`: the
    // response is HTTP 200 whose entire body is a meta-refresh to this endpoint
    // and nothing else. Not a CAPTCHA -- a JS gate -- but the agent still
    // received zero content with exit 0, which is the same defect.
    ("/httpservice/retry/enablejs", "generic"),
    // SearchGuard, which is BotGuard applied to Google Search. This row is a
    // different KIND of evidence from every other one above, and the
    // distinction is the point: the rest name a challenge a caller can
    // sometimes clear, this one names CONTINUOUS CLIENT ATTESTATION, where a
    // bytecode VM runs inside the browser and issues a verdict token.
    //
    // No stealth patch clears it. The CLI already masks `navigator.webdriver`,
    // already passes the `Runtime.enable` probe, and still lands here after the
    // submit. Attributing it separately is what lets `suggestion()` stop telling
    // the operator to tune one more fingerprint, which is the wrong loop and
    // burns the IP that the same machine's ordinary browser also uses.
    //
    // Matched on the redirect PATH rather than on body prose, because the page
    // served there is localized and has served an h-captcha on at least one
    // measurement, so its text is not a stable marker.
    ("/sorry/index", "searchguard"),
];
