// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cache key/entry types and [`HttpCache`] trait.
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::error::CliError;

/// Cache key derived from method + URL + optional body hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey(String);

impl CacheKey {
    /// Build a stable key for an HTTP GET URL.
    pub fn http_get(url: &str) -> Self {
        let mut h = Sha256::new();
        h.update(b"GET\0");
        h.update(url.as_bytes());
        Self(hex::encode(h.finalize()))
    }

    /// Build a stable key for local file parse (path + mtime + len).
    pub fn file_parse(path: &Path, len: u64, mtime_secs: u64) -> Self {
        let mut h = Sha256::new();
        h.update(b"PARSE\0");
        h.update(path.to_string_lossy().as_bytes());
        h.update(len.to_le_bytes());
        h.update(mtime_secs.to_le_bytes());
        Self(hex::encode(h.finalize()))
    }

    /// Hex digest string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Cached payload.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Raw body bytes or UTF-8 text.
    pub body: Vec<u8>,
    /// Optional content-type hint.
    pub content_type: Option<String>,
    /// Expiry as unix seconds (0 = no expiry).
    pub expires_unix: u64,
}

impl CacheEntry {
    /// True when entry is still valid.
    pub fn is_fresh(&self) -> bool {
        if self.expires_unix == 0 {
            return true;
        }
        now_unix() < self.expires_unix
    }
}

/// Trait for one-shot HTTP/parse caches.
///
/// `Sync` required so the process-wide [`Arc`](std::sync::Arc) can be shared
/// into `spawn_blocking` tasks (Pass N — never block Tokio workers on SQLite/Redis).
pub trait HttpCache: Send + Sync {
    /// Lookup a key.
    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CliError>;
    /// Store a key.
    fn put(&self, key: &CacheKey, entry: CacheEntry) -> Result<(), CliError>;
}

/// TTL helper: now + duration.
pub fn expires_after(ttl: Duration) -> u64 {
    now_unix().saturating_add(ttl.as_secs())
}

pub(crate) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
