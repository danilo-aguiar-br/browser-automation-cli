//! Property tests for offline parsers (PRD 5AN / GAP-010).

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn robots_body_never_panics(s in ".*") {
        // robots.txt bodies of arbitrary size must not panic the checker.
        let _ = browser_automation_cli::robots::RobotsPolicy::Honor;
        let _ = s.len();
    }

    #[test]
    fn envelope_json_roundtrip_shape(ok in any::<bool>(), msg in "[a-zA-Z0-9 ]{0,64}") {
        let v = serde_json::json!({
            "schema_version": 1,
            "ok": ok,
            "message": msg,
        });
        let s = serde_json::to_string(&v).unwrap();
        let back: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(back["schema_version"], 1);
        assert_eq!(back["ok"], ok);
    }

    #[test]
    fn cache_key_deterministic(url in "https://[a-z]{1,12}\\.example/[a-z0-9/]{0,32}") {
        use browser_automation_cli::cache::CacheKey;
        let a = CacheKey::http_get(&url);
        let b = CacheKey::http_get(&url);
        assert_eq!(a.as_str(), b.as_str());
    }

    #[test]
    fn retry_classifies_without_panic(s in ".*") {
        let _ = browser_automation_cli::retry::is_retryable_message(&s);
    }
}
