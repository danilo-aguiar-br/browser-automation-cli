// SPDX-License-Identifier: MIT OR Apache-2.0
//! SQLite L2 cache under XDG.
use std::path::PathBuf;

use crate::error::{CliError, ErrorKind};
use crate::xdg;

use super::types::{CacheEntry, CacheKey, HttpCache};

/// SQLite-backed L2 HTTP/parse cache under XDG.
pub struct SqliteCache {
    path: PathBuf,
}

impl SqliteCache {
    /// Open or create the product HTTP cache DB.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`] propagated from [`xdg::cache_dir`] when no home
    /// directory resolves, when `http_cache/` cannot be created, and from
    /// `init_schema` when `rusqlite::Connection::open` or the `CREATE TABLE`
    /// batch fails (read-only filesystem, corrupt DB file, or a locked page).
    pub fn open_default() -> Result<Self, CliError> {
        let dir = xdg::cache_dir()?.join("http_cache");
        std::fs::create_dir_all(&dir)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("http_cache mkdir: {e}")))?;
        Self::open_at(dir.join("cache.sqlite"))
    }

    /// Open or create a cache DB at an explicit path.
    ///
    /// Splitting this out keeps the schema migration on ONE path: a test that
    /// reached for its own `Connection` would exercise a different `init_schema`
    /// than production, which is exactly the gap a migration test exists to
    /// close.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::Io`] from `rusqlite::Connection::open`, from the `CREATE
    /// TABLE` batch, or from the `final_url` migration.
    pub(crate) fn open_at(path: PathBuf) -> Result<Self, CliError> {
        let db = Self { path };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<(), CliError> {
        let conn = rusqlite::Connection::open(&self.path)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("http_cache open: {e}")))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entries (
                key TEXT PRIMARY KEY,
                body BLOB NOT NULL,
                content_type TEXT,
                expires_unix INTEGER NOT NULL
            );",
        )
        .map_err(|e| CliError::new(ErrorKind::Io, format!("http_cache schema: {e}")))?;
        // `CREATE TABLE IF NOT EXISTS` does NOT add a column to a table that
        // already exists, so a cache written before `final_url` would keep the
        // old four columns forever and every `SELECT` naming the fifth would
        // fail. `ALTER TABLE` is the migration, and the duplicate-column error
        // is the ordinary answer on an already-migrated database, not a fault:
        // this runs on every open.
        if let Err(e) = conn.execute("ALTER TABLE entries ADD COLUMN final_url TEXT", []) {
            let msg = e.to_string();
            if !msg.contains("duplicate column name") {
                return Err(CliError::new(
                    ErrorKind::Io,
                    format!("http_cache migrate final_url: {e}"),
                ));
            }
        }
        Ok(())
    }
}

impl HttpCache for SqliteCache {
    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CliError> {
        let conn = rusqlite::Connection::open(&self.path)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("http_cache open: {e}")))?;
        let mut stmt = conn
            .prepare(
                "SELECT body, content_type, expires_unix, final_url FROM entries WHERE key = ?1",
            )
            .map_err(|e| CliError::new(ErrorKind::Io, format!("http_cache prepare: {e}")))?;
        let mut rows = stmt
            .query(rusqlite::params![key.as_str()])
            .map_err(|e| CliError::new(ErrorKind::Io, format!("http_cache query: {e}")))?;
        if let Some(row) = rows
            .next()
            .map_err(|e| CliError::new(ErrorKind::Io, format!("http_cache row: {e}")))?
        {
            let body: Vec<u8> = row
                .get(0)
                .map_err(|e| CliError::new(ErrorKind::Data, format!("http_cache body: {e}")))?;
            let content_type: Option<String> = row.get(1).ok();
            let expires_unix: i64 = row.get(2).unwrap_or(0);
            // Rows written before the migration carry SQL NULL here, which is
            // the same `None` a pre-`final_url` entry means: fall back to the
            // requested URL.
            let final_url: Option<String> = row.get(3).ok().flatten();
            let entry = CacheEntry {
                body,
                content_type,
                expires_unix: expires_unix.max(0) as u64,
                final_url,
            };
            if entry.is_fresh() {
                return Ok(Some(entry));
            }
        }
        Ok(None)
    }

    fn put(&self, key: &CacheKey, entry: CacheEntry) -> Result<(), CliError> {
        let conn = rusqlite::Connection::open(&self.path)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("http_cache open: {e}")))?;
        conn.execute(
            "INSERT OR REPLACE INTO entries (key, body, content_type, expires_unix, final_url) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                key.as_str(),
                entry.body,
                entry.content_type,
                entry.expires_unix as i64,
                entry.final_url
            ],
        )
        .map_err(|e| CliError::new(ErrorKind::Io, format!("http_cache put: {e}")))?;
        Ok(())
    }
}
