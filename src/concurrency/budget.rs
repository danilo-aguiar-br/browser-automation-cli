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

/// Fan-out ceiling for a batch of CDP round trips on one connection.
///
/// Five call sites spelled the literal `32` — snapshot cursor, snapshot URL
/// resolution, screenshot annotation, target enumeration and the network domain
/// filter. They are the same ceiling for the same reason (one WebSocket, one
/// browser), so a change had to be made in five places or in none, and nothing
/// said they belonged together.
pub const CDP_FANOUT_CAP: usize = 32;

/// Fan-out ceiling for CDP round trips that ATTACH to a target.
///
/// Deliberately lower than [`CDP_FANOUT_CAP`], and the difference is the point:
/// the sites above issue one message on an existing session, while these open a
/// session per target, so the browser-side cost is not comparable and one
/// ceiling for both would be wrong in whichever direction it moved.
///
/// It was the bare literal `8` at five sites — the four attach paths in
/// `cdp::client::page_attach` and the extension enumerator. Naming it separates
/// "these five share a ceiling" from "this ceiling equals the other one", which
/// a reader had no way to tell apart while both were unnamed numbers.
pub const CDP_ATTACH_FANOUT_CAP: usize = 8;

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

// `libc` deprecated its Mach bindings in 0.2.55 pointing at the `mach2` crate,
// but `mach2` exposes no `host_statistics64`, so the call below would still
// need `libc` anyway. Bind the single deprecated symbol here instead of taking
// a second FFI crate that cannot replace it; both resolve from libSystem,
// which is already linked on macOS.
#[cfg(target_os = "macos")]
extern "C" {
    fn mach_host_self() -> libc::mach_port_t;
}

#[cfg(target_os = "macos")]
fn free_ram_mb_macos() -> Option<u64> {
    // host_statistics64(HOST_VM_INFO64): free + inactive pages ≈ reclaimable.
    let mut count = libc::HOST_VM_INFO64_COUNT;
    // SAFETY: `vm_statistics64` is a plain C struct of integers, so an all-zero
    // bit pattern is a valid initial value for an out-buffer.
    let mut stat: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
    // SAFETY: `mach_host_self` takes no argument and returns the host name
    // port; it has no failure mode.
    let host = unsafe { mach_host_self() };
    // SAFETY: `host` is the host name port, `stat` is a live out-buffer, and
    // `count` states its length in `integer_t` units as the flavor requires.
    let kr = unsafe {
        libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            &mut stat as *mut _ as *mut _,
            &mut count,
        )
    };
    if kr != libc::KERN_SUCCESS {
        return None;
    }
    // SAFETY: `sysconf` reads a static system limit and reports failure as -1.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page <= 0 {
        return None;
    }
    let pages = (stat.free_count as u64).saturating_add(stat.inactive_count as u64);
    Some(pages.saturating_mul(page as u64) / (1024 * 1024))
}

#[cfg(windows)]
fn free_ram_mb_windows() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: `MEMORYSTATUSEX` is a plain C struct, so an all-zero bit pattern
    // is a valid initial value; `dwLength` is set on the next line.
    let mut st: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    st.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    // SAFETY: `st` is a live out-buffer whose `dwLength` is set, which is the
    // contract the call requires; it reports failure as 0 and writes the
    // structure fully on success.
    if unsafe { GlobalMemoryStatusEx(&mut st) } == 0 {
        return None;
    }
    Some(st.ullAvailPhys / (1024 * 1024))
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

/// Semaphore with an explicit permit count, clamped to the process bounds.
///
/// `io_semaphore()`, which wrapped `semaphore_with(effective_limit())` and had
/// no production caller, was removed on 2026-09-01. Its only reader was a unit
/// test, which is the exact shape the phantom-flag gate exists to reject.
pub fn semaphore_with(permits: usize) -> Arc<Semaphore> {
    Arc::new(Semaphore::new(permits.clamp(MIN_CONCURRENCY, HARD_CAP)))
}
