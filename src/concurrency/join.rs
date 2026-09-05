// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bounded fan-out helpers.

use super::budget::{HARD_CAP, MIN_CONCURRENCY};
use futures_util::stream::{self, StreamExt};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Run a list of futures with bounded concurrency via **`Arc<Semaphore>`**.
///
/// Gate of record (rules_rust_paralelismo): each future acquires one permit with
/// [`Semaphore::acquire_owned`] before polling body work; the permit is dropped
/// (RAII) when the future completes. Internally composed with
/// `buffer_unordered` so the stream polls at most `limit` futures, **and** the
/// Semaphore is the admission control agents / tests observe.
///
/// Results are returned in **completion order**. Prefer this over unbounded
/// `join_all` on collections of unknown size.
///
/// # Cancel safety
///
/// Each future is polled independently; dropping the returned future cancels
/// in-flight work at the next await point of those futures (permits return).
///
/// # Observability
///
/// Host-local only: `tracing::debug!` of `available_permits` at start (no remote
/// OTel — product law).
/// # Gate pattern
///
/// Uses [`Semaphore::acquire`] (not `acquire_owned`) because callers pass
/// borrowed CDP futures that are **not** `'static`. Work stays on the same
/// poller (`buffer_unordered`); permit is held for the future body and dropped
/// via RAII. For `tokio::spawn` fan-out, call sites use `acquire_owned` +
/// `JoinSet` instead (batch/crawl).
pub async fn join_bounded<F, T>(futures: Vec<F>, limit: usize) -> Vec<T>
where
    F: Future<Output = T>,
{
    let limit = limit.clamp(MIN_CONCURRENCY, HARD_CAP);
    let n = futures.len();
    let sem = Arc::new(Semaphore::new(limit));
    tracing::debug!(
        available_permits = sem.available_permits(),
        limit,
        n,
        "join_bounded fan-out (Arc<Semaphore>::acquire)"
    );
    let gated = futures.into_iter().map(|f| {
        let sem = Arc::clone(&sem);
        async move {
            // acquire (same-scope): RAII permit for the duration of f.
            let _permit = sem.acquire().await.ok();
            f.await
        }
    });
    stream::iter(gated).buffer_unordered(limit).collect().await
}

/// Like [`join_bounded`] but preserves input order via indexed futures.
pub async fn join_bounded_ordered<F, T>(futures: Vec<F>, limit: usize) -> Vec<T>
where
    F: Future<Output = T>,
{
    let limit = limit.clamp(MIN_CONCURRENCY, HARD_CAP);
    let indexed: Vec<_> = futures
        .into_iter()
        .enumerate()
        .map(|(i, f)| async move { (i, f.await) })
        .collect();
    let mut pairs = join_bounded(indexed, limit).await;
    pairs.sort_by_key(|(i, _)| *i);
    pairs.into_iter().map(|(_, v)| v).collect()
}
