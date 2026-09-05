// SPDX-License-Identifier: MIT OR Apache-2.0
//! Atomic byte writes (tmp + fsync + rename) for image artifacts.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::error::CliError;

/// Write `bytes` to `path` via sibling temp file + fsync + rename.
///
/// # Errors
///
/// [`crate::fs_roots::ensure_write_allowed`] when `path` falls outside the
/// allowed roots (GAP-026), then any IO error from `create_dir_all`, the temp
/// file, `fsync` or the rename.
pub fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    // GAP-026: this is the single funnel every media artifact reaches disk
    // through — `image` encode/avif/download/gif-frames plus the `audio` and
    // `video` downloaders — and it wrote wherever argv pointed. `--out` is
    // operator input, which is exactly what the root policy exists to bound.
    //
    // Same defect class the sibling helpers already carry a note about in
    // `concurrency::fs_block`: the jail held or not depending on which of
    // several interchangeable write helpers a caller happened to pick.
    //
    // The check runs BEFORE `create_dir_all`, matching `json_util::write_json`
    // and `fs_block::write_bytes_sync`, so a refused path never leaves a
    // directory behind as evidence of the attempt.
    crate::fs_roots::ensure_write_allowed(path)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| crate::image_local::magic::io_path_err(parent, "mkdir", &e))?;
        }
    }
    let tmp = {
        let mut t = path.as_os_str().to_os_string();
        t.push(".tmp");
        std::path::PathBuf::from(t)
    };
    {
        let mut f = File::create(&tmp)
            .map_err(|e| crate::image_local::magic::io_path_err(&tmp, "create", &e))?;
        f.write_all(bytes)
            .map_err(|e| crate::image_local::magic::io_path_err(&tmp, "write", &e))?;
        f.sync_all()
            .map_err(|e| crate::image_local::magic::io_path_err(&tmp, "fsync", &e))?;
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        crate::image_local::magic::io_path_err(path, "rename", &e)
    })?;
    Ok(())
}
