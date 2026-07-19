// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bounded parallelism and concurrency for the one-shot CLI.
//!
//! # Workload classification (rules_rust_paralelismo)
//!
//! | Class | Paths | Tool |
//! |-------|-------|------|
//! | **I/O-bound** | HTTP scrape/crawl/batch, CDP fan-out, robots | Tokio + `Arc<Semaphore>` / `JoinSet` / [`join_bounded`] |
//! | **CPU-bound** | Structural scan (`sg`), multi-file text match | Rayon `par_iter` |
//! | **Mista** | Browser session (CDP I/O + light DOM parse) | Multi-thread Tokio; no Rayon on async workers |
//! | **Subprocess** | Chrome residual | Existing residual kill path (no unbounded fork) |
//!
//! # Formula (permits)
//!
//! ```text
//! auto = min(
//!   available_parallelism(),
//!   max(1, (free_ram_mb * 50%) / ram_per_task_mb),
//!   HARD_CAP
//! )
//! effective = override_if_set_else(auto)
//! ```
//!
//! - **ram_per_task_mb:** 64 for HTTP I/O tasks (conservative floor).
//!   Measure with `/usr/bin/time -v` → "Maximum resident set size" via
//!   `scripts/rss-baseline.sh` (doctor offline / single scrape). Revalidate when
//!   reqwest/scraper/chromiumoxide jump materially.
//! - **50% safety margin** on free RAM so concurrent tasks cannot OOM the host.
//! - **HARD_CAP** prevents FD / Chrome overwhelm on large hosts.
//! - Override: global CLI `--max-concurrency=N` (`0` = auto).
//! - **One-shot:** auto budget is cached once (`OnceLock`); long daemon rebalance
//!   is N/A (BORN→EXECUTE→FINALIZE→DIE). Crawl/batch respect cancel token mid-run.
//!
//! # Product law
//!
//! One-shot CLI (BORN→EXECUTE→FINALIZE→DIE). No daemon fleets, no remote
//! metrics of `available_permits`, no `systemd-run` default (ops N/A).
//! Every fan-out **must** use [`effective_limit`] or an explicit local clamp
//! derived from it. Gate of record: `Arc<Semaphore>` + `acquire_owned`.

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use futures_util::stream::{self, StreamExt};
use tokio::sync::Semaphore;

/// Hard ceiling for any fan-out (FD budget + Chrome CDP politeness).
pub const HARD_CAP: usize = 64;

/// Floor when auto-detection fails.
pub const MIN_CONCURRENCY: usize = 1;

/// Conservative RSS budget per concurrent HTTP/CDP task (MiB).
///
/// Ground-truth method: `/usr/bin/time -v` → "Maximum resident set size"
/// (`scripts/rss-baseline.sh`). Value is a **rounded-up floor** so concurrent
/// tasks leave headroom under the 50% free-RAM margin. Revalidate when
/// reqwest/scraper or chromiumoxide versions jump materially.
pub const RAM_PER_IO_TASK_MB: u64 = 64;

/// Process-wide override from `--max-concurrency` (`0` = use auto).
static OVERRIDE: AtomicUsize = AtomicUsize::new(0);

/// Cached auto budget (computed once per process; one-shot CLI does not rebalance).
static AUTO_BUDGET: OnceLock<usize> = OnceLock::new();

/// Workload class for documentation and call-site comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadClass {
    /// Network / disk wait dominates (Tokio).
    IoBound,
    /// Pure CPU over in-memory or per-file data (Rayon).
    CpuBound,
    /// Mixed stages; isolate CPU with `spawn_blocking` / Rayon off async.
    Mixed,
    /// External process (Chrome); never unbounded spawn loops.
    Subprocess,
}

/// Install CLI override. Call once after clap parse.
///
/// - `0` → auto (CPU × RAM formula)
/// - `N>0` → clamp to `[MIN_CONCURRENCY, HARD_CAP]`
pub fn install_limit(max_concurrency: usize) {
    let v = if max_concurrency == 0 {
        0
    } else {
        max_concurrency.clamp(MIN_CONCURRENCY, HARD_CAP)
    };
    OVERRIDE.store(v, Ordering::Relaxed);
}

/// Effective concurrency for I/O fan-out (and Rayon thread hint).
pub fn effective_limit() -> usize {
    let over = OVERRIDE.load(Ordering::Relaxed);
    if over > 0 {
        return over;
    }
    *AUTO_BUDGET.get_or_init(compute_auto_budget)
}

/// Same as [`effective_limit`] but capped for a specific subsystem.
pub fn effective_limit_capped(cap: usize) -> usize {
    effective_limit().min(cap.max(MIN_CONCURRENCY))
}

/// CPU count used in the formula (`available_parallelism`, min 1).
pub fn cpu_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(MIN_CONCURRENCY)
        .max(MIN_CONCURRENCY)
}

/// Free / available RAM in MiB when the platform exposes it.
///
/// - **Linux:** `MemAvailable` (preferred) or `MemFree` from `/proc/meminfo`.
/// - **macOS:** free + inactive pages via `host_statistics64` (best-effort).
/// - **Windows:** `ullAvailPhys` via `GlobalMemoryStatusEx`.
/// - Other targets: `None` → formula falls back to CPU count only.
pub fn free_ram_mb() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let text = std::fs::read_to_string("/proc/meminfo").ok()?;
        // Prefer MemAvailable (accounts for cache reclaim).
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("MemAvailable:") {
                let kb: u64 = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())?;
                return Some(kb / 1024);
            }
        }
        // Fallback MemFree.
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("MemFree:") {
                let kb: u64 = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())?;
                return Some(kb / 1024);
            }
        }
        None
    }
    #[cfg(target_os = "macos")]
    {
        free_ram_mb_macos()
    }
    #[cfg(windows)]
    {
        free_ram_mb_windows()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn free_ram_mb_macos() -> Option<u64> {
    // host_statistics64(HOST_VM_INFO64): free + inactive pages ≈ reclaimable.
    // SAFETY: zeroed vm_statistics64 is a valid out-buffer; host is host_self.
    unsafe {
        let mut count = libc::HOST_VM_INFO64_COUNT;
        let mut stat: libc::vm_statistics64 = std::mem::zeroed();
        let host = libc::mach_host_self();
        let kr = libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            &mut stat as *mut _ as *mut _,
            &mut count,
        );
        if kr != libc::KERN_SUCCESS {
            return None;
        }
        let page = libc::sysconf(libc::_SC_PAGESIZE);
        if page <= 0 {
            return None;
        }
        let pages = (stat.free_count as u64).saturating_add(stat.inactive_count as u64);
        Some(pages.saturating_mul(page as u64) / (1024 * 1024))
    }
}

#[cfg(windows)]
fn free_ram_mb_windows() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{
        GlobalMemoryStatusEx, MEMORYSTATUSEX,
    };
    // SAFETY: dwLength set before call; structure fully written on success.
    unsafe {
        let mut st: MEMORYSTATUSEX = std::mem::zeroed();
        st.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut st) == 0 {
            return None;
        }
        Some(st.ullAvailPhys / (1024 * 1024))
    }
}

/// Auto budget: `min(cpus, ram_budget, HARD_CAP)`.
///
/// RAM side: `(free_ram_mb * 50%) / RAM_PER_IO_TASK_MB`.
pub fn compute_auto_budget() -> usize {
    let cpus = cpu_count();
    let ram_side = free_ram_mb()
        .map(|mb| {
            let usable = mb.saturating_mul(50) / 100; // 50% safety margin
            let tasks = usable / RAM_PER_IO_TASK_MB.max(1);
            (tasks as usize).max(MIN_CONCURRENCY)
        })
        .unwrap_or(cpus);
    cpus.min(ram_side).min(HARD_CAP).max(MIN_CONCURRENCY)
}

/// `Arc<Semaphore>` gate with [`effective_limit`] permits.
pub fn io_semaphore() -> Arc<Semaphore> {
    Arc::new(Semaphore::new(effective_limit()))
}

/// Semaphore with an explicit permit count (already clamped by caller).
pub fn semaphore_with(permits: usize) -> Arc<Semaphore> {
    Arc::new(Semaphore::new(permits.clamp(MIN_CONCURRENCY, HARD_CAP)))
}

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

/// Below this length, [`map_cpu`] stays sequential (rule: never parallelize when
/// cost ≪ coordination overhead). Measured trade-off for one-shot CLI filters.
pub const CPU_MAP_THRESHOLD: usize = 32;

/// Write bytes on the Tokio blocking pool (docsrs: never pin async workers with
/// `std::fs` for non-trivial payloads).
///
/// # Cancel safety
///
/// `spawn_blocking` work is **not** abortable after start (docsrs). Cancellation
/// must cut admission at the async gate; this helper is for short-lived disk I/O.
pub async fn write_bytes_blocking(
    path: std::path::PathBuf,
    bytes: Vec<u8>,
) -> Result<(), std::io::Error> {
    tokio::task::spawn_blocking(move || {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(&path, bytes)
    })
    .await
    .map_err(|e| std::io::Error::other(format!("write_bytes_blocking join: {e}")))?
}

/// `create_dir_all` on the blocking pool.
pub async fn create_dir_all_blocking(path: std::path::PathBuf) -> Result<(), std::io::Error> {
    tokio::task::spawn_blocking(move || std::fs::create_dir_all(path))
        .await
        .map_err(|e| std::io::Error::other(format!("create_dir_all_blocking join: {e}")))?
}

/// Read a file fully on the blocking pool.
pub async fn read_bytes_blocking(path: std::path::PathBuf) -> Result<Vec<u8>, std::io::Error> {
    tokio::task::spawn_blocking(move || std::fs::read(path))
        .await
        .map_err(|e| std::io::Error::other(format!("read_bytes_blocking join: {e}")))?
}

/// Read a UTF-8 file on the blocking pool (PAR-77: never `fs::read_to_string` on async worker).
pub async fn read_to_string_blocking(path: std::path::PathBuf) -> Result<String, std::io::Error> {
    tokio::task::spawn_blocking(move || std::fs::read_to_string(path))
        .await
        .map_err(|e| std::io::Error::other(format!("read_to_string_blocking join: {e}")))?
}

/// `rename` on the blocking pool (PAR-80: state rotation must not pin async workers).
pub async fn rename_blocking(
    from: std::path::PathBuf,
    to: std::path::PathBuf,
) -> Result<(), std::io::Error> {
    tokio::task::spawn_blocking(move || std::fs::rename(from, to))
        .await
        .map_err(|e| std::io::Error::other(format!("rename_blocking join: {e}")))?
}

/// Sync write helper for **outer CLI dispatch** (no active Tokio worker). Prefer
/// [`write_bytes_blocking`] when inside `async fn` / `block_on_*`.
pub fn write_bytes_sync(path: &std::path::Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, bytes)
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

/// Snapshot of budget for doctor / `--json` diagnostics.
pub fn budget_report() -> serde_json::Value {
    serde_json::json!({
        "effective": effective_limit(),
        "override": OVERRIDE.load(Ordering::Relaxed),
        "auto": compute_auto_budget(),
        "cpus": cpu_count(),
        "free_ram_mb": free_ram_mb(),
        "ram_per_io_task_mb": RAM_PER_IO_TASK_MB,
        "hard_cap": HARD_CAP,
        "cpu_map_threshold": CPU_MAP_THRESHOLD,
        "browser_workers": browser_worker_threads(),
        "formula": "min(cpus, (free_ram_mb*50%)/64, 64); --max-concurrency overrides",
        "workload_default": "I/O-bound (CDP/HTTP) + CPU-bound (sg scan via rayon)",
        "local_available_permits_note": "host-local only; no remote OTel of permits (product law)",
        "commands": command_workload_matrix(),
    })
}

/// Entry helper for [`command_by_command_matrix`] (avoids deep `json!` recursion).
fn cmd_entry(class: &str, gate: Option<&str>, reason: Option<&str>) -> serde_json::Value {
    let mut m = serde_json::Map::new();
    m.insert("class".into(), serde_json::Value::String(class.into()));
    if let Some(g) = gate {
        m.insert("gate".into(), serde_json::Value::String(g.into()));
    }
    if let Some(r) = reason {
        m.insert("reason".into(), serde_json::Value::String(r.into()));
    }
    serde_json::Value::Object(m)
}

/// Per-command map built programmatically (Pass 24; large object not via `json!`).
fn command_by_command_matrix() -> serde_json::Value {
    // (name, class, gate?, reason?) — gates MUST match code (PAR-73 honesty).
    // Nested multi-item subcommands use dotted keys (PAR-76).
    let rows: &[(&str, &str, Option<&str>, Option<&str>)] = &[
        // Top-level families
        (
            "doctor",
            "sequential_justified",
            None,
            Some("cheap path/which probes; cost ≪ Rayon (N-144/PAR-57)"),
        ),
        ("commands", "sequential_justified", None, Some("meta inventory")),
        ("schema", "sequential_justified", None, Some("meta schema emit")),
        ("version", "sequential_justified", None, Some("meta")),
        ("locale", "sequential_justified", None, Some("meta")),
        ("goto", "sequential_justified", None, Some("single interactive act (N-138)")),
        ("view", "mixed", Some("join_bounded multi-ref CDP"), Some("snapshot internal fan-out")),
        ("press", "sequential_justified", None, Some("single DOM act (N-135)")),
        ("click-at", "sequential_justified", None, Some("single coordinate act")),
        ("write", "sequential_justified", None, Some("single fill act")),
        ("keys", "sequential_justified", None, Some("ordered key events")),
        ("type", "sequential_justified", None, Some("ordered chars (N-141)")),
        ("wait", "sequential_justified", None, Some("poll loop single page")),
        ("hover", "sequential_justified", None, Some("single act")),
        ("drag", "sequential_justified", None, Some("ordered pointer path")),
        ("fill-form", "sequential_justified", None, Some("DOM focus order (N-135)")),
        ("select-option", "sequential_justified", None, Some("single act")),
        ("pick", "sequential_justified", None, Some("single act")),
        ("upload", "sequential_justified", None, Some("single act")),
        ("back", "sequential_justified", None, Some("single navigation")),
        ("forward", "sequential_justified", None, Some("single navigation")),
        ("reload", "sequential_justified", None, Some("single navigation")),
        ("eval", "mixed", Some("write_bytes_blocking on --file"), Some("single JS; disk off async")),
        (
            "grab",
            "mixed",
            Some("join_bounded multi-rect + save_screenshot_async"),
            Some("multi-target CDP"),
        ),
        ("print-pdf", "mixed", Some("write_bytes_blocking"), Some("single PDF off async worker")),
        ("monitor", "sequential_justified", None, Some("single baseline hash path")),
        (
            "run",
            "sequential_justified",
            None,
            Some("ordered script (N-134); internal steps may fan-out"),
        ),
        ("exec", "sequential_justified", None, Some("ordered script (N-134)")),
        ("extract", "sequential_justified", None, Some("single target or single LLM call")),
        ("text", "sequential_justified", None, Some("single target")),
        ("scroll", "sequential_justified", None, Some("single act")),
        ("cookie", "sequential_justified", None, Some("CDP cookie ops single session")),
        ("attr", "sequential_justified", None, Some("single target")),
        ("assert", "sequential_justified", None, Some("single check; console filters use filter_cpu")),
        (
            "console",
            "mixed",
            Some("filter_cpu when large"),
            Some("buffer filter threshold; dump write_bytes_blocking"),
        ),
        (
            "net",
            "mixed",
            Some("filter_cpu when large"),
            Some("buffer filter threshold; get path write_bytes_blocking"),
        ),
        (
            "page",
            "sequential_justified",
            None,
            Some("tab ops on single browser; multi-attach at launch"),
        ),
        ("dialog", "sequential_justified", None, Some("single dialog handle")),
        (
            "scrape",
            "mixed",
            Some("spawn_blocking parse (http)"),
            Some("single URL; batch for multi"),
        ),
        (
            "batch-scrape",
            "parallel_io",
            Some("JoinSet+Semaphore http; browser sequential N-129"),
            None,
        ),
        (
            "crawl",
            "parallel_io",
            Some("JoinSet+Semaphore http; browser sequential N-129"),
            None,
        ),
        ("map", "parallel_io", Some("crawl_http under budget"), None),
        ("search", "parallel_io", Some("scrape/map under budget"), None),
        ("parse", "mixed", Some("CPU parse sync path"), Some("single file")),
        ("qr", "sequential_justified", None, Some("single payload")),
        (
            "find-paths",
            "parallel_cpu",
            Some("WalkBuilder.threads + multi-root flat_map (no Mutex)"),
            None,
        ),
        (
            "sg-scan",
            "parallel_cpu",
            Some("multi-root par + par_iter files + sort_cpu"),
            None,
        ),
        ("sg-rewrite", "mixed", Some("dry-run par+sort_cpu; --apply sequential N-136"), None),
        ("sheet-write", "sequential_justified", None, Some("single writer N-137")),
        (
            "mitm",
            "mixed",
            Some("CA read_to_string_blocking; map_cpu+sort_cpu list filters"),
            None,
        ),
        ("workflow", "sequential_justified", None, Some("SQLite single-writer N-130")),
        ("config", "sequential_justified", None, Some("single config file")),
        ("emulate", "sequential_justified", None, Some("single CDP device")),
        ("resize", "sequential_justified", None, Some("single viewport")),
        (
            "perf",
            "mixed",
            Some("write_bytes_blocking stop; map_cpu insight when large"),
            None,
        ),
        ("lighthouse", "sequential_justified", None, Some("single subprocess N-140")),
        (
            "screencast",
            "mixed",
            Some("spawn_blocking+rayon frames on stop"),
            None,
        ),
        (
            "heap",
            "mixed",
            Some("node parse par; idom seq N-142; map_cpu/sort_cpu; write_bytes_blocking"),
            None,
        ),
        (
            "extension",
            "mixed",
            Some("join_bounded multi-closeTarget"),
            Some("single load; multi-target unload fan-out"),
        ),
        ("devtools3p", "sequential_justified", None, Some("single bridge")),
        ("webmcp", "sequential_justified", None, Some("single tool call")),
        ("completions", "sequential_justified", None, Some("meta emit")),
        ("man", "sequential_justified", None, Some("meta emit")),
        ("install", "sequential_justified", None, Some("few version dirs")),
        (
            "state",
            "mixed",
            Some("write/read_bytes_blocking; multi-origin sequential N-143"),
            None,
        ),
        ("cache", "sequential_justified", None, Some("single key ops")),
        (
            "residual",
            "mixed",
            Some("index_proc_cmdlines once + map_cpu check/wipe"),
            Some("PAR-89/90: never N×/proc under Rayon"),
        ),
        // Nested multi-item / disk subcommands (PAR-76)
        (
            "console.list",
            "mixed",
            Some("filter_cpu when large"),
            Some("type/sw filter on capture buffer"),
        ),
        (
            "console.dump",
            "mixed",
            Some("write_bytes_blocking"),
            Some("serialize+disk off async/block_on worker"),
        ),
        (
            "net.list",
            "mixed",
            Some("filter_cpu when large"),
            Some("resource_type filter on capture buffer"),
        ),
        (
            "net.get",
            "mixed",
            Some("write_bytes_blocking on --path"),
            Some("optional request/response path dumps"),
        ),
        (
            "heap.dup-strings",
            "parallel_cpu",
            Some("map_cpu"),
            Some("independent string score after idom"),
        ),
        (
            "heap.summary",
            "mixed",
            Some("map_cpu when large"),
            Some("offline parse; graph passes sequential"),
        ),
        (
            "heap.take",
            "mixed",
            Some("write_bytes_blocking"),
            Some("CDP chunks join then disk off async"),
        ),
        (
            "mitm.domains",
            "parallel_cpu",
            Some("map_cpu"),
            Some("host extract over capture items"),
        ),
        (
            "mitm.apis",
            "parallel_cpu",
            Some("map_cpu"),
            Some("API classify over capture items"),
        ),
        (
            "assert.console",
            "mixed",
            Some("filter_cpu when large"),
            Some("level filter on console buffer"),
        ),
        (
            "assert.console-empty",
            "sequential_justified",
            None,
            Some("count check; cost ≪ overhead"),
        ),
        (
            "assert.console-no-match",
            "mixed",
            Some("filter_cpu when large"),
            Some("pattern filter on console buffer"),
        ),
        (
            "state.save",
            "mixed",
            Some("write_bytes_blocking + create_dir_all_blocking"),
            Some("CDP collect then disk off async"),
        ),
        (
            "state.load",
            "mixed",
            Some("read_bytes_blocking; multi-origin sequential N-143"),
            Some("disk off async; navigates sequential"),
        ),
        (
            "state.list",
            "sequential_justified",
            None,
            Some("few session files; cost ≪ Rayon"),
        ),
        (
            "perf.stop",
            "mixed",
            Some("write_bytes_blocking"),
            Some("trace dump off async worker"),
        ),
        (
            "perf.insight",
            "mixed",
            Some("map_cpu when large"),
            Some("offline event fold with threshold"),
        ),
        (
            "screencast.stop",
            "mixed",
            Some("spawn_blocking+rayon frames"),
            Some("decode+write N frames"),
        ),
    ];
    let mut map = serde_json::Map::new();
    for (name, class, gate, reason) in rows {
        map.insert((*name).into(), cmd_entry(class, *gate, *reason));
    }
    serde_json::Value::Object(map)
}

/// Per-command parallelism posture (agent discovery / doctor).
///
/// Every multi-item command either fans out under [`effective_limit`] or has an
/// explicit sequential justification (single CDP session, single-writer journal,
/// atomic rewrite apply, or cost ≪ coordination overhead).
///
/// Pass 24–25: `by_command` maps each top-level CLI command **and** multi-item
/// nested subcommands so agents never treat sequential single-act paths as
/// forgotten parallelism. Gates must match code (PAR-73 honesty).
pub fn command_workload_matrix() -> serde_json::Value {
    let mut root = serde_json::Map::new();
    root.insert(
        "parallel_io".into(),
        serde_json::json!([
            "batch-scrape --engine http (JoinSet+Semaphore; parse via spawn_blocking)",
            "crawl --engine http (bounded frontier; discovery under same permit)",
            "map / search (HTTP; crawl/scrape under budget)",
            "view/snapshot multi-ref CDP resolve (join_bounded+Semaphore)",
            "grab multi-target rect resolve (join_bounded+Semaphore)",
            "find-paths (WalkBuilder.threads=walk_threads; multi-root par)",
            "robots shared client keep-alive under batch",
            "network sanitize multi-page (join_bounded CDP navigate)",
            "browser multi-target attach (join_bounded_ordered)",
            "cdp page forwarders multi-page (join_bounded)",
            "screencast stop frames (spawn_blocking+rayon decode/write)"
        ]),
    );
    root.insert(
        "parallel_cpu".into(),
        serde_json::json!([
            "sg-scan (rayon par_iter; multi-root collect par)",
            "sg-rewrite dry-run (rayon par_iter); --apply sequential",
            "heap score/filter independent passes (map_cpu); idom/RPO sequential",
            "mitm domains/apis filter when items >= CPU_MAP_THRESHOLD",
            "console/net list filter_cpu when buffer >= CPU_MAP_THRESHOLD",
            "residual multi-candidate scavenge when >= threshold",
            "perf_insight top-level events map_cpu when large"
        ]),
    );
    root.insert(
        "sequential_justified".into(),
        serde_json::json!({
            "batch-scrape --engine browser": "single residual Chrome / one Page (N-129); use --engine http for fan-out",
            "crawl --engine browser": "single CDP Page session (N-129)",
            "run / exec multi-step": "ordered script semantics + fail-fast (N-134)",
            "workflow run/resume": "SQLite journal single-writer (N-130); fan-out inside steps",
            "fill-form": "DOM focus/state must be sequential on one Page (N-135)",
            "sg-rewrite --apply": "atomic writers must not race the same tree (N-136)",
            "sheet-write": "single workbook writer (rust_xlsxwriter not Sync) (N-137)",
            "qr encode/decode": "single payload; multi-grid decode rare and cheap",
            "goto/press/type/click/keys/…": "single interactive act (N-138); cost ≪ spawn",
            "doctor/commands/schema/version/locale": "meta; doctor probes cheap — sequential (N-144); cost ≪ Rayon",
            "lighthouse": "single external subprocess (N-140)",
            "mitm start/capture": "one proxy task JoinHandle awaited; not multi-URL fan-out",
            "llm local": "single request; OnceLock HTTP client (N-139)",
            "residual FINALIZE": "map_cpu when candidates large; else cost ≪ overhead",
            "install chrome discovery": "few version dirs; sequential OK (N-cost)",
            "state list/clear/clean": "few session files; cost ≪ Rayon (PAR-72)",
            "state load multi-origin": "single CDP session navigate — sequential (N-143)",
            "cache get/put": "single key; Mutex short critical section no .await",
            "parse spreadsheet multi-sheet": "calamine Reader not Sync — sequential (PAR-59)",
            "type char-a-char": "ordered input semantics (N-141)",
            "snapshot tree build": "parent/child links ordered — sequential (N-145)"
        }),
    );
    root.insert("by_command".into(), command_by_command_matrix());
    root.insert(
        "bound_everywhere".into(),
        serde_json::Value::String(
            "JoinSet+Arc<Semaphore>::acquire_owned | join_bounded | WalkBuilder.threads(walk_threads) | rayon pool sized to budget | map_cpu/filter_cpu/sort_cpu threshold".into(),
        ),
    );
    root.insert(
        "cancel".into(),
        serde_json::Value::String(
            "Lifecycle CancellationToken checked mid batch/crawl acquire".into(),
        ),
    );
    root.insert(
        "helpers".into(),
        serde_json::json!([
            "write_bytes_blocking",
            "write_bytes_sync",
            "create_dir_all_blocking",
            "read_bytes_blocking",
            "read_to_string_blocking",
            "rename_blocking",
            "map_cpu",
            "filter_cpu",
            "sort_cpu",
            "sort_by_cpu",
            "sort_by_key_cpu",
            "join_bounded"
        ]),
    );
    root.insert(
        "na_product_law".into(),
        serde_json::json!([
            "multi-process Chrome pool (N-129/N-154)",
            "workflow multi-writer SQLite (N-130/N-155)",
            "systemd-run MemoryMax default (N-121)",
            "remote OTel available_permits (N-124)",
            "loom GHA (N-122)",
            "state multi-origin parallel same session (N-143)",
            "heap idom/RPO blind par_iter (N-142/N-152)",
            "doctor cheap probes Rayon (N-144/N-156)",
            "snapshot tree build blind par (N-145/N-153)",
            "DOM single-act parallel (N-151)",
            "mitm block/allow rules sync CLI (N-158)"
        ]),
    );
    serde_json::Value::Object(root)
}

/// Resolve permits for a fan-out: `0` → process budget; else clamp to hard cap.
pub fn resolve_permits(requested: usize) -> usize {
    if requested == 0 {
        effective_limit()
    } else {
        requested.clamp(MIN_CONCURRENCY, HARD_CAP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomOrd};
    use std::sync::Arc;

    #[test]
    fn auto_budget_is_bounded() {
        let b = compute_auto_budget();
        assert!(b >= MIN_CONCURRENCY);
        assert!(b <= HARD_CAP);
    }

    #[test]
    fn install_override_clamps() {
        install_limit(0);
        assert_eq!(OVERRIDE.load(Ordering::Relaxed), 0);
        install_limit(1);
        assert_eq!(effective_limit(), 1);
        install_limit(9999);
        assert_eq!(effective_limit(), HARD_CAP);
        // Reset for other tests in the same process.
        install_limit(0);
    }

    #[test]
    fn browser_workers_at_least_two() {
        install_limit(1);
        assert!(browser_worker_threads() >= 2);
        install_limit(0);
    }

    #[tokio::test]
    async fn join_bounded_respects_peak() {
        let peak = Arc::new(AtomicUsize::new(0));
        let current = Arc::new(AtomicUsize::new(0));
        let limit = 3usize;
        let mut futs = Vec::new();
        for _ in 0..12 {
            let peak = Arc::clone(&peak);
            let current = Arc::clone(&current);
            futs.push(async move {
                let n = current.fetch_add(1, AtomOrd::SeqCst) + 1;
                peak.fetch_max(n, AtomOrd::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                current.fetch_sub(1, AtomOrd::SeqCst);
                1u32
            });
        }
        let out = join_bounded(futs, limit).await;
        assert_eq!(out.len(), 12);
        assert!(
            peak.load(AtomOrd::SeqCst) <= limit,
            "peak {} exceeded limit {}",
            peak.load(AtomOrd::SeqCst),
            limit
        );
    }

    #[tokio::test]
    async fn join_bounded_ordered_preserves_order() {
        let futs: Vec<_> = (0..8u32)
            .map(|i| async move {
                tokio::time::sleep(std::time::Duration::from_millis((8 - i) as u64)).await;
                i
            })
            .collect();
        let out = join_bounded_ordered(futs, 4).await;
        assert_eq!(out, (0..8).collect::<Vec<_>>());
    }

    #[test]
    fn semaphore_has_effective_permits() {
        install_limit(4);
        let s = io_semaphore();
        assert_eq!(s.available_permits(), 4);
        install_limit(0);
    }

    #[test]
    fn resolve_permits_zero_is_effective() {
        install_limit(3);
        assert_eq!(resolve_permits(0), 3);
        assert_eq!(resolve_permits(2), 2);
        assert_eq!(resolve_permits(9999), HARD_CAP);
        install_limit(0);
    }

    #[test]
    fn command_matrix_lists_parallel_and_sequential() {
        let m = command_workload_matrix();
        assert!(m.get("parallel_io").and_then(|v| v.as_array()).is_some());
        assert!(m
            .get("sequential_justified")
            .and_then(|v| v.as_object())
            .is_some());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn free_ram_linux_reads_meminfo() {
        // On a normal Linux host MemAvailable is present; CI containers too.
        let mb = free_ram_mb();
        assert!(mb.is_some(), "expected MemAvailable/MemFree on Linux");
        assert!(mb.unwrap() > 0);
    }

    #[tokio::test]
    async fn semaphore_permit_returns_after_panic_in_joinset() {
        // Rule checklist: panic in task must not permanently leak permits.
        let limit = 2usize;
        let sem = Arc::new(Semaphore::new(limit));
        let mut set: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();
        for i in 0..4 {
            let permit = Arc::clone(&sem)
                .acquire_owned()
                .await
                .expect("sem open");
            set.spawn(async move {
                let _permit = permit;
                if i == 1 {
                    panic!("intentional concurrency panic");
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            });
        }
        let mut panics = 0u32;
        while let Some(joined) = set.join_next().await {
            if let Err(e) = joined {
                if e.is_panic() {
                    panics += 1;
                }
            }
        }
        assert_eq!(panics, 1);
        assert_eq!(
            sem.available_permits(),
            limit,
            "all permits must return after JoinSet drain (incl. panic tasks)"
        );
    }

    #[test]
    fn walk_threads_never_exceeds_hard_cap_or_cpus() {
        // Pure bound: walk threads ≤ min(HARD_CAP, cpus) regardless of override races.
        let w = walk_threads();
        assert!(w >= MIN_CONCURRENCY);
        assert!(w <= HARD_CAP);
        assert!(w <= cpu_count().max(MIN_CONCURRENCY));
    }

    #[test]
    fn command_matrix_has_na_and_cancel() {
        let m = command_workload_matrix();
        assert!(m.get("na_product_law").and_then(|v| v.as_array()).is_some());
        assert!(m.get("cancel").and_then(|v| v.as_str()).is_some());
        let seq = m
            .get("sequential_justified")
            .and_then(|v| v.as_object())
            .expect("seq");
        assert!(seq.contains_key("lighthouse"));
        assert!(seq.contains_key("mitm start/capture"));
    }

    #[test]
    fn map_cpu_sequential_below_threshold() {
        let items: Vec<u32> = (0..10).collect();
        let out = map_cpu(&items, |x| x * 2);
        assert_eq!(out, (0..10).map(|x| x * 2).collect::<Vec<_>>());
    }

    #[test]
    fn map_cpu_parallel_above_threshold() {
        let items: Vec<u32> = (0..(CPU_MAP_THRESHOLD as u32 + 8)).collect();
        let out = map_cpu(&items, |x| x.saturating_add(1));
        assert_eq!(out.len(), items.len());
        assert_eq!(out[0], 1);
        assert_eq!(out[items.len() - 1], items[items.len() - 1] + 1);
    }

    #[test]
    fn sort_cpu_orders_small_and_large() {
        // PAR-99: threshold path + parallel path both produce sorted output.
        let mut small = vec![3, 1, 2];
        sort_cpu(&mut small);
        assert_eq!(small, vec![1, 2, 3]);
        let mut large: Vec<u32> = (0..(CPU_MAP_THRESHOLD as u32 + 16)).rev().collect();
        sort_cpu(&mut large);
        assert!(large.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(large.first().copied(), Some(0));
    }

    #[test]
    fn sort_by_key_cpu_reverse_counts() {
        let mut items = vec![("a", 2u64), ("b", 9u64), ("c", 1u64)];
        sort_by_key_cpu(&mut items, |b| std::cmp::Reverse(b.1));
        assert_eq!(items[0].1, 9);
        assert_eq!(items[2].1, 1);
    }

    #[test]
    fn matrix_residual_mentions_index_proc() {
        let m = command_workload_matrix();
        let by = m.get("by_command").and_then(|v| v.as_object()).expect("by");
        let residual = by.get("residual").expect("residual");
        let gate = residual.get("gate").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            gate.contains("index_proc") || gate.contains("map_cpu"),
            "residual gate must document index-once scavenge: {gate}"
        );
        let helpers = m
            .get("helpers")
            .and_then(|v| v.as_array())
            .expect("helpers");
        let has_sort = helpers.iter().any(|h| h.as_str() == Some("sort_cpu"));
        assert!(has_sort, "helpers must list sort_cpu");
    }

    #[test]
    fn by_command_covers_inventory_minimum() {
        let m = command_workload_matrix();
        let by = m
            .get("by_command")
            .and_then(|v| v.as_object())
            .expect("by_command");
        for key in [
            "doctor",
            "goto",
            "view",
            "batch-scrape",
            "crawl",
            "find-paths",
            "sg-scan",
            "screencast",
            "heap",
            "workflow",
            "run",
            "mitm",
            "state",
            "residual",
            "lighthouse",
            "grab",
            "map",
            "search",
            "console",
            "net",
            // Pass 25 nested multi-item
            "console.list",
            "console.dump",
            "net.list",
            "net.get",
            "heap.dup-strings",
            "mitm.domains",
            "state.load",
            "perf.insight",
            "screencast.stop",
        ] {
            assert!(by.contains_key(key), "missing by_command entry: {key}");
        }
        assert!(m.get("helpers").and_then(|v| v.as_array()).is_some());
    }

    #[test]
    fn matrix_honesty_doctor_not_fake_map_cpu() {
        // PAR-73: doctor must not claim map_cpu when probes are sequential.
        let m = command_workload_matrix();
        let by = m
            .get("by_command")
            .and_then(|v| v.as_object())
            .expect("by_command");
        let doctor = by.get("doctor").and_then(|v| v.as_object()).expect("doctor");
        assert_eq!(
            doctor.get("class").and_then(|v| v.as_str()),
            Some("sequential_justified")
        );
        assert!(
            doctor.get("gate").is_none(),
            "doctor must not claim a parallel gate"
        );
        let helpers = m
            .get("helpers")
            .and_then(|v| v.as_array())
            .expect("helpers");
        let helper_names: Vec<&str> = helpers.iter().filter_map(|v| v.as_str()).collect();
        assert!(helper_names.contains(&"filter_cpu"));
        assert!(helper_names.contains(&"read_to_string_blocking"));
        assert!(helper_names.contains(&"rename_blocking"));
    }

    #[test]
    fn filter_cpu_sequential_below_threshold() {
        let items: Vec<u32> = (0..10).collect();
        let out = filter_cpu(items, |x| x % 2 == 0);
        assert_eq!(out, vec![0, 2, 4, 6, 8]);
    }

    #[test]
    fn filter_cpu_parallel_above_threshold() {
        let items: Vec<u32> = (0..(CPU_MAP_THRESHOLD as u32 + 16)).collect();
        let out = filter_cpu(items.clone(), |x| x % 2 == 0);
        assert_eq!(out.len(), items.len() / 2);
        assert_eq!(out[0], 0);
    }

    #[tokio::test]
    async fn write_bytes_blocking_roundtrip() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("par24.bin");
        write_bytes_blocking(path.clone(), b"pass24".to_vec())
            .await
            .expect("write");
        let got = read_bytes_blocking(path).await.expect("read");
        assert_eq!(got, b"pass24");
    }

    #[tokio::test]
    async fn read_to_string_and_rename_blocking_roundtrip() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("a.txt");
        let path2 = dir.path().join("b.txt");
        write_bytes_blocking(path.clone(), b"pass25".to_vec())
            .await
            .expect("write");
        let s = read_to_string_blocking(path.clone()).await.expect("read str");
        assert_eq!(s, "pass25");
        rename_blocking(path, path2.clone()).await.expect("rename");
        let s2 = read_to_string_blocking(path2).await.expect("read2");
        assert_eq!(s2, "pass25");
    }
}
