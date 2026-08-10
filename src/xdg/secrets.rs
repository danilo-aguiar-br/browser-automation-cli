// SPDX-License-Identifier: MIT OR Apache-2.0
//! Secret accessors with [`zeroize::Zeroizing`] wipe-on-drop (SEC-01).

use zeroize::Zeroize;

use super::config_io::load_config;

/// Load optional encryption key from XDG config only (never product env vars).
///
/// Returns [`zeroize::Zeroizing`] so the secret is wiped on drop (SEC-01 /
/// rules_rust_gerenciamento_memoria). Callers should keep the guard short-lived.
pub fn encryption_key() -> Option<zeroize::Zeroizing<String>> {
    let mut cfg = load_config().ok()?;
    let key = cfg.encryption_key.take().filter(|s| !s.is_empty())?;
    if let Some(ref mut k) = cfg.openrouter_api_key {
        k.zeroize();
    }
    if let Some(ref mut k) = cfg.cache_redis_url {
        k.zeroize();
    }
    // Persisted from 2026-08-10 onward, so it now reaches this struct and has to
    // be wiped with the other secrets rather than left in the dropped config.
    if let Some(ref mut k) = cfg.proxy_password {
        k.zeroize();
    }
    Some(zeroize::Zeroizing::new(key))
}

/// Optional LLM API key from XDG config only (never product env vars).
///
/// Returns [`zeroize::Zeroizing`] so the secret is wiped on drop (SEC-01).
pub fn openrouter_api_key() -> Option<zeroize::Zeroizing<String>> {
    let mut cfg = load_config().ok()?;
    let key = cfg.openrouter_api_key.take().filter(|s| !s.is_empty())?;
    if let Some(ref mut k) = cfg.encryption_key {
        k.zeroize();
    }
    if let Some(ref mut k) = cfg.cache_redis_url {
        k.zeroize();
    }
    // Same reason as in [`encryption_key`]: persisted, therefore loaded, therefore wiped.
    if let Some(ref mut k) = cfg.proxy_password {
        k.zeroize();
    }
    Some(zeroize::Zeroizing::new(key))
}
