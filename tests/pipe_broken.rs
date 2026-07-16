//! BrokenPipe → exit 141 integration (stdout closed early).

use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

#[test]
fn commands_json_closed_pipe_maps_to_141_or_success() {
    // When the reader closes after a few bytes, the writer may see BrokenPipe.
    // On some platforms the process may finish writing before EPIPE — both 0 and 141 accepted
    // only if stdout path is exercised; we force a large enough payload via commands --json.
    let mut child = Command::new(BIN)
        .args(["commands", "--json"])
        .env("NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    {
        let mut out = child.stdout.take().expect("stdout");
        // Read one byte then drop pipe (close).
        let mut buf = [0u8; 1];
        let _ = std::io::Read::read(&mut out, &mut buf);
        drop(out);
    }
    let status = child.wait().expect("wait");
    let code = status.code().unwrap_or(-1);
    // 141 = BrokenPipe, 0 = finished before close, 13 = SIGPIPE if not handled
    assert!(
        code == 141 || code == 0 || code == 13 || code == 128 + 13,
        "unexpected exit {code}"
    );
}

#[test]
fn error_kind_broken_pipe_is_141() {
    use browser_automation_cli::error::ErrorKind;
    assert_eq!(ErrorKind::BrokenPipe.exit_code(), 141);
}
