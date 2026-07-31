// SPDX-License-Identifier: MIT OR Apache-2.0
//! Preflight unit tests: `include` expansion, cycles and depth.

use std::path::{Path, PathBuf};

use crate::browser::CaptureOpts;
use crate::error::ErrorKind;

use super::super::flags::RunFlags;
use super::include::step_cmd;
use super::preflight_script;

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).expect("write fixture");
    p
}

/// An indirect cycle across three files is detected too.
#[test]
fn indirect_include_cycle_is_detected() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "b.jsonl",
        "{\"cmd\":\"include\",\"path\":\"c.jsonl\"}\n",
    );
    write(
        dir.path(),
        "c.jsonl",
        "{\"cmd\":\"include\",\"path\":\"a.jsonl\"}\n",
    );
    let a = write(
        dir.path(),
        "a.jsonl",
        "{\"cmd\":\"include\",\"path\":\"b.jsonl\"}\n",
    );
    let err = preflight_script(&a, RunFlags::default(), CaptureOpts::default())
        .expect_err("indirect cycle must fail");
    assert_eq!(err.kind(), ErrorKind::Data);
    assert!(err.message().contains("include cycle"));
}

/// A missing include target names the file that could not be read.

#[test]
fn missing_include_target_is_no_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = write(
        dir.path(),
        "a.jsonl",
        "{\"cmd\":\"include\",\"path\":\"gone.jsonl\"}\n",
    );
    let err = preflight_script(&a, RunFlags::default(), CaptureOpts::default())
        .expect_err("missing include must fail");
    assert_eq!(err.kind(), ErrorKind::NoInput);
    assert!(err.message().contains("gone.jsonl"));
}

/// An include without a path is a data error, never a silent no-op.

#[test]
fn include_without_path_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = write(dir.path(), "a.jsonl", "{\"cmd\":\"include\"}\n");
    let err = preflight_script(&a, RunFlags::default(), CaptureOpts::default())
        .expect_err("include without path must fail");
    assert_eq!(err.kind(), ErrorKind::Data);
}

/// An empty script is refused before BORN.

#[test]
fn empty_script_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = write(dir.path(), "a.jsonl", "\n");
    let err = preflight_script(&a, RunFlags::default(), CaptureOpts::default())
        .expect_err("empty script must fail");
    assert_eq!(err.kind(), ErrorKind::Data);
}

/// GAP-034 pillar 3: a typo field on an include is caught, not ignored.

#[test]
fn include_with_unknown_field_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "inner.jsonl", "{\"cmd\":\"view\"}\n");
    let a = write(
        dir.path(),
        "a.jsonl",
        "{\"cmd\":\"include\",\"path\":\"inner.jsonl\",\"nope\":1}\n",
    );
    let err = preflight_script(&a, RunFlags::default(), CaptureOpts::default())
        .expect_err("typo field must fail");
    assert_eq!(err.kind(), ErrorKind::Usage);
    assert!(err.message().contains("nope"), "{}", err.message());
}

/// The reusable-prefix use case: one auth file included by two scripts.

#[test]
fn shared_prefix_expands_into_each_caller() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "auth.jsonl",
        "{\"cmd\":\"goto\",\"url\":\"about:blank\"}\n{\"cmd\":\"view\"}\n",
    );
    for name in ["one.jsonl", "two.jsonl"] {
        let caller = write(
            dir.path(),
            name,
            "{\"cmd\":\"include\",\"path\":\"auth.jsonl\"}\n{\"cmd\":\"reload\"}\n",
        );
        let steps = preflight_script(&caller, RunFlags::default(), CaptureOpts::default())
            .expect("shared prefix must expand");
        let cmds: Vec<&str> = steps.iter().map(step_cmd).collect();
        assert_eq!(cmds, vec!["goto", "view", "reload"], "caller {name}");
    }
}

/// Nested includes expand transitively.

#[test]
fn nested_includes_expand_transitively() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "c.jsonl", "{\"cmd\":\"view\"}\n");
    write(
        dir.path(),
        "b.jsonl",
        "{\"cmd\":\"include\",\"path\":\"c.jsonl\"}\n{\"cmd\":\"reload\"}\n",
    );
    let a = write(
        dir.path(),
        "a.jsonl",
        "{\"cmd\":\"goto\",\"url\":\"about:blank\"}\n{\"cmd\":\"include\",\"path\":\"b.jsonl\"}\n",
    );
    let steps = preflight_script(&a, RunFlags::default(), CaptureOpts::default())
        .expect("nested include must expand");
    let cmds: Vec<&str> = steps.iter().map(step_cmd).collect();
    assert_eq!(cmds, vec!["goto", "view", "reload"]);
}

/// A valid script passes and returns its steps unchanged.

#[test]
fn valid_script_passes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = write(
        dir.path(),
        "a.jsonl",
        "{\"cmd\":\"goto\",\"url\":\"about:blank\"}\n{\"cmd\":\"view\"}\n",
    );
    let steps =
        preflight_script(&a, RunFlags::default(), CaptureOpts::default()).expect("must pass");
    assert_eq!(steps.len(), 2);
    assert_eq!(step_cmd(&steps[0]), "goto");
}
