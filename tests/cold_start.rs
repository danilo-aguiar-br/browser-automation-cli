//! NFR: measure light-path wall times (ms) for documentation.

use std::process::Command;
use std::time::Instant;

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

fn timed_ms(args: &[&str]) -> (i32, u128) {
    let t0 = Instant::now();
    let out = Command::new(BIN)
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn");
    let ms = t0.elapsed().as_millis();
    (out.status.code().unwrap_or(-1), ms)
}

#[test]
fn light_paths_finish_and_report_ms() {
    let (c1, ms_version) = timed_ms(&["--version"]);
    let (c2, ms_commands) = timed_ms(&["commands", "--json"]);
    let (c3, ms_schema) = timed_ms(&["schema", "--cmd", "goto", "--json"]);
    assert_eq!(c1, 0, "version");
    assert_eq!(c2, 0, "commands");
    assert_eq!(c3, 0, "schema");
    // Soft budget: light paths under 5s on CI (document actuals on stderr).
    eprintln!(
        "cold-start-ish light paths ms: version={ms_version} commands={ms_commands} schema={ms_schema}"
    );
    assert!(ms_version < 5000);
    assert!(ms_commands < 5000);
    assert!(ms_schema < 5000);
}
