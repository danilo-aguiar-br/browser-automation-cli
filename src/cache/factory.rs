// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cache factory from XDG `cache_backend`.
use crate::error::CliError;
use crate::xdg;

use super::layered::LayeredCache;
use super::memory::MemoryCache;
use super::redis::RedisCache;
use super::sqlite::SqliteCache;
use super::types::HttpCache;

/// Build the product cache from XDG `cache_backend` (sqlite|memory|redis).
///
/// # Errors
///
/// [`crate::error::ErrorKind::Usage`] or [`crate::error::ErrorKind::Unavailable`] propagated from
/// [`RedisCache::connect`] when `cache_backend = redis` — the first for an empty
/// `cache_redis_url`, the second when the PING round-trip fails.
/// [`crate::error::ErrorKind::Io`] propagated from [`SqliteCache::open_default`] on the
/// default sqlite path when the cache directory cannot be resolved or created,
/// or the schema cannot be initialised. Loading the config is deliberately
/// infallible here: an unreadable config falls back to defaults.
pub fn default_cache() -> Result<Box<dyn HttpCache>, CliError> {
    let cfg = xdg::load_config().unwrap_or_default();
    let backend = cfg
        .cache_backend
        .as_deref()
        .unwrap_or("sqlite")
        .to_ascii_lowercase();
    match backend.as_str() {
        "memory" => Ok(Box::new(MemoryCache::default())),
        "redis" => {
            let url = cfg.cache_redis_url.as_deref().unwrap_or("");
            Ok(Box::new(RedisCache::connect(url)?))
        }
        // default sqlite layered
        _ => Ok(Box::new(LayeredCache {
            l1: MemoryCache::default(),
            l2: SqliteCache::open_default()?,
        })),
    }
}
