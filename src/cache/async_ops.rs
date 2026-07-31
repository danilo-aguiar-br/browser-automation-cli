// SPDX-License-Identifier: MIT OR Apache-2.0
//! Async wrappers for sync [`HttpCache`] (spawn_blocking; never block Tokio workers).

use std::sync::Arc;
use std::sync::OnceLock;

use crate::error::{CliError, ErrorKind};

use super::factory::default_cache;
use super::types::{CacheEntry, CacheKey, HttpCache};

/// Process-wide default cache (one-shot reuse; avoids Redis PING per get/put).
fn process_cache() -> Result<&'static Arc<dyn HttpCache>, CliError> {
    static CACHE: OnceLock<Result<Arc<dyn HttpCache>, String>> = OnceLock::new();
    match CACHE.get_or_init(|| {
        default_cache()
            .map(Arc::from)
            .map_err(|e| e.message().to_string())
    }) {
        Ok(c) => Ok(c),
        Err(msg) => Err(CliError::new(ErrorKind::Unavailable, msg.clone())),
    }
}

/// Lookup cache key on the blocking pool.
pub async fn get_async(key: &CacheKey) -> Result<Option<CacheEntry>, CliError> {
    let cache = process_cache()?.clone();
    let key = key.clone();
    tokio::task::spawn_blocking(move || cache.get(&key))
        .await
        .map_err(|e| {
            if e.is_panic() {
                CliError::new(ErrorKind::Software, "cache get task panicked")
            } else {
                CliError::new(ErrorKind::Software, format!("cache get join: {e}"))
            }
        })?
}

/// Store cache entry on the blocking pool.
pub async fn put_async(key: &CacheKey, entry: CacheEntry) -> Result<(), CliError> {
    let cache = process_cache()?.clone();
    let key = key.clone();
    tokio::task::spawn_blocking(move || cache.put(&key, entry))
        .await
        .map_err(|e| {
            if e.is_panic() {
                CliError::new(ErrorKind::Software, "cache put task panicked")
            } else {
                CliError::new(ErrorKind::Software, format!("cache put join: {e}"))
            }
        })?
}
