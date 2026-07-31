// SPDX-License-Identifier: MIT OR Apache-2.0
//! Effective-value accessors over the one-shot policy snapshot.
//!
//! The XDG file is read at most once per process and cached, so hot loops
//! (event pump slices, poll intervals) never re-read disk.

use std::sync::OnceLock;

use super::knobs::{policy_default, policy_stored, PolicyConfig};

/// True when `key` names a promoted policy knob.
pub fn is_policy_key(key: &str) -> bool {
    policy_default(key).is_some()
}

/// Process-wide snapshot of the policy layer (one-shot: read disk at most once).
static SNAPSHOT: OnceLock<PolicyConfig> = OnceLock::new();

fn snapshot() -> &'static PolicyConfig {
    SNAPSHOT.get_or_init(|| {
        crate::xdg::load_config()
            .map(|c| c.policy)
            .unwrap_or_default()
    })
}

/// Effective `u64` for a policy key: XDG override when set, else the constant.
///
/// # Panics
///
/// Never. An unknown key is a programming error and yields `0`; call sites use
/// the generated [`super::key`] constants, so this is unreachable in practice.
pub fn policy_u64(name: &str) -> u64 {
    let stored = policy_stored(snapshot(), name).flatten().filter(|&n| n > 0);
    stored.or_else(|| policy_default(name)).unwrap_or_default()
}

/// Effective `usize` for a policy key (falls back when the value overflows).
pub fn policy_usize(name: &str) -> usize {
    usize::try_from(policy_u64(name))
        .ok()
        .or_else(|| policy_default(name).and_then(|d| usize::try_from(d).ok()))
        .unwrap_or_default()
}

/// Effective `u32` for a policy key (falls back when the value overflows).
pub fn policy_u32(name: &str) -> u32 {
    u32::try_from(policy_u64(name))
        .ok()
        .or_else(|| policy_default(name).and_then(|d| u32::try_from(d).ok()))
        .unwrap_or_default()
}

/// Effective `i32` for a policy key (falls back when the value overflows).
pub fn policy_i32(name: &str) -> i32 {
    i32::try_from(policy_u64(name))
        .ok()
        .or_else(|| policy_default(name).and_then(|d| i32::try_from(d).ok()))
        .unwrap_or_default()
}

/// Effective [`std::time::Duration`] from a seconds-valued policy key.
pub fn policy_secs(name: &str) -> std::time::Duration {
    std::time::Duration::from_secs(policy_u64(name))
}

/// Effective [`std::time::Duration`] from a milliseconds-valued policy key.
pub fn policy_millis(name: &str) -> std::time::Duration {
    std::time::Duration::from_millis(policy_u64(name))
}
