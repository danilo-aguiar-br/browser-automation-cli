// SPDX-License-Identifier: MIT OR Apache-2.0
//! Detect a bot check served in place of content.
//!
//! # Why this is not covered by the HTTP error path
//!
//! [`super::error_page`] models **transport** failure: `status_code`,
//! `http_error`, a message like `HTTP 404 for …`. A challenge is none of those.
//! It arrives as **HTTP 200 with valid HTML**, so every transport field reports
//! success while the body carries a CAPTCHA. The envelope was honest about the
//! only thing it measured, and the thing it measured was the wrong one.
//!
//! The cost of that gap is not a missing field. An agent quotes the block page as
//! content, and a retry loop hammers a WAF, which is the documented path from
//! "rate limited" to "banned".
//!
//! # What this module does and does not claim
//!
//! Detection, never evasion. Naming the wall does not climb it. What it buys is
//! that every later anti-detection change becomes **measurable**: today a
//! successful fetch and a CAPTCHA are indistinguishable by exit code, so no
//! evasion work can be validated.

use serde_json::{json, Value};

mod signatures;

use signatures::{BODY_SIGNATURES, MITIGATION_HEADERS, VENDOR_COOKIES, VENDOR_HEADERS};

/// Where in the response the block signature was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockPhase {
    /// A response header named the WAF.
    Header,
    /// A `Set-Cookie` named the WAF.
    Cookie,
    /// The rendered body carried a challenge marker.
    Body,
    /// The final URL or document title carried a marker after an automatic
    /// navigation replaced the interstitial.
    Location,
}

impl BlockPhase {
    /// Stable wire string.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Cookie => "cookie",
            Self::Body => "body",
            Self::Location => "location",
        }
    }
}

/// A matched block signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDetection {
    /// Vendor name, or `generic` when only a vendor-neutral marker matched.
    pub waf: &'static str,
    /// The exact token that matched, so the finding is auditable.
    pub signal: &'static str,
    /// Which part of the response carried it.
    pub phase: BlockPhase,
}

impl BlockDetection {
    /// `data.block_detection` payload.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "waf": self.waf,
            "signal": self.signal,
            "phase": self.phase.as_str(),
        })
    }

    /// Remediation hint. Never "retry": retrying is what escalates a block.
    ///
    /// # Why this is not a `format!`
    ///
    /// It used to be, and that cost two defects at once. The prose named
    /// `--proxy`, a flag that did not exist, so an agent obeying the remediation
    /// earned exit 2 — and the gate written to catch exactly that class of
    /// defect (`scripts/agent-ops-check.sh`) reads only `locales/*.ftl`, so a
    /// `format!` in Rust escaped it. Worse, a unit test asserted the string
    /// contained `--proxy`, ratifying the phantom flag.
    ///
    /// The second defect was language: `--lang pt-BR` localized every other
    /// suggestion and left this one in English, because a `format!` sits
    /// outside the message catalog by construction.
    ///
    /// `waf`, `signal` and `phase` are NOT repeated here. They already travel
    /// structurally in `data.block_detection` (see [`Self::to_json`]), so an
    /// agent reads them as fields. Interpolating them into prose duplicated
    /// machine-readable data as text and was the only reason a formatter was
    /// needed at all.
    /// # Why attestation gets its own advice
    ///
    /// The default advice — swap engine, change egress, wait — is sound for a
    /// challenge, and actively harmful for continuous attestation. There the
    /// verdict does not depend on the fingerprint the caller can shape, so
    /// every retry spends the same IP against a system that is watching it, and
    /// the operator reads exit 6 as "one more patch to write". Naming the
    /// ceiling is the only advice that saves time here.
    #[must_use]
    pub fn suggestion(&self) -> String {
        let key = if self.waf == "searchguard" {
            "blocked_by_attestation"
        } else {
            "blocked_by_waf"
        };
        crate::i18n::suggestion_key(key, None).to_string()
    }
}

/// Inspect headers, cookies, and body for evidence of an **active** block.
///
/// `headers` and `cookies` are as received; `body` is the extracted text or HTML.
/// Names are compared case-insensitively because HTTP field names are.
///
/// # Decision order
///
/// 1. A mitigation header ends it: the vendor asserted it acted on this request.
/// 2. Otherwise a challenge marker in the body, attributed to whichever vendor
///    the headers or cookies name.
/// 3. Otherwise no block -- a vendor fingerprint alone is a CDN, not a wall.
#[must_use]
pub fn detect<'a, H, C>(headers: H, cookies: C, body: &str) -> Option<BlockDetection>
where
    H: IntoIterator<Item = (&'a str, &'a str)>,
    C: IntoIterator<Item = &'a str>,
{
    let names: Vec<String> = headers
        .into_iter()
        .map(|(n, _)| n.to_ascii_lowercase())
        .collect();

    if let Some((signal, waf)) = MITIGATION_HEADERS
        .iter()
        .find(|(sig, _)| names.iter().any(|n| n == sig))
    {
        return Some(BlockDetection {
            waf,
            signal,
            phase: BlockPhase::Header,
        });
    }

    // No mitigation asserted, so the body has to carry the challenge itself.
    let hit = detect_in_body(body)?;
    if hit.waf != "generic" {
        return Some(hit);
    }

    // Attribute the generic challenge to whoever is in front, if anyone is.
    let vendor = VENDOR_HEADERS
        .iter()
        .find(|(sig, _)| names.iter().any(|n| n == sig))
        .map(|(_, waf)| *waf)
        .or_else(|| {
            let jar: Vec<String> = cookies.into_iter().map(str::to_ascii_lowercase).collect();
            VENDOR_COOKIES
                .iter()
                .find(|(sig, _)| jar.iter().any(|c| c.contains(*sig)))
                .map(|(_, waf)| *waf)
        });

    Some(BlockDetection {
        waf: vendor.unwrap_or("generic"),
        ..hit
    })
}

/// Body detection plus the one signal an automatic navigation leaves behind.
///
/// # The escape this closes
///
/// [`detect_in_body`] reads the DOM once, after settling. A challenge that
/// redirects on its own is therefore invisible to it in the window that matters:
/// the interstitial markup is replaced, and what remains can be a near-empty
/// document with no signature in it at all. The scrape then returns `ok: true`
/// with no content, which is precisely the silent-success shape this module
/// exists to abolish.
///
/// The navigation does leave a trace, and it is free to read: the address the
/// browser ended up on. A Cloudflare interstitial lands on
/// `/cdn-cgi/challenge-platform/...`, which the body table already knows.
///
/// # Why these two and nothing else
///
/// Both arrive with the navigation result the caller already holds, so this
/// costs no extra CDP round trip. A signal that charged every scrape, blocked or
/// not, to serve the rare case is how a detector becomes something callers turn
/// off.
///
/// Body wins when more than one matches: body evidence is about the page that
/// was served, while the address and the title only say where the browser was
/// sent and what the tab was called.
#[must_use]
pub fn detect_in_page(body: &str, final_url: &str, title: &str) -> Option<BlockDetection> {
    if let Some(hit) = detect_in_body(body) {
        return Some(hit);
    }
    if final_url.is_empty() && title.is_empty() {
        return None;
    }
    let lower = format!("{final_url}\n{title}").to_ascii_lowercase();
    BODY_SIGNATURES
        .iter()
        .find(|(sig, _)| lower.contains(*sig))
        .map(|(signal, waf)| BlockDetection {
            waf,
            signal,
            phase: BlockPhase::Location,
        })
}

/// Body-only detection, for callers that never saw the headers.
///
/// The browser engine reads the DOM after the fact and has no response headers to
/// offer, so it can only reach this half.
#[must_use]
pub fn detect_in_body(body: &str) -> Option<BlockDetection> {
    if body.is_empty() {
        return None;
    }
    // Lowercase once. Scanning the raw body per marker would be 14 passes, and
    // a case-insensitive comparison per byte is far more expensive than one
    // allocation the size of a page.
    let lower = body.to_ascii_lowercase();
    BODY_SIGNATURES
        .iter()
        .find(|(sig, _)| lower.contains(*sig))
        .map(|(signal, waf)| BlockDetection {
            waf,
            signal,
            phase: BlockPhase::Body,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_headers() -> Vec<(&'static str, &'static str)> {
        Vec::new()
    }

    fn no_cookies() -> Vec<&'static str> {
        Vec::new()
    }

    #[test]
    fn clean_page_is_not_a_block() {
        let body = "<html><body><h1>Example Domain</h1></body></html>";
        assert_eq!(detect(no_headers(), no_cookies(), body), None);
    }

    #[test]
    fn empty_body_is_not_a_block() {
        assert_eq!(detect_in_body(""), None);
    }

    #[test]
    fn a_cdn_in_front_of_a_good_page_is_not_a_block() {
        // Regression guard, measured against https://example.com: HTTP 200 with
        // real content and `cf-cache-status` present. An earlier draft treated
        // any vendor header as a block and flagged it, which would have broken
        // scraping for every site behind a CDN. Presence of a WAF is not action
        // by a WAF.
        let body = "<html><body><h1>Example Domain</h1></body></html>";
        for header in ["cf-cache-status", "cf-ray", "x-datadome", "x-iinfo"] {
            assert_eq!(
                detect(vec![(header, "HIT")], no_cookies(), body),
                None,
                "{header} alone must not read as a block"
            );
        }
    }

    #[test]
    fn a_solved_challenge_cookie_is_not_a_block() {
        // `cf_clearance` is issued AFTER a challenge is passed, so reading it as
        // a block inverts its meaning: it is proof the wall was already cleared.
        assert_eq!(
            detect(
                no_headers(),
                vec!["cf_clearance=xyz; Path=/; Secure; HttpOnly"],
                "<h1>Real content</h1>",
            ),
            None
        );
    }

    #[test]
    fn an_asserted_mitigation_is_a_block_on_its_own() {
        // Unlike a vendor header, `cf-mitigated` means the vendor acted on THIS
        // request, so it needs no corroboration from the body.
        let hit = detect(vec![("CF-Mitigated", "challenge")], no_cookies(), "").expect("detected");
        assert_eq!(hit.waf, "cloudflare");
        assert_eq!(hit.phase, BlockPhase::Header);
    }

    #[test]
    fn mitigation_header_names_match_case_insensitively() {
        for name in ["cf-mitigated", "CF-Mitigated", "Cf-MITIGATED"] {
            assert!(
                detect(vec![(name, "challenge")], no_cookies(), "").is_some(),
                "missed {name}"
            );
        }
    }

    #[test]
    fn a_generic_challenge_is_attributed_to_the_vendor_in_front() {
        // The body proves the block; the header only supplies the name, which is
        // what turns a bare `generic` into an actionable report.
        let hit = detect(vec![("cf-ray", "abc")], no_cookies(), "captcha-form").expect("detected");
        assert_eq!(hit.waf, "cloudflare");
        assert_eq!(hit.phase, BlockPhase::Body);

        let via_cookie = detect(no_headers(), vec!["ak_bmsc=1"], "captcha-form").expect("detected");
        assert_eq!(via_cookie.waf, "akamai");
    }

    #[test]
    fn a_challenge_with_no_vendor_in_front_stays_generic() {
        let hit = detect(no_headers(), no_cookies(), "captcha-form").expect("detected");
        assert_eq!(hit.waf, "generic");
    }

    #[test]
    fn captcha_body_is_detected() {
        // The exact payload measured against a live target: HTTP 200, valid HTML,
        // every transport field reporting success.
        let body = "...ative o JavaScript... detectamos tráfego incomum... \
                    document.getElementById('captcha-form').submit();";
        let hit = detect(no_headers(), no_cookies(), body).expect("detected");
        assert_eq!(hit.phase, BlockPhase::Body);
    }

    #[test]
    fn body_markers_match_case_insensitively() {
        let hit = detect_in_body("Checking Your Browser Before Accessing").expect("detected");
        assert_eq!(hit.waf, "cloudflare");
    }

    #[test]
    fn accented_and_unaccented_portuguese_both_match() {
        // The rendered text may lose accents through a lossy decode, so both
        // spellings are listed rather than assuming one normalization.
        assert!(detect_in_body("detectamos tráfego incomum").is_some());
        assert!(detect_in_body("detectamos trafego incomum").is_some());
    }

    #[test]
    fn every_row_in_every_table_is_reachable() {
        // Guards the tables themselves: a row added with a typo would never fire,
        // and a signature that cannot match is worse than an absent one because
        // it reads as coverage.
        for (sig, waf) in MITIGATION_HEADERS {
            let hit = detect(vec![(*sig, "v")], no_cookies(), "").expect(sig);
            assert_eq!(hit.waf, *waf, "mitigation {sig}");
        }
        for (sig, waf) in VENDOR_HEADERS {
            let hit = detect(vec![(*sig, "v")], no_cookies(), "captcha-form").expect(sig);
            assert_eq!(hit.waf, *waf, "vendor header {sig}");
        }
        for (sig, waf) in VENDOR_COOKIES {
            let hit = detect(no_headers(), vec![*sig], "captcha-form").expect(sig);
            assert_eq!(hit.waf, *waf, "vendor cookie {sig}");
        }
        for (sig, waf) in BODY_SIGNATURES {
            let hit = detect_in_body(sig).expect(sig);
            assert_eq!(hit.waf, *waf, "body {sig}");
        }
    }

    #[test]
    fn signatures_are_lowercase_so_the_comparison_is_symmetric() {
        // Detection lowercases the input; a signature with an uppercase byte
        // could then never match and would sit in the table as dead coverage.
        for (sig, _) in MITIGATION_HEADERS
            .iter()
            .chain(VENDOR_HEADERS)
            .chain(VENDOR_COOKIES)
            .chain(BODY_SIGNATURES)
        {
            assert_eq!(*sig, sig.to_ascii_lowercase(), "{sig} is not lowercase");
        }
    }

    #[test]
    fn json_payload_carries_all_three_fields() {
        let hit = detect(vec![("cf-mitigated", "challenge")], no_cookies(), "").expect("detected");
        let v = hit.to_json();
        assert_eq!(v["waf"], json!("cloudflare"));
        assert_eq!(v["signal"], json!("cf-mitigated"));
        assert_eq!(v["phase"], json!("header"));
    }

    /// Attestation is named as a CEILING, not as one more thing to tune.
    ///
    /// The default advice tells the operator to swap engine or change egress.
    /// Against continuous attestation that advice is worse than silence: the
    /// verdict does not depend on what the caller can shape, so each retry
    /// spends the same IP against a system watching it, and the operator reads
    /// exit 6 as "one more patch to write". This pins the two apart.
    #[test]
    fn attestation_gets_different_advice_from_a_challenge() {
        let waf = detect_in_body("captcha-form").expect("challenge detected");
        let att = detect_in_body("/sorry/index").expect("attestation detected");

        assert_eq!(att.waf, "searchguard", "attestation must be attributed");
        assert_ne!(
            waf.suggestion(),
            att.suggestion(),
            "a ceiling and a challenge cannot share advice"
        );

        let s = att.suggestion().to_ascii_lowercase();
        // The wrong loop must not be suggested at all.
        assert!(
            !s.contains("--engine browser") && !s.contains("--proxy"),
            "attestation advice must not send the operator back to fingerprint tuning: {s}"
        );
        // And it must name a route that actually exists.
        assert!(
            s.contains("duckduckgo-search-cli") || s.contains("searxng"),
            "attestation advice must name a different provider: {s}"
        );
    }

    #[test]
    fn suggestion_never_tells_the_agent_to_retry() {
        // Retrying against a WAF is the documented path from rate-limited to
        // banned, so the remediation text must not invite it.
        let hit = detect_in_body("captcha-form").expect("detected");
        let s = hit.suggestion().to_ascii_lowercase();
        assert!(!s.contains("try again"), "{s}");
        assert!(!s.contains("retry the"), "{s}");
        // The remediation must name a route that EXISTS. The previous version of
        // this assertion accepted `--proxy` while no such flag was declared, so
        // the test ratified the phantom instead of catching it. Both routes are
        // now real global flags; `tests/agent_ops_cli.rs` proves that by argv.
        assert!(s.contains("--engine browser"), "{s}");
        assert!(s.contains("--proxy"), "{s}");
        // The structured fields belong to `data.block_detection`, not to prose.
        assert!(
            !s.contains("cloudflare") && !s.contains("captcha-form"),
            "suggestion must not duplicate machine-readable fields: {s}"
        );
    }

    #[test]
    fn a_challenge_that_navigated_away_is_still_a_block() {
        // The interstitial replaced itself, so the DOM that survives carries no
        // marker at all. Body-only detection sees an empty page and reports
        // success, which is the silent-success shape this module exists to
        // abolish -- `ok: true` with no content.
        let landed = "https://example.test/cdn-cgi/challenge-platform/h/b/orchestrate";
        let hit = detect_in_page("<html><body></body></html>", landed, "")
            .expect("a navigation to a challenge endpoint is evidence");
        assert_eq!(hit.phase, BlockPhase::Location);
        assert_eq!(hit.waf, "cloudflare");
    }

    #[test]
    fn the_tab_title_is_evidence_when_the_body_is_gone() {
        let hit = detect_in_page("", "https://example.test/", "Verifying you are human")
            .expect("the title names the challenge");
        assert_eq!(hit.phase, BlockPhase::Location);
    }

    #[test]
    fn body_evidence_outranks_where_the_browser_landed() {
        // The body describes the page that was actually served; the address only
        // says where the browser was sent. When both match, the stronger claim
        // must be the one reported, or the report attributes the block to the
        // wrong phase and an operator debugs the wrong hop.
        let hit = detect_in_page(
            "g-recaptcha",
            "https://example.test/challenge-platform/x",
            "",
        )
        .expect("detected");
        assert_eq!(hit.phase, BlockPhase::Body);
        assert_eq!(hit.signal, "g-recaptcha");
    }

    #[test]
    fn an_ordinary_page_is_not_a_block_in_any_phase() {
        // The whole table is vendor-neutral prose, so a widened surface is a
        // widened chance of flagging a good page. A plain URL and a plain title
        // must stay clean, or the cure is worse than the escape it closes.
        assert!(detect_in_page(
            "<h1>Example Domain</h1>",
            "https://example.com/",
            "Example Domain"
        )
        .is_none());
    }
}
