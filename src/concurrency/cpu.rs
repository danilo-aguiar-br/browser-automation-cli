// SPDX-License-Identifier: MIT OR Apache-2.0
//! CPU-bound helpers (Rayon gated by threshold).

use std::sync::{Arc, OnceLock};

use tokio::sync::Semaphore;

use super::pool::{install_rayon_pool_once, rayon_threads, CPU_MAP_THRESHOLD};

/// Process-wide admission gate for CPU-heavy [`tokio::task::spawn_blocking`].
static CPU_BLOCKING_GATE: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn cpu_blocking_gate() -> Arc<Semaphore> {
    CPU_BLOCKING_GATE
        .get_or_init(|| Arc::new(Semaphore::new(rayon_threads())))
        .clone()
}

/// Run a CPU-bound closure on the blocking pool, gated to the CPU budget.
///
/// # Why a gate and not a bare `spawn_blocking`
///
/// The blocking pool is sized by
/// [`browser_max_blocking_threads`](crate::concurrency::browser_max_blocking_threads),
/// which returns up to **16**. That number is right for its usual tenants —
/// `std::fs` calls that spend their time in the kernel — and wrong for parsing.
/// A `batch-scrape` of sixteen PDFs admits sixteen parsers at once, and on a
/// four-core host twelve of them only add scheduler pressure and resident
/// memory. Tokio's own guidance is explicit: when the work is CPU-bound, cap it
/// with a semaphore rather than relying on the pool size.
///
/// # What this is NOT protecting against
///
/// It is not preventing "sixteen Rayon pools":
/// [`install_rayon_pool_once`] builds the **global** pool exactly once, so the
/// Rayon worker count is already bounded no matter how many callers arrive. The
/// contention this removes is the blocking work that does its own CPU inline —
/// PDF and HTML parsing — which Rayon never sees.
///
/// # Cancellation
///
/// Unchanged, and still cooperative: a started `spawn_blocking` task cannot be
/// aborted. The permit only decides **when work is admitted**; teardown is
/// bounded separately by [`crate::runtime_util::shutdown_runtime`].
///
/// # Errors
///
/// `tokio::task::JoinError` when the blocking closure panics, or when the task
/// is cancelled during runtime shutdown. Acquiring the admission permit cannot
/// fail: the semaphore lives in a `OnceLock` for the life of the process and is
/// never closed, and a failed acquire falls through to running ungated.
pub async fn spawn_cpu_blocking<F, R>(f: F) -> Result<R, tokio::task::JoinError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let gate = cpu_blocking_gate();
    // The semaphore is owned by a `OnceLock` for the life of the process and is
    // never closed, so `acquire_owned` cannot legitimately fail. If it somehow
    // does, running ungated is strictly better than failing the user's command
    // over an admission detail.
    let _permit = gate.acquire_owned().await.ok();
    tokio::task::spawn_blocking(f).await
}

/// CPU map over a slice: sequential when `items.len() < CPU_MAP_THRESHOLD`, else
/// Rayon under [`install_rayon_pool_once`] (pool sized to budget).
///
/// Prefer this over ad-hoc `par_iter` so small collections never pay Rayon overhead.
pub fn map_cpu<T, R, F>(items: &[T], f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync + Send,
{
    if items.len() < CPU_MAP_THRESHOLD {
        return items.iter().map(f).collect();
    }
    install_rayon_pool_once();
    use rayon::prelude::*;
    items.par_iter().map(f).collect()
}

/// Like [`map_cpu`] but for owned items that are consumed (into_par_iter).
pub fn map_cpu_owned<T, R, F>(items: Vec<T>, f: F) -> Vec<R>
where
    T: Send,
    R: Send,
    F: Fn(T) -> R + Sync + Send,
{
    if items.len() < CPU_MAP_THRESHOLD {
        return items.into_iter().map(f).collect();
    }
    install_rayon_pool_once();
    use rayon::prelude::*;
    items.into_par_iter().map(f).collect()
}

/// Parallel filter with the same threshold rule as [`map_cpu`] (PAR-84).
///
/// Sequential when `items.len() < CPU_MAP_THRESHOLD` so small console/net buffers
/// never pay Rayon coordination overhead.
///
/// Prefer [`count_cpu`] when only the match cardinality is needed — it never
/// clones the source buffer (rules_rust_economia_de_recursos: zero-copy read).
pub fn filter_cpu<T, F>(items: Vec<T>, pred: F) -> Vec<T>
where
    T: Send,
    F: Fn(&T) -> bool + Sync + Send,
{
    if items.len() < CPU_MAP_THRESHOLD {
        return items.into_iter().filter(pred).collect();
    }
    install_rayon_pool_once();
    use rayon::prelude::*;
    items.into_par_iter().filter(pred).collect()
}

/// Count matches without taking ownership of the buffer (ECO-11 / Pass 28).
///
/// Same threshold rule as [`filter_cpu`]: sequential below
/// [`CPU_MAP_THRESHOLD`], Rayon above. Use this for assert/console paths that
/// previously cloned the entire capture log just to call `.len()` on a filter.
pub fn count_cpu<T, F>(items: &[T], pred: F) -> usize
where
    T: Sync,
    F: Fn(&T) -> bool + Sync + Send,
{
    if items.len() < CPU_MAP_THRESHOLD {
        return items.iter().filter(|x| pred(x)).count();
    }
    install_rayon_pool_once();
    use rayon::prelude::*;
    items.par_iter().filter(|x| pred(x)).count()
}

/// In-place sort with Rayon when `items.len() >= CPU_MAP_THRESHOLD` (PAR-94).
///
/// Uses `par_sort_unstable` for large collections (rule: prefer unstable sort
/// when total order equality is acceptable for agent determinism of equal keys).
/// Small slices stay sequential to avoid coordination overhead.
pub fn sort_cpu<T>(items: &mut [T])
where
    T: Ord + Send,
{
    if items.len() < CPU_MAP_THRESHOLD {
        items.sort_unstable();
        return;
    }
    install_rayon_pool_once();
    use rayon::prelude::*;
    items.par_sort_unstable();
}

/// In-place sort by key with the same threshold as [`sort_cpu`] (PAR-94).
pub fn sort_by_key_cpu<T, K, F>(items: &mut [T], f: F)
where
    T: Send,
    K: Ord,
    F: Fn(&T) -> K + Sync + Send,
{
    if items.len() < CPU_MAP_THRESHOLD {
        items.sort_by_key(f);
        return;
    }
    install_rayon_pool_once();
    use rayon::prelude::*;
    items.par_sort_unstable_by_key(f);
}

/// In-place sort with comparator; Rayon when large (PAR-94).
pub fn sort_by_cpu<T, F>(items: &mut [T], compare: F)
where
    T: Send,
    F: Fn(&T, &T) -> std::cmp::Ordering + Sync + Send,
{
    if items.len() < CPU_MAP_THRESHOLD {
        items.sort_by(&compare);
        return;
    }
    install_rayon_pool_once();
    use rayon::prelude::*;
    items.par_sort_by(compare);
}
