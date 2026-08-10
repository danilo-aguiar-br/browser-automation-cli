// SPDX-License-Identifier: MIT OR Apache-2.0
//! HTTP / parse cache under XDG (PRD 5AF / GAP-011 / GAP-023).
//!
//! Backends: in-process L1 ([`FxHashMap`](rustc_hash::FxHashMap) of hex digests), SQLite under XDG
//! cache. Redis is optional via `config set cache_backend=redis` +
//! `cache_redis_url` (never env).
//!
//! # Workload / hashing
//!
//! L1 keys are SHA-256 hex digests produced by this process (trusted).
//! [`rustc_hash::FxHashMap`] avoids SipHash overhead on short fixed-width
//! keys (rules_rust_eficiencia_e_performance). Do **not** use FxHash for
//! untrusted external key spaces.
//!
//! **Concurrency:** single-key get/put under short `Mutex` critical sections
//! that never cross `.await` (rules). Not a multi-item fan-out surface.
//!
//! ## Module map (componentization)
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `types` | `CacheKey` / `CacheEntry` / `HttpCache` / TTL helpers |
//! | `memory` | L1 in-process map |
//! | `sqlite` | L2 XDG SQLite |
//! | `layered` | L1+L2 |
//! | `redis` | optional RESP backend |
//! | `factory` | `default_cache` from XDG |

mod async_ops;
mod factory;
mod layered;
mod memory;
mod redis;
mod sqlite;
mod types;

#[cfg(test)]
mod tests;

pub use async_ops::{get_async, put_async};
pub use factory::default_cache;
pub use layered::LayeredCache;
pub use memory::MemoryCache;
pub use redis::RedisCache;
pub use sqlite::SqliteCache;
pub use types::{expires_after, CacheContext, CacheEntry, CacheKey, HttpCache};
