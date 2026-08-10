// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pin one stealth identity across one-shot processes.
//!
//! # The problem
//!
//! `spider_fingerprint` draws a fresh identity on every call, and this product
//! is one-shot: the process that scrapes URL 1 is not the process that scrapes
//! URL 2. Measured across three identical runs on one host,
//! `navigator.hardwareConcurrency` came back 12, then 16, then 4.
//!
//! A `OnceLock` freezes the identity WITHIN a process, which is all a single
//! `goto` needs. It does nothing across processes, so a 50-URL crawl presents
//! 50 different machines from one address. That is not a weak signal being
//! masked — it is a strong signal being created. No real user's hardware
//! changes between page loads, and the pattern gets worse once a persistent
//! cookie ties the runs together: same cookie, same address, different
//! machine is more suspicious than being honest.
//!
//! # Why a file and not a seeded PRNG
//!
//! `emulate()` accepts no seed, so determinism would mean generating the whole
//! identity by hand and dropping the crate that tracks real Chrome builds. The
//! cheaper honest answer is to keep the crate and remember its answer.
//!
//! # Why this is off by default
//!
//! It writes to disk. A one-shot tool that leaves state behind without being
//! asked has changed its contract, and the state here describes the identity a
//! remote site will see. That is the caller's decision, so it needs
//! `--stealth-seed` or XDG `stealth_seed` to happen at all.

use std::fs;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

use crate::error::CliError;

/// Directory holding cached identity scripts.
fn cache_root() -> Result<PathBuf, CliError> {
    Ok(crate::xdg::state_dir()?.join("stealth"))
}

/// Filename for a seed/profile pair.
///
/// The seed is hashed rather than used directly: an operator may reasonably
/// seed with something meaningful, and a meaningful string does not belong in
/// a path where `ls` will show it. Hashing also removes every path-traversal
/// and illegal-character question in one step.
fn cache_file(seed: &str, profile: &str) -> Result<PathBuf, CliError> {
    let mut hasher = Sha256::new();
    // NUL between the fields so ("ab", "c") and ("a", "bc") cannot hash alike.
    hasher.update(seed.as_bytes());
    hasher.update([0u8]);
    hasher.update(profile.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    Ok(cache_root()?.join(format!("{}-{profile}.js", &digest[..16])))
}

/// Read the cached script for this seed, if one was stored.
///
/// A read failure is not an error: the cache is an optimisation for stability,
/// not a source of truth. Failing the run because a cache file is unreadable
/// would turn a stealth preference into an outage.
pub(super) fn load(seed: &str, profile: &str) -> Option<String> {
    let path = cache_file(seed, profile).ok()?;
    let body = fs::read_to_string(&path).ok()?;
    if body.trim().is_empty() {
        return None;
    }
    tracing::debug!(
        target: "browser_automation_cli::stealth",
        path = %path.display(),
        "reusing cached stealth identity"
    );
    Some(body)
}

/// Store the script for this seed so the next process reuses it.
///
/// Written through a temp file and renamed, so a process killed mid-write
/// cannot leave a truncated script that the next run would inject verbatim.
pub(super) fn store(seed: &str, profile: &str, script: &str) {
    let Ok(path) = cache_file(seed, profile) else {
        return;
    };
    let Ok(dir) = cache_root() else {
        return;
    };
    if crate::xdg::ensure_dir(&dir).is_err() {
        return;
    }
    let tmp = path.with_extension("js.tmp");
    if fs::write(&tmp, script).is_err() {
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // The file describes the identity a remote site will attribute to this
        // host, so it is no more world-readable than a credential.
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
    }
    if fs::rename(&tmp, &path).is_err() {
        let _ = fs::remove_file(&tmp);
    }
}

// No `clear()` here on purpose. Rotating the identity is what changing the
// seed already does, and shipping a second way to do it would mean shipping a
// public function with no caller — the exact defect this whole round exists to
// remove.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_is_stable_for_the_same_pair() {
        let _env = crate::test_utils::EnvGuard::for_reading();
        let a = cache_file("s", "chrome-linux").expect("path");
        let b = cache_file("s", "chrome-linux").expect("path");
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_do_not_collide() {
        let _env = crate::test_utils::EnvGuard::for_reading();
        let a = cache_file("one", "chrome-linux").expect("path");
        let b = cache_file("two", "chrome-linux").expect("path");
        assert_ne!(a, b);
    }

    #[test]
    fn different_profiles_do_not_collide() {
        let _env = crate::test_utils::EnvGuard::for_reading();
        // Same seed with a different profile must NOT reuse the script: the
        // cached patches encode a platform, so serving the Linux script under
        // the Windows profile would contradict the User-Agent it ships with.
        let a = cache_file("s", "chrome-linux").expect("path");
        let b = cache_file("s", "chrome-win").expect("path");
        assert_ne!(a, b);
    }

    #[test]
    fn seed_never_appears_in_the_path() {
        let _env = crate::test_utils::EnvGuard::for_reading();
        let p = cache_file("my-secret-seed", "chrome-linux").expect("path");
        assert!(!p.display().to_string().contains("my-secret-seed"));
    }
}
