//! Third parity layer: precondition and effect, not just name (GAP-044).
//!
//! # Why this gate exists
//!
//! The two pre-existing parity gates enumerate names and parameters. Both are
//! satisfiable by a scan. Semantics is not: it lives inside the reference
//! handler declaration (`readOnlyHint`, `conditions`, `blockedByDialog`), and
//! that is precisely why the pending-dialog guard, the `@eN` invalidation
//! marker and the undeclared intentional divergences all survived fifteen audit
//! sections with the scoreboard reading green.
//!
//! Those three subjects are covered elsewhere, not here: the dialog guard in
//! `src/capability/tests.rs`, the invalidation marker in `src/capability/` and
//! `src/commands/run/execute/`, and the divergence declarations in the matrix
//! this gate reads. Naming them above is context, not coverage.
//!
//! # Skip policy
//!
//! The reference tree and `docs_prd/` are gitignored, so a clean checkout does
//! not have them. When they are absent this test SKIPS LOUDLY rather than
//! passing quietly — a silent pass here would recreate the exact failure mode
//! the gate was written to prevent.

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Inputs the generator needs. Missing inputs mean "cannot verify", never "ok".
fn missing_inputs() -> Vec<String> {
    let r = root();
    [
        "base_conhecimento_chrome-devtools-mcp-main/src/tools",
        "docs_prd/parity_devtools_matrix.md",
        "docs_prd/parity_intentional_divergences.json",
        "scripts/gen-parity-matrix.py",
        "scripts/extract-toolref-handlers.py",
    ]
    .iter()
    .filter(|p| !r.join(p).exists())
    .map(|p| (*p).to_string())
    .collect()
}

fn built_binary() -> Option<PathBuf> {
    // The generator drives the real binary; `cargo test` may run before a
    // debug build exists (e.g. `--release` only).
    let candidate = root().join("target/debug/browser-automation-cli");
    candidate.exists().then_some(candidate)
}

#[test]
fn parity_matrix_is_current_and_has_no_undeclared_divergence() {
    let missing = missing_inputs();
    if !missing.is_empty() {
        eprintln!(
            "SKIP parity_semantics: absent inputs: {}. \
             This is NOT a pass; regenerate with scripts/gen-parity-matrix.py \
             on a tree that has the reference checkout.",
            missing.join(", ")
        );
        return;
    }
    if built_binary().is_none() {
        eprintln!(
            "SKIP parity_semantics: target/debug/browser-automation-cli absent. \
             This is NOT a pass; run `cargo build` first."
        );
        return;
    }

    let out = Command::new("python3")
        .arg(root().join("scripts/gen-parity-matrix.py"))
        .arg("--check")
        .current_dir(root())
        .output()
        .expect("run gen-parity-matrix.py");

    assert!(
        out.status.success(),
        "three-layer parity check failed.\n\
         Either the matrix is stale (run `python3 scripts/gen-parity-matrix.py`) \
         or a divergence is open and undeclared. Declare deliberate ones in \
         docs_prd/parity_intentional_divergences.json WITH a justification.\n\
         stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Every reference `conditions` token must resolve to a CLI gate flag.
///
/// An unmapped token means the CLI is more permissive than the reference on a
/// surface the reference chose to gate — a real precondition divergence, and
/// the class of finding a name-only matrix cannot see.
#[test]
fn reference_conditions_are_enumerated_by_the_generator() {
    if !missing_inputs().is_empty() {
        eprintln!("SKIP: reference tree absent; this is NOT a pass");
        return;
    }
    let src = std::fs::read_to_string(root().join("scripts/gen-parity-matrix.py"))
        .expect("read generator");
    let out = Command::new("python3")
        .arg(root().join("scripts/extract-toolref-handlers.py"))
        .current_dir(root())
        .output()
        .expect("run extractor");
    assert!(out.status.success(), "extractor failed");
    let text = String::from_utf8_lossy(&out.stdout);

    let mut unknown: Vec<String> = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let row: serde_json::Value = serde_json::from_str(line).expect("ndjson row");
        for c in row["conditions"].as_array().into_iter().flatten() {
            let token = c.as_str().unwrap_or_default();
            if !token.is_empty() && !src.contains(&format!("\"{token}\"")) {
                unknown.push(token.to_string());
            }
        }
    }
    unknown.sort();
    unknown.dedup();
    assert!(
        unknown.is_empty(),
        "reference gate conditions absent from CONDITION_TO_FLAG in \
         scripts/gen-parity-matrix.py: {unknown:?}. \
         Map each to a CLI flag, or to None to report it as an open gap."
    );
}

/// The tool inventory must come from the live surface, never a frozen number.
///
/// A hard-coded count is what let the PRD claim 51 tools while the reference
/// carried 53, including one whose name is built from a module constant and is
/// invisible to a literal scan.
#[test]
fn tool_count_is_not_hardcoded_in_the_matrix_generator() {
    let src = std::fs::read_to_string(root().join("scripts/gen-parity-matrix.py"))
        .expect("read generator");
    for frozen in ["51", "52", "53"] {
        let needle = format!("== {frozen}");
        assert!(
            !src.contains(&needle),
            "generator compares against the frozen count {frozen}; \
             the inventory must be read live"
        );
    }
    assert!(
        src.contains("len(reference)"),
        "generator must report the live reference length"
    );
}

/// The reference extractor must not silently drop the `slim/` variant surface
/// into the main inventory, and must resolve constant-named tools.
#[test]
fn extractor_matches_the_independent_fixture_inventory() {
    if !missing_inputs().is_empty() {
        eprintln!("SKIP: reference tree absent; this is NOT a pass");
        return;
    }
    let fixture = root().join("tests/fixtures/tool-reference.md");
    if !fixture.exists() {
        eprintln!("SKIP: fixture absent; this is NOT a pass");
        return;
    }
    let out = Command::new("python3")
        .arg(root().join("scripts/extract-toolref-handlers.py"))
        .current_dir(root())
        .output()
        .expect("run extractor");
    assert!(out.status.success(), "extractor failed");
    let mut extracted: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l).expect("ndjson")["tool"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    extracted.sort();

    let text = std::fs::read_to_string(&fixture).expect("read fixture");
    let mut listed: Vec<String> = text
        .lines()
        .filter_map(|l| l.strip_prefix("### `"))
        .filter_map(|l| l.strip_suffix('`'))
        .map(str::to_string)
        .collect();
    listed.sort();

    assert_eq!(
        extracted, listed,
        "handler extraction disagrees with the independent fixture inventory"
    );
}

/// A triaged open divergence must carry a tracked gap id.
///
/// Without this, `known_open` degrades into a silencer: anyone could park a
/// finding there and the gate would go green with nobody owning the decision.
#[test]
fn triaged_divergences_carry_a_tracked_gap_id() {
    let path = root().join("docs_prd/parity_intentional_divergences.json");
    if !path.exists() {
        eprintln!("SKIP: registry absent; this is NOT a pass");
        return;
    }
    let raw = std::fs::read_to_string(&path).expect("read registry");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("registry json");
    for entry in v["known_open"].as_array().into_iter().flatten() {
        let id = entry["id"].as_str().unwrap_or("<no id>");
        let tracked = entry["tracked_in"].as_str().unwrap_or_default();
        assert!(
            tracked.starts_with("GAP-"),
            "known_open entry `{id}` has no tracked gap id"
        );
        assert!(
            !entry["decision_needed"]
                .as_str()
                .unwrap_or_default()
                .is_empty(),
            "known_open entry `{id}` must state the pending decision"
        );
    }
    for entry in v["tools"].as_array().into_iter().flatten() {
        let id = entry["id"].as_str().unwrap_or("<no id>");
        assert!(
            !entry["why"].as_str().unwrap_or_default().is_empty(),
            "intentional divergence `{id}` must carry a justification"
        );
    }
}

/// The PRD must never freeze the reference tool count in prose (GAP-024/GAP-025).
///
/// The frozen number said 51 while the reference carried 53. Two of the missing
/// tools were invisible to a name scan: one is named through a module constant,
/// and the `slim/` family is a separate reduced surface. A number that wrong,
/// repeated across nine sections, is what made the coverage claim false.
#[test]
fn prd_does_not_freeze_the_reference_tool_count() {
    let prd = root().join("docs_prd/prd_browser-automation-cli.md");
    if !prd.exists() {
        eprintln!("SKIP: PRD absent (gitignored); this is NOT a pass");
        return;
    }
    let text = std::fs::read_to_string(&prd).expect("read PRD");
    let mut offenders: Vec<String> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let lower = line.to_lowercase();
        // A bare number next to "tools" is the shape that went stale.
        let frozen = lower.contains(" tools")
            && line.split_whitespace().any(|w| {
                w.trim_matches(|c: char| !c.is_ascii_digit()).len() == 2
                    && w.chars().any(|c| c.is_ascii_digit())
                    && w.chars().all(|c| c.is_ascii_digit() || c == '/')
            });
        if frozen {
            offenders.push(format!("{}: {}", i + 1, line.trim()));
        }
    }
    assert!(
        offenders.is_empty(),
        "PRD freezes a reference tool count in prose; point at \
         docs_prd/parity_devtools_matrix.md instead:\n{}",
        offenders.join("\n")
    );
}

/// `view`'s detail flag is `--detailed`; `--verbose` is the global log flag.
///
/// The PRD mapped `take_snapshot` onto `view --verbose`. That invocation exits
/// zero and returns a reduced tree, so an agent following the PRD gets wrong
/// output with no error to notice.
#[test]
fn prd_maps_take_snapshot_to_the_detail_flag_not_the_log_flag() {
    let prd = root().join("docs_prd/prd_browser-automation-cli.md");
    if !prd.exists() {
        eprintln!("SKIP: PRD absent (gitignored); this is NOT a pass");
        return;
    }
    let text = std::fs::read_to_string(&prd).expect("read PRD");
    let bad: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("take_snapshot") && l.contains("--verbose"))
        .collect();
    assert!(
        bad.is_empty(),
        "PRD maps take_snapshot onto the global log flag `--verbose`; \
         the detail flag is `--detailed`:\n{}",
        bad.join("\n")
    );
    assert!(
        text.contains("`take_snapshot` → `view` com `--detailed`"),
        "PRD must state the take_snapshot -> view --detailed mapping"
    );
}

/// Commands the PRD promises but the binary lacks must be marked PENDENTE.
#[test]
fn prd_marks_absent_commands_as_pending() {
    let prd = root().join("docs_prd/prd_browser-automation-cli.md");
    if !prd.exists() {
        eprintln!("SKIP: PRD absent (gitignored); this is NOT a pass");
        return;
    }
    let text = std::fs::read_to_string(&prd).expect("read PRD");
    for cmd in ["download", "feed", "sitemap", "agent", "stats"] {
        let marked = text.lines().any(|l| {
            l.trim_start().starts_with(&format!("- `{cmd}"))
                && l.contains("PENDENTE")
                && l.contains("não existe no binário")
        });
        assert!(
            marked,
            "PRD lists `{cmd}` without marking it PENDENTE; an agent that \
             discovers the surface from the PRD gets exit 2"
        );
    }
}

/// The flag reconciliation must stay in step with the PRD and the binary.
#[test]
fn flag_reconciliation_is_current() {
    let prd = root().join("docs_prd/prd_browser-automation-cli.md");
    let gen = root().join("scripts/gen-flag-reconciliation.py");
    if !prd.exists() || !gen.exists() || built_binary().is_none() {
        eprintln!("SKIP: inputs absent; this is NOT a pass");
        return;
    }
    let out = Command::new("python3")
        .arg(&gen)
        .arg("--check")
        .current_dir(root())
        .output()
        .expect("run gen-flag-reconciliation.py");
    assert!(
        out.status.success(),
        "flag reconciliation is stale; run \
         `python3 scripts/gen-flag-reconciliation.py`\nstderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
