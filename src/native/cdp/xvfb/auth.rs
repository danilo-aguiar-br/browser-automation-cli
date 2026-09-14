// SPDX-License-Identifier: MIT OR Apache-2.0
//! A private MIT-MAGIC-COOKIE-1 for the private X server.
//!
//! # The defect this closes
//!
//! Xvfb was started without `-auth`. Measured on 2026-09-13: `xhost` against
//! the product's display connected with no cookie at all, and the socket in
//! `/tmp/.X11-unix` was mode `0777`. Any local process could therefore read the
//! screen of a headed Chrome — logged-in sessions included — and inject input
//! into it. With `-auth`, the server accepts only clients that present the
//! cookie written here, and the file holding it is readable by this user alone.
//!
//! # Why the file is written here and not by `xauth`
//!
//! The format is five length-prefixed fields; calling the `xauth` binary would
//! add a host dependency and a subprocess for eighty bytes of data.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Family that matches any address, so the entry works over the local socket.
const FAMILY_WILD: u16 = 0xFFFF;
/// The only authorization protocol Xvfb and Chrome both speak without extras.
const COOKIE_PROTOCOL: &[u8] = b"MIT-MAGIC-COOKIE-1";
/// MIT-MAGIC-COOKIE-1 is sixteen random bytes.
const COOKIE_BYTES: usize = 16;
/// File-name prefix, followed by the creator's pid and a UUID.
const FILE_PREFIX: &str = "browser-automation-cli-xauth-";

/// An authority file that exists only while its owner does.
pub struct XAuthority {
    path: PathBuf,
}

impl XAuthority {
    /// Create a `0600` authority file holding a fresh cookie for display `n`.
    ///
    /// # Errors
    ///
    /// Fails when the system RNG cannot produce the cookie or the file cannot
    /// be created exclusively and written in full.
    pub fn create(n: u32) -> Result<Self, String> {
        let mut cookie = [0u8; COOKIE_BYTES];
        getrandom::getrandom(&mut cookie).map_err(|e| format!("X cookie RNG failed: {e}"))?;
        let dir = dirs::runtime_dir().unwrap_or_else(std::env::temp_dir);
        sweep_orphans(&dir);
        let path = dir.join(format!(
            "{FILE_PREFIX}{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let bytes = entry(&n.to_string(), &cookie);

        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&path)
            .map_err(|e| format!("cannot create X authority {}: {e}", path.display()))?;
        // Owned from here on, so a failed write still removes the file.
        let authority = Self { path };
        file.write_all(&bytes)
            .and_then(|()| file.flush())
            .map_err(|e| format!("cannot write X authority: {e}"))?;
        Ok(authority)
    }

    /// Where the cookie lives, for `-auth` and `XAUTHORITY`.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for XAuthority {
    fn drop(&mut self) {
        // Best effort: a leftover here is one small file in the per-user
        // runtime directory, which the session manager clears at logout.
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Remove authority files whose creating process no longer exists.
///
/// [`Drop`] cannot run when the CLI is killed with `SIGKILL`, and a killed CLI
/// was measured leaving its cookie behind. The creator's pid is part of the
/// name, so a later launch can tell an orphan from a sibling's live file.
/// A reused pid only defers the removal; it never deletes a live file.
fn sweep_orphans(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|n| n.strip_prefix(FILE_PREFIX))
            .and_then(|rest| rest.split_once('-'))
            .and_then(|(pid, _)| pid.parse::<u32>().ok())
        else {
            continue;
        };
        if pid != std::process::id() && !super::display::process_exists(pid) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Serialize one Xauthority entry: family, address, number, name, data.
fn entry(display_number: &str, cookie: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&FAMILY_WILD.to_be_bytes());
    for field in [&[][..], display_number.as_bytes(), COOKIE_PROTOCOL, cookie] {
        // Every field is far below u16::MAX; the conversion cannot fail here.
        let len = u16::try_from(field.len()).unwrap_or(u16::MAX);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(field);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_follows_the_xauthority_layout() {
        let cookie = [7u8; COOKIE_BYTES];
        let bytes = entry("99", &cookie);
        assert_eq!(&bytes[0..2], &[0xFF, 0xFF], "family wild");
        assert_eq!(&bytes[2..4], &[0, 0], "empty address");
        assert_eq!(&bytes[4..6], &[0, 2]);
        assert_eq!(&bytes[6..8], b"99");
        assert_eq!(&bytes[8..10], &[0, 18]);
        assert_eq!(&bytes[10..28], COOKIE_PROTOCOL);
        assert_eq!(&bytes[28..30], &[0, 16]);
        assert_eq!(&bytes[30..46], &cookie);
        assert_eq!(bytes.len(), 46);
    }

    #[test]
    #[cfg(unix)]
    fn the_file_is_private_and_removed_with_its_owner() {
        use std::os::unix::fs::PermissionsExt;
        let authority = XAuthority::create(4242).expect("authority file");
        let path = authority.path().to_path_buf();
        let mode = std::fs::metadata(&path)
            .expect("exists")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "the cookie must be readable by this user only"
        );
        drop(authority);
        assert!(!path.exists(), "the cookie must not outlive its owner");
    }

    /// A cookie left by a killed CLI is swept; a live owner's cookie is kept.
    #[test]
    #[cfg(unix)]
    fn orphaned_cookies_are_swept_and_live_ones_kept() {
        let Some(program) = crate::platform::which_bin("true") else {
            crate::test_utils::skip_unit_test("xauth_sweep", "true not found on this host.");
            return;
        };
        let mut gone = std::process::Command::new(program)
            .spawn()
            .expect("true spawns");
        let dead_pid = gone.id();
        let _ = gone.wait();
        let dir = tempfile::tempdir().expect("temp dir");
        let orphan = dir.path().join(format!("{FILE_PREFIX}{dead_pid}-a"));
        let live = dir
            .path()
            .join(format!("{FILE_PREFIX}{}-b", std::process::id()));
        let foreign = dir.path().join("unrelated-file");
        for path in [&orphan, &live, &foreign] {
            std::fs::write(path, b"x").expect("fixture written");
        }
        sweep_orphans(dir.path());
        assert!(!orphan.exists(), "a dead creator's cookie must be removed");
        assert!(live.exists(), "a live creator's cookie must be kept");
        assert!(
            foreign.exists(),
            "files without the prefix are never touched"
        );
    }
}
