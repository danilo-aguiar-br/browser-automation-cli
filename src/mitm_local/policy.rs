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
}

/// Publish the capture policy for this process. Called once from CLI dispatch.
pub fn install(flags: &CaptureFlags) {
    MAX_BODY_BYTES.store(
        flags.max_body_bytes.unwrap_or(DEFAULT_MAX_BODY_BYTES),
        Ordering::Relaxed,
    );
    SKIP_MEDIA.store(flags.no_media_bodies, Ordering::Relaxed);
    // Asking for masking and asking to turn it off is a contradiction, and the
    // safe reading of a contradiction about secrets is to mask.
    let redact = flags.redact_secrets || !flags.no_redact_secrets;
    REDACT_SECRETS.store(redact, Ordering::Relaxed);
    let _ = HOSTS.set(
        flags
            .hosts
            .as_deref()
            .map(super::handler::parse_hosts_public)
            .unwrap_or_default(),
    );
    let _ = HAR_PATH.set(flags.har.clone());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe_before_install() {
        // Read without install: a capture started from a code path that forgot
        // to publish policy must still redact and must still be bounded.
        assert!(redact_secrets(), "secrets must be masked by default");
        assert!(max_body_bytes() > 0, "bodies must have a ceiling");
    }

    #[test]
    fn install_publishes_every_field() {
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
        install(&CaptureFlags::default());
        assert_eq!(max_body_bytes(), DEFAULT_MAX_BODY_BYTES);
        assert!(redact_secrets());
    }

    #[test]
    fn asking_to_mask_and_to_unmask_resolves_to_masking() {
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
