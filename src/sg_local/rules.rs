//! The forbidden-pattern rules the local `sg` scanner enforces, and nothing else.
//!
//! # Why these live apart from the engine
//!
//! `sg_local` crossed the 300 production-line ceiling that
//! `scripts/filesize-check.sh` enforces, and the seam that cost the least to cut
//! was the one between POLICY and MOTOR. Everything here answers "what counts as
//! a violation"; the parent module answers "how the tree is walked, read and
//! rewritten". A new rule is added here without reading the scan loop, and the
//! scan loop is changed without re-reading four regexes.
//!
//! The exemption is a FIELD rather than a branch on the rule name, so every rule
//! added has to state whether test code is exempt instead of inheriting silence.

use std::sync::LazyLock;

use regex::Regex;

/// One compiled rule plus the exemption it grants.
///
/// # Why the exemption is a field
///
/// It used to be `if *rule == "unwrap_prod"` inside the scan loop, which ties a
/// policy decision to a string literal: adding a fifth rule silently inherited
/// "not exempt" without anyone deciding it, and the scan loop had to be read to
/// discover which rule the branch meant. As a field, the decision lives at the
/// point of definition and every new rule has to state it.
pub(super) struct Rule {
    /// Stable identifier emitted in findings.
    pub(super) name: &'static str,
    /// The line pattern this rule looks for.
    pub(super) re: Regex,
    /// Whether test code is exempt from this rule.
    ///
    /// Only `unwrap_prod` is, and its name says why: in a fixture an `unwrap()`
    /// IS the assertion. Remote telemetry, product secret env reads and dotenv
    /// loads are forbidden in every target the crate builds, so a test file
    /// gets no pass on them.
    pub(super) exempt_in_tests: bool,
}

pub(super) fn re_rust_log_export() -> &'static Regex {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)export\s+RUST_LOG\s*="#).expect("RUST_LOG export regex")
    });
    &RE
}

pub(super) fn compiled_rules() -> &'static [Rule] {
    static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
        vec![
            Rule {
                name: "telemetry_string",
                re: Regex::new(r"(?i)\b(opentelemetry|sentry\.io|telemetry\.|posthog|datadog)\b")
                    .expect("telemetry regex"),
                // A test that reaches a telemetry endpoint is still a product
                // that reaches a telemetry endpoint.
                exempt_in_tests: false,
            },
            Rule {
                name: "product_env_secret",
                re: Regex::new(
                    r#"std::env::var\(\s*"(API_KEY|OPENAI_API_KEY|SECRET|TOKEN|PASSWORD)""#,
                )
                .expect("env secret regex"),
                // Reading a product secret from the ambient environment is
                // banned in every target; fixtures parameterize instead.
                exempt_in_tests: false,
            },
            Rule {
                name: "unwrap_prod",
                re: Regex::new(r"\.unwrap\(\)").expect("unwrap regex"),
                // The one exemption, and the rule name carries it: in a fixture
                // an unwrap IS the assertion, and rewriting it would bury the
                // failure. Same reasoning as `allow-unwrap-in-tests` in
                // clippy.toml, kept in agreement with it on purpose.
                exempt_in_tests: true,
            },
            Rule {
                name: "dotenv",
                // `\.env\b` treats `(` as a word boundary, so it matched
                // `Command::env(k, v)` — the very call the product uses to stop
                // inheriting the ambient environment. Measured on 2026-08-25: 68
                // hits across 28 files, 66 of them `Command::env` and none of
                // them dotenv. Requiring a non-word, non-`(` character after
                // `env` keeps the dotenv FILE (`".env"`, `read(".env")`) and
                // drops the method call.
                re: Regex::new(r"(?i)\bdotenv\b|\.env(?:[^\w(]|$)").expect("dotenv regex"),
                // A fixture that loads a dotenv file is the exact runtime the
                // product forbids, so tests get no pass here either.
                exempt_in_tests: false,
            },
        ]
    });
    RULES.as_slice()
}
