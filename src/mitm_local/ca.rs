// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local MITM CA, under `--mitm-ca-dir` or XDG data (`mitm/ca`) by default.

use rcgen::{CertificateParams, KeyPair};
use serde_json::{json, Value};

use crate::constants::MITM_BIND_HOST;
use crate::error::{CliError, ErrorKind};
use crate::xdg;

use super::util::atomic_write;

/// Ensure CA key/cert exist; return paths.
///
/// The directory is `--mitm-ca-dir` when the operator named one, and the XDG
/// data location otherwise. The returned envelope publishes `ca_dir`, so the
/// caller can see WHICH of the two answered rather than assuming the default.
pub fn ensure_ca() -> Result<Value, CliError> {
    // `--mitm-ca-dir` wins over the XDG default; absent, the default stands.
    let ca_dir = match super::policy::ca_dir() {
        Some(explicit) => explicit.to_path_buf(),
        None => xdg::mitm_ca_dir()?,
    };
    xdg::ensure_dir(&ca_dir)?;
    let cert_path = ca_dir.join("ca.pem");
    let key_path = ca_dir.join("ca.key.pem");
    if !cert_path.exists() || !key_path.exists() {
        let mut params = CertificateParams::new(vec!["browser-automation-cli MITM CA".into()])
            .map_err(|e| CliError::new(ErrorKind::Software, format!("rcgen params: {e}")))?;
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let key_pair = KeyPair::generate()
            .map_err(|e| CliError::new(ErrorKind::Software, format!("rcgen key: {e}")))?;
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| CliError::new(ErrorKind::Software, format!("rcgen self-signed: {e}")))?;
        // `atomic_write` creates its temp file `0600` and propagates any
        // permission failure, so both files are private from the moment they
        // exist. The explicit chmod that used to follow these two calls is gone
        // for a reason worth recording: it ran AFTER the key was already on
        // disk, so it narrowed the exposure window instead of closing it, and
        // it discarded its own error, so a filesystem that refused the change
        // left the CA root key world-readable with nothing reported.
        atomic_write(&cert_path, cert.pem().as_bytes())?;
        atomic_write(&key_path, key_pair.serialize_pem().as_bytes())?;
    }
    Ok(json!({
        "ca_dir": ca_dir.display().to_string(),
        "cert_path": cert_path.display().to_string(),
        "key_path": key_path.display().to_string(),
        "bind": MITM_BIND_HOST,
        "note": "CA ready for local one-shot MITM; never bind 0.0.0.0",
    }))
}

/// Load CA cert+key PEMs on the Tokio blocking pool (PAR-91 / PAR-100).
///
/// Ensures CA files exist via [`ensure_ca`], then reads both PEMs with
/// [`crate::concurrency::read_to_string_blocking`] so async oneshot proxy paths
/// never pin workers with `std::fs::read_to_string`.
pub(super) async fn load_ca_pems_blocking() -> Result<(String, String), CliError> {
    let ca_meta = ensure_ca()?;
    let cert_path = ca_meta
        .get("cert_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::new(ErrorKind::Config, "CA cert path missing"))?
        .to_string();
    let key_path = ca_meta
        .get("key_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::new(ErrorKind::Config, "CA key path missing"))?
        .to_string();
    let ca_cert = crate::concurrency::read_to_string_blocking(std::path::PathBuf::from(cert_path))
        .await
        .map_err(|e| CliError::new(ErrorKind::Io, format!("read CA cert: {e}")))?;
    let ca_key = crate::concurrency::read_to_string_blocking(std::path::PathBuf::from(key_path))
        .await
        .map_err(|e| CliError::new(ErrorKind::Io, format!("read CA key: {e}")))?;
    Ok((ca_cert, ca_key))
}
