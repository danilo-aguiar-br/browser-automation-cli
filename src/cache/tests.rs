// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cache unit tests.
use super::redis::{checked_resp_bulk_len, read_resp_line};
use super::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// Minimal RESP server speaking the subset used by [`RedisCache`] (GAP-A008).
struct RespMockServer {
    port: u16,
    /// Stop flag is a pure boolean shared across threads → `AtomicBool`
    /// (rules: never `Mutex<bool>` when an atomic suffices).
    stop: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

impl RespMockServer {
    fn spawn() -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        // Relaxed: isolated stop flag; no dependent data publication.
        let stop = Arc::new(AtomicBool::new(false));
        let stop_t = Arc::clone(&stop);
        let store: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
        let join = thread::spawn(move || {
            let _ = listener.set_nonblocking(true);
            while !stop_t.load(Ordering::Relaxed) {
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
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn lock_store(
    store: &Mutex<HashMap<String, String>>,
) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
    // Test mock: recover poison so one handler panic cannot freeze the mock.
    crate::sync_util::lock_recover(store)
}

// Owns the Arc: this runs on a spawned connection thread, so borrowing the
// caller's handle would outlive the borrow.
#[allow(clippy::needless_pass_by_value)]
fn handle_resp_client(
    mut stream: TcpStream,
    store: Arc<Mutex<HashMap<String, String>>>,
) -> Result<(), String> {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(
        crate::constants::REDIS_SHORT_IO_TIMEOUT_SECS,
    )));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(
        crate::constants::REDIS_SHORT_IO_TIMEOUT_SECS,
    )));
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
                lock_store(&store).insert(key, val);
                "+OK\r\n".to_string()
            }
            "GET" if cmd.len() >= 2 => {
                let key = &cmd[1];
                let val = lock_store(&store).get(key).cloned();
                match val {
                    Some(v) => format!("${}\r\n{}\r\n", v.len(), v),
                    None => "$-1\r\n".to_string(),
                }
            }
            "DEL" if cmd.len() >= 2 => {
                let key = &cmd[1];
                let n = if lock_store(&store).remove(key).is_some() {
                    1
                } else {
                    0
                };
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

/// Hard cap for RESP array arity in the test mock parser.
const MAX_RESP_ARRAY_LEN: usize = 1024;

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
    if n as u64 > MAX_RESP_ARRAY_LEN as u64 {
        return Err(format!(
            "redis array too large: {n} > {MAX_RESP_ARRAY_LEN} (allocation budget)"
        ));
    }
    let mut out = Vec::new();
    out.try_reserve_exact(n as usize)
        .map_err(|e| format!("redis array reserve failed: {e}"))?;
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
    let len = checked_resp_bulk_len(n)?;
    let mut buf = Vec::new();
    buf.try_reserve_exact(len.saturating_add(2))
        .map_err(|e| format!("redis bulk reserve failed: {e}"))?;
    buf.resize(len + 2, 0);
    stream
        .read_exact(&mut buf)
        .map_err(|e| format!("bulk body: {e}"))?;
    if buf.len() >= 2 {
        buf.truncate(buf.len() - 2);
    }
    String::from_utf8(buf).map_err(|e| format!("bulk utf8: {e}"))
}

#[test]
fn resp_bulk_rejects_oversized_length() {
    assert!(checked_resp_bulk_len(-1).is_err());
    assert!(
        checked_resp_bulk_len((crate::constants::CACHE_MAX_RESP_BULK_BYTES as i64) + 1).is_err()
    );
    assert_eq!(checked_resp_bulk_len(0).unwrap(), 0);
    assert_eq!(checked_resp_bulk_len(64).unwrap(), 64);
}

fn which_bin(name: &str) -> Option<String> {
    crate::platform::which_bin(name).map(|p| p.display().to_string())
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
        .arg(crate::constants::LOOPBACK_HOST)
        .arg("--protected-mode")
        .arg("no")
        .stdin(std::process::Stdio::null())
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
