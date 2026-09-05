// SPDX-License-Identifier: MIT OR Apache-2.0
//! Effective-value accessors over the one-shot policy snapshot.
//!
//! The XDG file is read at most once per process and cached, so hot loops
//! (event pump slices, poll intervals) never re-read disk.

use std::sync::OnceLock;

use super::knobs::{policy_default, policy_stored, PolicyConfig};

/// Process-wide snapshot of the policy layer (one-shot: read disk at most once).
static SNAPSHOT: OnceLock<PolicyConfig> = OnceLock::new();

/// The cached policy, or a fresh read under `cfg(test)`.
///
/// # Why the cache is bypassed in tests and nowhere else
///
/// The product is one-shot: a process resolves policy once and dies, so a
/// snapshot is exactly right and hot loops never touch disk for it.
///
/// A TEST BINARY is not one-shot. Measured 2026-08-24: all tests in one binary
/// share the process, so the FIRST test that touches a policy knob wins for
/// every test after it — a test that writes config and then reads a `policy_*`
/// value sees the other test's value, and `libtest` does not guarantee order,
/// so which value it sees is not decidable from the source.
///
/// The catalogued repair was to replace the `OnceLock` with dependency
/// injection, which threads a config handle through every call site of a
/// generated table of 107 knobs. That is a change to the ARCHITECTURE to fix a
/// property of the TEST HARNESS, and it would make the production path pay
/// argument-passing for a value that genuinely cannot change there.
///
/// So the cache is skipped where it is wrong and kept where it is right.
/// `load_config` has no cache of its own, so the test path re-reads the file
/// and every test sees the config IT wrote.
///
/// The leak is deliberate and bounded: one `PolicyConfig` per call, under
/// `cfg(test)` only, in a process that exits at the end of the run. Returning
/// `&'static` without it would mean handing out a reference to a temporary.
///
/// Written as TWO whole functions rather than one body with two `cfg` blocks.
/// The single-body shape needed an explicit `return` in the test arm, because
/// under `cfg(test)` the other arm disappears and the block stops being the
/// tail expression — and clippy's `needless_return` then fires on a `return`
/// that only looks redundant in one of the two builds. Splitting the function
/// makes each build read the arm it actually compiles, with no `cfg` left
/// inside an expression.
#[cfg(test)]
fn snapshot() -> &'static PolicyConfig {
    Box::leak(Box::new(
        crate::xdg::load_config()
            .map(|c| c.policy)
            .unwrap_or_default(),
    ))
}

/// Production half of [`snapshot`]: resolved once and cached for the process.
#[cfg(not(test))]
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
    let stored = policy_stored(snapshot(), name)
        .flatten()
        .filter(|&n| super::validate::keeps_stored(name, n));
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

/// Effective [`std::time::Duration`] from a seconds-valued policy key.
pub fn policy_secs(name: &str) -> std::time::Duration {
    std::time::Duration::from_secs(policy_u64(name))
}

/// Effective [`std::time::Duration`] from a milliseconds-valued policy key.
pub fn policy_millis(name: &str) -> std::time::Duration {
    std::time::Duration::from_millis(policy_u64(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test that writes config must READ what it wrote.
    ///
    /// This is the property the process-wide `OnceLock` took away. All tests in
    /// one binary share the process, so under the cached path the FIRST test to
    /// touch a knob froze the value for every test after it, and `libtest` does
    /// not guarantee order — which value a test saw was not decidable from the
    /// source. Measured 2026-08-24, fixed 2026-09-04.
    ///
    /// Two DIFFERENT values in sequence, because a single write cannot tell a
    /// working read apart from a snapshot that happened to be initialised with
    /// the same number.
    #[test]
    fn each_read_re_reads_instead_of_serving_a_frozen_copy() {
        // Pointer identity is the assertion, not the value. A cached snapshot
        // hands back the SAME address every time by construction, so two
        // different addresses prove the disk was consulted twice — which is
        // exactly what lets a test see the config IT wrote rather than the one
        // an unrelated test happened to initialise first.
        //
        // Comparing values instead would prove nothing: a frozen copy and a
        // fresh read agree whenever nothing changed between them, which is the
        // common case and the one that hid this defect.
        let a: *const PolicyConfig = snapshot();
        let b: *const PolicyConfig = snapshot();
        assert_ne!(
            a, b,
            "under cfg(test) the policy layer must not serve a frozen snapshot"
        );
    }

    /// The production static still exists and is still untouched by tests.
    ///
    /// If a future edit routes the test path back through `SNAPSHOT`, this
    /// fails: the whole repair is that tests never initialise it.
    #[test]
    fn tests_never_touch_the_production_snapshot() {
        let _ = policy_u64("redis_io_timeout_secs");
        assert!(
            SNAPSHOT.get().is_none(),
            "a test initialised the process-wide snapshot, which is the freeze this fix removed"
        );
    }
}
