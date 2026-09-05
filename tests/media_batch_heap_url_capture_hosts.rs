// SPDX-License-Identifier: MIT OR Apache-2.0
//! Surface and behaviour gates for three capabilities audited as absent:
//! `heap take --url`, `--paths-file` batch on the local media families, and
//! the MITM capture-record host filter.
//!
//! Everything here is offline. No case launches Chrome: the media batch runs on
//! local files, and the other two assert on `--help` and on argv rejection,
//! which are decided before any browser is spawned.

mod common;

/// Run the CLI and return (exit code, stdout, stderr).
fn run(args: &[&str]) -> (i32, String, String) {
    let out = common::cmd()
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn cli");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Write a small real PNG and return its path.
///
/// Encoded rather than inlined as a byte literal: a hand-written PNG is a
/// hand-written CRC, and a fixture the decoder rejects turns every batch
/// assertion below into a test of the fixture.
fn write_png(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let mut img = image::RgbImage::new(4, 4);
    for (x, y, px) in img.enumerate_pixels_mut() {
        *px = image::Rgb([(x * 60) as u8, (y * 60) as u8, 0x20]);
    }
    let path = dir.join(name);
    img.save(&path).expect("write png fixture");
    path
}

#[test]
fn heap_take_help_declares_url() {
    let (code, stdout, _) = run(&["heap", "take", "--help"]);
    assert_eq!(code, 0, "heap take --help must succeed");
    assert!(
        stdout.contains("--url"),
        "heap take must offer --url; without it the snapshot measures about:blank\n{stdout}"
    );
}

#[test]
fn media_help_declares_paths_file() {
    for args in [
        ["image", "info", "--help"],
        ["image", "exif", "--help"],
        ["image", "convert", "--help"],
        ["image", "resize", "--help"],
        ["video", "info", "--help"],
        ["video", "manifest", "--help"],
        ["video", "convert", "--help"],
        ["video", "to-mp3", "--help"],
        ["video", "trim", "--help"],
        ["video", "thumbnail", "--help"],
        ["audio", "info", "--help"],
        ["audio", "convert", "--help"],
        ["audio", "trim", "--help"],
    ] {
        let (code, stdout, _) = run(&args);
        assert_eq!(code, 0, "{args:?} --help must succeed");
        assert!(
            stdout.contains("--paths-file"),
            "{args:?} must offer --paths-file\n{stdout}"
        );
    }
}

#[test]
fn mitm_capture_url_help_separates_hosts_from_capture_hosts() {
    let (code, stdout, _) = run(&["mitm", "capture-url", "--help"]);
    assert_eq!(code, 0, "mitm capture-url --help must succeed");
    assert!(
        stdout.contains("--capture-hosts"),
        "capture-url must offer --capture-hosts\n{stdout}"
    );
    // The two flags are routinely confused; the help must say which is which,
    // or an operator narrows decryption and expects a narrower record.
    assert!(
        stdout.contains("DECRYPTION") && stdout.contains("CAPTURE RECORD"),
        "help must distinguish decryption from record\n{stdout}"
    );
}

#[test]
fn paths_file_and_path_together_are_usage_exit_2() {
    let dir = tempfile::tempdir().expect("tempdir");
    let img = write_png(dir.path(), "a.png");
    let list = dir.path().join("list.txt");
    std::fs::write(&list, format!("{}\n", img.display())).expect("write list");

    let (code, _, stderr) = run(&[
        "--json",
        "image",
        "info",
        "--path",
        img.to_str().expect("utf8"),
        "--paths-file",
        list.to_str().expect("utf8"),
    ]);
    assert_eq!(
        code, 2,
        "naming the input twice is a usage error, not a silent winner\n{stderr}"
    );
}

#[test]
fn paths_file_and_stdin_together_are_usage_exit_2() {
    let dir = tempfile::tempdir().expect("tempdir");
    let list = dir.path().join("list.txt");
    std::fs::write(&list, "/nonexistent.png\n").expect("write list");

    let (code, _, _) = run(&[
        "--json",
        "audio",
        "info",
        "--stdin",
        "--paths-file",
        list.to_str().expect("utf8"),
    ]);
    assert_eq!(code, 2, "--stdin with --paths-file must be usage");
}

#[test]
fn image_info_batch_reports_each_item_and_survives_a_bad_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let good = write_png(dir.path(), "good.png");
    let bad = dir.path().join("missing.png");
    let list = dir.path().join("list.txt");
    std::fs::write(
        &list,
        format!(
            "# a comment line is skipped\n{}\n\n{}\n",
            good.display(),
            bad.display()
        ),
    )
    .expect("write list");

    let (code, stdout, stderr) = run(&[
        "--json",
        "image",
        "info",
        "--paths-file",
        list.to_str().expect("utf8"),
    ]);
    assert_eq!(code, 0, "a batch with one bad item still ran\n{stderr}");

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json envelope");
    let data = v.get("data").expect("data");
    assert_eq!(data.get("count").and_then(|c| c.as_u64()), Some(2));
    assert_eq!(data.get("ok_count").and_then(|c| c.as_u64()), Some(1));
    assert_eq!(data.get("error_count").and_then(|c| c.as_u64()), Some(1));

    let items = data
        .get("items")
        .and_then(|i| i.as_array())
        .expect("items array");
    assert_eq!(items.len(), 2, "one entry per listed path");
    assert_eq!(
        items[0].get("ok").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        items[1].get("ok").and_then(serde_json::Value::as_bool),
        Some(false)
    );
    // The failing item must name itself, or the operator cannot tell which of
    // N inputs went wrong — the whole reason a batch reports per item.
    assert!(items[1]
        .get("path")
        .and_then(|p| p.as_str())
        .is_some_and(|p| p.contains("missing.png")));
    assert!(
        items[1].get("error").is_some(),
        "failing item carries error"
    );
}

/// Write a paths file listing `paths` and return it.
fn write_list(dir: &std::path::Path, paths: &[&std::path::Path]) -> std::path::PathBuf {
    let list = dir.join("list.txt");
    let body: String = paths
        .iter()
        .map(|p| format!("{}\n", p.display()))
        .collect::<Vec<_>>()
        .concat();
    std::fs::write(&list, body).expect("write list");
    list
}

#[test]
fn producing_action_refuses_out_together_with_paths_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = write_png(dir.path(), "a.png");
    let list = write_list(dir.path(), &[&a]);
    let dest = dir.path().join("single.webp");

    let (code, _, stderr) = run(&[
        "--json",
        "image",
        "convert",
        "--paths-file",
        list.to_str().expect("utf8"),
        "--format",
        "webp",
        "-o",
        dest.to_str().expect("utf8"),
    ]);
    // One destination cannot serve N inputs; picking a winner silently is the
    // exact failure this contract exists to prevent.
    assert_eq!(code, 2, "--out with --paths-file must be usage\n{stderr}");
    assert!(!dest.exists(), "the refused run must not have written");
}

#[test]
fn producing_batch_derives_one_output_beside_each_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = write_png(dir.path(), "a.png");
    let b = write_png(dir.path(), "b.png");
    let list = write_list(dir.path(), &[&a, &b]);

    let (code, stdout, stderr) = run(&[
        "--json",
        "image",
        "convert",
        "--paths-file",
        list.to_str().expect("utf8"),
        "--format",
        "webp",
    ]);
    assert_eq!(code, 0, "batch convert must run\n{stderr}");

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json envelope");
    let data = v.get("data").expect("data");
    assert_eq!(data.get("ok_count").and_then(|c| c.as_u64()), Some(2));
    assert_eq!(data.get("error_count").and_then(|c| c.as_u64()), Some(0));

    // The XDG cache default is never used in a batch: its name carries a
    // millisecond stamp, which collides between fast items.
    for name in ["a.webp", "b.webp"] {
        assert!(
            dir.path().join(name).exists(),
            "expected {name} beside its input"
        );
    }

    let items = data
        .get("items")
        .and_then(|i| i.as_array())
        .expect("items array");
    assert!(
        items
            .iter()
            .all(|i| i.get("path_out").and_then(|p| p.as_str()).is_some()),
        "every item reports the destination it wrote"
    );
}

#[test]
fn producing_batch_refuses_to_overwrite_and_keeps_going() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = write_png(dir.path(), "a.png");
    let b = write_png(dir.path(), "b.png");
    // The destination of `a` is already taken; `b` is untouched.
    std::fs::write(dir.path().join("a.webp"), b"not mine to destroy").expect("occupy dest");
    let list = write_list(dir.path(), &[&a, &b]);

    let (code, stdout, stderr) = run(&[
        "--json",
        "image",
        "convert",
        "--paths-file",
        list.to_str().expect("utf8"),
        "--format",
        "webp",
    ]);
    assert_eq!(code, 0, "one blocked item must not end the batch\n{stderr}");

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json envelope");
    let data = v.get("data").expect("data");
    assert_eq!(data.get("ok_count").and_then(|c| c.as_u64()), Some(1));
    assert_eq!(data.get("error_count").and_then(|c| c.as_u64()), Some(1));

    // The occupied file is untouched, and the healthy item still produced.
    assert_eq!(
        std::fs::read(dir.path().join("a.webp")).expect("read dest"),
        b"not mine to destroy",
        "an existing output must never be overwritten"
    );
    assert!(
        dir.path().join("b.webp").exists(),
        "the item that could run still ran"
    );
}

#[test]
fn producing_batch_refuses_when_output_would_be_its_own_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = write_png(dir.path(), "a.png");
    let list = write_list(dir.path(), &[&a]);
    let before = std::fs::read(&a).expect("read input");

    let (code, stdout, _) = run(&[
        "--json",
        "image",
        "convert",
        "--paths-file",
        list.to_str().expect("utf8"),
        "--format",
        "png",
    ]);
    // The sole item failed, so nothing was produced: exit 65, not a green.
    assert_eq!(code, 65, "a run that produced nothing must exit 65");

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json envelope");
    let data = v.get("data").expect("items ride along in data");
    assert_eq!(
        data.get("error_count").and_then(|c| c.as_u64()),
        Some(1),
        "png to png would truncate the source mid-read"
    );
    assert_eq!(
        std::fs::read(&a).expect("read input after"),
        before,
        "the input survives untouched"
    );
}

#[test]
fn a_batch_where_nothing_succeeded_is_a_failure_not_a_green() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = dir.path().join("missing-a.png");
    let b = dir.path().join("missing-b.png");
    let list = write_list(dir.path(), &[&a, &b]);

    let (code, stdout, _) = run(&[
        "--json",
        "image",
        "info",
        "--paths-file",
        list.to_str().expect("utf8"),
    ]);
    // `ok` is the first field the contract tells an agent to read. Zero
    // successes answering `ok: true` is a green that means nothing.
    assert_eq!(code, 65, "a run that produced nothing must exit 65");

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json envelope");
    assert_eq!(
        v.get("ok").and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        v.pointer("/error/kind").and_then(|k| k.as_str()),
        Some("data")
    );
    assert!(
        v.pointer("/error/message")
            .and_then(|m| m.as_str())
            .is_some_and(|m| m.contains("none of the 2 inputs")),
        "the top-level message must say no input produced a result: {v}"
    );
    // The per-item detail must survive: the caller still needs to know WHICH
    // inputs failed and why.
    let items = v
        .pointer("/data/items")
        .and_then(|i| i.as_array())
        .expect("items ride along in data");
    assert_eq!(items.len(), 2, "every failed item is still reported");
    assert!(items
        .iter()
        .all(|i| i.get("error").is_some() && i.get("path").is_some()));
}

#[test]
fn a_partial_batch_still_succeeds_because_it_produced() {
    let dir = tempfile::tempdir().expect("tempdir");
    let good = write_png(dir.path(), "good.png");
    let bad = dir.path().join("missing.png");
    let list = write_list(dir.path(), &[&good, &bad]);

    let (code, stdout, _) = run(&[
        "--json",
        "image",
        "info",
        "--paths-file",
        list.to_str().expect("utf8"),
    ]);
    assert_eq!(code, 0, "a batch that produced something is a success");

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json envelope");
    assert_eq!(v.get("ok").and_then(serde_json::Value::as_bool), Some(true));
    // `error_count` is the signal here; `ok: true` never means "all passed".
    assert_eq!(
        v.pointer("/data/error_count").and_then(|c| c.as_u64()),
        Some(1)
    );
    assert_eq!(
        v.pointer("/data/ok_count").and_then(|c| c.as_u64()),
        Some(1)
    );
}

#[test]
fn empty_paths_file_is_usage_not_an_empty_success() {
    let dir = tempfile::tempdir().expect("tempdir");
    let list = dir.path().join("empty.txt");
    std::fs::write(&list, "# only a comment\n\n").expect("write list");

    let (code, _, _) = run(&[
        "--json",
        "image",
        "info",
        "--paths-file",
        list.to_str().expect("utf8"),
    ]);
    // Zero work requested is a mistake in the list, and reporting success for
    // it hides the mistake behind a green exit.
    assert_eq!(code, 2, "an empty list must be reported, not succeed");
}
