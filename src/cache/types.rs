// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cache key/entry types and [`HttpCache`] trait.
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::error::CliError;

/// Everything besides the URL that changes what a GET returns.
///
/// # Why the URL alone is not the identity of a response
///
/// A cache key answers "have I already asked this question?". Keying on the
/// URL alone answers a narrower one — "has anyone asked about this address?" —
/// and the two diverge the moment the request can leave the host by more than
/// one route.
///
/// Measured before this type existed: with a URL already cached, adding
/// `--proxy http://127.0.0.1:1` returned `ok: true` and `cache_hit: true`. A
/// dead proxy read as success, so the isolation the operator asked for was
/// silently not applied. The same collision hits `extra_headers`: two callers
/// with different `Authorization` values shared one entry.
#[derive(Debug, Clone, Copy)]
pub struct CacheContext<'a> {
    /// Egress proxy URL, when one is configured for this process.
    pub proxy: Option<&'a str>,
    /// Resolved stealth profile name; changes the request headers sent.
    pub stealth_profile: &'a str,
    /// Caller-supplied request headers, in any order.
    pub extra_headers: &'a [(String, String)],
}

impl<'a> CacheContext<'a> {
    /// A context with no proxy and no extra headers.
    ///
    /// For call sites that genuinely have one route, such as local file parse.
    #[must_use]
    pub fn direct(stealth_profile: &'a str) -> Self {
        Self {
            proxy: None,
            stealth_profile,
            extra_headers: &[],
        }
    }
}

/// Cache key derived from method + URL + optional body hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey(String);

impl CacheKey {
    /// Build a stable key for an HTTP GET URL and its egress context.
    ///
    /// Headers are sorted before hashing. A `HeaderMap` has no insertion order
    /// and neither does a caller assembling flags, so hashing them as given
    /// would make the key depend on argument order — an unstable key defeats
    /// the cache entirely instead of merely partitioning it.
    pub fn http_get(url: &str, ctx: &CacheContext<'_>) -> Self {
        let mut h = Sha256::new();
        h.update(b"GET\0");
        h.update(url.as_bytes());
        h.update(b"\0proxy\0");
        h.update(ctx.proxy.unwrap_or("").as_bytes());
        h.update(b"\0profile\0");
        h.update(ctx.stealth_profile.as_bytes());
        h.update(b"\0headers\0");
        let mut pairs: Vec<(&str, &str)> = ctx
            .extra_headers
            .iter()
            .map(|(n, v)| (n.as_str(), v.as_str()))
            .collect();
        pairs.sort_unstable();
        for (name, value) in pairs {
            // Length-prefixed so ("ab", "c") and ("a", "bc") cannot collide.
            h.update((name.len() as u64).to_le_bytes());
            h.update(name.as_bytes());
            h.update((value.len() as u64).to_le_bytes());
            h.update(value.as_bytes());
        }
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
    /// URL the response actually came from, after redirects.
    ///
    /// # Why the cache has to carry this
    ///
    /// A fresh fetch reports `source_url` from `final_url`, which is where the
    /// body was really served; a cache hit had no such field and could only
    /// echo the URL that was ASKED for. Following a redirect therefore changed
    /// the reported origin between the first collection and every later one,
    /// for a body that never changed.
    ///
    /// `None` means an entry written before this field existed. It keeps the
    /// old behaviour exactly — fall back to the requested URL — so a populated
    /// cache stays usable instead of being silently invalidated.
    pub final_url: Option<String>,
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
