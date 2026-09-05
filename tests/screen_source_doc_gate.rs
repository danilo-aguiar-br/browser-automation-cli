//! Permanent gate: every `screen_source` token the binary can emit must be
//! written down in the live cookbook, in BOTH languages.
//!
//! # Why this file exists
//!
//! `screen_source` is an OUTPUT contract. `docs/schemas/emulate.schema.json`
//! and `resize.schema.json` describe the `--screen` INPUT and stop there, so
//! nothing in `docs/` ever named the field or its tokens. The only enumeration
//! in the tree lived in `gaps.md`, which `doc_binary_numeral_gate` excludes on
//! purpose as a dated audit log — a record of what was measured on a day, not a
//! contract to keep current.
//!
//! That left the token set with no owner. Measured 2026-09-04: `ScreenSource`
//! gained a fifth variant, `Floor`, and neither the doc nor the unit test that
//! exists to freeze the tokens gained a line. A consumer matching exhaustively
//! on `argv|step|xdg|derived` breaks on the first `floor` it receives, and the
//! tree gave it no way to learn the value existed.
//!
//! # Why the doc and not the schema
//!
//! Adding an output section to every schema is a documentation-architecture
//! change and would not have caught this one any earlier. The cookbook already
//! gathers the three provenances of `screen` in one bullet, so it is where a
//! reader who cares about provenance already is.
//!
//! # What makes this gate bite
//!
//! It reads `ScreenSource::ALL`, so it cannot fall behind the enum: a variant
//! added without a doc line fails HERE, and a variant added without a token
//! fails to compile in `screen.rs`. Together the two make the list unable to
//! rot silently.

use browser_automation_cli::native::stealth::ScreenSource;

/// Documents that must name every token, one per language.
const LIVE_DOCS: [&str; 2] = ["docs/COOKBOOK.md", "docs/COOKBOOK.pt-BR.md"];

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn every_screen_source_token_is_documented_in_both_languages() {
    let mut missing: Vec<String> = Vec::new();

    for doc in LIVE_DOCS {
        let text = std::fs::read_to_string(root().join(doc))
            .unwrap_or_else(|e| panic!("{doc} must be readable: {e}"));

        for source in ScreenSource::ALL {
            // Backticked so a prose word like "step" cannot satisfy the gate by
            // accident; the doc has to name the token as a token.
            let needle = format!("`{}`", source.as_str());
            if !text.contains(&needle) {
                missing.push(format!("{doc} does not name {needle}"));
            }
        }

        assert!(
            text.contains("`screen_source`"),
            "{doc} must name the field itself, not only its values"
        );
    }

    assert!(
        missing.is_empty(),
        "screen_source tokens missing from the live cookbook:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn the_gate_reads_the_enum_and_not_a_frozen_list() {
    // A hand-copied list here would rot exactly like the one this gate exists
    // to replace, so assert the source is the enum and that it is non-empty.
    assert!(
        ScreenSource::ALL.len() >= 5,
        "ScreenSource lost variants; confirm the envelope contract changed on purpose"
    );
    assert!(ScreenSource::ALL.contains(&ScreenSource::Floor));
}
