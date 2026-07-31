// SPDX-License-Identifier: MIT OR Apache-2.0
//! Concurrency budget, free-RAM probe, and I/O semaphore.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
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
///
/// # Concurrency
///
/// `AtomicUsize` with a single stable address (`static`, not `const` — interior
/// mutability must not be inlined per call site). Uses `Ordering::Relaxed`
/// because the value is a pure capacity hint: no other memory is published or
/// synchronized through this atomic (install once after clap parse; one-shot
/// readers do not form a release/acquire data protocol).
pub(crate) static OVERRIDE: AtomicUsize = AtomicUsize::new(0);

/// Cached auto budget (computed once per process; one-shot CLI does not rebalance).
///
/// # Concurrency
///
/// `OnceLock` (not `LazyLock`): init depends on runtime free-RAM / CPU probes
/// that may fail and must not re-run every access. After first compute the
/// value is immutable for the process lifetime.
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
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
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
    cpus.min(ram_side).clamp(MIN_CONCURRENCY, HARD_CAP)
}

/// `Arc<Semaphore>` gate with [`effective_limit`] permits.
pub fn io_semaphore() -> Arc<Semaphore> {
    Arc::new(Semaphore::new(effective_limit()))
}

/// Semaphore with an explicit permit count (already clamped by caller).
pub fn semaphore_with(permits: usize) -> Arc<Semaphore> {
    Arc::new(Semaphore::new(permits.clamp(MIN_CONCURRENCY, HARD_CAP)))
}
