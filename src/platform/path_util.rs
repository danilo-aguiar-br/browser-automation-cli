// SPDX-License-Identifier: MIT OR Apache-2.0
//! PATH lookup and executable probes (host discovery, not product env knobs).

use std::path::{Path, PathBuf};

/// Locate an executable on `$PATH` without shelling out to `which`/`where`.
///
/// On Windows, also tries `{name}.exe`. Returns the first existing regular file.
pub fn which_bin(name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    // Absolute / relative path with separators: honor directly when executable.
    let as_path = Path::new(name);
    if as_path.components().count() > 1 || as_path.is_absolute() {
        if is_executable_file(as_path) {
            return Some(as_path.to_path_buf());
        }
        return None;
    }
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let with_exe = dir.join(format!("{name}.exe"));
            if is_executable_file(&with_exe) {
                return Some(with_exe);
            }
            let with_cmd = dir.join(format!("{name}.cmd"));
            if with_cmd.is_file() {
                return Some(with_cmd);
            }
            let with_bat = dir.join(format!("{name}.bat"));
            if with_bat.is_file() {
                return Some(with_bat);
            }
        }
    }
    None
}

/// Create a file that is private to its owner from the moment it exists.
///
/// # Why creation and not a later chmod
///
/// `File::create` honours the process umask, which is commonly `0o022`, so the
/// file is born world-readable. Restricting it afterwards leaves a window in
/// which any local user can read it, and for a file that holds a private key or
/// a proxy password that window is the whole vulnerability. Passing the mode to
/// `open` closes it: the file never exists in a readable state.
///
/// # Why the error is returned instead of ignored
///
/// The call sites this replaces all discarded the result. A chmod that fails —
/// an exotic filesystem, a restrictive mount — then left the secret readable
/// permanently, with nothing in the output to say so. A security control that
/// cannot report its own failure is indistinguishable from one that is absent.
///
/// # Non-Unix platforms
///
/// Windows inherits the ACL of the containing directory, which the previous
/// `cfg(unix)` blocks left entirely unaddressed. Callers holding a secret
/// should keep it under a directory the product itself created private.
pub fn create_private_file(path: &Path) -> std::io::Result<std::fs::File> {
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
}

/// Restrict an existing path to its owner, reporting failure instead of hiding it.
///
/// Prefer [`create_private_file`] when the file is being created: this function
/// still leaves the window between creation and restriction, and exists only
/// for paths the product did not open itself, such as a directory.
pub fn restrict_to_owner(path: &Path, mode: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
    }
    Ok(())
}

/// True when `path` is a regular file and (on Unix) has any execute bit.
pub fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match path.metadata() {
            Ok(meta) => meta.permissions().mode() & 0o111 != 0,
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// First existing executable among candidate paths (skips missing / non-exec).
pub fn first_existing_executable<'a, I>(candidates: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = &'a Path>,
{
    for p in candidates {
        if is_executable_file(p) {
            return Some(p.to_path_buf());
        }
    }
    None
}

/// Probe `exe --version` for doctor smoke (never on the hot launch path).
///
/// Uses [`super::process_util::run_capture_with_timeout`] (explicit Stdio +
/// deadline + reap). Fails closed to `None` on spawn/timeout/unsafe binary.
pub fn probe_binary_version(path: &Path) -> Option<String> {
    use std::process::Command;
    use std::time::Duration;

    if !super::process_util::is_spawn_safe_binary(path) {
        return None;
    }
    let mut cmd = Command::new(path);
    cmd.arg("--version");
    let out = super::process_util::run_capture_with_timeout(
        &mut cmd,
        Duration::from_secs(crate::xdg::policy::policy_u64(
            crate::xdg::policy::key::PLATFORM_CHILD_WAIT_SECS,
        )),
    )
    .ok()?;
    if !out.status.success() && out.stdout.is_empty() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}
