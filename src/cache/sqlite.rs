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
    pub fn open_default() -> Result<Self, CliError> {
        let dir = xdg::cache_dir()?.join("http_cache");
        std::fs::create_dir_all(&dir)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("http_cache mkdir: {e}")))?;
        let path = dir.join("cache.sqlite");
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
        Ok(())
    }
}

impl HttpCache for SqliteCache {
    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CliError> {
        let conn = rusqlite::Connection::open(&self.path)
            .map_err(|e| CliError::new(ErrorKind::Io, format!("http_cache open: {e}")))?;
        let mut stmt = conn
            .prepare("SELECT body, content_type, expires_unix FROM entries WHERE key = ?1")
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
            let entry = CacheEntry {
                body,
                content_type,
                expires_unix: expires_unix.max(0) as u64,
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
            "INSERT OR REPLACE INTO entries (key, body, content_type, expires_unix) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                key.as_str(),
                entry.body,
                entry.content_type,
                entry.expires_unix as i64
            ],
        )
        .map_err(|e| CliError::new(ErrorKind::Io, format!("http_cache put: {e}")))?;
        Ok(())
    }
}
