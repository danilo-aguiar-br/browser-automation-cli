// SPDX-License-Identifier: MIT OR Apache-2.0
//! Windows Job Object helpers for residual-zero Chrome reap (PRD 5W / GAP-009).
//!
//! On Windows, launched Chrome is assigned to a Job Object with
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` so FINALIZE/Drop kills the full tree.
//! On non-Windows targets this module is a validated stub (always returns 0).
//!
//! # Platform
//!
//! Real Job Object APIs are compiled only on `cfg(windows)`. docs.rs multi-target
//! builds label those items via `#[doc(cfg(windows))]` under `--cfg docsrs`.

#![allow(dead_code)]

/// True when this build can create real Job Objects (Windows only).
pub fn platform_supports_job_objects() -> bool {
    cfg!(windows)
}

/// Human-readable capability line for doctor / diagnostics.
pub fn capability_summary() -> &'static str {
    if platform_supports_job_objects() {
        "windows_job_object:available (KILL_ON_JOB_CLOSE)"
    } else {
        "windows_job_object:stub (non-windows host)"
    }
}

/// Create a Job Object and assign `pid` to it. Returns handle as `usize` (0 on failure/non-Windows).
///
/// # Safety / multi-op policy (`rules_rust` / D-08)
///
/// Each Win32 call lives in its **own** `unsafe` block with a local SAFETY
/// comment. Safe Rust owns control flow, handle cleanup, and the pid==0 guard.
#[cfg(windows)]
#[cfg_attr(docsrs, doc(cfg(windows)))]
pub fn create_and_assign(pid: u32) -> usize {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Threading::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation, OpenProcess,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };

    if pid == 0 {
        return 0;
    }

    // SAFETY: null security attributes and name are valid for an anonymous job.
    // See: CreateJobObjectW — https://learn.microsoft.com/windows/win32/api/jobapi2/
    let job: HANDLE = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job.is_null() {
        return 0;
    }

    // SAFETY: `info` is fully zero-initialized before LimitFlags is set; size matches the struct.
    // See: SetInformationJobObject + JOBOBJECT_EXTENDED_LIMIT_INFORMATION.
    let set_ok = unsafe {
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of_val(&info) as u32,
        )
    };
    if set_ok == 0 {
        // SAFETY: `job` is an owned handle from CreateJobObjectW on this path.
        unsafe {
            let _ = CloseHandle(job);
        }
        return 0;
    }

    // SAFETY: PROCESS_SET_QUOTA|PROCESS_TERMINATE is the minimum rights for AssignProcessToJobObject.
    // `pid` is a Chrome child recorded in the residual ledger (never 0).
    // See: OpenProcess — https://learn.microsoft.com/windows/win32/api/processthreadsapi/
    let proc: HANDLE =
        unsafe { OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid) };
    if proc.is_null() {
        unsafe {
            let _ = CloseHandle(job);
        }
        return 0;
    }

    // SAFETY: both handles are open and valid; assignment fails cleanly with 0.
    // See: AssignProcessToJobObject.
    let assigned = unsafe { AssignProcessToJobObject(job, proc) };
    // SAFETY: process handle is owned and no longer needed after assign attempt.
    unsafe {
        let _ = CloseHandle(proc);
    }
    if assigned == 0 {
        unsafe {
            let _ = CloseHandle(job);
        }
        return 0;
    }
    job as usize
}

/// Terminate all processes in the job.
#[cfg(windows)]
#[cfg_attr(docsrs, doc(cfg(windows)))]
pub fn terminate_job(handle: usize) {
    use windows_sys::Win32::System::Threading::TerminateJobObject;
    if handle == 0 {
        return;
    }
    // SAFETY: `handle` is a Job Object HANDLE returned by `create_and_assign` (or 0).
    // TerminateJobObject is valid for any open job handle; exit code 1 is conventional.
    // See: TerminateJobObject — https://learn.microsoft.com/windows/win32/api/jobapi2/
    unsafe {
        let _ = TerminateJobObject(handle as _, 1);
    }
}

/// Close job handle.
#[cfg(windows)]
#[cfg_attr(docsrs, doc(cfg(windows)))]
pub fn close_job(handle: usize) {
    use windows_sys::Win32::Foundation::CloseHandle;
    if handle == 0 {
        return;
    }
    // SAFETY: `handle` is an owned Job Object HANDLE; CloseHandle releases it once.
    // See: CloseHandle — Win32 Foundation.
    unsafe {
        let _ = CloseHandle(handle as _);
    }
}

/// Terminate a single process by pid (fallback when no job).
#[cfg(windows)]
#[cfg_attr(docsrs, doc(cfg(windows)))]
pub fn terminate_pid(pid: u32) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    if pid == 0 {
        return;
    }
    // SAFETY: PROCESS_TERMINATE on a ledger Chrome child pid (never 0).
    // See: OpenProcess Win32 docs.
    let proc = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if proc.is_null() {
        return;
    }
    // SAFETY: `proc` is a valid process handle; exit code 1 is conventional for residual kill.
    // See: TerminateProcess.
    unsafe {
        let _ = TerminateProcess(proc, 1);
    }
    // SAFETY: owned handle from OpenProcess; close exactly once.
    unsafe {
        let _ = CloseHandle(proc);
    }
}

/// Self-test: on Windows, create a job for the current process and tear it down
/// without terminating the process (do not set kill-on-close for self-test path).
///
/// Returns `Ok(handle)` if assignment succeeded; caller must `close_job` only
/// (must NOT `terminate_job` on the current process).
#[cfg(windows)]
#[cfg_attr(docsrs, doc(cfg(windows)))]
pub fn validate_assign_current_process() -> Result<usize, String> {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Threading::{
        AssignProcessToJobObject, CreateJobObjectW, GetCurrentProcess, GetCurrentProcessId,
        OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };

    // SAFETY: GetCurrentProcessId has no preconditions; returns this process id.
    // See: GetCurrentProcessId Win32 docs.
    let pid = unsafe { GetCurrentProcessId() };
    // SAFETY:
    // - Contract: doctor self-test creates a job and assigns the current process.
    // - Invariant: no KILL_ON_JOB_CLOSE so closing the job does not kill this process.
    // - Handles closed on failure; success returns owned job handle for `close_job`.
    // - See: CreateJobObjectW / AssignProcessToJobObject.
    unsafe {
        let job: HANDLE = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return Err("CreateJobObjectW failed".into());
        }
        // Assign current process WITHOUT kill-on-close so the test process survives.
        let proc: HANDLE = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
        if proc.is_null() {
            // Fall back to current process pseudo-handle path.
            let cur = GetCurrentProcess();
            let assigned = AssignProcessToJobObject(job, cur);
            if assigned == 0 {
                let _ = CloseHandle(job);
                return Err("AssignProcessToJobObject(current) failed".into());
            }
            return Ok(job as usize);
        }
        let assigned = AssignProcessToJobObject(job, proc);
        let _ = CloseHandle(proc);
        if assigned == 0 {
            let _ = CloseHandle(job);
            return Err("AssignProcessToJobObject failed".into());
        }
        Ok(job as usize)
    }
}

/// Stub: Job Object only exists on Windows (returns `0`).
#[cfg(not(windows))]
#[cfg_attr(docsrs, doc(cfg(not(windows))))]
pub fn create_and_assign(_pid: u32) -> usize {
    0
}

/// Stub: no-op on non-Windows hosts.
#[cfg(not(windows))]
#[cfg_attr(docsrs, doc(cfg(not(windows))))]
pub fn terminate_job(_handle: usize) {}

/// Stub: no-op on non-Windows hosts.
#[cfg(not(windows))]
#[cfg_attr(docsrs, doc(cfg(not(windows))))]
pub fn close_job(_handle: usize) {}

/// Stub: no-op on non-Windows hosts.
#[cfg(not(windows))]
#[cfg_attr(docsrs, doc(cfg(not(windows))))]
pub fn terminate_pid(_pid: u32) {}

/// Stub validation: reports that job objects are not available on this OS.
#[cfg(not(windows))]
#[cfg_attr(docsrs, doc(cfg(not(windows))))]
pub fn validate_assign_current_process() -> Result<usize, String> {
    Err("job objects unavailable on non-windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_summary_is_non_empty() {
        assert!(!capability_summary().is_empty());
    }

    #[test]
    fn stub_or_create_does_not_panic() {
        let h = create_and_assign(0);
        assert_eq!(h, 0, "pid 0 must never produce a job handle");
        terminate_job(0);
        close_job(0);
        terminate_pid(0);
    }

    #[test]
    fn platform_flag_matches_cfg() {
        assert_eq!(platform_supports_job_objects(), cfg!(windows));
    }

    #[test]
    fn validate_current_process_behavior() {
        match validate_assign_current_process() {
            Ok(h) => {
                // On Linux this branch is unreachable; on Windows we must close only.
                assert_ne!(h, 0);
                close_job(h);
            }
            Err(e) => {
                #[cfg(windows)]
                assert!(!e.is_empty());
                #[cfg(not(windows))]
                assert!(e.contains("non-windows"));
            }
        }
    }
}
