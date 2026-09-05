// SPDX-License-Identifier: MIT OR Apache-2.0
//! Schema-file drift: every `docs/schemas/<cmd>.schema.json` must describe the
//! same INPUT surface the live binary publishes under `schema --cmd <cmd>`.
//!
//! # Why this exists as a Rust gate
//!
//! The capability already existed as `scripts/generate_command_schemas.sh
//! --check`, reached through `scripts/schema-drift-check.sh`. A shell script is
//! not run by `cargo test`, so nothing in the ordinary loop compares the two
//! sides, and the files went stale in silence.
//!
//! Measured 2026-09-04: `docs/schemas/config.schema.json` listed neither
//! `user_data_dir` nor `input_typo_permille`, the two keys added by the most
//! recent waves. Both were added to the binary, to `config list-keys`, and to
//! `docs/CONFIGURATION.md`; the published schema was the one surface with no
//! gate behind it, and `doc_binary_numeral_gate` cannot see this because it
//! compares a COUNT in prose, not the contents of a schema.
//!
//! # Why only `properties` and `required`
//!
//! The file is the INPUT schema — `docs/schemas/version.schema.json` says
//! `"version command input"` in its own title — while `schema --cmd` answers a
//! richer introspection object carrying `output_schema`, `surfaces` and
//! `error_schema` alongside the input shape. Comparing whole documents makes
//! all 72 files diverge on envelope alone, which measures the instrument
//! rather than the artifact. The comparable pair is `properties` and
//! `required`, which is the contract an agent reads before composing argv.
//!
//! # Why three files are exempt
//!
//! `envelope-error`, `envelope-success` and `run-script-step` describe shapes
//! rather than commands, so `schema --cmd` has nothing to answer for them.
//! They are listed by name so a NEW file with no live counterpart fails here
//! instead of being skipped by a wildcard.

use std::collections::BTreeSet;

use serde_json::Value;

mod common;

/// Schema files that describe a shape rather than a command.
const NOT_COMMANDS: &[&str] = &["envelope-error", "envelope-success", "run-script-step"];

/// A near-empty walk would make every file look clean; 60+ is the live floor.
const MIN_FILES: usize = 60;

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The input pair the file and the binary must agree on.
fn input_shape(v: &Value) -> (Value, Value) {
    (
        v.get("properties").cloned().unwrap_or(Value::Null),
        v.get("required").cloned().unwrap_or(Value::Array(vec![])),
    )
}

#[test]
fn every_published_schema_matches_the_live_input_surface() {
    let dir = root().join("docs/schemas");
    let mut checked = 0usize;
    let mut drifted: Vec<String> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();

    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("docs/schemas must be readable")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".schema.json"))
        .collect();
    files.sort();

    for path in files {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_suffix(".schema.json"))
            .expect("schema file name")
            .to_string();

        let disk: Value = serde_json::from_str(
            &std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {name}: {e}")),
        )
        .unwrap_or_else(|e| panic!("{name} must be valid JSON: {e}"));

        let out = common::cmd()
            .args(["--json", "schema", "--cmd", &name])
            .output()
            .expect("schema --cmd must run");
        let live: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
        let data = live.get("data").cloned().unwrap_or(Value::Null);

        if data.is_null() || data.get("properties").is_none() {
            if !NOT_COMMANDS.contains(&name.as_str()) {
                unknown.push(name);
            }
            continue;
        }

        checked += 1;
        if input_shape(&disk) != input_shape(&data) {
            drifted.push(name);
        }
    }

    assert!(
        unknown.is_empty(),
        "schema files with no live counterpart (add to NOT_COMMANDS only if they \
         describe a shape rather than a command): {unknown:?}"
    );
    assert!(
        checked >= MIN_FILES,
        "only {checked} schemas compared; the walk or the binary is broken, and a \
         broken walk makes every file look clean"
    );
    assert!(
        drifted.is_empty(),
        "published schema differs from the live input surface for: {drifted:?}\n\
         The binary is the source of truth; regenerate the file, do not edit the code."
    );
}

#[test]
fn the_gate_reads_the_directory_and_not_a_frozen_list() {
    let names: BTreeSet<String> = std::fs::read_dir(root().join("docs/schemas"))
        .expect("readable")
        .filter_map(Result::ok)
        .filter_map(|e| {
            e.file_name()
                .to_str()
                .and_then(|s| s.strip_suffix(".schema.json"))
                .map(str::to_string)
        })
        .collect();
    assert!(
        names.len() >= MIN_FILES,
        "docs/schemas lost files; confirm the contract shrank on purpose"
    );
    assert!(names.contains("config"), "config schema must be present");
}
