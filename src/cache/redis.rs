// SPDX-License-Identifier: MIT OR Apache-2.0
//! Redis-backed cache (RESP over TCP). XDG `cache_redis_url` only.
use crate::error::{CliError, ErrorKind};

use super::types::{CacheEntry, CacheKey, HttpCache};

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
                crate::i18n::suggestion_key("redis_config_required", None),
            ));
        }
        let c = Self {
            url: url.to_string(),
        };
        c.cmd_simple(&["PING"]).map_err(|e| {
            CliError::with_suggestion(
                ErrorKind::Unavailable,
                format!("redis PING failed: {e}"),
                crate::i18n::suggestion_key("redis_config_required", None),
            )
        })?;
        Ok(c)
    }

    pub(crate) fn parse_host_port_db(url: &str) -> Result<(String, u16, i64), String> {
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
        // N20: default loopback-only; remote requires XDG redis_allow_remote.
        crate::net::assert_redis_host_allowed(&host).map_err(|e| e.message().to_string())?;
        Ok((host, port, db))
    }

    fn with_stream<T>(
        &self,
        f: impl FnOnce(&mut std::net::TcpStream) -> Result<T, String>,
    ) -> Result<T, String> {
        use std::io::Write as _;
        use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
        use std::time::Duration;

        let (host, port, db) = Self::parse_host_port_db(&self.url)?;
        let connect_secs = crate::xdg::resolve_redis_connect_timeout_secs();
        let addrs: Vec<SocketAddr> = (host.as_str(), port)
            .to_socket_addrs()
            .map_err(|e| format!("resolve {host}:{port}: {e}"))?
            .collect();
        if addrs.is_empty() {
            return Err(format!("no addresses for {host}:{port}"));
        }
        // Prefer first resolved address with an explicit connect deadline.
        let mut last_err = String::from("connect failed");
        let mut stream = None;
        for addr in &addrs {
            match TcpStream::connect_timeout(addr, Duration::from_secs(connect_secs.max(1))) {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(e) => last_err = format!("connect {addr}: {e}"),
            }
        }
        let mut stream = stream.ok_or(last_err)?;
        let _ = stream.set_nodelay(true);
        stream
            .set_read_timeout(Some(Duration::from_secs(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::REDIS_IO_TIMEOUT_SECS,
            ))))
            .ok();
        stream
            .set_write_timeout(Some(Duration::from_secs(crate::xdg::policy::policy_u64(
                crate::xdg::policy::key::REDIS_IO_TIMEOUT_SECS,
            ))))
            .ok();
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
        let v: serde_json::Value = crate::json_util::from_str(&raw)
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
            let now = super::types::now_unix();
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

pub(crate) fn write_resp_array(
    stream: &mut impl std::io::Write,
    parts: &[&str],
) -> Result<(), String> {
    let mut buf = format!("*{}\r\n", parts.len());
    for p in parts {
        buf.push_str(&format!("${}\r\n{}\r\n", p.len(), p));
    }
    stream
        .write_all(buf.as_bytes())
        .map_err(|e| format!("redis write: {e}"))
}

pub(crate) fn checked_resp_bulk_len(n: i64) -> Result<usize, String> {
    if n < 0 {
        return Err("negative bulk length".into());
    }
    let n = n as u64;
    if n > crate::xdg::policy::policy_usize(crate::xdg::policy::key::CACHE_MAX_RESP_BULK_BYTES)
        as u64
    {
        return Err(format!(
            "redis bulk too large: {n} > {} (allocation budget)",
            crate::xdg::policy::policy_usize(crate::xdg::policy::key::CACHE_MAX_RESP_BULK_BYTES)
        ));
    }
    Ok(n as usize)
}

pub(crate) fn read_resp_line(stream: &mut impl std::io::Read) -> Result<String, String> {
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
        if line.len()
            > crate::xdg::policy::policy_usize(crate::xdg::policy::key::CACHE_MAX_RESP_LINE_BYTES)
        {
            return Err("redis line too large".into());
        }
    }
    String::from_utf8(line).map_err(|e| format!("redis utf8: {e}"))
}

pub(crate) fn read_resp_value(stream: &mut impl std::io::Read) -> Result<String, String> {
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
            let len = checked_resp_bulk_len(n)?;
            // Size validated → fallible reserve then fill (OOM-safe path for untrusted n).
            let mut buf = Vec::new();
            buf.try_reserve_exact(len.saturating_add(2))
                .map_err(|e| format!("redis bulk reserve failed: {e}"))?;
            buf.resize(len + 2, 0);
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
