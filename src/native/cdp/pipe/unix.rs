// SPDX-License-Identifier: MIT OR Apache-2.0
//! POSIX half of the DevTools pipe: two pipes, moved onto descriptors 3 and 4.

use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::process::Command;

/// Descriptor Chrome reads DevTools commands from.
const CHROME_READ_FD: RawFd = 3;
/// Descriptor Chrome writes DevTools replies to.
const CHROME_WRITE_FD: RawFd = 4;

/// The pipe ends that belong in the Chrome child. Closed in the parent once the
/// fork has happened, which is what lets the parent see end-of-file later.
pub struct ChildEnds {
    read: OwnedFd,
    write: OwnedFd,
}

/// The pipe ends this process keeps.
pub struct ParentEnds {
    read: OwnedFd,
    write: OwnedFd,
}

impl ParentEnds {
    /// Reader of Chrome's replies and writer of commands, as files.
    #[must_use]
    pub fn into_files(self) -> (File, File) {
        (File::from(self.read), File::from(self.write))
    }
}

/// Create both pipes with close-on-exec set, so no other child inherits them.
///
/// # Errors
///
/// Fails when the kernel refuses a pipe or the close-on-exec flag.
pub fn create() -> std::io::Result<(ChildEnds, ParentEnds)> {
    let (chrome_reads, parent_writes) = pipe_cloexec()?;
    let (parent_reads, chrome_writes) = pipe_cloexec()?;
    Ok((
        ChildEnds {
            read: chrome_reads,
            write: chrome_writes,
        },
        ParentEnds {
            read: parent_reads,
            write: parent_writes,
        },
    ))
}

/// Arrange for the child to find its ends on descriptors 3 and 4.
pub fn install_child_ends(command: &mut Command, ends: &ChildEnds) {
    use std::os::unix::process::CommandExt;
    let read = ends.read.as_raw_fd();
    let write = ends.write.as_raw_fd();
    let hook = move || -> std::io::Result<()> {
        // `dup2(read, 3)` would close a write end that happens to sit on 3, so
        // move it out of the way first.
        let mut write = write;
        if write == CHROME_READ_FD {
            // SAFETY:
            // - Contract: duplicate a valid descriptor above 4, close-on-exec.
            // - Invariant: `fcntl` is async-signal-safe and allocates nothing.
            // - See: `man 2 fcntl` (F_DUPFD_CLOEXEC), `man 7 signal-safety`.
            let moved = unsafe { libc::fcntl(write, libc::F_DUPFD_CLOEXEC, CHROME_WRITE_FD + 1) };
            if moved < 0 {
                return Err(std::io::Error::last_os_error());
            }
            write = moved;
        }
        place(read, CHROME_READ_FD)?;
        place(write, CHROME_WRITE_FD)?;
        Ok(())
    };
    // SAFETY:
    // - Contract: `pre_exec` requires an async-signal-safe closure.
    // - Invariant: the closure calls only `fcntl` and `dup2`, both
    //   async-signal-safe, and allocates nothing; the descriptors it copies are
    //   owned by `ends`, which the caller keeps alive until the fork returns.
    // - See: `std::os::unix::process::CommandExt::pre_exec`, `man 7 signal-safety`.
    unsafe {
        command.pre_exec(hook);
    }
}

/// Put `fd` on `target` without close-on-exec, in the forked child.
fn place(fd: RawFd, target: RawFd) -> std::io::Result<()> {
    if fd == target {
        // Already there: only the inherited close-on-exec flag must go.
        // SAFETY:
        // - Contract: clear FD_CLOEXEC on a valid descriptor.
        // - Invariant: `fcntl` is async-signal-safe and allocates nothing.
        // - See: `man 2 fcntl` (F_SETFD).
        if unsafe { libc::fcntl(fd, libc::F_SETFD, 0) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        return Ok(());
    }
    // SAFETY:
    // - Contract: copy a valid descriptor onto `target`; the copy has no FD_CLOEXEC.
    // - Invariant: `dup2` is async-signal-safe and allocates nothing.
    // - See: `man 2 dup2`, `man 7 signal-safety`.
    if unsafe { libc::dup2(fd, target) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// One pipe, both ends close-on-exec.
fn pipe_cloexec() -> std::io::Result<(OwnedFd, OwnedFd)> {
    let mut fds: [libc::c_int; 2] = [-1, -1];
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // SAFETY:
        // - Contract: create a pipe with close-on-exec set atomically.
        // - Invariant: `fds` has room for the two descriptors `pipe2` writes.
        // - See: `man 2 pipe2`.
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        // macOS has no `pipe2`; the flag is set right after. A fork on another
        // thread in that gap would inherit the ends — a known, narrow window
        // that the atomic `pipe2` path closes on Linux.
        // SAFETY:
        // - Contract: create a pipe.
        // - Invariant: `fds` has room for the two descriptors `pipe` writes.
        // - See: `man 2 pipe`.
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        for fd in fds {
            // SAFETY:
            // - Contract: set FD_CLOEXEC on a descriptor `pipe` just returned.
            // - See: `man 2 fcntl` (F_SETFD).
            if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
                let err = std::io::Error::last_os_error();
                // SAFETY: both descriptors are valid and owned by nobody else yet.
                unsafe { libc::close(fds[0]) };
                // SAFETY: as above.
                unsafe { libc::close(fds[1]) };
                return Err(err);
            }
        }
    }
    // SAFETY:
    // - Contract: take ownership of the two descriptors the kernel returned.
    // - Invariant: both are valid, open, and owned by nothing else.
    // - See: `std::os::fd::FromRawFd`.
    let read = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    // SAFETY: same contract, for the second descriptor.
    let write = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    Ok((read, write))
}
