// SPDX-License-Identifier: MIT OR Apache-2.0
//! Live-process index for residual wipe safety (PAR-89 / GAP-045 / GAP-052).
//!
//! # Fail-closed contract (GAP-045)
//!
//! The index is [`Option`]-shaped on purpose:
//!
//! | Value | Meaning | Wipe policy |
//! |-------|---------|-------------|
//! | `None` | host process table **could not be enumerated** | **refuse the wipe** |
//! | `Some(idx)` with `idx.is_empty()` | enumeration succeeded, no live process | wipe allowed |
//! | `Some(idx)` non-empty | enumeration succeeded | wipe allowed when no live holder |
//!
//! Absence of information is never evidence of absence of a holder. The previous
//! design read `/proc` directly and returned an empty `Vec` on any read failure,
//! so on hosts without `/proc` (macOS, Windows) every candidate looked orphaned
//! and the collector deleted the profile of a **live sibling invocation**.
//!
//! Enumeration now goes through `sysinfo`, which covers Linux, macOS and Windows
//! with one code path.

use std::collections::HashSet;
use std::path::Path;

use super::classify::cmdline_holds_path;
use super::owner::read_owner_pid;

/// Snapshot of the live processes on this host.
///
/// **PAR-89:** build **once** per scavenge, then pass by reference into
/// `map_cpu` tasks. Never rebuild inside a parallel task.
#[derive(Debug, Default, Clone)]
pub struct LiveProcessIndex {
    pids: HashSet<u32>,
    cmdlines: Vec<String>,
}

impl LiveProcessIndex {
    /// True when `pid` is a live process in this snapshot.
    #[must_use]
    pub fn contains_pid(&self, pid: u32) -> bool {
        self.pids.contains(&pid)
    }

    /// Command lines observed in this snapshot.
    #[must_use]
    pub fn cmdlines(&self) -> &[String] {
        &self.cmdlines
    }

    /// Number of live processes in this snapshot.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pids.len()
    }

    /// True when enumeration succeeded but observed no process.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pids.is_empty()
    }

    /// Build an index from explicit parts (tests and alternative backends).
    #[must_use]
    pub fn from_parts(pids: HashSet<u32>, cmdlines: Vec<String>) -> Self {
        Self { pids, cmdlines }
    }
}

/// Snapshot the live process table, or `None` when the host cannot be enumerated.
///
/// Callers **must** treat `None` as "unknown" and refuse any destructive action
/// (see `super::wipe::wipe_safe_candidates`).
#[must_use]
pub fn index_live_processes() -> Option<LiveProcessIndex> {
    backend::collect()
}

/// Legacy cmdline-only view for callers that only need command lines.
///
/// Returns an empty slice when the process table is unavailable; use
/// [`index_live_processes`] whenever the distinction matters.
#[must_use]
pub fn index_proc_cmdlines() -> Vec<String> {
    index_live_processes()
        .map(|idx| idx.cmdlines)
        .unwrap_or_default()
}

/// True when some live process holds `path`.
///
/// Both signals are consulted; either one proves a live holder:
///
/// 1. **Owner PID marker (GAP-052).** A live owner pid is conclusive proof that
///    the profile is in use, with no substring matching involved. A dead owner is
///    *not* conclusive on its own — the CLI parent can die while an orphaned
///    Chrome keeps writing to the profile — so it falls through to (2).
/// 2. **Browser-shaped cmdline.** Required for paths with no marker (Chromium
///    side-channels, pre-marker leftovers) and as the orphaned-browser backstop.
///    The browser shape requirement is what stops an editor or an `rg` invocation
///    that merely mentions the path from pinning it forever.
pub(crate) fn path_has_live_process(path: &Path, index: &LiveProcessIndex) -> bool {
    if read_owner_pid(path).is_some_and(|pid| index.contains_pid(pid)) {
        return true;
    }
    let needle = path.display().to_string();
    if needle.is_empty() {
        return false;
    }
    index
        .cmdlines
        .iter()
        .any(|cmd| cmdline_holds_path(cmd, &needle))
}

mod backend {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

    use super::LiveProcessIndex;

    /// Enumerate the host process table on Linux, macOS and Windows alike.
    ///
    /// Only process command lines are refreshed; CPU, memory, disk and user data
    /// are left out so the snapshot stays cheap.
    ///
    /// Returns `None` when enumeration cannot be trusted. The self-test is that
    /// **this** process must appear in its own snapshot: a live host always has at
    /// least one process, so a table that omits us is a failure, not an empty
    /// host. `None` makes every caller fail closed (GAP-045).
    pub(super) fn collect() -> Option<LiveProcessIndex> {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_processes(ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always)),
        );

        let mut index = LiveProcessIndex::default();
        for (pid, process) in system.processes() {
            index.pids.insert(pid.as_u32());
            if process.cmd().is_empty() {
                continue;
            }
            // Space-joined, not NUL-joined: the browser/text-tool heuristics in
            // `classify` match on `" --type="` and `"rg "`, which only hold for a
            // space-separated rendering of argv.
            let cmd = process
                .cmd()
                .iter()
                .map(|part| part.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");
            index.cmdlines.push(cmd);
        }

        if !index.pids.contains(&std::process::id()) {
            return None;
        }
        Some(index)
    }
}
