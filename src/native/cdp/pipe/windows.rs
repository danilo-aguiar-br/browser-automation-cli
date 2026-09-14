// SPDX-License-Identifier: MIT OR Apache-2.0
//! Windows half of the DevTools pipe: inheritable handles named in argv.
//!
//! Chrome adopts the two handles given in `--remote-debugging-io-pipes` as
//! unsigned integers, input first (`devtools_agent_host_impl.cc`, `AdoptPipes`).
//! `std::process::Command` creates the child with handle inheritance on, so an
//! inheritable handle alive at spawn time reaches Chrome with the same value.

use std::fs::File;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

use windows_sys::Win32::Foundation::{SetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::System::Pipes::CreatePipe;

/// The handles Chrome inherits. Closed in the parent after the spawn.
pub struct ChildEnds {
    read: OwnedHandle,
    write: OwnedHandle,
}

impl ChildEnds {
    /// `--remote-debugging-io-pipes=<read>,<write>` for these handles.
    #[must_use]
    pub fn io_pipes_switch(&self) -> String {
        format!(
            "--remote-debugging-io-pipes={},{}",
            self.read.as_raw_handle() as usize,
            self.write.as_raw_handle() as usize
        )
    }
}

/// The handles this process keeps, never inheritable.
pub struct ParentEnds {
    read: OwnedHandle,
    write: OwnedHandle,
}

impl ParentEnds {
    /// Reader of Chrome's replies and writer of commands, as files.
    #[must_use]
    pub fn into_files(self) -> (File, File) {
        (File::from(self.read), File::from(self.write))
    }
}

/// Create both pipes: Chrome's ends inheritable, ours not.
///
/// # Errors
///
/// Fails when `CreatePipe` or `SetHandleInformation` fails.
pub fn create() -> std::io::Result<(ChildEnds, ParentEnds)> {
    let (chrome_reads, parent_writes) = pipe()?;
    let (parent_reads, chrome_writes) = pipe()?;
    not_inheritable(&parent_writes)?;
    not_inheritable(&parent_reads)?;
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

/// One anonymous pipe with both handles inheritable.
fn pipe() -> std::io::Result<(OwnedHandle, OwnedHandle)> {
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: 1,
    };
    let mut read: HANDLE = 0;
    let mut write: HANDLE = 0;
    // SAFETY:
    // - Contract: create an anonymous pipe and receive its two handles.
    // - Invariant: the out-pointers are valid locals and `attributes` lives
    //   for the call.
    // - See: Win32 `CreatePipe`.
    if unsafe { CreatePipe(&mut read, &mut write, &attributes, 0) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY:
    // - Contract: take ownership of the two handles `CreatePipe` returned.
    // - Invariant: both are valid and owned by nothing else.
    // - See: `std::os::windows::io::FromRawHandle`.
    Ok(unsafe {
        (
            OwnedHandle::from_raw_handle(read as _),
            OwnedHandle::from_raw_handle(write as _),
        )
    })
}

/// Clear the inherit flag on a handle this process keeps.
fn not_inheritable(handle: &OwnedHandle) -> std::io::Result<()> {
    // SAFETY:
    // - Contract: change the inherit flag of a valid handle we own.
    // - See: Win32 `SetHandleInformation`.
    if unsafe { SetHandleInformation(handle.as_raw_handle() as HANDLE, HANDLE_FLAG_INHERIT, 0) }
        == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
