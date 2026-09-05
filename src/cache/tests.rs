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
        // Accept BLOCKS, and `Drop` wakes it by connecting to its own port.
        //
        // The self-connect in `Drop` only ever made sense for a blocking accept;
        // pairing it with a non-blocking listener left both shutdown mechanisms
        // half-applied and bought nothing but a latency floor. Every `get` and
        // every `put` opens a FRESH connection through `RedisCache::with_stream`,
        // so each one needs its own `accept`, and the client gives up after
        // `REDIS_SHORT_IO_TIMEOUT_SECS` (2 s) of silence.
        //
        // Measured 2026-09-04: with a 5 ms poll between `WouldBlock` returns,
        // `redis_roundtrip_via_resp_mock` failed 1 run in 10 under load 116 on a
        // 10-core host, reporting `redis GET: empty redis response`. The poll
        // thread was simply not scheduled soon enough, and a sleep-based poll
        // turns scheduler pressure into a missed deadline. Blocking accept has
        // no such floor: the kernel wakes the thread when the connection lands.
        let join = thread::spawn(move || {
            while !stop_t.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let store = Arc::clone(&store);
                        thread::spawn(move || {
                            let _ = handle_resp_client(stream, store);
                        });
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
    let k = CacheKey::http_get(
        "https://example.com/",
        &CacheContext::direct("chrome-linux"),
    );
    assert!(c.get(&k).unwrap().is_none());
    c.put(
        &k,
        CacheEntry {
            body: b"hi".to_vec(),
            content_type: Some("text/html".into()),
            expires_unix: 0,
            final_url: None,
        },
    )
    .unwrap();
    let e = c.get(&k).unwrap().unwrap();
    assert_eq!(e.body, b"hi");
}

#[test]
fn key_stable() {
    assert_eq!(
        CacheKey::http_get("https://a", &CacheContext::direct("chrome-linux")).as_str(),
        CacheKey::http_get("https://a", &CacheContext::direct("chrome-linux")).as_str()
    );
    assert_ne!(
        CacheKey::http_get("https://a", &CacheContext::direct("chrome-linux")).as_str(),
        CacheKey::http_get("https://b", &CacheContext::direct("chrome-linux")).as_str()
    );
}

#[test]
fn a_different_egress_route_is_a_different_question() {
    // The regression this guards: with the URL already cached, a dead
    // `--proxy` returned `ok: true` and `cache_hit: true`. Reusing an entry
    // across egress routes cancels the isolation the proxy exists to give.
    let direct = CacheKey::http_get("https://a/", &CacheContext::direct("chrome-linux"));
    let proxied = CacheKey::http_get(
        "https://a/",
        &CacheContext {
            proxy: Some("http://127.0.0.1:1"),
            stealth_profile: "chrome-linux",
            extra_headers: &[],
        },
    );
    assert_ne!(direct.as_str(), proxied.as_str());
}

#[test]
fn stealth_off_is_not_the_host_profile() {
    // The defect: `stealth_profile()` resolves `auto` against the host even
    // when stealth is OFF, so both runs produced the same token and shared one
    // entry. Measured 2026-09-04 against a loopback header echo, a
    // `--no-stealth` scrape came back carrying a full Chrome User-Agent and the
    // three `sec-ch-ua` hints, with `cache_hit: true` next to `stealth: false`.
    //
    // Written against the TOKEN and not against `stealth_cache_token()`, because
    // that function reads process-wide policy and a unit test that set it would
    // decide the value for every other test in the binary.
    let impersonating = CacheKey::http_get("https://a/", &CacheContext::direct("chrome-mac"));
    let honest = CacheKey::http_get("https://a/", &CacheContext::direct("off"));
    assert_ne!(
        impersonating.as_str(),
        honest.as_str(),
        "a body fetched under impersonation must not answer a request that sent \
         the product's own User-Agent"
    );
}

#[test]
fn a_different_identity_is_a_different_question() {
    let linux = CacheKey::http_get("https://a/", &CacheContext::direct("chrome-linux"));
    let windows = CacheKey::http_get("https://a/", &CacheContext::direct("chrome-win"));
    assert_ne!(linux.as_str(), windows.as_str());
}

#[test]
fn a_different_authorization_is_a_different_question() {
    let anon = CacheKey::http_get("https://a/", &CacheContext::direct("chrome-linux"));
    let authed = CacheKey::http_get(
        "https://a/",
        &CacheContext {
            proxy: None,
            stealth_profile: "chrome-linux",
            extra_headers: &[("authorization".into(), "Bearer one".into())],
        },
    );
    let other = CacheKey::http_get(
        "https://a/",
        &CacheContext {
            proxy: None,
            stealth_profile: "chrome-linux",
            extra_headers: &[("authorization".into(), "Bearer two".into())],
        },
    );
    assert_ne!(anon.as_str(), authed.as_str());
    assert_ne!(authed.as_str(), other.as_str());
}

#[test]
fn header_order_does_not_change_the_key() {
    // A `HeaderMap` has no insertion order, so a key that depended on it would
    // be unstable — which defeats the cache rather than partitioning it.
    let forward = CacheKey::http_get(
        "https://a/",
        &CacheContext {
            proxy: None,
            stealth_profile: "chrome-linux",
            extra_headers: &[("a".into(), "1".into()), ("b".into(), "2".into())],
        },
    );
    let reversed = CacheKey::http_get(
        "https://a/",
        &CacheContext {
            proxy: None,
            stealth_profile: "chrome-linux",
            extra_headers: &[("b".into(), "2".into()), ("a".into(), "1".into())],
        },
    );
    assert_eq!(forward.as_str(), reversed.as_str());
}

#[test]
fn header_boundaries_cannot_be_shifted_to_collide() {
    // Length-prefixing exists so ("ab", "c") and ("a", "bc") stay distinct.
    let left = CacheKey::http_get(
        "https://a/",
        &CacheContext {
            proxy: None,
            stealth_profile: "chrome-linux",
            extra_headers: &[("ab".into(), "c".into())],
        },
    );
    let right = CacheKey::http_get(
        "https://a/",
        &CacheContext {
            proxy: None,
            stealth_profile: "chrome-linux",
            extra_headers: &[("a".into(), "bc".into())],
        },
    );
    assert_ne!(left.as_str(), right.as_str());
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
    let k = CacheKey::http_get(
        "https://redis-mock.example/",
        &CacheContext::direct("chrome-linux"),
    );
    c.put(
        &k,
        CacheEntry {
            body: b"live-mock".to_vec(),
            content_type: Some("text/plain".into()),
            expires_unix: 0,
            final_url: None,
        },
    )
    .expect("put");
    let e = c.get(&k).expect("get").expect("hit");
    assert_eq!(e.body, b"live-mock");
    drop(mock);
}

/// When `redis-server` is on PATH, spawn ephemeral instance and roundtrip (R-LIVE-4).
///
/// Declines through [`crate::test_utils::skip_unit_test`] when the binary is
/// absent, so the decline is a FAILURE under `--features strict-gates` rather
/// than a silent libtest PASS. A missing tool is fixable by installing it,
/// which is exactly the shape strict gates exist to refuse.
#[test]
fn redis_real_server_if_present() {
    let Some(bin) = which_bin("redis-server") else {
        crate::test_utils::skip_unit_test(
            "redis_real_server_if_present",
            "redis-server not on PATH.",
        );
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
    let k = CacheKey::http_get(
        "https://redis-real.example/",
        &CacheContext::direct("chrome-linux"),
    );
    c.put(
        &k,
        CacheEntry {
            body: b"live-real".to_vec(),
            content_type: Some("text/plain".into()),
            expires_unix: 0,
            final_url: None,
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
    let k = CacheKey::http_get(
        "https://cache-audit.example/",
        &CacheContext::direct("chrome-linux"),
    );
    c.put(
        &k,
        CacheEntry {
            body: b"ok".to_vec(),
            content_type: Some("text/plain".into()),
            expires_unix: 0,
            final_url: None,
        },
    )
    .unwrap();
    let e = c.get(&k).unwrap().unwrap();
    assert_eq!(e.body, b"ok");
}

/// The memory backend carries `final_url` through a round-trip.
#[test]
fn memory_round_trips_the_final_url() {
    let c = MemoryCache::default();
    let k = CacheKey::http_get(
        "https://example.com/asked",
        &CacheContext::direct("chrome-linux"),
    );
    c.put(
        &k,
        CacheEntry {
            body: b"x".to_vec(),
            content_type: Some("text/html".into()),
            expires_unix: 0,
            final_url: Some("https://example.com/served".into()),
        },
    )
    .unwrap();
    let e = c.get(&k).unwrap().unwrap();
    assert_eq!(e.final_url.as_deref(), Some("https://example.com/served"));
}

/// The sqlite backend persists `final_url` across separate connections.
#[test]
fn sqlite_persists_the_final_url() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("cache.sqlite");
    let k = CacheKey::http_get(
        "https://example.com/asked",
        &CacheContext::direct("chrome-linux"),
    );

    let writer = SqliteCache::open_at(path.clone()).expect("open");
    writer
        .put(
            &k,
            CacheEntry {
                body: b"x".to_vec(),
                content_type: Some("text/html".into()),
                expires_unix: 0,
                final_url: Some("https://example.com/served".into()),
            },
        )
        .expect("put");

    // A separate handle, because the value has to survive the process, not the
    // in-memory struct.
    let reader = SqliteCache::open_at(path).expect("reopen");
    let e = reader.get(&k).expect("get").expect("hit");
    assert_eq!(e.final_url.as_deref(), Some("https://example.com/served"));
}

/// A cache written before `final_url` existed migrates without losing entries.
///
/// This is the case `CREATE TABLE IF NOT EXISTS` does NOT handle: the table is
/// already there, so the new column is only added by the `ALTER TABLE`. Without
/// it every `SELECT` naming the column would fail against a real user's cache.
#[test]
fn a_pre_migration_sqlite_cache_is_upgraded_in_place() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("legacy.sqlite");

    // Build the OLD four-column shape by hand and seed a row.
    {
        let conn = rusqlite::Connection::open(&path).expect("open legacy");
        conn.execute_batch(
            "CREATE TABLE entries (
                key TEXT PRIMARY KEY,
                body BLOB NOT NULL,
                content_type TEXT,
                expires_unix INTEGER NOT NULL
            );",
        )
        .expect("legacy schema");
        conn.execute(
            "INSERT INTO entries (key, body, content_type, expires_unix) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["legacy-key", b"old-body".to_vec(), "text/html", 0i64],
        )
        .expect("seed");
    }

    let cache = SqliteCache::open_at(path.clone()).expect("migrate on open");

    // The seeded row survives, and reads as the `None` a legacy entry means.
    let conn = rusqlite::Connection::open(&path).expect("verify");
    let (body, final_url): (Vec<u8>, Option<String>) = conn
        .query_row(
            "SELECT body, final_url FROM entries WHERE key = 'legacy-key'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("legacy row must survive the migration");
    assert_eq!(body, b"old-body");
    assert_eq!(
        final_url, None,
        "a legacy entry asserts nothing about origin"
    );

    // And the migrated cache accepts the new field from here on.
    let k = CacheKey::http_get(
        "https://example.com/",
        &CacheContext::direct("chrome-linux"),
    );
    cache
        .put(
            &k,
            CacheEntry {
                body: b"new".to_vec(),
                content_type: None,
                expires_unix: 0,
                final_url: Some("https://example.com/served".into()),
            },
        )
        .expect("put after migration");
    let e = cache.get(&k).expect("get").expect("hit");
    assert_eq!(e.final_url.as_deref(), Some("https://example.com/served"));

    // Re-opening runs the ALTER again; the duplicate-column error is the normal
    // answer and must not be treated as a fault.
    SqliteCache::open_at(path).expect("second open must be idempotent");
}

/// The redis backend carries `final_url` through the JSON payload.
///
/// A payload written before the field existed simply lacks the key, and the
/// reader turns that absence into `None` — the same meaning a legacy entry
/// carries — so this backend needs no migration at all.
#[test]
fn redis_round_trips_the_final_url_via_resp_mock() {
    let mock = RespMockServer::spawn().expect("mock listen");
    let url = format!("redis://127.0.0.1:{}/0", mock.port);
    let c = RedisCache::connect(&url).expect("connect mock redis");
    let k = CacheKey::http_get(
        "https://redis-mock.example/asked",
        &CacheContext::direct("chrome-linux"),
    );
    c.put(
        &k,
        CacheEntry {
            body: b"x".to_vec(),
            content_type: Some("text/plain".into()),
            expires_unix: 0,
            final_url: Some("https://redis-mock.example/served".into()),
        },
    )
    .expect("put");
    let e = c.get(&k).expect("get").expect("hit");
    assert_eq!(
        e.final_url.as_deref(),
        Some("https://redis-mock.example/served")
    );
    drop(mock);
}
