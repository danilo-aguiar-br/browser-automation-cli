//! A number frozen in shipped prose must be re-measurable, or it is a claim
//! with no owner.
//!
//! # Why this gate exists
//!
//! `docs/AGENTS.md` states, as evidence for the `lang` policy, how many
//! `CliError::new` and `CliError::with_suggestion` sites this crate has. The
//! numbers were correct when written and nothing kept them correct: a doc
//! sentence has no compiler, so the day a wave adds fifty error sites the
//! published evidence becomes false and every gate stays green.
//!
//! Measured 2026-08-30, while auditing that very sentence: the prose said
//! "343 `with_suggestion` sites", and the bare pattern `with_suggestion`
//! measures 351. The number was right for `CliError::with_suggestion` and the
//! WORDING named a different pattern, so the claim could be read as wrong
//! while being right. That ambiguity is why this gate pins the pattern as well
//! as the count.

use std::path::{Path, PathBuf};

/// Repository root, from this test binary's manifest directory.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every `.rs` file under `dir`, walked without an external dependency.
fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        // Fail-closed on both counts. The first shape of this walker used
        // `let Ok(..) else { continue }` here and `.ok()` on the read below,
        // so a directory it could not open and a file it could not decode both
        // vanished from the total in silence — and the gate would then report
        // a LOWER count and blame the documentation. Measured 2026-08-30: it
        // did exactly that, answering 426 against a true 427.
        let entries =
            std::fs::read_dir(&d).unwrap_or_else(|e| panic!("cannot walk {}: {e}", d.display()));
        for e in entries {
            let e = e.unwrap_or_else(|err| panic!("cannot read entry in {}: {err}", d.display()));
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    out
}

/// Occurrences of `needle` across every Rust source under `src/`.
fn count_in_src(needle: &str) -> usize {
    rust_sources(&root().join("src"))
        .iter()
        .map(|p| {
            std::fs::read_to_string(p)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
                .matches(needle)
                .count()
        })
        .sum()
}

/// How far left of the pattern citation a published count may sit, in chars.
///
/// The English wording puts four characters between the digits and the
/// citation, the Portuguese one puts fourteen. Thirty-two clears both while
/// stopping the search from wandering across the line and adopting an
/// unrelated figure.
const CLAIM_WINDOW_CHARS: usize = 32;

/// The count a doc line asserts for `pattern`, in either published language.
///
/// Both pages cite the pattern between backticks and place its count to the
/// left of that citation, differing only in word order: English reads
/// "426 <cite> sites", Portuguese reads "426 sitios de <cite>". Reading the
/// last digit run inside a short window before the citation covers both with
/// one parser, and keeps the figure ANCHORED to the pattern it describes.
///
/// Returns `None` when no line cites the pattern, which the caller treats as a
/// failure: a gate that silently passes when its subject disappeared is worth
/// less than no gate, because it reports confidence it never earned.
fn claimed(doc: &str, pattern: &str) -> Option<usize> {
    let cite = format!("`{pattern}`");
    for line in doc.lines() {
        let Some(at) = line.find(&cite) else { continue };
        let num: String = line[..at]
            .chars()
            .rev()
            .take(CLAIM_WINDOW_CHARS)
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(char::is_ascii_digit)
            .collect();
        if !num.is_empty() {
            return num.chars().rev().collect::<String>().parse().ok();
        }
    }
    None
}

/// The patterns this gate re-measures.
///
/// Naming them in one place is what lets the coverage case at the bottom of
/// this file ask whether the PAGE has outgrown the list.
const CHECKED_PATTERNS: &[&str] = &["CliError::new", "CliError::with_suggestion"];

/// Every `N `pattern` sites` claim on the page is one this gate re-measures.
///
/// The cases above check the claims they were TOLD about. Neither notices a
/// third claim appearing beside them, so the day someone publishes one more
/// count the gate stays green while the very thing it exists to prevent — a
/// frozen number with no compiler — walks straight back onto the page.
///
/// Same shape as `every_dispatched_cmd_has_a_field_row` in
/// `src/commands/run/inventory.rs`: enumerate the surface, then fail when a
/// member of it has no row.
///
/// Reads the English page only, because `the_pt_br_page_carries_the_same_two_numbers`
/// already forces the pair to agree.
#[test]
fn every_site_count_claimed_on_the_page_is_one_this_gate_re_measures() {
    let doc = std::fs::read_to_string(root().join("docs/AGENTS.md")).expect("AGENTS.md");
    const MARKER: &str = "` sites";
    let mut unchecked: Vec<String> = Vec::new();
    for line in doc.lines() {
        let mut from = 0usize;
        while let Some(rel) = line[from..].find(MARKER) {
            let close = from + rel;
            if let Some(open) = line[..close].rfind('`') {
                let pattern = &line[open + 1..close];
                if !pattern.is_empty() && !CHECKED_PATTERNS.contains(&pattern) {
                    unchecked.push(pattern.to_string());
                }
            }
            from = close + MARKER.len();
        }
    }
    assert!(
        unchecked.is_empty(),
        "docs/AGENTS.md publishes a site count for {unchecked:?}, which this gate \
         never re-measures, so that number is frozen with nothing to break when \
         it goes stale. Add the pattern to CHECKED_PATTERNS, or drop the claim"
    );
}

#[test]
fn the_error_site_counts_published_in_agents_md_still_hold() {
    let doc_path = root().join("docs/AGENTS.md");
    let doc = std::fs::read_to_string(&doc_path).expect("docs/AGENTS.md must be readable");

    for pattern in CHECKED_PATTERNS.iter().copied() {
        let claim = claimed(&doc, pattern).unwrap_or_else(|| {
            panic!(
                "docs/AGENTS.md no longer carries a `N \\`{pattern}\\` sites` claim; either the \
                 sentence was reworded and this gate must follow it, or the evidence was dropped \
                 and the policy is now asserted without any"
            )
        });
        let actual = count_in_src(pattern);
        assert_eq!(
            claim, actual,
            "docs/AGENTS.md publishes {claim} `{pattern}` sites and the tree has {actual}; a \
             number frozen in prose has no compiler, so update the sentence in BOTH \
             docs/AGENTS.md and docs/AGENTS.pt-BR.md rather than deleting this gate"
        );
    }
}

#[test]
fn the_pt_br_page_carries_the_same_two_numbers() {
    // The two pages are a translated PAIR. A count corrected in one and not the
    // other leaves a reader of the other language holding the stale figure,
    // and `audit_bilingual_docs.sh` compares structure, never digits.
    let en = std::fs::read_to_string(root().join("docs/AGENTS.md")).expect("AGENTS.md");
    let pt = std::fs::read_to_string(root().join("docs/AGENTS.pt-BR.md")).expect("AGENTS.pt-BR.md");
    // The first shape of this test asked only whether the digits appeared
    // ANYWHERE in the page. That accepts the right number attached to the
    // wrong subject, and accepts a page that dropped the sentence entirely so
    // long as some unrelated paragraph happens to carry the same figure. It
    // reported a comparison it never performed. Parsing both pages with the
    // same anchored reader is what makes the claim mean something.
    for pattern in CHECKED_PATTERNS.iter().copied() {
        let n = claimed(&en, pattern).expect("EN claim");
        let pt_n = claimed(&pt, pattern).unwrap_or_else(|| {
            panic!(
                "docs/AGENTS.pt-BR.md no longer states a count beside its `{pattern}` citation, \
                 so the translated pair stopped carrying the same evidence"
            )
        });
        assert_eq!(
            n, pt_n,
            "docs/AGENTS.md publishes {n} for `{pattern}` and docs/AGENTS.pt-BR.md publishes \
             {pt_n}; one of the two languages ships a stale number"
        );
    }
}

/// How many top-level files of one extension live under `dir`.
///
/// Top-level ONLY, mirroring the `fd -d 1` the documented measurement used.
/// Recursing would count `tests/common/mod.rs` and every `scripts/` helper
/// subdirectory, and answer a larger number than the sentence claims — a gate
/// that fails for measuring something else is worse than no gate, because the
/// reader corrects the prose to match a figure nobody meant.
fn count_top_level(dir: &str, ext: &str) -> usize {
    std::fs::read_dir(root().join(dir))
        .unwrap_or_else(|e| panic!("cannot read {dir}: {e}"))
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .filter(|e| e.path().extension().is_some_and(|x| x == ext))
        .count()
}

/// The two `**N**` figures on the line that cites both directories.
///
/// Anchored to the sentence rather than to the page: the same digits appear in
/// unrelated paragraphs, and a reader that scans the whole file would accept
/// the right number attached to the wrong subject. That is the exact defect
/// `the_pt_br_page_carries_the_same_two_numbers` was rewritten to remove, so
/// this reader is anchored from the start.
///
/// Returns `None` when no line cites both, which the caller treats as failure.
fn bolded_pair(doc: &str) -> Option<(usize, usize)> {
    for line in doc.lines() {
        if !(line.contains("`scripts/`") && line.contains("`tests/`")) {
            continue;
        }
        let nums: Vec<usize> = line
            .split("**")
            .filter_map(|piece| piece.parse::<usize>().ok())
            .collect();
        if let [a, b] = nums[..] {
            return Some((a, b));
        }
    }
    None
}

/// THE SUITE-SCALE CLAIM: `docs/TESTING.md` counts its own verifiers.
///
/// # Why this case was added
///
/// Measured 2026-09-01: the page published 42 top-level `.sh` files under
/// `scripts/` and 73 `.rs` files under `tests/`. The tree held 44 and 80. The
/// figures were measured on 2026-08-28 and were correct that day; seven gate
/// files and two scripts landed afterwards and no gate noticed, because this
/// file re-measured error-site counts and nothing else.
///
/// That is the same class the header of this file describes, found a second
/// time on a second page. Correcting 42 to 44 closes the instance. Only
/// re-measuring it here closes the class, which is why both were done.
#[test]
fn the_verifier_suite_scale_published_in_testing_md_still_holds() {
    let en = std::fs::read_to_string(root().join("docs/TESTING.md")).expect("TESTING.md");
    let (claimed_scripts, claimed_tests) = bolded_pair(&en).expect(
        "docs/TESTING.md no longer carries a line citing both `scripts/` and `tests/` with two \
         bolded counts; either the sentence was reworded and this gate must follow it, or the \
         scale claim was dropped",
    );
    assert_eq!(
        claimed_scripts,
        count_top_level("scripts", "sh"),
        "docs/TESTING.md publishes {claimed_scripts} top-level `.sh` files under `scripts/` and \
         the tree has {}; update BOTH docs/TESTING.md and docs/TESTING.pt-BR.md",
        count_top_level("scripts", "sh")
    );
    assert_eq!(
        claimed_tests,
        count_top_level("tests", "rs"),
        "docs/TESTING.md publishes {claimed_tests} top-level `.rs` gate files under `tests/` and \
         the tree has {}; update BOTH docs/TESTING.md and docs/TESTING.pt-BR.md",
        count_top_level("tests", "rs")
    );
}

/// The translated page must carry the SAME pair, not merely a pair.
///
/// `audit_bilingual_docs.sh` compares structure and never digits, so a count
/// corrected in one language and forgotten in the other ships a stale figure to
/// half the readers with every gate green.
#[test]
fn the_pt_br_testing_page_carries_the_same_suite_scale() {
    let en = std::fs::read_to_string(root().join("docs/TESTING.md")).expect("TESTING.md");
    let pt =
        std::fs::read_to_string(root().join("docs/TESTING.pt-BR.md")).expect("TESTING.pt-BR.md");
    let en_pair = bolded_pair(&en).expect("EN suite-scale claim");
    let pt_pair = bolded_pair(&pt).expect(
        "docs/TESTING.pt-BR.md no longer states the suite scale beside its `scripts/` and \
         `tests/` citations, so the translated pair stopped carrying the same evidence",
    );
    assert_eq!(
        en_pair, pt_pair,
        "docs/TESTING.md publishes {en_pair:?} and docs/TESTING.pt-BR.md publishes {pt_pair:?}; \
         one of the two languages ships a stale number"
    );
}

/// The three states a parity row can carry, per published language.
///
/// The pages translate the state words, so the counter needs the pair rather
/// than one hardcoded vocabulary; comparing the two languages against the same
/// arithmetic is what proves the translation did not drift.
const PARITY_STATES: &[(&str, [&str; 3])] = &[
    ("docs/STEALTH_PARITY.md", ["COVERED", "PARTIAL", "ABSENT"]),
    (
        "docs/STEALTH_PARITY.pt-BR.md",
        ["COBERTO", "PARCIAL", "AUSENTE"],
    ),
];

/// How many data rows of `doc` carry `state` in their state column.
///
/// Cells are written `| STATE |` without backticks and the prose cites the
/// same word WITH backticks, so the pipes are what separate the table from the
/// sentence that summarises it. Counting the pipe form is therefore counting
/// rows, never counting mentions.
fn parity_rows(doc: &str, state: &str) -> usize {
    let cell = format!("| {state} |");
    doc.lines()
        .filter(|l| l.starts_with("| ") && l.contains(&cell))
        .count()
}

/// Every run of ASCII digits in `line`, left to right.
fn digit_runs(line: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in line.chars() {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(cur.parse().unwrap_or(0));
            cur = String::new();
        }
    }
    if !cur.is_empty() {
        out.push(cur.parse().unwrap_or(0));
    }
    out
}

/// The summary line of the gap section: the one citing both non-parity states.
///
/// Anchored BELOW the last table row on purpose. Both pages explain the three
/// states in a "how to read the matrix" bullet that also cites `PARTIAL` and
/// `ABSENT`, and that bullet sits ABOVE the table. Measured 2026-09-04: taking
/// the first line citing both picked the explanation, so the gate compared the
/// prose against a sentence that carries no counts and reported an empty digit
/// list — it failed for the wrong reason, and its sibling word-numeral case
/// passed while the spelled-out numbers it exists to catch sat one section
/// lower. A gate anchored to the wrong line is a gate that lies in both
/// directions.
fn summary_line<'a>(doc: &'a str, partial: &str, absent: &str) -> Option<&'a str> {
    let (p, a) = (format!("`{partial}`"), format!("`{absent}`"));
    // `str::Lines` is not `ExactSizeIterator`, so `rposition` is unavailable;
    // scanning forward and keeping the last hit is the same answer.
    let last_row = doc
        .lines()
        .enumerate()
        .filter(|(_, l)| l.starts_with("| ") && l.contains(" | "))
        .map(|(i, _)| i)
        .last()?;
    doc.lines()
        .skip(last_row + 1)
        .find(|l| l.contains(&p) && l.contains(&a))
}

/// The parity summary must agree with the table printed right above it.
///
/// # Why this case exists
///
/// Measured 2026-09-04, the day `docs/STEALTH_PARITY.md` was written: the
/// table held 23 data rows split 13/2/8, and the sentence summarising it said
/// "Seven rows are not at parity: two `PARTIAL` and five `ABSENT`, out of
/// eighteen defences compared". Three numbers were wrong in both languages,
/// and the document existed precisely to stop unmeasured claims.
///
/// The reason the error survived its own author's re-reading is the interesting
/// part, and it is why the sibling case below exists: every numeral was spelled
/// out. `doc_binary_numeral_gate` and the cases above this one all look for a
/// run of ASCII digits, so `eighteen` is invisible to the whole instrument
/// family. The claim did not slip past the gates — no gate could see it.
///
/// The comparison is multiset-based rather than positional because the two
/// languages order the clause differently, and pinning word order would make
/// the gate fail on a legitimate translation instead of on a wrong number.
#[test]
fn the_parity_matrix_prose_counts_match_the_table_it_sits_on() {
    for (path, [covered, partial, absent]) in PARITY_STATES {
        let doc =
            std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
        let (c, p, a) = (
            parity_rows(&doc, covered),
            parity_rows(&doc, partial),
            parity_rows(&doc, absent),
        );
        assert!(
            c + p + a > 0,
            "{path} has no parity rows at all, so either the table was removed or the state \
             words were renamed; a gate that passes on an empty subject reports confidence it \
             never earned"
        );
        let line = summary_line(&doc, partial, absent).unwrap_or_else(|| {
            panic!(
                "{path} no longer has a line citing both `{partial}` and `{absent}`, so the \
                 table lost the sentence that summarises it"
            )
        });
        let mut found = digit_runs(line);
        found.sort_unstable();
        let mut want = vec![p + a, p, a, c + p + a];
        want.sort_unstable();
        assert_eq!(
            found, want,
            "{path} summarises the matrix as {found:?} but the table holds {c} {covered}, {p} \
             {partial} and {a} {absent}, which is {want:?}; the prose and the rows disagree"
        );
    }
}

/// Numerals spelled out in the parity summary hide from every numeral gate.
///
/// This is not a style rule. The whole instrument family in this file, plus
/// `doc_binary_numeral_gate`, reads a run of ASCII digits; a word-number is
/// unreachable to all of them. Allowing one back into the summary line would
/// silently restore the blind spot that let the 2026-09-04 miscount ship.
///
/// Only the summary line is checked. Prose elsewhere on the page may spell
/// numbers out freely, because no gate re-measures those.
#[test]
fn no_spelled_out_numeral_hides_in_the_parity_summary() {
    const WORDS: &[&str] = &[
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
        "twenty",
        "uma",
        "duas",
        "tres",
        "três",
        "quatro",
        "cinco",
        "seis",
        "sete",
        "oito",
        "nove",
        "dez",
        "onze",
        "doze",
        "treze",
        "quatorze",
        "catorze",
        "quinze",
        "dezesseis",
        "dezessete",
        "dezoito",
        "dezenove",
        "vinte",
    ];
    for (path, [_, partial, absent]) in PARITY_STATES {
        let doc =
            std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
        let Some(line) = summary_line(&doc, partial, absent) else {
            continue;
        };
        let lower = line.to_lowercase();
        let hit = WORDS.iter().find(|w| {
            lower
                .split(|c: char| !c.is_alphabetic())
                .any(|tok| tok == **w)
        });
        assert!(
            hit.is_none(),
            "{path} writes the numeral `{}` as a word in the parity summary; every numeral gate \
             in this repository reads ASCII digits, so a spelled-out count is invisible to all \
             of them and cannot be re-measured",
            hit.copied().unwrap_or_default()
        );
    }
}

/// Every `src/…rs:N` the parity matrix cites, as `(path, line)` pairs.
///
/// Citations live inside backticks, so splitting on the tick and taking the odd
/// segments reads code spans and skips the prose between them. A span that is
/// not a source citation — a shell line such as `rg -n "Backspace" src/` — has
/// no trailing `:digits` and drops out on its own.
fn source_citations(doc: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    for span in doc.split('`').skip(1).step_by(2) {
        let Some((path, line)) = span.rsplit_once(':') else {
            continue;
        };
        if !path.starts_with("src/") || !path.ends_with(".rs") {
            continue;
        }
        let Ok(n) = line.parse::<usize>() else {
            continue;
        };
        out.push((path.to_string(), n));
    }
    out
}

/// Every `path:line` the parity matrix cites as evidence must still resolve.
///
/// # Why this case exists
///
/// Measured 2026-09-04: the typo-injection row claimed "the only `Backspace`
/// in the tree is `src/native/interaction/tests.rs:105`, a key-name test",
/// while `rg -n "Backspace" src/ -g '*.rs'` returned THREE files — the key-name
/// map at `src/native/interaction/keys.rs:130` and a production call at
/// `src/browser/session/content/input.rs:310` beside the test. The row's
/// VERDICT was right and its EVIDENCE was false.
///
/// That ordering is what makes the failure worth a gate. A wrong verdict
/// invites argument and gets corrected; false evidence invites belief, and the
/// next reader who runs the cited `rg` finds three files where the page
/// promised one and has no way to tell which half of the row rotted.
///
/// The cases above re-measure the matrix's COUNTS and nothing re-measured its
/// CITATIONS, so a line number could go stale as code moved and no instrument
/// in this repository would notice.
///
/// Existence and reach are checked, never content. Asserting what lives AT a
/// line would freeze every cited file against legitimate edits, and a gate that
/// fires on correct work is a gate people learn to switch off.
///
/// # What green here does NOT mean
///
/// A line that DRIFTED still passes. Measured while writing this case: adding
/// sixteen comment lines above the cited `Backspace` arm moved it from
/// `src/native/interaction/keys.rs:130` to `:145`, and the check stayed green
/// because 130 is still a line the file has. Existence and reach catch a file
/// that MOVED or SHRANK; nothing here catches a citation that slid a few lines
/// inside a file that still contains it. Read a pass as "the citation still
/// resolves", never as "the citation still points at the right code".
#[test]
fn every_source_citation_in_the_parity_matrix_still_resolves() {
    let mut checked = 0usize;
    for (path, _) in PARITY_STATES {
        let doc =
            std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
        let cites = source_citations(&doc);
        assert!(
            !cites.is_empty(),
            "{path} cites no `src/…rs:N` evidence at all; either the matrix stopped anchoring \
             its rows to the tree or the citation format changed, and a gate that passes on an \
             empty subject reports confidence it never earned"
        );
        for (src, line) in cites {
            let abs = root().join(&src);
            let body = std::fs::read_to_string(&abs).unwrap_or_else(|e| {
                panic!(
                    "{path} cites `{src}:{line}` as evidence, but that file cannot be read: {e}; \
                     the code moved and the matrix kept pointing at where it used to live"
                )
            });
            let total = body.lines().count();
            assert!(
                line <= total,
                "{path} cites `{src}:{line}`, but `{src}` holds only {total} lines; the citation \
                 points past the end of the file it claims to prove"
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "no citation was checked across either published language, so this gate proved nothing"
    );
}
