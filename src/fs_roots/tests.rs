// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests for allowed-root containment.

use super::*;
use crate::error::ErrorKind;
use std::path::Path;

/// A scratch dir under the system temp root, removed when the guard drops.
///
/// The guard must be BOUND by the caller. `temp_child("x").path()` compiles and
/// then deletes the directory before the assertion runs, because the temporary
/// lives only to the end of the statement.
///
/// This used to be `temp_dir().join("bac-roots-test-{name}")` with no removal at
/// all, so every `cargo test` left three directories behind for good.
fn temp_child(name: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("bac-roots-test-{name}-"))
        .tempdir()
        .expect("create scratch dir")
}

#[test]
fn temp_dir_is_inside_default_roots() {
    let tmp = temp_child("inside");
    let dir = tmp.path();
    let file = dir.join("fixture.html");
    std::fs::write(&file, b"<html></html>").expect("write fixture");
    assert!(ensure_within_roots(&file, PathUse::Read, false).is_ok());
    let _ = std::fs::remove_file(&file);
}

#[test]
fn etc_passwd_is_rejected_for_read() {
    let err = ensure_within_roots(Path::new("/etc/passwd"), PathUse::Read, false);
    // On hosts without /etc the canonicalisation fails, which is also a refusal.
    assert!(err.is_err(), "reading /etc/passwd must not be allowed");
}

#[test]
fn escape_flag_permits_the_same_path() {
    if !Path::new("/etc/passwd").exists() {
        return;
    }
    assert!(ensure_within_roots(Path::new("/etc/passwd"), PathUse::Read, true).is_ok());
}

#[test]
fn traversal_out_of_a_root_is_rejected() {
    let tmp = temp_child("traversal");
    let dir = tmp.path();
    let sneaky = dir.join("../../../../etc/passwd");
    if !Path::new("/etc/passwd").exists() {
        return;
    }
    assert!(ensure_within_roots(&sneaky, PathUse::Read, false).is_err());
}

#[test]
fn file_url_outside_roots_is_rejected() {
    if !Path::new("/etc/passwd").exists() {
        return;
    }
    let err = ensure_file_url_allowed("file:///etc/passwd", false);
    assert!(err.is_err(), "file:///etc/passwd must be refused");
    assert!(ensure_file_url_allowed("file:///etc/passwd", true).is_ok());
}

#[test]
fn refusal_is_policy_not_argv() {
    // GAP-020 class: `usage` would tell the agent to fix an argv that is
    // already correct, and the correction loop never converges.
    if !Path::new("/etc/passwd").exists() {
        return;
    }
    let err = ensure_within_roots(Path::new("/etc/passwd"), PathUse::Read, false)
        .expect_err("must refuse");
    assert_eq!(err.kind(), ErrorKind::CapabilityDisabled);
    assert_eq!(err.exit_code(), 64);
    assert!(
        err.suggestion()
            .map(|s| s.contains("allow-outside-roots"))
            .unwrap_or(false),
        "suggestion must name the flag: {:?}",
        err.suggestion()
    );
}

#[test]
fn non_file_urls_are_not_gated_here() {
    assert!(ensure_file_url_allowed("https://example.com/", false).is_ok());
    assert!(ensure_file_url_allowed(crate::constants::ABOUT_BLANK, false).is_ok());
}

#[test]
fn write_target_that_does_not_exist_yet_resolves_via_parent() {
    let tmp = temp_child("write");
    let dir = tmp.path();
    let target = dir.join("new-artifact.png");
    let _ = std::fs::remove_file(&target);
    assert!(ensure_within_roots(&target, PathUse::Write, false).is_ok());
}
