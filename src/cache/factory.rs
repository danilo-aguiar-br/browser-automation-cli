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
