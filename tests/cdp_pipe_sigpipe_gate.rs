//! Permanent gate: a Chrome that dies at once must not kill the CLI by SIGPIPE.
//!
//! # What has to break for this to fail
//!
//! `entry.rs` restores the default `SIGPIPE` disposition so a closed stdout
//! ends the CLI with exit 141. The DevTools pipe writer thread inherited it, and
//! when Chrome exited before that thread's first write the kernel killed the
//! whole process. Measured: 26 of 40 launches with `chrome_path=/usr/bin/false`
//! died by signal 13, with no envelope on stdout and a profile left on disk.
//!
//! The race is probabilistic, so the gate launches many times and requires
//! EVERY run to end by exit code with a JSON envelope. A single signal death
//! fails it.
//!
//! # Skip policy
//!
//! No binary or no `/usr/bin/false` means SKIP LOUDLY. Chrome is not needed.

mod common;
use common::{binary_or_skip, skip_with_reason};

const GATE: &str = "cdp_pipe_sigpipe_gate";
const LAUNCHES: usize = 20;

#[test]
#[cfg(unix)]
fn a_chrome_that_exits_at_once_never_kills_the_cli_by_signal() {
    use std::os::unix::process::ExitStatusExt;

    let Some(bin) = binary_or_skip(GATE) else {
        return;
    };
    let fake_chrome = std::path::Path::new("/usr/bin/false");
    if !fake_chrome.exists() {
        skip_with_reason(GATE, "/usr/bin/false is not present on this host.");
        return;
    }
    // Own XDG tree: `config set chrome_path` must not leak into sibling gates
    // that share the suite sandbox.
    let xdg = tempfile::tempdir().expect("temp XDG root");
    let with_env = |c: &mut std::process::Command| {
        c.env("HOME", xdg.path())
            .env("XDG_CONFIG_HOME", xdg.path().join("config"))
            .env("XDG_CACHE_HOME", xdg.path().join("cache"))
            .env("XDG_DATA_HOME", xdg.path().join("data"))
            .env("XDG_STATE_HOME", xdg.path().join("state"));
    };
    let mut set = std::process::Command::new(&bin);
    with_env(&mut set);
    let configured = set
        .args(["-q", "--json", "config", "set", "chrome_path"])
        .arg(fake_chrome)
        .output()
        .expect("config set runs");
    assert!(configured.status.success(), "config set chrome_path failed");

    for launch in 0..LAUNCHES {
        let mut goto = std::process::Command::new(&bin);
        with_env(&mut goto);
        let out = goto
            .args([
                "-q",
                "--json",
                "--timeout",
                "30",
                "--headless",
                "goto",
                "about:blank",
            ])
            .output()
            .expect("goto runs");
        assert_eq!(
            out.status.signal(),
            None,
            "launch {launch} was killed by signal {:?} instead of exiting",
            out.status.signal()
        );
        let envelope: serde_json::Value = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|e| panic!("launch {launch} left no JSON envelope: {e}"));
        assert_eq!(envelope["ok"], false, "a missing browser cannot succeed");
    }
}
