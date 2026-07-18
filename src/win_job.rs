//! Windows Job Object helpers for residual-zero Chrome reap (PRD 5W / GAP-009).
//!
//! On Windows, launched Chrome is assigned to a Job Object with
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` so FINALIZE/Drop kills the full tree.
//! On non-Windows targets this module is a validated stub (always returns 0).

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
#[cfg(windows)]
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

    unsafe {
        let job: HANDLE = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return 0;
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ok = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of_val(&info) as u32,
        );
        if ok == 0 {
            let _ = CloseHandle(job);
            return 0;
        }
        let proc: HANDLE = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
        if proc.is_null() {
            let _ = CloseHandle(job);
            return 0;
        }
        let assigned = AssignProcessToJobObject(job, proc);
        let _ = CloseHandle(proc);
        if assigned == 0 {
            let _ = CloseHandle(job);
            return 0;
        }
        job as usize
    }
}

/// Terminate all processes in the job.
#[cfg(windows)]
pub fn terminate_job(handle: usize) {
    use windows_sys::Win32::System::Threading::TerminateJobObject;
    if handle == 0 {
        return;
    }
    unsafe {
        let _ = TerminateJobObject(handle as _, 1);
    }
}

/// Close job handle.
#[cfg(windows)]
pub fn close_job(handle: usize) {
    use windows_sys::Win32::Foundation::CloseHandle;
    if handle == 0 {
        return;
    }
    unsafe {
        let _ = CloseHandle(handle as _);
    }
}

/// Terminate a single process by pid (fallback when no job).
#[cfg(windows)]
pub fn terminate_pid(pid: u32) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    if pid == 0 {
        return;
    }
    unsafe {
        let proc = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if !proc.is_null() {
            let _ = TerminateProcess(proc, 1);
            let _ = CloseHandle(proc);
        }
    }
}

/// Self-test: on Windows, create a job for the current process and tear it down
/// without terminating the process (do not set kill-on-close for self-test path).
///
/// Returns `Ok(handle)` if assignment succeeded; caller must `close_job` only
/// (must NOT `terminate_job` on the current process).
#[cfg(windows)]
pub fn validate_assign_current_process() -> Result<usize, String> {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Threading::{
        AssignProcessToJobObject, CreateJobObjectW, GetCurrentProcess, GetCurrentProcessId,
        OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };

    let pid = unsafe { GetCurrentProcessId() };
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

#[cfg(not(windows))]
/// Stub: Job Object only exists on Windows.
pub fn create_and_assign(_pid: u32) -> usize {
    0
}

#[cfg(not(windows))]
/// Stub.
pub fn terminate_job(_handle: usize) {}

#[cfg(not(windows))]
/// Stub.
pub fn close_job(_handle: usize) {}

#[cfg(not(windows))]
/// Stub.
pub fn terminate_pid(_pid: u32) {}

#[cfg(not(windows))]
/// Stub validation: reports that job objects are not available on this OS.
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
