// SPDX-License-Identifier: MIT OR Apache-2.0
//! Rayon / walk / browser worker sizing.

use super::budget::{cpu_count, effective_limit_capped};
use std::sync::OnceLock;

/// Walk / Rayon thread hint: budget capped by CPUs (respects `--max-concurrency`).
pub fn walk_threads() -> usize {
    effective_limit_capped(cpu_count())
}

/// Suggested Rayon thread count (respects `RAYON_NUM_THREADS` via Rayon itself
/// when using the global pool; this value is for explicit `ThreadPoolBuilder`).
pub fn rayon_threads() -> usize {
    effective_limit_capped(cpu_count())
}

/// Run a CPU-bound closure on a Rayon pool sized to the process budget.
///
/// Prefer `par_iter` at call sites when the work is already an iterator map.
/// This helper is for one-shot “run this block under a sized pool” cases.
pub fn install_rayon_pool_once() {
    // Rayon global pool: build at most once. If RAYON_NUM_THREADS is set, Rayon
    // honors it; otherwise we pin to our budget so a 128-core box does not
    // spawn 128 workers for a one-shot CLI scan.
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let n = rayon_threads();
        // Ignore error if another crate already built the global pool.
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .thread_name(|i| format!("bac-rayon-{i}"))
            .build_global();
    });
}

/// Browser Tokio worker threads: enough for CDP event fan-out, capped.
///
/// Scales with budget but stays small (CDP is I/O; extra workers beyond ~8
/// mostly burn RSS on a one-shot process).
pub fn browser_worker_threads() -> usize {
    effective_limit_capped(8).max(2)
}

/// Cap for Tokio `max_blocking_threads` on the browser runtime.
pub fn browser_max_blocking_threads() -> usize {
    effective_limit_capped(16).max(4)
}

/// Below this length, [`map_cpu`](crate::concurrency::map_cpu) stays sequential (rule: never parallelize when
/// cost ≪ coordination overhead). Measured trade-off for one-shot CLI filters.
pub const CPU_MAP_THRESHOLD: usize = 32;
