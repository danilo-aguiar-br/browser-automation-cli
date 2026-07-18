//! HTTP / parse cache under XDG (PRD 5AF / GAP-011 / GAP-023).
//!
//! Backends: in-process L1 (HashMap), SQLite under XDG cache. Redis is optional
//! via `config set cache_backend=redis` + `cache_redis_url` (never env).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::error::{CliError, ErrorKind};
use crate::xdg;

/// Cache key derived from method + URL + optional body hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey(String);

impl CacheKey {
    /// Build a stable key for an HTTP GET URL.
    pub fn http_get(url: &str) -> Self {
        let mut h = Sha256::new();
        h.update(b"GET\0");
        h.update(url.as_bytes());
        Self(hex::encode(h.finalize()))
    }

    /// Build a stable key for local file parse (path + mtime + len).
    pub fn file_parse(path: &Path, len: u64, mtime_secs: u64) -> Self {
        let mut h = Sha256::new();
        h.update(b"PARSE\0");
        h.update(path.to_string_lossy().as_bytes());
        h.update(len.to_le_bytes());
        h.update(mtime_secs.to_le_bytes());
        Self(hex::encode(h.finalize()))
    }

    /// Hex digest string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Cached payload.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Raw body bytes or UTF-8 text.
    pub body: Vec<u8>,
    /// Optional content-type hint.
    pub content_type: Option<String>,
    /// Expiry as unix seconds (0 = no expiry).
    pub expires_unix: u64,
}

impl CacheEntry {
    /// True when entry is still valid.
    pub fn is_fresh(&self) -> bool {
        if self.expires_unix == 0 {
            return true;
        }
        now_unix() < self.expires_unix
    }
}

/// Trait for one-shot HTTP/parse caches.
pub trait HttpCache: Send {
    /// Lookup a key.
    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CliError>;
    /// Store a key.
    fn put(&self, key: &CacheKey, entry: CacheEntry) -> Result<(), CliError>;
}

/// In-process L1 cache (dies with the process — one-shot safe).
#[derive(Debug, Default)]
pub struct MemoryCache {
    inner: Mutex<HashMap<String, CacheEntry>>,
}

impl HttpCache for MemoryCache {
    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CliError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| CliError::new(ErrorKind::Software, "cache lock poisoned"))?;
        Ok(guard.get(key.as_str()).filter(|e| e.is_fresh()).cloned())
    }

    fn put(&self, key: &CacheKey, entry: CacheEntry) -> Result<(), CliError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| CliError::new(ErrorKind::Software, "cache lock poisoned"))?;
        guard.insert(key.as_str().to_string(), entry);
        Ok(())
    }
}

/// SQLite-backed cache under XDG cache directory.
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
        let db = Self { path: path.clone() };
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

/// Layered L1 memory + L2 sqlite.
pub struct LayeredCache {
    /// In-process layer.
    pub l1: MemoryCache,
    /// Disk layer.
    pub l2: SqliteCache,
}

impl HttpCache for LayeredCache {
    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CliError> {
        if let Some(e) = self.l1.get(key)? {
            return Ok(Some(e));
        }
        if let Some(e) = self.l2.get(key)? {
            let _ = self.l1.put(key, e.clone());
            return Ok(Some(e));
        }
        Ok(None)
    }

    fn put(&self, key: &CacheKey, entry: CacheEntry) -> Result<(), CliError> {
        self.l1.put(key, entry.clone())?;
        self.l2.put(key, entry)?;
        Ok(())
    }
}

/// Redis-backed cache (RESP over TCP). Enabled when
/// `config set cache_backend redis` and `cache_redis_url` is set (XDG only).
#[derive(Debug)]
pub struct RedisCache {
    url: String,
}

impl RedisCache {
    /// Connect and PING. URL form: `redis://127.0.0.1:6379` or `redis://host:port/db`.
    pub fn connect(url: &str) -> Result<Self, CliError> {
        let url = url.trim();
        if url.is_empty() {
            return Err(CliError::with_suggestion(
                ErrorKind::Usage,
                "cache_backend=redis requires cache_redis_url",
                "browser-automation-cli config set cache_redis_url redis://127.0.0.1:6379",
            ));
        }
        let c = Self {
            url: url.to_string(),
        };
        c.cmd_simple(&["PING"]).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Unavailable,
                format!("redis PING failed: {e}"),
                "Start redis-server or switch: config set cache_backend sqlite",
            )
        })?;
        Ok(c)
    }

    fn parse_host_port_db(url: &str) -> Result<(String, u16, i64), String> {
        // GAP-A007: rediss:// implies TLS; this client is plain TCP only — fail closed.
        if url.trim().to_ascii_lowercase().starts_with("rediss://") {
            return Err(
                "rediss:// (TLS) is not supported by the built-in Redis client; use redis://127.0.0.1:6379 (plain local) or config set cache_backend sqlite"
                    .into(),
            );
        }
        // Minimal parser: redis://host:port[/db]
        let rest = url.strip_prefix("redis://").unwrap_or(url);
        let rest = rest.split('@').next_back().unwrap_or(rest);
        let (hostport, db) = match rest.split_once('/') {
            Some((hp, d)) => (hp, d.parse::<i64>().unwrap_or(0)),
            None => (rest, 0),
        };
        let (host, port) = if let Some((h, p)) = hostport.rsplit_once(':') {
            (h.to_string(), p.parse::<u16>().unwrap_or(6379))
        } else {
            (hostport.to_string(), 6379)
        };
        if host.is_empty() {
            return Err("empty redis host".into());
        }
        Ok((host, port, db))
    }

    fn with_stream<T>(
        &self,
        f: impl FnOnce(&mut std::net::TcpStream) -> Result<T, String>,
    ) -> Result<T, String> {
        use std::io::Write as _;
        use std::net::TcpStream;
        use std::time::Duration;

        let (host, port, db) = Self::parse_host_port_db(&self.url)?;
        let mut stream = TcpStream::connect((host.as_str(), port))
            .map_err(|e| format!("connect {host}:{port}: {e}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(3))).ok();
        if db != 0 {
            let select = format!(
                "*2\r\n$6\r\nSELECT\r\n${}\r\n{db}\r\n",
                db.to_string().len()
            );
            stream
                .write_all(select.as_bytes())
                .map_err(|e| format!("SELECT write: {e}"))?;
            let _ = read_resp_line(&mut stream)?;
        }
        f(&mut stream)
    }

    fn cmd_simple(&self, parts: &[&str]) -> Result<String, String> {
        self.with_stream(|stream| {
            write_resp_array(stream, parts)?;
            read_resp_value(stream)
        })
    }

    fn redis_key(key: &CacheKey) -> String {
        format!("browser-automation-cli:cache:v1:{}", key.as_str())
    }
}

impl HttpCache for RedisCache {
    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CliError> {
        let rk = Self::redis_key(key);
        let raw = self
            .cmd_simple(&["GET", &rk])
            .map_err(|e| CliError::new(ErrorKind::Unavailable, format!("redis GET: {e}")))?;
        if raw == "$-1" || raw.is_empty() || raw == "(nil)" {
            return Ok(None);
        }
        // Payload is JSON: {body_b64, content_type, expires_unix}
        let v: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| CliError::new(ErrorKind::Data, format!("redis cache decode: {e}")))?;
        let body_b64 = v
            .get("body_b64")
            .and_then(|x| x.as_str())
            .ok_or_else(|| CliError::new(ErrorKind::Data, "redis cache missing body_b64"))?;
        use base64::Engine;
        let body = base64::engine::general_purpose::STANDARD
            .decode(body_b64)
            .map_err(|e| CliError::new(ErrorKind::Data, format!("redis body b64: {e}")))?;
        let entry = CacheEntry {
            body,
            content_type: v
                .get("content_type")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            expires_unix: v.get("expires_unix").and_then(|x| x.as_u64()).unwrap_or(0),
        };
        if entry.is_fresh() {
            Ok(Some(entry))
        } else {
            let _ = self.cmd_simple(&["DEL", &rk]);
            Ok(None)
        }
    }

    fn put(&self, key: &CacheKey, entry: CacheEntry) -> Result<(), CliError> {
        use base64::Engine;
        let rk = Self::redis_key(key);
        let body_b64 = base64::engine::general_purpose::STANDARD.encode(&entry.body);
        let payload = serde_json::json!({
            "body_b64": body_b64,
            "content_type": entry.content_type,
            "expires_unix": entry.expires_unix,
        })
        .to_string();
        let ttl = if entry.expires_unix > 0 {
            let now = now_unix();
            entry.expires_unix.saturating_sub(now).max(1)
        } else {
            86_400
        };
        let ttl_s = ttl.to_string();
        self.with_stream(|stream| {
            write_resp_array(stream, &["SET", &rk, &payload, "EX", &ttl_s])?;
            let _ = read_resp_value(stream)?;
            Ok(())
        })
        .map_err(|e| CliError::new(ErrorKind::Unavailable, format!("redis SET: {e}")))
    }
}

fn write_resp_array(stream: &mut impl std::io::Write, parts: &[&str]) -> Result<(), String> {
    let mut buf = format!("*{}\r\n", parts.len());
    for p in parts {
        buf.push_str(&format!("${}\r\n{}\r\n", p.len(), p));
    }
    stream
        .write_all(buf.as_bytes())
        .map_err(|e| format!("redis write: {e}"))
}

fn read_resp_line(stream: &mut impl std::io::Read) -> Result<String, String> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = stream
            .read(&mut byte)
            .map_err(|e| format!("redis read: {e}"))?;
        if n == 0 {
            break;
        }
        if byte[0] == b'\n' {
            break;
        }
        if byte[0] != b'\r' {
            line.push(byte[0]);
        }
        if line.len() > 16 * 1024 * 1024 {
            return Err("redis line too large".into());
        }
    }
    String::from_utf8(line).map_err(|e| format!("redis utf8: {e}"))
}

fn read_resp_value(stream: &mut impl std::io::Read) -> Result<String, String> {
    let line = read_resp_line(stream)?;
    if line.is_empty() {
        return Err("empty redis response".into());
    }
    match line.as_bytes()[0] {
        b'+' | b':' | b'-' => Ok(line[1..].to_string()),
        b'$' => {
            let n: i64 = line[1..].parse().map_err(|e| format!("bulk len: {e}"))?;
            if n < 0 {
                return Ok(String::new());
            }
            let mut buf = vec![0u8; n as usize + 2]; // payload + CRLF
            stream
                .read_exact(&mut buf)
                .map_err(|e| format!("bulk read: {e}"))?;
            // drop trailing CRLF
            if buf.len() >= 2 {
                buf.truncate(buf.len() - 2);
            }
            String::from_utf8(buf).map_err(|e| format!("bulk utf8: {e}"))
        }
        b'*' => {
            // For simple commands we only need first line acknowledgement.
            Ok(line)
        }
        _ => Ok(line),
    }
}

/// Build the product cache from XDG `cache_backend` (sqlite|memory|redis).
pub fn default_cache() -> Result<Box<dyn HttpCache>, CliError> {
    let cfg = xdg::load_config().unwrap_or_default();
    let backend = cfg
        .cache_backend
        .as_deref()
        .unwrap_or("sqlite")
        .to_ascii_lowercase();
    match backend.as_str() {
        "memory" => Ok(Box::new(MemoryCache::default())),
        "redis" => {
            let url = cfg.cache_redis_url.as_deref().unwrap_or("");
            Ok(Box::new(RedisCache::connect(url)?))
        }
        // default sqlite layered
        _ => Ok(Box::new(LayeredCache {
            l1: MemoryCache::default(),
            l2: SqliteCache::open_default()?,
        })),
    }
}

/// Layered only (tests / explicit sqlite path).
pub fn sqlite_layered_cache() -> Result<LayeredCache, CliError> {
    Ok(LayeredCache {
        l1: MemoryCache::default(),
        l2: SqliteCache::open_default()?,
    })
}

/// TTL helper: now + duration.
pub fn expires_after(ttl: Duration) -> u64 {
    now_unix().saturating_add(ttl.as_secs())
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread;

    /// Minimal RESP server speaking the subset used by [`RedisCache`] (GAP-A008).
    struct RespMockServer {
        port: u16,
        stop: Arc<Mutex<bool>>,
        join: Option<thread::JoinHandle<()>>,
    }

    impl RespMockServer {
        fn spawn() -> Result<Self, String> {
            let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
            let port = listener.local_addr().map_err(|e| e.to_string())?.port();
            let stop = Arc::new(Mutex::new(false));
            let stop_t = Arc::clone(&stop);
            let store: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
            let join = thread::spawn(move || {
                let _ = listener.set_nonblocking(true);
                while !*stop_t.lock().unwrap_or_else(|e| e.into_inner()) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let store = Arc::clone(&store);
                            thread::spawn(move || {
                                let _ = handle_resp_client(stream, store);
                            });
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(_) => break,
                    }
                }
            });
            thread::sleep(std::time::Duration::from_millis(20));
            Ok(Self {
                port,
                stop,
                join: Some(join),
            })
        }
    }

    impl Drop for RespMockServer {
        fn drop(&mut self) {
            if let Ok(mut g) = self.stop.lock() {
                *g = true;
            }
            let _ = TcpStream::connect(("127.0.0.1", self.port));
            if let Some(j) = self.join.take() {
                let _ = j.join();
            }
        }
    }

    fn handle_resp_client(
        mut stream: TcpStream,
        store: Arc<Mutex<HashMap<String, String>>>,
    ) -> Result<(), String> {
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
        let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(2)));
        while let Ok(cmd) = read_resp_array(&mut stream) {
            if cmd.is_empty() {
                break;
            }
            let name = cmd[0].to_ascii_uppercase();
            let reply = match name.as_str() {
                "PING" => "+PONG\r\n".to_string(),
                "SELECT" => "+OK\r\n".to_string(),
                "SET" if cmd.len() >= 3 => {
                    let key = cmd[1].clone();
                    let val = cmd[2].clone();
                    if let Ok(mut g) = store.lock() {
                        g.insert(key, val);
                    }
                    "+OK\r\n".to_string()
                }
                "GET" if cmd.len() >= 2 => {
                    let key = &cmd[1];
                    let val = store.lock().ok().and_then(|g| g.get(key).cloned());
                    match val {
                        Some(v) => format!("${}\r\n{}\r\n", v.len(), v),
                        None => "$-1\r\n".to_string(),
                    }
                }
                "DEL" if cmd.len() >= 2 => {
                    let key = &cmd[1];
                    let n = store
                        .lock()
                        .map(|mut g| if g.remove(key).is_some() { 1 } else { 0 })
                        .unwrap_or(0);
                    format!(":{n}\r\n")
                }
                _ => "-ERR unknown command\r\n".to_string(),
            };
            stream
                .write_all(reply.as_bytes())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn read_resp_array(stream: &mut impl Read) -> Result<Vec<String>, String> {
        let line = read_resp_line(stream)?;
        if line.is_empty() {
            return Err("eof".into());
        }
        if !line.starts_with('*') {
            return Err(format!("expected array, got {line}"));
        }
        let n: i64 = line[1..].parse().map_err(|e| format!("array len: {e}"))?;
        if n < 0 {
            return Ok(Vec::new());
        }
        let mut out = Vec::with_capacity(n as usize);
        for _ in 0..n {
            out.push(read_resp_bulk(stream)?);
        }
        Ok(out)
    }

    fn read_resp_bulk(stream: &mut impl Read) -> Result<String, String> {
        let line = read_resp_line(stream)?;
        if !line.starts_with('$') {
            return Err(format!("expected bulk, got {line}"));
        }
        let n: i64 = line[1..].parse().map_err(|e| format!("bulk len: {e}"))?;
        if n < 0 {
            return Ok(String::new());
        }
        let mut buf = vec![0u8; n as usize + 2];
        stream
            .read_exact(&mut buf)
            .map_err(|e| format!("bulk body: {e}"))?;
        if buf.len() >= 2 {
            buf.truncate(buf.len() - 2);
        }
        String::from_utf8(buf).map_err(|e| format!("bulk utf8: {e}"))
    }

    fn which_bin(name: &str) -> Option<String> {
        std::env::var_os("PATH").and_then(|paths| {
            for dir in std::env::split_paths(&paths) {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate.display().to_string());
                }
            }
            None
        })
    }

    fn free_port() -> Result<u16, String> {
        let l = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        Ok(l.local_addr().map_err(|e| e.to_string())?.port())
    }

    #[test]
    fn memory_hit_miss() {
        let c = MemoryCache::default();
        let k = CacheKey::http_get("https://example.com/");
        assert!(c.get(&k).unwrap().is_none());
        c.put(
            &k,
            CacheEntry {
                body: b"hi".to_vec(),
                content_type: Some("text/html".into()),
                expires_unix: 0,
            },
        )
        .unwrap();
        let e = c.get(&k).unwrap().unwrap();
        assert_eq!(e.body, b"hi");
    }

    #[test]
    fn key_stable() {
        assert_eq!(
            CacheKey::http_get("https://a").as_str(),
            CacheKey::http_get("https://a").as_str()
        );
        assert_ne!(
            CacheKey::http_get("https://a").as_str(),
            CacheKey::http_get("https://b").as_str()
        );
    }

    #[test]
    fn redis_url_parse() {
        let (h, p, d) = RedisCache::parse_host_port_db("redis://127.0.0.1:6379/2").unwrap();
        assert_eq!(h, "127.0.0.1");
        assert_eq!(p, 6379);
        assert_eq!(d, 2);
    }

    #[test]
    fn redis_rediss_tls_rejected_fail_closed() {
        // GAP-A007: never open plain TCP for rediss://
        let err = RedisCache::parse_host_port_db("rediss://example.com:6380/0").unwrap_err();
        assert!(
            err.contains("rediss://") || err.contains("TLS"),
            "expected TLS rejection, got: {err}"
        );
    }

    #[test]
    fn redis_connect_empty_url_errors() {
        let e = RedisCache::connect("").unwrap_err();
        assert!(e.message().contains("cache_redis_url") || e.message().contains("redis"));
    }

    /// Always-on TCP roundtrip against an in-process RESP mock (GAP-A008 / R-LIVE-1).
    /// No product env; no external redis-server required.
    #[test]
    fn redis_roundtrip_via_resp_mock() {
        let mock = RespMockServer::spawn().expect("mock listen");
        let url = format!("redis://127.0.0.1:{}/0", mock.port);
        let c = RedisCache::connect(&url).expect("connect mock redis");
        let k = CacheKey::http_get("https://redis-mock.example/");
        c.put(
            &k,
            CacheEntry {
                body: b"live-mock".to_vec(),
                content_type: Some("text/plain".into()),
                expires_unix: 0,
            },
        )
        .expect("put");
        let e = c.get(&k).expect("get").expect("hit");
        assert_eq!(e.body, b"live-mock");
        drop(mock);
    }

    /// When `redis-server` is on PATH, spawn ephemeral instance and roundtrip (R-LIVE-4).
    /// Skips cleanly (pass) when the binary is absent — no product env.
    #[test]
    fn redis_real_server_if_present() {
        let Some(bin) = which_bin("redis-server") else {
            eprintln!("skip redis_real_server_if_present: redis-server not on PATH");
            return;
        };
        let dir = tempfile::tempdir().expect("tmp");
        let port = free_port().expect("port");
        let mut child = std::process::Command::new(&bin)
            .arg("--port")
            .arg(port.to_string())
            .arg("--dir")
            .arg(dir.path())
            .arg("--save")
            .arg("")
            .arg("--appendonly")
            .arg("no")
            .arg("--bind")
            .arg("127.0.0.1")
            .arg("--protected-mode")
            .arg("no")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn redis-server");
        let url = format!("redis://127.0.0.1:{port}/15");
        let mut ok = false;
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if RedisCache::connect(&url).is_ok() {
                ok = true;
                break;
            }
        }
        if !ok {
            let _ = child.kill();
            let _ = child.wait();
            panic!("redis-server did not accept connections on {url}");
        }
        let c = RedisCache::connect(&url).expect("connect real redis");
        let k = CacheKey::http_get("https://redis-real.example/");
        c.put(
            &k,
            CacheEntry {
                body: b"live-real".to_vec(),
                content_type: Some("text/plain".into()),
                expires_unix: 0,
            },
        )
        .expect("put");
        let e = c.get(&k).expect("get").expect("hit");
        assert_eq!(e.body, b"live-real");
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn default_cache_sqlite_works() {
        let c = default_cache().expect("sqlite layered");
        let k = CacheKey::http_get("https://cache-audit.example/");
        c.put(
            &k,
            CacheEntry {
                body: b"ok".to_vec(),
                content_type: Some("text/plain".into()),
                expires_unix: 0,
            },
        )
        .unwrap();
        let e = c.get(&k).unwrap().unwrap();
        assert_eq!(e.body, b"ok");
    }
}
