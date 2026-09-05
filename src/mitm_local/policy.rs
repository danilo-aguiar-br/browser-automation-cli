// SPDX-License-Identifier: MIT OR Apache-2.0
//! Process-wide MITM capture policy, published once from CLI dispatch.
//!
//! # Why this module exists
//!
//! Three flags were declared on the CLI and read by nobody:
//! `--mitm-max-body-bytes`, `--mitm-no-media-bodies` and
//! `--mitm-redact-secrets`. `--help` promised a body ceiling, a media filter
//! and a redaction switch; the capture applied none of them.
//!
//! Redaction did happen, but by accident of call site rather than by decision:
//! every caller passed `true` literally, so the flag could neither turn it on
//! nor off. A flag that parses and does nothing is the same defect class the
//! phantom-flag gate exists to catch — it just was not looking at this family.
//!
//! The values live in atomics rather than being threaded through the proxy
//! constructor because the hudsucker handler is cloned per request/response
//! pair, and widening every constructor to carry three fields would put the
//! policy in a dozen signatures that have no interest in it.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Bytes retained per body. Zero keeps no bodies at all.
static MAX_BODY_BYTES: AtomicUsize = AtomicUsize::new(DEFAULT_MAX_BODY_BYTES);

/// Whether image/video/audio payloads are dropped.
static SKIP_MEDIA: AtomicBool = AtomicBool::new(false);

/// Whether Authorization/Cookie families are masked. On unless turned off.
static REDACT_SECRETS: AtomicBool = AtomicBool::new(true);

/// Whether argv spoke about redaction at all, as opposed to staying silent.
///
/// Kept apart from the resolved value because the two answer different
/// questions: `REDACT_SECRETS` is WHAT the policy is, this is WHERE it came
/// from. `mitm redact` reported `source: "persisted"` for a value argv had just
/// overridden, which is a worse answer than none — an operator debugging why a
/// capture is masked would go edit a file that is not in charge.
static REDACT_FROM_ARGV: AtomicBool = AtomicBool::new(false);

/// Default retained bytes per body.
///
/// Large enough for an API response or an HTML head, small enough that a
/// capture of a media-heavy page does not become the largest artifact in the
/// run. The operator raises it with `--mitm-max-body-bytes`.
pub const DEFAULT_MAX_BODY_BYTES: usize = 65_536;

/// Global `--mitm-hosts`, for commands other than `mitm capture-url`.
static HOSTS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

/// Global `--mitm-har`, for commands other than `mitm har`.
static HAR_PATH: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();

/// Global `--mitm-ca-dir`, empty when the operator did not name one.
static CA_DIR: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();

/// Everything the operator asked of the capture, resolved from argv.
#[derive(Debug, Default)]
pub struct CaptureFlags {
    /// `--mitm-max-body-bytes`
    pub max_body_bytes: Option<usize>,
    /// `--mitm-no-media-bodies`
    pub no_media_bodies: bool,
    /// `--mitm-redact-secrets`, an explicit restatement of the default.
    pub redact_secrets: bool,
    /// `--mitm-no-redact-secrets`, the only way to turn masking off.
    pub no_redact_secrets: bool,
    /// `--mitm-hosts`, comma separated.
    pub hosts: Option<String>,
    /// `--mitm-har`
    pub har: Option<std::path::PathBuf>,
    /// `--mitm-ca-dir`, overriding the XDG data location for the CA pair.
    ///
    /// Measured 2026-09-01: the flag was declared, printed in six documentation
    /// pages as "(default: XDG data)", and the field had ZERO readers — the CA
    /// always landed under `data_dir()/mitm/ca`. The name collided with the
    /// helper `xdg::mitm_ca_dir()`, so a naive search found seven hits and the
    /// flag looked wired. Promising an alternative location and ignoring it is
    /// worse than not offering one, because the operator believes the key
    /// material moved.
    pub ca_dir: Option<std::path::PathBuf>,
}

/// Publish the capture policy for this process. Called once from CLI dispatch.
pub fn install(flags: &CaptureFlags) {
    MAX_BODY_BYTES.store(
        flags.max_body_bytes.unwrap_or(DEFAULT_MAX_BODY_BYTES),
        Ordering::Relaxed,
    );
    SKIP_MEDIA.store(flags.no_media_bodies, Ordering::Relaxed);
    // Precedence: argv, then the persisted preference, then the safe default.
    //
    // The disk is consulted ONLY when argv said nothing, so `mitm redact` sets a
    // default and never a lock — an operator who persisted `false` and then runs
    // a capture with no flag gets their choice, and one who passes
    // `--mitm-redact-secrets` on that same machine still gets masking.
    let redact = if flags.redact_secrets || flags.no_redact_secrets {
        // Asking for masking and asking to turn it off is a contradiction, and
        // the safe reading of a contradiction about secrets is to mask.
        flags.redact_secrets || !flags.no_redact_secrets
    } else {
        super::store::persisted_redact_secrets().unwrap_or(true)
    };
    REDACT_SECRETS.store(redact, Ordering::Relaxed);
    REDACT_FROM_ARGV.store(
        flags.redact_secrets || flags.no_redact_secrets,
        Ordering::Relaxed,
    );
    let _ = HOSTS.set(
        flags
            .hosts
            .as_deref()
            .map(super::handler::parse_hosts_public)
            .unwrap_or_default(),
    );
    let _ = HAR_PATH.set(flags.har.clone());
    let _ = CA_DIR.set(flags.ca_dir.clone());
}

/// Directory named by the global `--mitm-ca-dir`, if any.
///
/// `None` means "use the XDG default", which is what [`super::ca::ensure_ca`]
/// falls back to. Returning the raw override rather than resolving it here
/// keeps the XDG error path in one place.
#[must_use]
pub fn ca_dir() -> Option<&'static std::path::Path> {
    CA_DIR.get().and_then(|o| o.as_deref())
}

/// Hosts named by the global `--mitm-hosts`, empty when the flag was absent.
///
/// The per-command `--hosts` of `mitm capture-url` wins where both exist; this
/// is what makes the global flag mean something for `scrape --mitm` and the
/// other commands that can turn interception on.
#[must_use]
pub fn hosts() -> &'static [String] {
    HOSTS.get().map(Vec::as_slice).unwrap_or(&[])
}

/// Path named by the global `--mitm-har`, if any.
#[must_use]
pub fn har_path() -> Option<&'static std::path::Path> {
    HAR_PATH.get().and_then(|o| o.as_deref())
}

/// Bytes retained per body in this process.
#[must_use]
pub fn max_body_bytes() -> usize {
    MAX_BODY_BYTES.load(Ordering::Relaxed)
}

/// Whether media payloads are dropped in this process.
#[must_use]
pub fn skip_media() -> bool {
    SKIP_MEDIA.load(Ordering::Relaxed)
}

/// Whether secrets are masked in this process.
///
/// # Why the default is on
///
/// A capture is written to disk and read back by an agent. Defaulting to
/// masked means forgetting the flag costs a missing header, while defaulting to
/// clear would mean forgetting it costs a leaked session cookie.
#[must_use]
pub fn redact_secrets() -> bool {
    REDACT_SECRETS.load(Ordering::Relaxed)
}

/// Set the redaction switch directly. Tests only.
///
/// The production path reaches `REDACT_SECRETS` through `publish`, which also
/// consumes argv, the persisted store and the host list. A test that wants to
/// state a property about redaction ALONE cannot pay for all of that, and
/// building a fake `MitmFlags` to reach one boolean would couple the test to a
/// struct it is not about.
#[cfg(test)]
pub(super) fn set_redact_secrets_for_test(redact: bool) {
    REDACT_SECRETS.store(redact, Ordering::Relaxed);
}

/// Whether the redaction policy in force was named on the command line.
///
/// `false` means argv was silent and the value came from the persisted
/// preference or from the default. Only `mitm redact` needs this, to report a
/// `source` it can stand behind.
#[must_use]
pub fn redact_from_argv() -> bool {
    REDACT_FROM_ARGV.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use super::*;

    /// These tests read and write PROCESS-WIDE atomics, so they cannot overlap.
    ///
    /// Restoring the defaults at the end of each test — which the code below
    /// already did — is cleanup, not exclusion: it does nothing about the window
    /// between one test's `install` and its own assert, during which a sibling
    /// test publishes different values. MEASURED on 2026-08-28: one run in six
    /// of `cargo test --lib mitm` failed with `left: 65536, right: 10`, the
    /// default overwriting the value this test had just installed.
    ///
    /// The full-suite run hid it. `cargo test --lib` passed 945/945 every time,
    /// because 933 other tests change the scheduling enough that the window
    /// never opened — a green suite is one sample of one interleaving, not proof
    /// that no race exists.
    ///
    /// Same guard shape as `residual::tests::SHARED_ROOTS`, which keeps the
    /// default test parallelism instead of forcing `--test-threads 1`.
    static POLICY_LOCK: Mutex<()> = Mutex::new(());

    /// Take the policy guard, recovering from a poisoned lock.
    ///
    /// A failing test poisons the mutex; without this the sibling tests would
    /// report a `PoisonError` instead of their own result, turning one failure
    /// into three and hiding which assert actually broke.
    fn policy_guard() -> MutexGuard<'static, ()> {
        POLICY_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn defaults_are_safe_before_install() {
        let _guard = policy_guard();
        // Read without install: a capture started from a code path that forgot
        // to publish policy must still redact and must still be bounded.
        assert!(redact_secrets(), "secrets must be masked by default");
        assert!(max_body_bytes() > 0, "bodies must have a ceiling");
    }

    #[test]
    fn install_publishes_every_field() {
        let _guard = policy_guard();
        install(&CaptureFlags {
            max_body_bytes: Some(10),
            no_media_bodies: true,
            no_redact_secrets: true,
            ..Default::default()
        });
        assert_eq!(max_body_bytes(), 10);
        assert!(skip_media());
        assert!(!redact_secrets());
        // Restore, since this is process-wide state shared with other tests.
        //
        // `redact_secrets: true` rather than `Default::default()` on purpose: a
        // silent `CaptureFlags` now falls through to the operator's persisted
        // preference, so a developer who had run `mitm redact --secrets false`
        // on their own machine would fail this assert for a reason that has
        // nothing to do with the code under test.
        install(&CaptureFlags {
            redact_secrets: true,
            ..Default::default()
        });
        assert_eq!(max_body_bytes(), DEFAULT_MAX_BODY_BYTES);
        assert!(redact_secrets());
    }

    #[test]
    fn an_explicit_flag_beats_whatever_is_on_disk() {
        let _guard = policy_guard();
        // The persisted preference is a DEFAULT. Whatever this machine holds,
        // naming the flag has to win, or `mitm redact` would be a lock.
        install(&CaptureFlags {
            redact_secrets: true,
            ..Default::default()
        });
        assert!(
            redact_secrets(),
            "explicit mask lost to the persisted value"
        );
        install(&CaptureFlags {
            no_redact_secrets: true,
            ..Default::default()
        });
        assert!(
            !redact_secrets(),
            "explicit unmask lost to the persisted value"
        );
        install(&CaptureFlags {
            redact_secrets: true,
            ..Default::default()
        });
    }

    #[test]
    fn asking_to_mask_and_to_unmask_resolves_to_masking() {
        let _guard = policy_guard();
        // A contradiction about secrets has one safe reading.
        install(&CaptureFlags {
            redact_secrets: true,
            no_redact_secrets: true,
            ..Default::default()
        });
        assert!(redact_secrets());
        install(&CaptureFlags::default());
    }
}
