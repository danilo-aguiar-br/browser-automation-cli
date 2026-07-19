//! GAP-017 / RES-03: residual zero for CLI marker profiles + Chromium side-channels
//! after one-shot DIE (PRD §5N).

use std::fs;
use std::process::Command;
use std::time::Duration;

const BIN: &str = env!("CARGO_BIN_EXE_browser-automation-cli");

#[test]
fn goto_leaves_zero_cli_chrome_marker_dirs() {
    let before = browser_automation_cli::residual::list_cli_chrome_marker_dirs();
    let output = Command::new(BIN)
        .args(["--json", "goto", "about:blank"])
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn goto");
    assert!(
        output.status.success(),
        "goto failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let after = browser_automation_cli::residual::list_cli_chrome_marker_dirs();
    // Any dirs that appeared during the run must be gone after DIE.
    let leaked: Vec<_> = after.iter().filter(|p| !before.contains(p)).collect();
    assert!(
        leaked.is_empty(),
        "leaked CLI chrome marker dirs after one-shot: {leaked:?}"
    );
}

#[test]
fn goto_does_not_leave_new_chromium_singleton_orphans() {
    let before = count_chromium_singleton_dirs();
    let pdf = std::env::temp_dir().join(format!(
        "browser-automation-cli-residual-{}.pdf",
        std::process::id()
    ));
    let output = Command::new(BIN)
        .args(["--json", "print-pdf", "--url", "about:blank", "--path"])
        .arg(&pdf)
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn print-pdf");
    assert!(
        output.status.success(),
        "print-pdf failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let after = count_chromium_singleton_dirs();
    // FINALIZE dual scavenge + BORN of this process may *reduce* count.
    // Product law: must not grow.
    assert!(
        after <= before,
        "chromium singleton tmp dirs grew after one-shot: before={before} after={after}"
    );
}

#[test]
fn born_gc_wipes_stale_singleton_fixture() {
    let tmp = std::env::temp_dir();
    let dir = tmp.join(format!(
        "org.chromium.Chromium.rstest{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("mkdir fixture");
    let _ = fs::write(dir.join("SingletonSocket"), b"");
    #[cfg(unix)]
    let _ = std::os::unix::fs::symlink("fixture", dir.join("SingletonCookie"));
    #[cfg(not(unix))]
    let _ = fs::write(dir.join("SingletonCookie"), b"fixture");

    // Force age floor zero via library API (unit path). Integration BORN uses 60s;
    // here we prove the wipe predicate for Singleton-only owned dirs.
    let wiped = browser_automation_cli::residual::scavenge_stale_singleton_orphans_with_min_age(
        Duration::ZERO,
    );
    assert!(
        wiped.iter().any(|p| p == &dir) || !dir.exists(),
        "fixture must be wiped by stale GC: wiped={wiped:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn doctor_json_includes_residual_report() {
    let output = Command::new(BIN)
        .args(["--json", "doctor", "--quick", "--offline"])
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn doctor");
    assert!(
        output.status.success(),
        "doctor failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("residual") || stdout.contains("cli_marker_dirs"),
        "doctor JSON must include residual fields: {stdout}"
    );
    assert!(
        stdout.contains("residual_disk") || stdout.contains("chromium_tmp_singleton"),
        "doctor checks must include residual_disk: {stdout}"
    );
}

fn count_chromium_singleton_dirs() -> usize {
    let tmp = std::env::temp_dir();
    let Ok(entries) = fs::read_dir(tmp) else {
        return 0;
    };
    let mut n = 0usize;
    for ent in entries.flatten() {
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("org.chromium.Chromium.")
            || name.starts_with(".org.chromium.Chromium.")
        {
            n += 1;
        }
    }
    n
}
