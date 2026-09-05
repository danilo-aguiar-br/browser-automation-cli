//! A count of configuration keys written into prose is a claim the binary can
//! settle, and nothing was asking it.
//!
//! # Why this gate exists
//!
//! MEASURED 2026-09-04: seventeen points across the published documentation
//! said the XDG surface had 206 keys while `config list-keys` returned 215.
//! Every gate was green. `scripts/doc-coverage-check.sh` validates key
//! COVERAGE — that each live key appears in `docs/CONFIGURATION.md` — and never
//! reads the NUMERAL in the prose beside it. Coverage of the items and a claim
//! about how many items there are are two different assertions, and only one of
//! them had an owner.
//!
//! This is the same class as `tests/doc_measured_claims_gate.rs`, found on a
//! different subject: a number frozen in a sentence has no compiler. That file
//! re-measures error-site counts from the source tree; this one re-measures the
//! configuration surface from the RUNNING BINARY, which is the only authority
//! on it.
//!
//! # Why CHANGELOG and gaps.md are excluded
//!
//! A CHANGELOG entry records what was measured on a date. `CHANGELOG.md` still
//! says 205 and 206 keys in its 0.1.9 entries, and those sentences were true
//! when written. Rewriting them so they match today's binary would be editing
//! the historical record to hide that the surface moved — the opposite of what
//! a changelog is for. `gaps.md` is a dated audit log and carries the same
//! immunity.
//!
//! That exclusion is precisely why this gate is hand-written instead of being a
//! blind repository-wide replace: the correct action differs by document, and
//! only prose that asserts the CURRENT surface is in scope.

use std::path::{Path, PathBuf};

mod common;

/// Repository root, from this test binary's manifest directory.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory names never walked.
///
/// The first four hold no shipped document. `docs_rules/` is different and the
/// reason is worth writing down: it is a generic Rust engineering-rules corpus
/// that describes no part of THIS product, and the noun collides across
/// domains. MEASURED 2026-09-04, before it was excluded, this gate read
/// "MANTER 2 chaves ativas simultaneamente" (signing keys), "10.000 chaves"
/// (an LRU) and "100 chaves" (a join) as claims about the XDG configuration
/// surface and demanded they equal 215. A gate that fails on the wrong subject
/// teaches the reader to correct prose nobody meant, which is worse than no
/// gate at all.
const SKIPPED_DIRS: &[&str] = &[".git", "target", "node_modules", ".atomwrite", "docs_rules"];

/// File names excluded from the scan, each because it is a dated record.
///
/// See the module header: correcting a historical entry to match today's
/// binary would falsify the history rather than fix a claim.
const DATED_RECORDS: &[&str] = &["CHANGELOG.md", "CHANGELOG.pt-BR.md", "gaps.md"];

/// Every shipped `.md` and `.txt` document, walked without a dependency.
///
/// Fail-closed for the reason `doc_measured_claims_gate.rs` records at length:
/// its first walker used `.ok()` and a directory it could not open vanished
/// from the total in silence, so the gate reported a smaller number and blamed
/// the documentation for it.
///
/// Duplicated from that file rather than shared, because an integration test is
/// its own crate and `tests/common/mod.rs` is not this wave's to edit.
fn live_documents() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root()];
    while let Some(d) = stack.pop() {
        let entries =
            std::fs::read_dir(&d).unwrap_or_else(|e| panic!("cannot walk {}: {e}", d.display()));
        for e in entries {
            let e = e.unwrap_or_else(|err| panic!("cannot read entry in {}: {err}", d.display()));
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                if !SKIPPED_DIRS.contains(&name.as_str()) {
                    stack.push(p);
                }
            } else if p.extension().is_some_and(|x| x == "md" || x == "txt")
                && !DATED_RECORDS.contains(&name.as_str())
            {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// A sentence asserting how large the configuration surface is.
#[derive(Debug, PartialEq, Eq)]
struct KeyCountClaim {
    /// One-based line number, so a failure names a place to go.
    line: usize,
    /// The number the sentence publishes.
    number: usize,
    /// The line with backticks and bold markers removed, as it was parsed.
    text: String,
}

/// Phrases that mark the count on a line as a HISTORICAL one.
///
/// The corpus deliberately keeps old figures beside new ones — INTEGRATIONS
/// writes "config surface grows 176 → **204** keys" inside its 0.1.8 paragraph
/// and MIGRATION writes "It was `204` keys at `0.1.8`". Both are correct and
/// must stay correct, so a gate that compares them against today's binary would
/// be demanding that the documentation forget its own history.
///
/// Anchored on the transition VERB rather than on a version tag, which was the
/// first shape tried and was wrong: `INTEGRATIONS.md:29` states the live count
/// and mentions `0.1.8` in the same sentence purely to disown the older figure,
/// so a version-tag rule would have skipped a live claim.
const HISTORICAL_MARKERS: &[&str] = &[
    "grows",   // "config surface grows 176 → 204 keys"
    "grew",    // "the surface grew from 176 keys at 0.1.7 to 204"
    "cresce",  // also matches "cresceu": "cresce de 176 para 204 chaves"
    "It was ", // "It was 204 keys at 0.1.8"
    "Eram ",   // "Eram 204 chaves na 0.1.8"
    "→",       // the transition arrow itself
];

/// Nouns a count can attach to, in the two published languages.
const KEY_NOUNS: &[&str] = &[" keys", " chaves"];

/// Words tolerated between the digits and the noun.
///
/// Exactly one, `XDG`, because both languages write the qualifier on opposite
/// sides — "215 XDG keys" and "215 chaves XDG". Everything wider was rejected:
/// the precedent's 32-character window would read `"16 config keys"` and
/// `"scrape_no_cache — complete reference with all 215 keys"` alike, and the
/// second is a claim while the first is a quoted anti-pattern. Requiring the
/// digits to TOUCH the noun is what keeps the figure anchored to the thing it
/// counts instead of to the nearest number on the line.
const TOLERATED_QUALIFIER: &str = " XDG";

/// Every claim about the size of the configuration surface in `doc`.
///
/// Pure on purpose: `the_detector_fires_on_a_stale_count` drives this same
/// function with synthetic text, so the detector that guards the repository is
/// the detector that was proven to bite.
fn key_count_claims(doc: &str) -> Vec<KeyCountClaim> {
    let mut found = Vec::new();
    for (idx, raw) in doc.lines().enumerate() {
        // Backticks and bold markers are decoration around the same figure:
        // the corpus writes `206`, **215** and 215 for the same kind of claim.
        let line: String = raw.chars().filter(|c| *c != '`' && *c != '*').collect();
        if HISTORICAL_MARKERS.iter().any(|m| line.contains(m)) {
            continue;
        }
        for noun in KEY_NOUNS {
            let mut from = 0usize;
            while let Some(rel) = line[from..].find(noun) {
                let at = from + rel;
                from = at + noun.len();
                let head = line[..at]
                    .strip_suffix(TOLERATED_QUALIFIER)
                    .unwrap_or(&line[..at]);
                let digits: String = head
                    .chars()
                    .rev()
                    .take_while(char::is_ascii_digit)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                if digits.is_empty() {
                    continue;
                }
                // A figure the page quotes in order to REJECT it. Both
                // CROSS_PLATFORM and MIGRATION tell the reader not to hard-code
                // “16 keys”; asserting that 16 equals the live count would fail
                // the page for saying the right thing.
                let before = head[..head.len() - digits.len()].chars().last();
                if matches!(before, Some('“') | Some('"')) {
                    continue;
                }
                let Ok(number) = digits.parse::<usize>() else {
                    continue;
                };
                found.push(KeyCountClaim {
                    line: idx + 1,
                    number,
                    text: line.trim().to_string(),
                });
            }
        }
    }
    found
}

/// How many configuration keys the product actually has, asked of the product.
///
/// `.data.keys` is the array `config list-keys` publishes; CONFIRMED 2026-09-04
/// by running the command, which answered 215. Read through `common::cmd()` so
/// the count comes from an isolated home rather than from whatever config the
/// operator happens to have.
fn live_key_count() -> usize {
    let out = common::cmd()
        .args(["--json", "config", "list-keys"])
        .output()
        .expect("`config list-keys` must run");
    let envelope: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "`config list-keys --json` must emit an envelope: {e}; stderr={}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    envelope["data"]["keys"]
        .as_array()
        .expect("the envelope must carry `data.keys` as an array")
        .len()
}

#[test]
fn every_published_key_count_matches_the_running_binary() {
    let live = live_key_count();
    let mut stale: Vec<String> = Vec::new();
    let mut seen = 0usize;
    for path in live_documents() {
        let doc = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let rel = path.strip_prefix(root()).unwrap_or(&path).to_path_buf();
        for claim in key_count_claims(&doc) {
            seen += 1;
            if claim.number != live {
                stale.push(format!(
                    "{}:{} says {} keys, binary has {live} — {}",
                    rel.display(),
                    claim.line,
                    claim.number,
                    claim.text
                ));
            }
        }
    }
    // A scan that finds nothing to check is not a pass. If the wording moved,
    // this gate stopped guarding anything and must follow it.
    assert!(
        seen > 0,
        "no document states a configuration-key count any more; either the \
         sentences were reworded and this gate must follow them, or the claim \
         was dropped and the surface is now published with no figure at all"
    );
    assert!(
        stale.is_empty(),
        "documents publish a configuration-key count the binary disagrees \
         with. `config list-keys` is the authority; update the prose, do not \
         relax this gate:\n{}",
        stale.join("\n")
    );
}

/// The detector must fail when the violation is present, or it proves nothing.
///
/// Driven on synthetic text rather than on a temporarily corrupted repository
/// file, so the proof leaves no edit behind and survives a reader who never
/// runs it.
#[test]
fn the_detector_fires_on_a_stale_count() {
    // The exact shape of the 2026-09-04 incident: the live surface is 215 and
    // the sentence still says 206.
    let stale = "- Complete reference: all **206** keys in `docs/CONFIGURATION.md`";
    let claims = key_count_claims(stale);
    assert_eq!(claims.len(), 1, "one claim expected, got {claims:?}");
    assert_eq!(claims[0].number, 206);
    assert_ne!(claims[0].number, 215, "the synthetic claim must be stale");

    // Portuguese word order puts the qualifier on the other side.
    let pt = "- Superfície XDG viva: 206 chaves XDG documentadas";
    assert_eq!(key_count_claims(pt)[0].number, 206);
}

/// The three ways a line is legitimately allowed to carry a different number.
///
/// Each of these is a real line from the corpus, MEASURED 2026-09-04. A gate
/// that flagged them would push the reader to delete true history in order to
/// get a green run, which is how a gate earns the reputation that gets it
/// deleted.
#[test]
fn the_detector_stays_silent_on_history_and_quoted_anti_patterns() {
    let historical = [
        "- Config surface grows from 176 to **204** keys (`config list-keys --json`)",
        "- config surface grows 176 → **204** keys while the 0.1.8 inventory tip stayed 69",
        "- A superfície de configuração cresce de 176 para **204** chaves",
        "- The surface grew from `176` keys at `0.1.7` to `204` at `0.1.8`",
        "- It was `204` keys at `0.1.8`",
        "- Eram `204` chaves na `0.1.8`",
        "- do **not** claim a fixed “16 keys” count — always discover with `config list-keys --json`",
        "- não fixe contagem como “16 chaves”",
    ];
    for line in historical {
        assert!(
            key_count_claims(line).is_empty(),
            "this line is history or a quoted anti-pattern, not a live claim: {line}"
        );
    }

    // Adjacency, not proximity: a number elsewhere on the line is not a count
    // of keys, and `list-keys` is a verb rather than a noun phrase.
    let unrelated = [
        "- Agents that hard-coded “16 config keys” must switch to `config list-keys --json`",
        "- browser-automation-cli --timeout 90 --json run --script /tmp/keys.jsonl",
        "- inventory **71** agent names via `commands --json`",
    ];
    for line in unrelated {
        assert!(
            key_count_claims(line).is_empty(),
            "this line states no key count: {line}"
        );
    }
}

/// The live count must come from the binary, not from a constant in this file.
///
/// Guards the failure mode where someone "fixes" a red run by freezing the
/// expected number here, which would restore the exact defect the gate exists
/// to remove — one more number with no compiler.
#[test]
fn the_authority_is_the_binary_and_it_answers() {
    let live = live_key_count();
    assert!(
        live > 0,
        "`config list-keys` published an empty surface; the gate cannot compare \
         prose against nothing"
    );
    let _: &Path = &root();
}
