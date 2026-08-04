// SPDX-License-Identifier: MIT OR Apache-2.0
//! Atomic byte writes (tmp + fsync + rename) for image artifacts.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::error::CliError;

/// Write `bytes` to `path` via sibling temp file + fsync + rename.
pub fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
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
