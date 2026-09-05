// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fail when `print-pdf` reports a rotation it did not perform.
//!
//! # Why this gate exists
//!
//! The `run` step declared `landscape`, read it, and wrote it onto the returned
//! object AFTER the PDF had already been produced, under a comment claiming it
//! was "passed through if session supports". It was passed nowhere: `print_pdf`
//! took no such argument, so `Page.printToPDF` used its own default of `false`.
//!
//! Measured 2026-08-31, before the fix: `{"cmd":"print-pdf","landscape":true}`
//! answered `ok: true` carrying `landscape: true` and produced a PORTRAIT
//! document. That is the worst of the three ways a declared key can go wrong.
//! A key that is REFUSED tells the caller. A key that is IGNORED at least says
//! nothing. A key STAMPED onto the envelope fabricates positive evidence, so
//! every automatic check that reads the envelope approves, and only opening the
//! artifact disagrees.
//!
//! # Why it reads the artifact and not the envelope
//!
//! Asserting on `data.landscape` would have passed against the broken build,
//! because that field was exactly the lie. The page geometry lives in the PDF's
//! `/MediaBox`, which comes from Chrome and not from this codebase, so it is
//! the one witness the product cannot author.
//!
//! # Why the page is blank
//!
//! `allow_empty` prints the open `about:blank` with no navigation, so the gate
//! needs no fixture server and no network egress. Rotation is a property of the
//! page box, not of its content.
//!
//! # Why `/MediaBox` and not `/Rotate`
//!
//! A PDF can also express landscape as a portrait box carrying `/Rotate 90`,
//! which would make this witness blind. Measured 2026-08-31 on three documents
//! this binary produced: `/Rotate` appears zero times and each file carries
//! exactly one `/MediaBox`, so Chrome swaps the box rather than rotating it.
//!
//! Should that ever change, this gate FAILS rather than passing quietly, and a
//! loud false alarm is the direction a witness should break in.

use std::path::Path;

mod common;
use common::{binary_or_skip, chrome_not_ready, isolated_cmd};

const GATE: &str = "print_pdf_landscape_gate";

/// Width and height of the first `/MediaBox` in a PDF, in points.
///
/// Chrome writes the box uncompressed in the page object, so a byte scan is
/// enough and pulling a PDF parser in for four numbers would not be.
fn media_box(bytes: &[u8]) -> Option<(f64, f64)> {
    let needle = b"/MediaBox";
    let start = bytes.windows(needle.len()).position(|w| w == needle)? + needle.len();
    let open = start + bytes[start..].iter().position(|b| *b == b'[')?;
    let close = open + bytes[open..].iter().position(|b| *b == b']')?;
    let inner = std::str::from_utf8(&bytes[open + 1..close]).ok()?;
    let nums: Vec<f64> = inner
        .split_whitespace()
        .filter_map(|t| t.parse::<f64>().ok())
        .collect();
    match nums[..] {
        [x1, y1, x2, y2] => Some(((x2 - x1).abs(), (y2 - y1).abs())),
        _ => None,
    }
}

/// Run one `print-pdf` step and hand back the geometry of what it wrote.
fn print_and_measure(bin: &Path, dir: &Path, name: &str, landscape: bool) -> (f64, f64) {
    let pdf = dir.join(format!("{name}.pdf"));
    let script = dir.join(format!("{name}.ndjson"));
    let step = if landscape {
        format!(
            "{{\"cmd\":\"print-pdf\",\"path\":\"{}\",\"landscape\":true,\"allow_empty\":true}}\n",
            pdf.display()
        )
    } else {
        format!(
            "{{\"cmd\":\"print-pdf\",\"path\":\"{}\",\"allow_empty\":true}}\n",
            pdf.display()
        )
    };
    std::fs::write(&script, step).expect("write step script");

    let output = isolated_cmd(bin)
        .args([
            "--timeout",
            "120",
            "--json",
            "run",
            "--script",
            script.to_str().expect("script path is utf-8"),
        ])
        .output()
        .expect("spawn run --script");
    assert!(
        output.status.success(),
        "print-pdf (landscape={landscape}) failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let bytes = std::fs::read(&pdf).unwrap_or_else(|e| {
        panic!(
            "the step reported success but wrote no {}: {e}",
            pdf.display()
        )
    });
    media_box(&bytes).unwrap_or_else(|| panic!("no parseable /MediaBox in {}", pdf.display()))
}

#[test]
fn landscape_rotates_the_page_and_the_default_does_not() {
    let Some(bin) = binary_or_skip(GATE) else {
        return;
    };
    if chrome_not_ready(GATE, &bin) {
        return;
    }
    let dir = std::env::temp_dir().join(format!("ba-{GATE}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create work dir");

    let (lw, lh) = print_and_measure(&bin, &dir, "landscape", true);
    let (pw, ph) = print_and_measure(&bin, &dir, "portrait", false);

    // The control is what makes the first assertion mean anything: without it a
    // build that printed landscape unconditionally would also pass.
    assert!(
        lw > lh,
        "landscape:true produced a {lw}x{lh} page, which is not wider than it is tall; the \
         parameter is not reaching `Page.printToPDF` and the envelope's `landscape` field is \
         reporting a rotation nobody performed"
    );
    assert!(
        ph > pw,
        "the default produced a {pw}x{ph} page, which is not taller than it is wide; either the \
         CDP default changed or landscape is now being sent unconditionally, and in both cases \
         the control no longer discriminates"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
