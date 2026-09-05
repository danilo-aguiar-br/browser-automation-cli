// SPDX-License-Identifier: MIT OR Apache-2.0
//! Test-only helpers (not part of the public agent API).

use std::sync::{Mutex, MutexGuard};

/// Announce that a unit test declined to run, and fail when the suite is strict.
///
/// # Why this exists
///
/// Several unit tests need a host tool (`/bin/sh`, Xvfb, a mock script) and used
/// to `eprintln!("skip: …")` and then return. libtest counts that as PASSED, so
/// a gate that never executed was indistinguishable from a gate that executed
/// and held. The integration suite closed the same hole in
/// `tests/common::enforce_strict`; this is its in-crate twin.
///
/// The switch is the `strict-gates` cargo feature, never an environment
/// variable, because this product bans product environment variables.
///
/// # Panics
///
/// Whenever the `strict-gates` feature is enabled.
/// `print_stderr` is denied package-wide so that agent-consumable output stays
/// in `src/output.rs`. This is the one announcement that must NOT go there: a
/// skip is a message to the human running the suite, and routing it through the
/// agent envelope would make a gate that declined look like a gate that ran.
#[allow(clippy::print_stderr)]
pub fn skip_unit_test(gate: &str, reason: &str) {
    eprintln!("skip {gate}: {reason} This is NOT a pass.");
    if cfg!(feature = "strict-gates") {
        panic!("{gate} declined to run under --features strict-gates: {reason}");
    }
}

/// Global mutex shared across all test modules to prevent parallel tests from
/// interfering with each other when mutating environment variables.
///
/// # Concurrency
///
/// Direct `Mutex::new(())` (const constructor, MSRV ≥ 1.63). Tests that touch
/// process env must hold this lock for the full fixture lifetime.
/// Poison is recovered so one panicked test cannot cascade to the suite.
pub static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// RAII guard that locks [`ENV_MUTEX`] and restores environment variables on drop.
///
/// # This guard is UNSOUND, and is legacy — do not extend it
///
/// It calls `std::env::set_var`, which the std documentation says is unsound in
/// a multi-threaded program on every platform except Windows. The requirement
/// is not merely that no other thread WRITES the environment: no other thread
/// may even READ it concurrently, and libc reads it without announcing —
/// address resolution through `ToSocketAddrs` is the example the std docs
/// themselves give.
///
/// [`ENV_MUTEX`] does not fix that. A lock binds only the threads that choose
/// to take it, and libc never will. libtest runs the tests inside one binary on
/// many threads, so a fixture mutating its OWN process environment is undefined
/// behaviour by construction, mutex or no mutex.
///
/// `set_var` and `remove_var` also become `unsafe` in edition 2024, so this
/// compiles today only because the crate is on 2021.
///
/// # What to do instead
///
/// Parameterize. A function that resolves policy from XDG should delegate to a
/// core that TAKES the policy, and the test should call the core — the shape
/// `net::assert_safe_http_url` / `assert_safe_http_url_mode` uses. When a child
/// process is involved, hand the variables to the CHILD via `Command::env`,
/// which is the documented alternative and mutates nothing in this process;
/// `tests/common/mod.rs` does exactly that.
///
/// This guard remains only for fixtures not yet migrated. Every new test must
/// use one of the two approaches above.
///
/// Lifetime is elided at the impl boundary (`clippy::elidable_lifetime_names`);
/// the struct still names `'a` because it holds a `MutexGuard<'a, ()>`.
pub struct EnvGuard<'a> {
    _lock: MutexGuard<'a, ()>,
    vars: Vec<(String, Option<String>)>,
}

impl EnvGuard<'_> {
    /// Take the global env lock and snapshot `var_names` for restore on drop.
    ///
    /// Register EVERY variable the fixture touches, including `HOME`:
    /// `directories::ProjectDirs` falls back to it when the XDG vars are unset,
    /// so redirecting only `XDG_*` leaves the real home reachable.
    ///
    /// Bind the guard to a named local (`let guard = ...`). `let _ = ...` drops
    /// it immediately and silently reintroduces the race it exists to prevent.
    pub fn new(var_names: &[&str]) -> Self {
        let lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| {
            // Test isolation must not sticky-fail if a prior test panicked.
            poisoned.into_inner()
        });
        let vars = var_names
            .iter()
            .map(|&name| (name.to_string(), std::env::var(name).ok()))
            .collect();
        Self { _lock: lock, vars }
    }

    /// Take the global env lock without changing anything.
    ///
    /// # Why a reader needs the lock too
    ///
    /// [`EnvGuard::new`] protects tests that WRITE the environment from each
    /// other. It does nothing for a test that only READS it, and process env is
    /// shared by every test in the binary — so a reader running beside a writer
    /// observes the fixture of an unrelated test.
    ///
    /// Measured on 2026-08-10: `seed_cache::tests` resolved an XDG path twice
    /// and compared the two. A concurrent `state::tests` fixture redirected
    /// `XDG_STATE_HOME` between the two calls, so the assertion compared the
    /// real state directory against a temporary one. The suite failed roughly
    /// one run in three, and passed every time under `--test-threads=1`, which
    /// is how `ci-check` runs it — so the race stayed invisible to the gate.
    pub fn for_reading() -> Self {
        Self::new(&[])
    }

    /// Set a registered variable for the lifetime of this guard.
    ///
    /// # Panics
    ///
    /// Debug builds assert the variable was registered in [`EnvGuard::new`];
    /// an unregistered write would never be restored.
    /// `clippy::disallowed_methods` bans `set_var` package-wide (see
    /// `clippy.toml`). This guard is the legacy holder of that debt, documented
    /// as unsound on the type itself; the exemption is scoped to it so a new
    /// call site elsewhere still fails the gate.
    #[allow(clippy::disallowed_methods)]
    pub fn set(&self, name: &str, value: &str) {
        debug_assert!(
            self.vars.iter().any(|(n, _)| n == name),
            "EnvGuard::set called with unregistered var: {name}"
        );
        std::env::set_var(name, value);
    }

    /// Unset a registered variable for the lifetime of this guard.
    ///
    /// # Panics
    ///
    /// Debug builds assert the variable was registered in [`EnvGuard::new`].
    /// See [`EnvGuard::set`] for why this call is exempt from
    /// `clippy::disallowed_methods`.
    #[allow(clippy::disallowed_methods)]
    pub fn remove(&self, name: &str) {
        debug_assert!(
            self.vars.iter().any(|(n, _)| n == name),
            "EnvGuard::remove called with unregistered var: {name}"
        );
        std::env::remove_var(name);
    }
}

impl Drop for EnvGuard<'_> {
    /// See [`EnvGuard::set`] for why these calls are exempt from
    /// `clippy::disallowed_methods`.
    #[allow(clippy::disallowed_methods)]
    fn drop(&mut self) {
        for (name, value) in &self.vars {
            match value {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }
}
