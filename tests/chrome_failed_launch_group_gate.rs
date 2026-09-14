//! Permanent gate: a failed Chrome launch leaves no member of Chrome's group alive.
//!
//! # What has to break for this to fail
//!
//! When the launch failed, the owner killed Chrome's pid and nothing else, and
//! FINALIZE signalled nothing because the lifecycle ledger learns the process
//! group only after a SUCCESSFUL launch. Measured: a descendant that stayed in
//! Chrome's group outlived the CLI, was adopted by the subreaper, and kept the
//! DevTools and output pipes open.
//!
//! The fake browser below forks a `sleep` that stays in its group and records
//! the pid, then exits before DevTools ever answers. The gate requires that pid
//! to be gone once the CLI has returned.
//!
//! # Skip policy
//!
//! No binary, no `/bin/sh` or no `sleep` means SKIP LOUDLY. Chrome is not needed.

mod common;
use common::{binary_or_skip, skip_with_reason};

const GATE: &str = "chrome_failed_launch_group_gate";

#[test]
#[cfg(target_os = "linux")]
fn a_failed_launch_kills_the_descendants_left_in_chromes_group() {
    use std::os::unix::fs::PermissionsExt;

    let Some(bin) = binary_or_skip(GATE) else {
        return;
    };
    if !std::path::Path::new("/bin/sh").exists() {
        skip_with_reason(GATE, "/bin/sh is not present on this host.");
        return;
    }
    let root = tempfile::tempdir().expect("temp root");
    let pid_file = root.path().join("descendant.pid");
    let fake = root.path().join("fake-chrome");
    std::fs::write(
        &fake,
        format!(
            "#!/bin/sh\nsleep 60 &\necho $! > '{}'\nexit 0\n",
            pid_file.display()
        ),
    )
    .expect("fake browser written");
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let with_env = |c: &mut std::process::Command| {
        c.env("HOME", root.path())
            .env("XDG_CONFIG_HOME", root.path().join("config"))
            .env("XDG_CACHE_HOME", root.path().join("cache"))
            .env("XDG_DATA_HOME", root.path().join("data"))
            .env("XDG_STATE_HOME", root.path().join("state"));
    };
    for (key, value) in [
        ("chrome_path", fake.display().to_string()),
        ("chrome_startup_timeout_secs", "3".to_string()),
    ] {
        let mut set = std::process::Command::new(&bin);
        with_env(&mut set);
        let out = set
            .args(["-q", "--json", "config", "set", key, &value])
            .output()
            .expect("config set runs");
        assert!(out.status.success(), "config set {key} failed");
    }

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
    let envelope: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("a failed launch still emits an envelope");
    assert_eq!(envelope["ok"], false, "a fake browser cannot succeed");

    let Ok(pid) = std::fs::read_to_string(&pid_file) else {
        skip_with_reason(GATE, "the fake browser never ran, so nothing was measured.");
        return;
    };
    let stat = std::path::Path::new("/proc").join(pid.trim()).join("stat");
    // A zombie waiting for its new parent to reap it is already dead.
    let alive = std::fs::read_to_string(&stat)
        .ok()
        .and_then(|s| s.rsplit_once(") ").map(|(_, rest)| !rest.starts_with('Z')))
        .unwrap_or(false);
    if alive {
        let _ = std::process::Command::new("kill").arg(pid.trim()).status();
    }
    assert!(
        !alive,
        "pid {} from Chrome's group outlived the failed launch",
        pid.trim()
    );
}
