//! RES-03: residual zero for CLI marker profiles + Chromium side-channels
//! after one-shot DIE (PRD §5N).
//!
//! This file covers disk residue only. It says nothing about help text, about
//! `run` dispatch coverage, or about console/network capture surviving
//! shutdown — do not read a green run here as evidence for any of those.
//!
//! # Why every residue assertion runs inside a sandbox
//!
//! These tests need a **zero** age floor: they assert that a profile is
//! collectable the instant its owner dies, and the production floor of
//! [`STALE_MIN_AGE_SECS`] would make them sleep a minute to prove it.
//!
//! That floor is not decoration. It is what protects the window between
//! `create_dir_all` of a profile and Chrome appearing in the process table: in
//! that window the directory exists and **no process holds it yet**, so the
//! liveness guard cannot save it. Dropping the floor to zero against the real
//! roots therefore deletes the in-flight profile of any concurrent invocation —
//! and `cargo` runs test binaries concurrently, so the victim is usually a
//! sibling test, which then fails with `SingletonLock: No such file or directory`.
//!
//! So each test that lowers the floor also narrows the roots to a directory it
//! created, passed to the child through `TMPDIR` / `XDG_CACHE_HOME` and to the
//! library through the `_in_roots` entry points. Zero-age GC is only ever
//! pointed at paths this test owns.
//!
//! [`STALE_MIN_AGE_SECS`]: browser_automation_cli::residual::STALE_MIN_AGE_SECS

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

mod common;

/// A private pair of residual roots: an OS temp dir and an XDG cache dir.
///
/// Both are handed to the child process by environment, which is per-child and
/// therefore safe to use while other tests run in this binary — unlike mutating
/// this process's own environment.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!("bac-residual-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("tmp")).expect("mkdir sandbox tmp");
        fs::create_dir_all(root.join("cache")).expect("mkdir sandbox cache");
        Self { root }
    }

    fn tmp(&self) -> PathBuf {
        self.root.join("tmp")
    }

    /// Where the product puts ephemeral profiles under this sandbox's cache.
    ///
    /// Mirrors `xdg::chrome_profiles_dir()`: `$XDG_CACHE_HOME/<pkg>/chrome-profiles`.
    fn chrome_profiles(&self) -> PathBuf {
        self.root
            .join("cache")
            .join(env!("CARGO_PKG_NAME"))
            .join("chrome-profiles")
    }

    /// A path the child is allowed to read or write.
    ///
    /// The product derives its allowed roots from the process temp dir, which the
    /// sandbox has moved: a file next to the sandbox root but outside `tmp/` is
    /// refused with `capability-disabled`. Everything handed to a child goes here.
    fn child_file(&self, name: &str) -> PathBuf {
        self.tmp().join(name)
    }

    /// The roots to hand the `_in_roots` library entry points.
    fn roots(&self) -> Vec<PathBuf> {
        vec![self.tmp(), self.chrome_profiles()]
    }

    /// Point a child invocation at this sandbox.
    fn apply(&self, cmd: &mut Command) {
        cmd.env("TMPDIR", self.tmp())
            .env("XDG_CACHE_HOME", self.root.join("cache"))
            .env("NO_COLOR", "1");
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn goto_leaves_zero_cli_chrome_marker_dirs() {
    let sandbox = Sandbox::new("goto");
    let mut cmd = common::cmd();
    cmd.args(["--json", "goto", "about:blank"]);
    sandbox.apply(&mut cmd);
    let output = cmd.output().expect("spawn goto");
    assert!(
        output.status.success(),
        "goto failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let leaked =
        browser_automation_cli::residual::list_cli_chrome_marker_dirs_in_roots(&sandbox.roots());
    assert!(
        leaked.is_empty(),
        "leaked CLI chrome marker dirs after one-shot: {leaked:?}"
    );
}

#[test]
fn goto_does_not_leave_new_chromium_singleton_orphans() {
    let sandbox = Sandbox::new("singleton");
    let pdf = sandbox.child_file("residual.pdf");
    let mut cmd = common::cmd();
    cmd.args(["--json", "print-pdf", "--url", "about:blank", "--path"])
        .arg(&pdf);
    sandbox.apply(&mut cmd);
    let output = cmd.output().expect("spawn print-pdf");
    assert!(
        output.status.success(),
        "print-pdf failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Product law: whatever this one-shot creates, our own GC must be able to
    // reclaim it. The sandbox is what makes the zero floor honest here — nothing
    // inside it was created by anyone but this test's child, so a dir that
    // survives the scavenge is a genuine leak rather than a neighbour's profile.
    let _ = browser_automation_cli::residual::scavenge_stale_singleton_orphans_in_roots(
        &sandbox.roots(),
        Duration::ZERO,
    );
    let leaked = chromium_singleton_dirs(&sandbox.tmp());
    assert!(
        leaked.is_empty(),
        "one-shot left chromium singleton dirs that GC could not reclaim: {leaked:?}"
    );
}

#[test]
fn born_gc_wipes_stale_singleton_fixture() {
    let sandbox = Sandbox::new("fixture");
    let dir = sandbox.tmp().join("org.chromium.Chromium.rstest");
    fs::create_dir_all(&dir).expect("mkdir fixture");
    let _ = fs::write(dir.join("SingletonSocket"), b"");
    #[cfg(unix)]
    let _ = std::os::unix::fs::symlink("fixture", dir.join("SingletonCookie"));
    #[cfg(not(unix))]
    let _ = fs::write(dir.join("SingletonCookie"), b"fixture");

    // Force age floor zero via library API (unit path). Integration BORN uses 60s;
    // here we prove the wipe predicate for Singleton-only owned dirs.
    let wiped = browser_automation_cli::residual::scavenge_stale_singleton_orphans_in_roots(
        &sandbox.roots(),
        Duration::ZERO,
    );
    assert!(
        wiped.iter().any(|p| p == &dir) || !dir.exists(),
        "fixture must be wiped by stale GC: wiped={wiped:?}"
    );
}

#[test]
fn doctor_json_includes_residual_report() {
    let output = common::cmd()
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

/// GAP-045/GAP-052: SIGTERM mid-`run` leaves a profile whose owner pid is dead;
/// the next BORN must collect it.
#[cfg(unix)]
#[test]
fn sigterm_during_run_leaves_profile_collected_by_next_born() {
    use std::io::Write;

    let sandbox = Sandbox::new("sigterm");
    let script = sandbox.child_file("steps.jsonl");
    {
        let mut f = fs::File::create(&script).expect("write script");
        // Long enough that SIGTERM lands while Chrome owns the profile.
        writeln!(f, r#"{{"cmd":"goto","url":"about:blank"}}"#).unwrap();
        for _ in 0..40 {
            writeln!(
                f,
                r#"{{"cmd":"wait","selector":".bac-never-matches","wait_timeout_ms":2000}}"#
            )
            .unwrap();
        }
    }

    let mut cmd = common::cmd();
    cmd.args(["--json", "--timeout", "120", "run", "--script"])
        .arg(&script);
    sandbox.apply(&mut cmd);
    let mut child = cmd.spawn().expect("spawn run");

    // Wait for the CONDITION the signal needs, not for a number.
    //
    // This was `sleep(4s)`. A fixed sleep is wrong in both directions: too
    // short on a loaded machine and SIGTERM lands during BORN, where no profile
    // exists yet and the test proves nothing while still reporting PASS; too
    // long and every run pays the worst case. The observable the assertion
    // actually depends on is a marker profile inside this sandbox's roots, so
    // that is what we wait for.
    //
    // The deadline is a HANG guard, not a budget: reaching it means no profile
    // ever appeared, and the test then signals anyway and asserts on the empty
    // set, which is the same honest no-op it performed before this change.
    let roots = sandbox.roots();
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while browser_automation_cli::residual::list_cli_chrome_marker_dirs_in_roots(&roots).is_empty()
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(50));
    }

    // SIGTERM first — never a bare SIGKILL as the normal cancel path.
    let pid = child.id() as i32;
    // SAFETY:
    // - Contract: deliver SIGTERM to a child this test spawned and still owns.
    // - Invariant: `kill` has no preconditions beyond a valid pid; the child has
    //   not been reaped yet, so the pid cannot have been recycled.
    // - See: `man 2 kill`.
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
    let _ = child.wait();

    // Any profile left behind now has a dead owner pid. Give the age floor a
    // zero override through the library API, which is the same predicate BORN uses.
    let before = browser_automation_cli::residual::list_cli_chrome_marker_dirs_in_roots(&roots);
    let wiped = browser_automation_cli::residual::scavenge_stale_singleton_orphans_in_roots(
        &roots,
        Duration::ZERO,
    );
    let after = browser_automation_cli::residual::list_cli_chrome_marker_dirs_in_roots(&roots);
    assert!(
        after.len() <= before.len(),
        "next BORN must not grow residue: before={before:?} after={after:?} wiped={wiped:?}"
    );
    for dir in &before {
        // A profile whose owner pid is dead must be gone.
        let owner_alive = browser_automation_cli::residual::read_owner_pid(dir)
            .and_then(|pid| {
                browser_automation_cli::residual::index_live_processes()
                    .map(|idx| idx.contains_pid(pid))
            })
            .unwrap_or(true);
        if !owner_alive {
            assert!(
                !dir.exists(),
                "dead-owner profile survived next BORN: {}",
                dir.display()
            );
        }
    }
}

/// GAP-002/GAP-006: concurrent invocations must not make `doctor` fail.
#[test]
fn concurrent_invocation_keeps_doctor_exit_zero() {
    let sandbox = Sandbox::new("concurrent");
    let mut cmd = common::cmd();
    cmd.args(["--json", "--timeout", "60", "goto", "about:blank"]);
    sandbox.apply(&mut cmd);
    let sibling = cmd.spawn().expect("spawn sibling");

    let doctor = common::cmd()
        .args(["--json", "doctor", "--quick", "--offline"])
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn doctor");

    let mut sibling = sibling;
    reap_or_kill(&mut sibling, Duration::from_secs(90));

    assert!(
        doctor.status.success(),
        "doctor must stay green next to a live sibling: code={:?} stdout={} stderr={}",
        doctor.status.code(),
        String::from_utf8_lossy(&doctor.stdout),
        String::from_utf8_lossy(&doctor.stderr)
    );
    let stdout = String::from_utf8_lossy(&doctor.stdout);
    assert!(
        stdout.contains("sibling_live_processes"),
        "doctor must report sibling_live_processes: {stdout}"
    );
}

/// Reap a child within `budget`, killing it if it overstays.
///
/// A bare `child.wait()` blocks FOREVER. The sibling here is a real browser
/// invocation, and a browser that wedges — waiting on a socket, on a profile
/// lock, on a display that never arrives — takes the whole suite down with it,
/// with no output and no name to blame, because libtest cannot interrupt a
/// blocked thread.
///
/// The budget is generous on purpose. It is a HANG guard, never a performance
/// assertion: a slow machine must not turn this into a red test, and the child
/// already carries its own `--timeout`.
fn reap_or_kill(child: &mut std::process::Child, budget: Duration) {
    let deadline = std::time::Instant::now() + budget;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(_) => return,
        }
    }
}

/// Chromium side-channel directories present under `root`.
fn chromium_singleton_dirs(root: &std::path::Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for ent in entries.flatten() {
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("org.chromium.Chromium.") || name.starts_with(".org.chromium.Chromium.")
        {
            out.push(ent.path());
        }
    }
    out
}
