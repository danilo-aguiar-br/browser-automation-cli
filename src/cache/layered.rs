// SPDX-License-Identifier: MIT OR Apache-2.0
//! Layered L1 memory + L2 sqlite.
use super::memory::MemoryCache;
use super::sqlite::SqliteCache;
use super::types::{CacheEntry, CacheKey, HttpCache};
use crate::error::CliError;

/// Layered L1 memory + L2 sqlite.
pub struct LayeredCache {
    /// In-process layer.
    pub l1: MemoryCache,
    /// Disk layer.
    pub l2: SqliteCache,
}

impl HttpCache for LayeredCache {
    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CliError> {
        if let Some(e) = self.l1.get(key)? {
            return Ok(Some(e));
        }
        if let Some(e) = self.l2.get(key)? {
            let _ = self.l1.put(key, e.clone());
            return Ok(Some(e));
        }
        Ok(None)
    }

    fn put(&self, key: &CacheKey, entry: CacheEntry) -> Result<(), CliError> {
        self.l1.put(key, entry.clone())?;
        self.l2.put(key, entry)?;
        Ok(())
    }
}
