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
use std::path::{Path, PathBuf};

use super::classify::entry_holds_path_protective;
use super::owner::read_owner_pid;

/// One live process in the snapshot.
///
/// # Why `exe` is carried next to `cmdline`
///
/// argv is written by the process itself and can say anything. `sysinfo`'s own
/// documentation for `Process::exe` puts it plainly: a process "may change its
/// `cmd[0]` value freely, making this an untrustworthy source of information".
/// The executable path comes from the kernel instead, so it is the only field
/// here that a process cannot forge about itself.
///
/// That distinction is not academic. Classifying browsers by argv substring let
/// a shell script holding `--user-data-dir=<marker> --type=renderer` be counted
/// as a live Chrome, which failed `doctor` on a clean host and — worse — put an
/// unrelated pid in front of the reaper.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ProcessEntry {
    /// OS process id.
    pub pid: u32,
    /// Parent process id, when the host reports one.
    pub ppid: Option<u32>,
    /// Space-joined argv, empty when the host hid it.
    pub cmdline: String,
    /// Kernel-reported executable path, `None` when it could not be read.
    ///
    /// `None` means "unknown", never "not a browser": the two polarities in
    /// the classifiers treat that ignorance in opposite directions on purpose.
    pub exe: Option<PathBuf>,
}

impl ProcessEntry {
    /// A process observed without a trustworthy executable path.
    ///
    /// `#[non_exhaustive]` makes this the only way to build one from outside the
    /// crate, which is deliberate: adding `exe` as a bare public field would have
    /// broken every fixture silently on the next field after it.
    #[must_use]
    pub fn new(pid: u32, ppid: Option<u32>, cmdline: impl Into<String>) -> Self {
        Self {
            pid,
            ppid,
            cmdline: cmdline.into(),
            exe: None,
        }
    }

    /// Attach the kernel-reported executable path.
    #[must_use]
    pub fn with_exe(mut self, exe: impl Into<PathBuf>) -> Self {
        self.exe = Some(exe.into());
        self
    }

    /// Executable path as `&str`, when it is present and valid UTF-8.
    #[must_use]
    pub fn exe_str(&self) -> Option<&str> {
        self.exe.as_deref().and_then(Path::to_str)
    }
}

/// Snapshot of the live processes on this host.
///
/// **PAR-89:** build **once** per scavenge, then pass by reference into
/// `map_cpu` tasks. Never rebuild inside a parallel task.
///
/// # Why entries are keyed by pid
///
/// The previous shape stored a bare `Vec<String>` of command lines. Counting
/// that vector answers "how many argv strings mention a marker", which is not
/// the same question as "how many processes hold a marker": Chrome renders each
/// process's argv once, but a single invocation contributes renderer, GPU,
/// zygote and utility children, and the same pid can be observed under more than
/// one rendering. Measured on this host, the old count reported 368 where 22
/// processes existed. Keeping the pid alongside the cmdline is what makes the
/// count deduplicable, and it is also what makes a parent→child walk possible
/// without a second process-table implementation.
#[derive(Debug, Default, Clone)]
pub struct LiveProcessIndex {
    pids: HashSet<u32>,
    entries: Vec<ProcessEntry>,
}

impl LiveProcessIndex {
    /// True when `pid` is a live process in this snapshot.
    #[must_use]
    pub fn contains_pid(&self, pid: u32) -> bool {
        self.pids.contains(&pid)
    }

    /// Every process observed in this snapshot.
    #[must_use]
    pub fn entries(&self) -> &[ProcessEntry] {
        &self.entries
    }

    /// Command lines observed in this snapshot, one per process.
    #[must_use]
    pub fn cmdlines(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.cmdline.as_str()).collect()
    }

    /// Direct children of `parent` in this snapshot.
    #[must_use]
    pub fn children_of(&self, parent: u32) -> Vec<u32> {
        self.entries
            .iter()
            .filter(|e| e.ppid == Some(parent))
            .map(|e| e.pid)
            .collect()
    }

    /// Distinct pids whose entry satisfies `predicate`.
    ///
    /// Deduplicating by pid is the point: a count over raw command lines
    /// answers a different question and inflates by roughly the Chrome
    /// subprocess factor.
    ///
    /// The predicate takes the whole [`ProcessEntry`] and not just the command
    /// line. Passing `&str` was what forced every caller to decide identity from
    /// argv alone, which is exactly the substring heuristic that let a shell
    /// script be counted as a browser.
    #[must_use]
    pub fn count_distinct_pids<F>(&self, predicate: F) -> usize
    where
        F: Fn(&ProcessEntry) -> bool,
    {
        self.entries
            .iter()
            .filter(|e| predicate(e))
            .map(|e| e.pid)
            .collect::<HashSet<u32>>()
            .len()
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
    ///
    /// Command lines arrive without pids here, so each is given a synthetic pid
    /// drawn from outside `pids`. That keeps every entry distinct, which is what
    /// the dedupe path assumes.
    #[must_use]
    pub fn from_parts(pids: HashSet<u32>, cmdlines: Vec<String>) -> Self {
        let mut next_synthetic = pids.iter().copied().max().unwrap_or(0).saturating_add(1);
        let entries = cmdlines
            .into_iter()
            .map(|cmdline| {
                let pid = next_synthetic;
                next_synthetic = next_synthetic.saturating_add(1);
                // No executable path: these fixtures describe argv only. That is
                // faithful rather than lossy — it exercises the "unknown exe"
                // branch, which is the one that must stay protective.
                ProcessEntry::new(pid, None, cmdline)
            })
            .collect();
        Self { pids, entries }
    }

    /// Build an index from fully specified entries (tests, tree-walk fixtures).
    #[must_use]
    pub fn from_entries(entries: Vec<ProcessEntry>) -> Self {
        let pids = entries.iter().map(|e| e.pid).collect();
        Self { pids, entries }
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
        .map(|idx| idx.entries.into_iter().map(|entry| entry.cmdline).collect())
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
        .entries
        .iter()
        .any(|entry| entry_holds_path_protective(entry, &needle))
}

mod backend {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

    use super::{LiveProcessIndex, ProcessEntry};

    /// Enumerate the host process table on Linux, macOS and Windows alike.
    ///
    /// Command lines and executable paths are refreshed; CPU, memory, disk and
    /// user data are left out so the snapshot stays cheap.
    ///
    /// Returns `None` when enumeration cannot be trusted. The self-test is that
    /// **this** process must appear in its own snapshot: a live host always has at
    /// least one process, so a table that omits us is a failure, not an empty
    /// host. `None` makes every caller fail closed (GAP-045).
    ///
    /// # Why `without_tasks`
    ///
    /// `ProcessRefreshKind::nothing()` is not as empty as it reads: its `tasks`
    /// field defaults to `true`, because Linux treats tasks as processes. So the
    /// previous shape enumerated every THREAD, paid a `/proc` read for each, and
    /// then threw them away in the `thread_kind()` filter below. Chrome runs on
    /// the order of sixteen threads per process, and the index is built on every
    /// invocation during BORN, so that cost was charged to every single run.
    /// `without_tasks()` declines the work instead of undoing it.
    pub(super) fn collect() -> Option<LiveProcessIndex> {
        let system = System::new_with_specifics(
            RefreshKind::nothing().with_processes(
                ProcessRefreshKind::nothing()
                    .with_cmd(UpdateKind::Always)
                    // Kernel-reported identity. argv is self-declared and cannot
                    // be trusted to say what a process actually is.
                    .with_exe(UpdateKind::Always)
                    .without_tasks(),
            ),
        );

        let mut index = LiveProcessIndex::default();
        for (pid, process) in system.processes() {
            // On Linux `processes()` enumerates every task in `/proc/<pid>/task`,
            // so each THREAD arrives as its own entry carrying the parent's argv.
            // Chrome runs on the order of sixteen threads per process, and the
            // measured effect was a residual report claiming 382 marker-holding
            // browsers on a host that had 23 — an inflation of ~16.6x that made
            // the number useless for any agent decision.
            //
            // `thread_kind()` is `None` exactly for real processes, and is
            // documented to return `None` on every non-Linux platform, so this
            // filter is a no-op on macOS and Windows rather than a `#[cfg]`.
            //
            // Kept even though `without_tasks()` above should make it moot:
            // sysinfo documents that ruling out a refresh does not guarantee the
            // information is withheld when it comes for free. Belt and braces on
            // a filter this cheap is better than re-deriving the 16.6x inflation
            // the day that guarantee changes.
            if process.thread_kind().is_some() {
                continue;
            }
            let pid = pid.as_u32();
            index.pids.insert(pid);
            // Space-joined, not NUL-joined: the browser/text-tool heuristics in
            // `classify` match on `" --type="` and `"rg "`, which only hold for a
            // space-separated rendering of argv.
            let cmdline = process
                .cmd()
                .iter()
                .map(|part| part.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");
            // On Linux a failed `/proc/<pid>/exe` read yields an EMPTY path, not
            // `None` — different uid, a different pid namespace, or a deleted
            // binary all land here. Folding empty into `None` is what keeps
            // "unknown" distinguishable from "known to be something"; without it
            // an empty path would compare against the browser allowlist and quietly
            // answer "not a browser" for a process we simply could not read.
            let exe = process
                .exe()
                .filter(|p| !p.as_os_str().is_empty())
                .map(std::path::Path::to_path_buf);
            // Kept even when argv is hidden: the entry still carries the ppid,
            // which the residual tree walk needs to reach deeper descendants.
            let mut entry =
                ProcessEntry::new(pid, process.parent().map(sysinfo::Pid::as_u32), cmdline);
            entry.exe = exe;
            index.entries.push(entry);
        }

        if !index.pids.contains(&std::process::id()) {
            return None;
        }
        Some(index)
    }
}
