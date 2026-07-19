// SPDX-License-Identifier: MIT OR Apache-2.0
//! Test-only helpers (not part of the public agent API).

use std::sync::{Mutex, MutexGuard};

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
/// Lifetime is elided at the impl boundary (`clippy::elidable_lifetime_names`);
/// the struct still names `'a` because it holds a `MutexGuard<'a, ()>`.
pub struct EnvGuard<'a> {
    _lock: MutexGuard<'a, ()>,
    vars: Vec<(String, Option<String>)>,
}

impl EnvGuard<'_> {
    pub fn new(var_names: &[&str]) -> Self {
        let lock = ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let vars = var_names
            .iter()
            .map(|&name| (name.to_string(), std::env::var(name).ok()))
            .collect();
        Self { _lock: lock, vars }
    }

    pub fn set(&self, name: &str, value: &str) {
        debug_assert!(
            self.vars.iter().any(|(n, _)| n == name),
            "EnvGuard::set called with unregistered var: {name}"
        );
        std::env::set_var(name, value);
    }

    pub fn remove(&self, name: &str) {
        debug_assert!(
            self.vars.iter().any(|(n, _)| n == name),
            "EnvGuard::remove called with unregistered var: {name}"
        );
        std::env::remove_var(name);
    }
}

impl Drop for EnvGuard<'_> {
    fn drop(&mut self) {
        for (name, value) in &self.vars {
            match value {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }
}
