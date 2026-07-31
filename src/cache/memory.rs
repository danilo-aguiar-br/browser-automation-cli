// SPDX-License-Identifier: MIT OR Apache-2.0
//! In-process L1 memory cache.
use std::sync::Mutex;

use rustc_hash::FxHashMap;

use crate::error::{CliError, ErrorKind};

use super::types::{CacheEntry, CacheKey, HttpCache};

/// In-process L1 cache (dies with the process — one-shot safe).
///
/// # Interior mutability
///
/// `Mutex` is required so [`HttpCache`] can take `&self` while remaining
/// `Send` for optional multi-thread use. Critical sections only clone/insert
/// map entries (no `.await`). Poison is **propagated** as [`CliError`] (not
/// recovered) because a poisoned L1 is a software fault, not residual cleanup.
#[derive(Debug, Default)]
pub struct MemoryCache {
    inner: Mutex<FxHashMap<String, CacheEntry>>,
}

impl HttpCache for MemoryCache {
    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CliError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| CliError::new(ErrorKind::Software, "cache lock poisoned"))?;
        Ok(guard.get(key.as_str()).filter(|e| e.is_fresh()).cloned())
    }

    fn put(&self, key: &CacheKey, entry: CacheEntry) -> Result<(), CliError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| CliError::new(ErrorKind::Software, "cache lock poisoned"))?;
        guard.insert(key.as_str().to_string(), entry);
        Ok(())
    }
}
