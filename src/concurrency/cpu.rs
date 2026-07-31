// SPDX-License-Identifier: MIT OR Apache-2.0
//! CPU-bound helpers (Rayon gated by threshold).

use super::pool::{install_rayon_pool_once, CPU_MAP_THRESHOLD};

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

/// Like [`map_cpu`](crate::concurrency::map_cpu) but for owned items that are consumed (into_par_iter).
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

/// Parallel filter with the same threshold rule as [`map_cpu`](crate::concurrency::map_cpu) (PAR-84).
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
