//! Property tests for offline parsers (PRD 5AN / GAP-010).

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// The arbitrary body must REACH the robots parser.
    ///
    /// The previous version of this case built a `RobotsPolicy::Honor` value,
    /// called `s.len()`, and asserted nothing — so it proved that `str::len`
    /// does not panic. The generated body never touched robots code at all.
    #[test]
    fn robots_body_never_panics(s in ".*") {
        let allowed = browser_automation_cli::robots::url_allowed_by_robots_body(
            &s,
            "browser-automation-cli-proptest",
            "https://example.test/some/path",
        );
        // A body that parses to nothing must not silently DENY: robots.txt is
        // allow-by-default, and a parser that fails closed on garbage would
        // block every crawl the moment a site served a malformed file.
        if s.trim().is_empty() {
            prop_assert!(allowed, "empty robots body must allow");
        }
        let _ = browser_automation_cli::robots::parse_crawl_delay_secs(
            &s,
            "browser-automation-cli-proptest",
        );
    }

    /// Absent optional envelope fields must be OMITTED, never serialized null.
    ///
    /// The previous version built a `serde_json::json!` literal and round-tripped
    /// it through `serde_json`, touching no type from this crate: it tested the
    /// serde library. What actually needs pinning is the `skip_serializing_if`
    /// contract on [`SuccessEnvelope`], because an agent that receives
    /// `correlation_id: null` cannot distinguish it from one deliberately cleared.
    #[test]
    fn envelope_omits_absent_optional_fields(ok in any::<bool>(), msg in "[a-zA-Z0-9 ]{0,64}") {
        use browser_automation_cli::envelope::SuccessEnvelope;
        let env = SuccessEnvelope {
            schema_version: 1,
            ok,
            correlation_id: None,
            data: serde_json::json!({ "message": msg }),
            agent_ops: None,
        };
        let v = serde_json::to_value(&env).unwrap();
        prop_assert!(v.get("correlation_id").is_none(), "None must not reach the wire: {v}");
        prop_assert!(v.get("agent_ops").is_none(), "None must not reach the wire: {v}");
        prop_assert_eq!(&v["schema_version"], &serde_json::json!(1));
        prop_assert_eq!(&v["ok"], &serde_json::json!(ok));
    }

    /// A cache key must be stable for one URL and DIFFERENT across URLs.
    ///
    /// The previous version compared `http_get(&url, &ctx)` against itself with
    /// the same arguments, which a constant function passes. Determinism alone
    /// is the cheap half; the half that matters is sensitivity, because a key
    /// that ignored the URL would still be perfectly deterministic while
    /// serving one page's cached body for another.
    #[test]
    fn cache_key_deterministic(
        url in "https://[a-z]{1,12}\\.example/[a-z0-9/]{0,32}",
        other in "https://[a-z]{1,12}\\.example/[a-z0-9/]{0,32}",
    ) {
        use browser_automation_cli::cache::{CacheContext, CacheKey};
        let ctx = CacheContext::direct("chrome-linux");
        // `as_str` borrows from the key, so each key needs a binding that
        // outlives the comparison.
        let first = CacheKey::http_get(&url, &ctx);
        let again = CacheKey::http_get(&url, &ctx);
        prop_assert_eq!(first.as_str(), again.as_str(), "same url must yield the same key");
        if url != other {
            let differing = CacheKey::http_get(&other, &ctx);
            prop_assert_ne!(
                first.as_str(),
                differing.as_str(),
                "distinct urls must not collide"
            );
        }
    }

    #[test]
    fn retry_classifies_without_panic(s in ".*") {
        let _ = browser_automation_cli::retry::is_retryable_message(&s);
    }
}
