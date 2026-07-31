// SPDX-License-Identifier: MIT OR Apache-2.0
//! SQLite journal path helpers under XDG state.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::{CliError, ErrorKind};
use crate::xdg;

/// Open or create journal DB under XDG state.
pub fn journal_path(name: Option<&str>) -> Result<PathBuf, CliError> {
    let dir = xdg::workflow_dir()?;
    xdg::ensure_dir(&dir)?;
    let file = name.unwrap_or("default");
    let safe: String = file
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Ok(dir.join(format!("{safe}.sqlite")))
}

pub(crate) fn open_db(path: &Path) -> Result<Connection, CliError> {
    let conn = Connection::open(path).map_err(|e| {
        CliError::new(
            ErrorKind::Io,
            format!("open workflow journal {}: {e}", path.display()),
        )
    })?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS steps (
            step_id TEXT PRIMARY KEY,
            cmd TEXT NOT NULL,
            status TEXT NOT NULL,
            depends_on TEXT NOT NULL DEFAULT '[]',
            result_json TEXT,
            error TEXT,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS runs (
            run_id TEXT PRIMARY KEY,
            correlation_id TEXT,
            status TEXT NOT NULL,
            started_at TEXT NOT NULL,
            finished_at TEXT
        );
        "#,
    )
    .map_err(|e| CliError::new(ErrorKind::Software, format!("workflow schema: {e}")))?;
    Ok(conn)
}

pub(crate) fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}
