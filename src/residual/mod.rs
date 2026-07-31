// SPDX-License-Identifier: MIT OR Apache-2.0
//! Owned residual path discovery for one-shot Chrome (GAP-017 / GAP-020 / RES-01…12).
//!
//! Never wipes host Chromes (Flatpak/VS Code). Only paths that are:
//! - under our marker prefix `browser-automation-cli-chrome-*`, or
//! - Singleton files inside our profile, or
//! - `/tmp/org.chromium.Chromium.*` / `/tmp/.org.chromium.Chromium.*` that are
//!   Singleton-only (or empty), owned by this uid, with no live process holding
//!   the path — either from this invocation window or cross-run stale GC.
//!
//! Host Flatpak tmp noise (`.com.google.Chrome.*` / `com.google.Chrome.*`) is
//! **never** deleted by cross-run GC (product law).
//!
//! # Workload
//!
//! **I/O-bound** temp/`/proc` scan during BORN and FINALIZE. Discovery of temp
//! entries stays sequential (directory iterator).
//!
//! **PAR-89:** live-process checks use a **single** `/proc` cmdline index
//! (`index_proc_cmdlines`) then [`crate::concurrency::map_cpu`] over candidates
//! (threshold 32). Never rescan `/proc` inside each parallel task (that would
//! be O(N×P) thrashing under Rayon).
//!
//! **PAR-90:** independent wipe of disjoint paths uses `map_cpu` when large.
//! Subprocess Chrome itself is never unbounded-forked (one launch per one-shot
//! process). The brief `std::thread::sleep` settle in browser residual path is
//! on the **sync** FINALIZE path (not a Tokio worker).
//!
//! # Modules
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`constants`] | Named GC policy constants |
//! | `roots` | Dual-root scan (temp + XDG chrome-profiles) |
//! | `classify` | Name/shape/ownership classifiers |
//! | `discover` | Invocation + marker discovery |
//! | `scavenge` | BORN/FINALIZE scavengers |
//! | `wipe` | Safe wipe with map_cpu (fail-closed, GAP-045) |
//! | `proc` | Live-process index (PAR-89 / GAP-045) |
//! | `owner` | Owner-pid marker for CLI profiles (GAP-052) |
//! | `report` | `ResidualDiskReport` |

mod classify;
mod constants;
mod discover;
mod owner;
mod proc;
mod report;
mod roots;
mod scavenge;
mod wipe;

#[cfg(test)]
mod tests;

pub use constants::{
    CHROMIUM_TMP_DOT_PREFIX, CHROMIUM_TMP_PREFIX, CLI_CHROME_MARKER_PREFIX,
    INVOCATION_SIDE_CHANNEL_WINDOW_SECS, MTIME_SKEW_SECS, SINGLETON_MAX_BYTES,
    SINGLETON_MAX_ENTRIES, STALE_MIN_AGE_SECS,
};
pub use discover::{
    discover_owned_chromium_tmp_side_channels, list_cli_chrome_marker_dirs,
    list_cli_chrome_marker_dirs_in_roots,
};
pub use owner::{has_owner_pid, owner_pid_path, read_owner_pid, write_owner_pid, OWNER_PID_FILE};
pub use proc::{index_live_processes, index_proc_cmdlines, LiveProcessIndex};
pub use report::{residual_disk_report, ResidualDiskReport};
pub use scavenge::{
    scavenge_owned_chromium_tmp_orphans, scavenge_stale_singleton_orphans,
    scavenge_stale_singleton_orphans_in_roots, scavenge_stale_singleton_orphans_with_min_age,
};
