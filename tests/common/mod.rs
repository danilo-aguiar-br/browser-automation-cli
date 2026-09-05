// SPDX-License-Identifier: MIT OR Apache-2.0
//! Helpers shared by the integration test suite.
//!
//! This file is `tests/common/mod.rs` and NEVER `tests/common.rs`, because cargo
//! treats every `.rs` at the root of `tests/` as its own test binary. Under a
//! subdirectory the module is merely included by whoever writes `mod common;`.
//!
//! Each test binary includes the whole module and uses only a subset of it, so
//! `dead_code` would fire in every other binary. The allow below is the correct
//! suppression for that shape, not a way to hide an unused helper.
//!
//! # This suite REQUIRES `--test-threads=1`
//!
//! Run it as `cargo test --tests --all-features -- --test-threads=1`.
//!
//! libtest runs the tests inside one binary on parallel threads, and roughly
//! ten gates here each launch Chrome. Concurrent launches produced three
//! distinct measured failures — `SingletonLock: No such file or directory`,
//! `No chromiumoxide Page for session_id`, and `Page.navigate: Request timed
//! out` — all of them absent when serialized.
//!
//! Serial is also FASTER: 101s serial against 148s parallel. Thirty-two
//! contending Chromes cost more than they save, so serialization buys
//! reliability at negative time cost.
//!
//! This requirement is enforced NOWHERE in the code: there is no `serial_test`
//! dependency, no `#[serial]` attribute, and no mutex gating the launches. It
//! lives in `scripts/ci-check.sh` only, which means a bare `cargo test` typed
//! by hand, an IDE runner, or `cargo nextest` all reproduce the three failures
//! above. That is why it is written here too — the person who hits it is
//! reading this file, not the CI script.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

/// Crate root. Every fixture and binary path resolves from here, because cargo
/// runs the test binary with a working directory that is not guaranteed.
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the binary cargo just built for this test.
///
/// `CARGO_BIN_EXE_*` ALWAYS resolves, so a caller of `bin()` has no build gate:
/// the binary exists by construction. Use this form when the test must not skip
/// for a missing artifact.
pub fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_browser-automation-cli"))
}

/// Debug binary looked up explicitly under `target/debug/`.
///
/// Differs from [`bin`] on purpose: it returns `None` when the artifact is not
/// there, and the gates that depend on it SKIP rather than fail. Do not unify
/// the two — that difference is exactly the build gate of 24 files, measured on
/// 2026-08-25.
///
/// The cost of that property is that this form pins NOTHING about the build:
/// it hands back whatever binary last landed at that shared path, under
/// whatever feature set produced it. So a caller of `binary()` must not assert
/// on feature-gated behaviour, and `tests/binary_resolver_boundary_gate.rs`
/// asserts that none does.
/// Why the artifact at that path cannot be used, when it cannot.
///
/// Split out from [`binary`] so the skip can name WHICH of the two problems it
/// hit. A single "absent" message for both is how a stale artifact gets read as
/// a missing one and answered with a `cargo build` that changes nothing.
pub enum BinaryState {
    /// Present and newer than every file under `src/`.
    Ready(PathBuf),
    /// Nothing at `target/debug/browser-automation-cli`.
    Absent,
    /// Present, but older than a source file — so it predates the code.
    Stale,
}

/// Resolve the debug binary and say whether it can be trusted.
///
/// # Why existence was not enough
///
/// This check was `p.exists()` and nothing more, so ANY artifact left at that
/// shared path satisfied it, from any build, of any age. Measured 2026-08-30:
/// an integration run reported `INT=0` with 1428 tests green while the five
/// `allowed_roots_gate` assertions were measured against a debug binary from an
/// earlier session, because nothing compared it to the sources it claims to
/// exercise.
///
/// That is the same defect `scripts/doc-coverage-check.sh` was hardened against
/// days earlier, one layer down: a derived artifact compared to something that
/// may predate the code it derives from. A gate that cannot tell a current
/// binary from an old one reports on a product that no longer exists.
///
/// The comparison is mtime against the newest file under `src/`, which is what
/// `cargo` itself uses to decide a rebuild. It is computed once per process.
pub fn binary_state() -> BinaryState {
    let p = root().join("target/debug/browser-automation-cli");
    let Ok(built) = std::fs::metadata(&p).and_then(|m| m.modified()) else {
        return BinaryState::Absent;
    };
    static NEWEST_SOURCE: std::sync::OnceLock<Option<std::time::SystemTime>> =
        std::sync::OnceLock::new();
    let newest = *NEWEST_SOURCE.get_or_init(|| newest_source_mtime(&root().join("src")));
    match newest {
        Some(src) if src > built => BinaryState::Stale,
        _ => BinaryState::Ready(p),
    }
}

/// Newest modification time under `dir`, or `None` when it cannot be read.
///
/// Returning `None` keeps the check FAIL-OPEN on purpose: a tree the test
/// process cannot walk is not evidence that the binary is stale, and refusing
/// every gate over an unreadable directory would trade a sharp check for an
/// unusable suite.
fn newest_source_mtime(dir: &Path) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(t) = entry.metadata().and_then(|m| m.modified()) {
                if newest.is_none_or(|n| t > n) {
                    newest = Some(t);
                }
            }
        }
    }
    newest
}

/// Debug binary looked up explicitly under `target/debug/`.
pub fn binary() -> Option<PathBuf> {
    match binary_state() {
        BinaryState::Ready(p) => Some(p),
        BinaryState::Absent | BinaryState::Stale => None,
    }
}

/// The reason and remedy a skip should print for the current binary state.
///
/// Returns `None` when the binary is usable.
fn binary_problem() -> Option<(&'static str, &'static str)> {
    match binary_state() {
        BinaryState::Ready(_) => None,
        BinaryState::Absent => Some((
            "target/debug/browser-automation-cli absent.",
            "run `cargo build` first.",
        )),
        BinaryState::Stale => Some((
            "target/debug/browser-automation-cli is OLDER than a file under src/, so it predates the code this gate claims to exercise.",
            "run `cargo build` again; note that building with --target-dir elsewhere leaves this path stale.",
        )),
    }
}

/// Per-process sandbox root that every spawned product process points at.
///
/// # Why `Command::env` and NEVER `std::env::set_var`
///
/// `set_var` is unsound in a multi-threaded program. The std docs require that
/// no other thread even READS the environment concurrently, and libc reads it
/// without announcing — DNS resolution through `ToSocketAddrs` is the example
/// the docs themselves give. libtest runs tests on many threads, so a suite that
/// mutates its OWN environment is undefined behaviour by construction, mutex or
/// no mutex: a lock only binds the threads that opt into it. The documented
/// alternative is to hand the variables to the CHILD process, which is exactly
/// what [`isolate_env`] does. `set_var` also becomes `unsafe` in edition 2024.
///
/// Lives under `CARGO_TARGET_TMPDIR` so `cargo clean` collects it and nothing
/// escapes into the user's `/tmp`.
pub fn sandbox_root() -> &'static Path {
    static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("xdg-sandbox-{}", std::process::id()));
        for sub in ["config", "cache", "data", "state"] {
            let _ = std::fs::create_dir_all(root.join(sub));
        }
        root
    })
}

/// Point every XDG surface AND `HOME` at the per-process sandbox.
///
/// # Why all five and not just `XDG_CONFIG_HOME`
///
/// The product resolves four independent surfaces through `ProjectDirs`:
/// config, cache, data and state. Redirecting only the config left the other
/// three on the real home, so `src/cache/sqlite.rs` wrote to the operator's real
/// HTTP cache and `src/failure_dump.rs` to the real state directory. `HOME` is
/// required on top because `ProjectDirs` derives from it whenever an XDG
/// variable is unset — an override that omits `HOME` is inert on a host that
/// does not export the XDG vars, which is the common case.
pub fn isolate_env(cmd: &mut Command) -> &mut Command {
    for (k, v) in sandbox_env() {
        cmd.env(k, v);
    }
    cmd
}

/// The five (variable, value) pairs that define the sandbox.
///
/// Kept as ONE list because there are two spawn flavours in this suite —
/// `std::process::Command` and `assert_cmd::Command` — and they are different
/// types, so each needs its own call site. Writing the list twice is how a
/// suite ends up isolating four surfaces on one path and five on the other,
/// which is the exact defect this module exists to prevent.
fn sandbox_env() -> [(&'static str, PathBuf); 5] {
    let root = sandbox_root();
    [
        ("HOME", root.to_path_buf()),
        ("XDG_CONFIG_HOME", root.join("config")),
        ("XDG_CACHE_HOME", root.join("cache")),
        ("XDG_DATA_HOME", root.join("data")),
        ("XDG_STATE_HOME", root.join("state")),
    ]
}

/// [`assert_cmd::Command`] for the product binary, already isolated.
///
/// NOT interchangeable with [`cmd`]: `assert_cmd`'s `ok()` treats a non-zero
/// exit as `Err`, while `Command::output()` returns `Ok` and would let the JSON
/// of a FAILED run be read as though it had succeeded. Tests that depend on
/// that distinction need this flavour — and they need the isolation just as
/// much, which is why the two arrive together instead of the macro being called
/// inline. `chrome_ready_via_doctor_checks` is the case that proves it: an
/// unisolated probe consults the operator's real config to decide whether other
/// gates run at all.
pub fn assert_bin() -> assert_cmd::Command {
    let mut c = assert_cmd::cargo::cargo_bin_cmd!("browser-automation-cli");
    for (k, v) in sandbox_env() {
        c.env(k, v);
    }
    c
}

/// [`Command`] for the product binary, already isolated from the real home.
///
/// Prefer this over `common::cmd()` in every new test: a bare `Command`
/// inherits the operator's `config.toml`, which makes the result depend on the
/// machine that ran it.
pub fn cmd() -> Command {
    isolated_cmd(&bin())
}

/// [`Command`] for an arbitrary product binary path, already isolated.
///
/// The Chrome probes take the binary as a parameter rather than resolving it,
/// so they need this form. They matter as much as the test itself: a probe that
/// consults the real config decides whether the gate runs, so leaving it
/// unisolated defeated the isolation of every test that depended on it.
pub fn isolated_cmd(program: &Path) -> Command {
    let mut c = Command::new(program);
    isolate_env(&mut c);
    c
}

/// Print the real reason for a skip on stderr, in the format the suite uses.
///
/// A silent skip is indistinguishable from a PASS in cargo's output. Every
/// early return MUST pass through here so the operator can see why.
///
/// # Panics
///
/// Under `--features strict-gates`, which `scripts/ci-check.sh` turns on through
/// `--all-features`. See [`enforce_strict`] for why the announcement alone was
/// not enough.
#[allow(clippy::print_stderr)] // A skip announcement addresses the human running the suite.
pub fn skip_with_reason(gate: &str, reason: &str) {
    eprintln!("SKIP {gate}: {reason} This is NOT a pass.");
    enforce_strict(gate, reason);
}

/// Variant of [`skip_with_reason`] that appends the concrete remedy.
///
/// # Panics
///
/// Same condition as [`skip_with_reason`].
#[allow(clippy::print_stderr)] // Same reason as `skip_with_reason`.
pub fn skip_with_remedy(gate: &str, reason: &str, remedy: &str) {
    eprintln!("SKIP {gate}: {reason} This is NOT a pass; {remedy}");
    enforce_strict(gate, &format!("{reason} {remedy}"));
}

/// Turn an announced skip into a failure when the suite is asked to be strict.
///
/// # Why an announcement was not enough
///
/// Every early return in this suite already printed "This is NOT a pass" — and
/// then returned normally, so libtest counted the test as PASSED and
/// `scripts/ci-check.sh` counted the binary as PASS. The line went to stderr,
/// which nothing parses. `ci-check.sh` was hardened to treat a verifier SCRIPT
/// that declines to run (exit 3) as a failure, but that hardening stopped at the
/// script boundary: a TEST that declined to run still looked identical to a test
/// that ran and passed.
///
/// This closes the same hole one layer down. Measured 2026-08-18: zero skips on
/// a host with Chrome and a built binary, so the strict build stays green here
/// and starts failing exactly where coverage was silently absent.
///
/// The switch is a cargo FEATURE and never an environment variable: this product
/// bans product environment variables, and a test harness that contradicted that
/// rule would teach the wrong shape.
///
/// # Panics
///
/// Whenever the `strict-gates` feature is enabled.
pub fn enforce_strict(gate: &str, reason: &str) {
    if cfg!(feature = "strict-gates") {
        panic!("{gate} declined to run under --features strict-gates: {reason}");
    }
}

/// Refuse outright when the binary predates the code under test.
///
/// # Why `Stale` must NOT be a skip, while `Absent` may be
///
/// The two states look alike and mean opposite things. `Absent` is an
/// environment that never built the product: skipping is honest, because the
/// gate has nothing to exercise and the operator gets a remedy.
///
/// `Stale` happens to the person who just edited `src/`. That is precisely the
/// moment a gate is worth the most, and precisely the moment a skip is worth
/// the least: `libtest` prints `ok`, the "This is NOT a pass" line goes to
/// stderr where nothing looks without `--nocapture`, and the developer reads a
/// green suite as evidence about code the binary does not contain.
///
/// Measured 2026-09-04 by an agent chasing an unrelated defect: a green e2e
/// gate immediately after editing `src/` was not proof, and nothing on screen
/// said so. `enforce_strict` already converts every skip into a failure under
/// `--features strict-gates`, which `scripts/ci-check.sh` turns on — so CI was
/// covered and the daily `cargo test` was not. This closes the daily path for
/// the one state where a skip actively misleads.
///
/// # Panics
///
/// Whenever the binary is older than the newest file under `src/`.
/// Whether `state` must refuse outright instead of announcing a skip.
///
/// Separated from [`refuse_stale`] so the POLICY can be asserted without
/// touching the clock or the filesystem: the decision is the thing that must
/// not silently change, and it is untestable while it lives inside a function
/// that reads mtimes. `tests/harness_contract_gate.rs` pins all three arms.
pub fn must_refuse(state: &BinaryState) -> bool {
    match state {
        // The one state where a skip actively misleads: the operator just
        // edited `src/` and would read `ok` as evidence about that edit.
        BinaryState::Stale => true,
        // An environment that never built the product has nothing to exercise,
        // and a remedy the operator can act on.
        BinaryState::Absent | BinaryState::Ready(_) => false,
    }
}

fn refuse_stale(gate: &str) {
    if must_refuse(&binary_state()) {
        let (reason, remedy) = binary_problem().unwrap_or(("stale binary.", "run `cargo build`."));
        panic!("{gate} cannot report on this tree: {reason} {remedy}");
    }
}

/// `true` when `target/debug/browser-automation-cli` is absent, having already
/// reported the canonical reason with its remedy.
///
/// # Panics
///
/// Via [`refuse_stale`] when the binary is older than `src/`.
pub fn missing_binary(gate: &str) -> bool {
    refuse_stale(gate);
    match binary_problem() {
        Some((reason, remedy)) => {
            skip_with_remedy(gate, reason, remedy);
            true
        }
        None => false,
    }
}

/// Resolve the debug binary, or report the skip and return `None`.
///
/// # Panics
///
/// Via [`refuse_stale`] when the binary is older than `src/`.
pub fn binary_or_skip(gate: &str) -> Option<PathBuf> {
    refuse_stale(gate);
    match binary_state() {
        BinaryState::Ready(p) => Some(p),
        _ => {
            if let Some((reason, remedy)) = binary_problem() {
                skip_with_remedy(gate, reason, remedy);
            }
            None
        }
    }
}

/// Report a missing fixture and return `true` so the caller can skip.
pub fn missing_fixture(gate: &str, description: &str) -> bool {
    skip_with_reason(gate, description);
    true
}

/// `true` when the offline `doctor` says the host is not ready for Chrome.
///
/// Reads the envelope's top-level `ok`. Any spawn, parse or field failure counts
/// as NOT ready, because a Chrome gate that fails open turns into a false PASS.
pub fn chrome_not_ready(gate: &str, bin: &Path) -> bool {
    let chrome_ok = isolated_cmd(bin)
        .args(["-q", "--json", "doctor", "--offline", "--quick"])
        .output()
        .ok()
        .and_then(|o| serde_json::from_slice::<Value>(&o.stdout).ok())
        .and_then(|v| v.get("ok").and_then(|b| b.as_bool()))
        .unwrap_or(false);
    if !chrome_ok {
        skip_with_reason(gate, "doctor reports the host is not ready for Chrome.");
        return true;
    }
    false
}

/// `true` when a host on the public internet cannot be reached, having skipped.
///
/// # Why a Chrome gate is not enough
///
/// A test that navigates to a public URL depends on TWO facts about the host:
/// that Chrome runs, and that the network answers. The suite only ever guarded
/// the first, so on a machine with Chrome and no network the test did not skip
/// — it ran, navigated to nothing, captured nothing, and any assertion weaker
/// than "there was traffic" still passed. A guard that never fires is
/// indistinguishable from a guard that was not needed, which is why this one is
/// named after what it actually measures.
///
/// Fails CLOSED: any resolution, connect or timeout failure counts as offline,
/// because an environment probe that fails open is how a false PASS is born.
/// `TcpStream` is used instead of an HTTP client because reachability is the
/// question and a TCP handshake answers it without a request body or TLS.
pub fn public_network_unreachable(gate: &str) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    let reachable = "example.com:443"
        .to_socket_addrs()
        .ok()
        .into_iter()
        .flatten()
        .any(|addr| TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(5)).is_ok());
    if !reachable {
        skip_with_remedy(
            gate,
            "no route to a host on the public internet.",
            "connect this machine to the network, or run the gate where example.com:443 answers.",
        );
        return true;
    }
    false
}

// --------------------------------------------------------------------------
// The five Chrome probes the suite inherited.
//
// They arrived here under two names only — `chrome_available` and
// `chrome_ready` — spread across five files, which led any reader to assume
// they were the same check. They are NOT: they differ in what they consult and,
// above all, in how they fail. Unifying them would change the skip behaviour of
// every test that uses them, so they sit side by side under distinct names and
// the difference is explicit here instead of hidden across five files.
// --------------------------------------------------------------------------

/// Reads `/data/chrome_found` from `doctor` and FAILS OPEN.
///
/// The `unwrap_or(true)` is deliberate: when doctor does not answer or omits the
/// field, the test PROCEEDS rather than skipping. It is the only one of the five
/// probes that assumes presence when in doubt.
pub fn chrome_found_or_assume_present(bin: &Path) -> bool {
    isolated_cmd(bin)
        .args(["--json", "doctor", "--offline", "--quick"])
        .output()
        .ok()
        .and_then(|o| serde_json::from_slice::<Value>(&o.stdout).ok())
        .and_then(|v| v.pointer("/data/chrome_found").and_then(|c| c.as_bool()))
        .unwrap_or(true)
}

/// Asks `--version` of every known browser name on `PATH`.
///
/// Does not consult the product: it measures the host directly. Fails closed.
pub fn chrome_responds_to_version() -> bool {
    ["google-chrome", "chromium", "chromium-browser"]
        .iter()
        .any(|b| {
            isolated_cmd(Path::new(b))
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
}

/// Substring heuristic over the `doctor` stdout.
///
/// Accepts process success OR a mention of "chrome" OR of "ok". It is the
/// loosest of the five, and it exists because the test using it only has to
/// choose between a long path and a short one — never between pass and skip.
pub fn chrome_hinted_by_doctor_text(bin: &Path) -> bool {
    isolated_cmd(bin)
        .args(["doctor", "--offline", "--quick", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            o.status.success() || s.contains("chrome") || s.contains("ok")
        })
        .unwrap_or(false)
}

/// Looks for "chrome" or "chromium" in the WHOLE serialized `doctor` envelope.
///
/// Inspects no field in particular: doctor publishes the resolved browser path
/// somewhere in the envelope, and this probe accepts any of them. Fails closed
/// on a missing binary, a failed spawn, or unparseable JSON.
pub fn chrome_mentioned_in_doctor_json() -> bool {
    let Some(bin) = binary() else {
        return false;
    };
    let out = isolated_cmd(&bin)
        .args(["-q", "--json", "doctor", "--offline", "--quick"])
        .output();
    let Ok(out) = out else {
        return false;
    };
    let Ok(v) = serde_json::from_slice::<Value>(&out.stdout) else {
        return false;
    };
    let text = v.to_string();
    text.contains("chrome") || text.contains("chromium")
}

/// Accepts top-level `ok`, `data.ok`, or a `chrome` check with status `pass`.
///
/// Uses `assert_cmd`, whose `ok()` treats a non-zero exit as `Err` — unlike
/// `Command::output()`, which returns `Ok` and would let the JSON be read even
/// from a doctor that failed. Swapping in `std::process::Command` would change
/// the result.
pub fn chrome_ready_via_doctor_checks() -> bool {
    assert_bin()
        .args(["doctor", "--quick", "--json"])
        .ok()
        .map(|out| {
            let v: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
            v["ok"] == true
                || v["data"]["ok"] == true
                || v.pointer("/data/checks")
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .any(|x| x["id"] == "chrome" && x["status"] == "pass")
                    })
                    .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Plain `PATH` sweep for a Chrome or Chromium.
///
/// NEVER shells out to `which`: the suite runs on hosts with no guaranteed
/// POSIX shell.
pub fn chrome_discoverable() -> bool {
    let paths = match std::env::var_os("PATH") {
        Some(p) => p,
        None => return false,
    };
    for name in [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
    ] {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return true;
            }
        }
    }
    false
}

/// Run the binary and require stdout to be a JSON envelope.
///
/// Returns `(exit_code, envelope, stderr)`. stderr comes back because the
/// product's diagnostic message lives there, and a test that reads only stdout
/// loses the cause.
pub fn run_json(args: &[&str]) -> (i32, Value, String) {
    let out = cmd()
        .args(args)
        .output()
        .expect("spawn browser-automation-cli");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let json = serde_json::from_str::<Value>(&stdout).unwrap_or_else(|e| {
        panic!("stdout was not JSON (exit {code}): {e}\nstdout={stdout}\nstderr={stderr}")
    });
    (code, json, stderr)
}

/// Variant of [`run_json`] that feeds the process stdin.
pub fn run_json_stdin(args: &[&str], stdin_body: Option<&str>) -> Value {
    use std::io::Write;

    let mut c = cmd();
    c.args(args).stdout(Stdio::piped()).stderr(Stdio::null());
    if stdin_body.is_some() {
        c.stdin(Stdio::piped());
    }
    let mut child = c.spawn().expect("spawn browser-automation-cli");
    if let Some(body) = stdin_body {
        child
            .stdin
            .as_mut()
            .expect("stdin pipe")
            .write_all(body.as_bytes())
            .expect("feed stdin");
    }
    let out = child.wait_with_output().expect("collect output");
    serde_json::from_slice(&out.stdout).expect("stdout must be a JSON envelope")
}
