//! The three metadata paths answer without a browser, and without hanging.
//!
//! # This file is NOT a performance gate
//!
//! It used to call itself one — "measure light-path wall times for
//! documentation" — while asserting each path finished under five seconds. Two
//! things were wrong with that.
//!
//! First, five seconds is not a performance claim, it is a HANG claim. Nobody
//! would accept `--version` taking four seconds, so the threshold never
//! detected a regression; it only detected a process that never came back. The
//! threshold is now named for what it does and set far enough out that machine
//! load cannot turn a correct product red.
//!
//! Second, the measured numbers were printed with `eprintln!` and described as
//! documentation. libtest CAPTURES the stderr of a test that passes, so on a
//! green run those numbers went nowhere. They surface only under
//! `--nocapture`, which is the honest way to read them.
//!
//! If this project ever wants a real budget for these paths, it belongs in a
//! benchmark with a baseline to compare against, not in a functional assertion
//! whose verdict depends on who else is using the CPU.

use std::time::Instant;

mod common;

fn timed_ms(args: &[&str]) -> (i32, u128) {
    let t0 = Instant::now();
    let out = common::cmd()
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn");
    let ms = t0.elapsed().as_millis();
    (out.status.code().unwrap_or(-1), ms)
}

/// Past this, a metadata path is wedged rather than slow. See the module doc.
const HANG_MS: u128 = 30_000;

#[test]
fn light_paths_finish_and_report_ms() {
    let (c1, ms_version) = timed_ms(&["--version"]);
    let (c2, ms_commands) = timed_ms(&["commands", "--json"]);
    let (c3, ms_schema) = timed_ms(&["schema", "--cmd", "goto", "--json"]);
    assert_eq!(c1, 0, "version");
    assert_eq!(c2, 0, "commands");
    assert_eq!(c3, 0, "schema");
    // Visible only under `--nocapture`; libtest swallows the stderr of a green test.
    eprintln!("light paths ms: version={ms_version} commands={ms_commands} schema={ms_schema}");

    // Hang guard, NOT a budget. A metadata path that needs half a minute has
    // stopped answering; anything under that is the machine, not the product.
    for (name, ms) in [
        ("version", ms_version),
        ("commands", ms_commands),
        ("schema", ms_schema),
    ] {
        assert!(
            ms < HANG_MS,
            "`{name}` took {ms}ms, past the {HANG_MS}ms hang guard — it is not answering"
        );
    }
}
