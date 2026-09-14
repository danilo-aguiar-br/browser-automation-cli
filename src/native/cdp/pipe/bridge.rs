// SPDX-License-Identifier: MIT OR Apache-2.0
//! The loopback WebSocket bridge between `chromiumoxide` and Chrome's pipe.
//!
//! See the parent module for why the bridge exists and why it is safe.

use super::ParentEnds;
use std::io::{BufRead, BufReader, Read, Write};
use std::time::Duration;

use async_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use async_tungstenite::tungstenite::http::StatusCode;
use async_tungstenite::tungstenite::protocol::WebSocketConfig;
use async_tungstenite::tungstenite::Message;
use futures_util::StreamExt;

/// A running relay between one WebSocket client and Chrome's pipe.
///
/// Owns the relay task and both pipe threads. [`PipeBridge::shutdown`] is the
/// only teardown; [`Drop`] calls it, so a bridge cannot outlive its owner.
pub struct PipeBridge {
    ws_url: String,
    relay: Option<tokio::task::JoinHandle<()>>,
    threads: Vec<std::thread::JoinHandle<()>>,
    /// Disconnects once both pipe threads have returned, so teardown can wait
    /// for them with a deadline instead of an unbounded join.
    /// Behind a mutex only because the receiver is not `Sync` and the bridge
    /// travels inside the browser handle.
    threads_done: Option<std::sync::Mutex<std::sync::mpsc::Receiver<()>>>,
    ready: Option<tokio::sync::oneshot::Receiver<()>>,
}

impl PipeBridge {
    /// Start the pipe threads, the loopback listener and the relay.
    ///
    /// Sends the readiness probe immediately; Chrome answers it once the
    /// DevTools handler is up, which [`Self::wait_ready`] observes.
    ///
    /// # Errors
    ///
    /// Fails when the loopback listener cannot bind or a pipe thread cannot be
    /// started.
    pub async fn start(parent: ParentEnds, accept_deadline: Duration) -> Result<Self, String> {
        let listener = tokio::net::TcpListener::bind((crate::constants::LOOPBACK_HOST, 0))
            .await
            .map_err(|e| format!("DevTools bridge cannot bind loopback: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("DevTools bridge has no local address: {e}"))?
            .port();
        let path = format!("/devtools/browser/{}", uuid::Uuid::new_v4().simple());
        let ws_url = format!("ws://{}:{port}{path}", crate::constants::LOOPBACK_HOST);

        let capacity = crate::constants::CDP_PIPE_CHANNEL_CAPACITY;
        let (from_chrome_tx, from_chrome_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(capacity);
        let (to_chrome_tx, to_chrome_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(capacity);
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

        let probe = format!(
            "{{\"id\":{},\"method\":\"Browser.getVersion\"}}",
            crate::constants::CDP_PIPE_READY_PROBE_ID
        );
        // Queued before the relay exists, so it is the first thing Chrome reads.
        to_chrome_tx
            .try_send(probe.into_bytes())
            .map_err(|e| format!("DevTools readiness probe could not be queued: {e}"))?;

        let (read, write) = parent.into_files();
        // Each thread holds a sender and drops it on return, whatever the path.
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        let reader_done = done_tx.clone();
        let reader = std::thread::Builder::new()
            .name("cdp-pipe-read".to_string())
            .spawn(move || {
                let _done = reader_done;
                read_from_chrome(read, &from_chrome_tx, ready_tx);
            })
            .map_err(|e| format!("DevTools pipe reader thread failed to start: {e}"))?;
        let writer = std::thread::Builder::new()
            .name("cdp-pipe-write".to_string())
            .spawn(move || {
                let _done = done_tx;
                write_to_chrome(write, to_chrome_rx);
            })
            .map_err(|e| format!("DevTools pipe writer thread failed to start: {e}"))?;

        let relay = tokio::spawn(relay(
            listener,
            path,
            accept_deadline,
            from_chrome_rx,
            to_chrome_tx,
        ));
        Ok(Self {
            ws_url,
            relay: Some(relay),
            threads: vec![reader, writer],
            threads_done: Some(std::sync::Mutex::new(done_rx)),
            ready: Some(ready_rx),
        })
    }

    /// The WebSocket URL the CDP client must connect to.
    #[must_use]
    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }

    /// Wait until Chrome answered the readiness probe.
    ///
    /// # Errors
    ///
    /// Fails when Chrome closed the pipe before answering — it exited or
    /// refused the switch — or when no answer came inside `budget`.
    pub async fn wait_ready(&mut self, budget: Duration) -> Result<(), String> {
        let Some(ready) = self.ready.take() else {
            return Ok(());
        };
        match tokio::time::timeout(budget, ready).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err("Chrome closed the DevTools pipe before it answered".to_string()),
            Err(_) => Err(format!(
                "Chrome did not answer on the DevTools pipe within {}s",
                budget.as_secs()
            )),
        }
    }

    /// Stop the relay and reap both pipe threads, waiting at most
    /// [`CDP_PIPE_SHUTDOWN_GRACE_MS`](crate::constants::CDP_PIPE_SHUTDOWN_GRACE_MS).
    /// Idempotent.
    ///
    /// Call after the child is reaped: the reader then sees end-of-file, and
    /// aborting the relay drops the last sender the writer waits on. Neither is
    /// guaranteed in bounded time — a descendant can keep Chrome's write end
    /// open, and an aborted task is dropped only when a worker next polls it —
    /// so a thread still running at the deadline is left detached rather than
    /// holding the command past its own `--timeout`.
    pub fn shutdown(&mut self) {
        if let Some(relay) = self.relay.take() {
            relay.abort();
        }
        let Some(done) = self.threads_done.take() else {
            return;
        };
        let grace = Duration::from_millis(crate::constants::CDP_PIPE_SHUTDOWN_GRACE_MS);
        let waited = match done.into_inner() {
            Ok(done) => done.recv_timeout(grace),
            Err(poisoned) => poisoned.into_inner().recv_timeout(grace),
        };
        match waited {
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) | Ok(()) => {
                for handle in std::mem::take(&mut self.threads) {
                    let _ = handle.join();
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                tracing::warn!(
                    target: "browser_automation_cli::cdp_pipe",
                    grace_ms = crate::constants::CDP_PIPE_SHUTDOWN_GRACE_MS,
                    "DevTools pipe threads still blocked at teardown; left detached \
                     (a descendant of Chrome may still hold the pipe open)"
                );
                self.threads.clear();
            }
        }
    }
}

impl Drop for PipeBridge {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Split Chrome's output on zero bytes and forward each message.
fn read_from_chrome(
    read: impl Read,
    out: &tokio::sync::mpsc::Sender<Vec<u8>>,
    ready: tokio::sync::oneshot::Sender<()>,
) {
    let probe_marker = format!("\"id\":{}", crate::constants::CDP_PIPE_READY_PROBE_ID);
    let mut ready = Some(ready);
    let mut reader = BufReader::new(read);
    let ceiling = crate::constants::CDP_PIPE_MAX_MESSAGE_BYTES;
    loop {
        let mut message = Vec::new();
        // One byte over the ceiling, so a message of exactly the ceiling plus
        // its terminator still fits and anything larger is recognisable.
        let limit = u64::try_from(ceiling).unwrap_or(u64::MAX).saturating_add(1);
        match (&mut reader).take(limit).read_until(0, &mut message) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        if message.pop() != Some(0) {
            if message.len() >= ceiling {
                tracing::warn!(
                    target: "browser_automation_cli::cdp_pipe",
                    ceiling,
                    "DevTools message exceeded the size ceiling; closing the pipe"
                );
            }
            // End of file mid-message, or an oversized one: the stream can no
            // longer be framed, so the bridge ends as if Chrome were gone.
            break;
        }
        if ready.is_some() && contains(&message, probe_marker.as_bytes()) {
            record_chrome_major(&message);
            if let Some(tx) = ready.take() {
                let _ = tx.send(());
            }
            continue;
        }
        if out.blocking_send(message).is_err() {
            break;
        }
    }
}

/// Hand the major in the readiness reply to the stealth identity.
///
/// The reply to `Browser.getVersion` carries `product` (`Chrome/152.0.7977.82`)
/// and used to be dropped here. Reading it costs nothing the bridge was not
/// already paying, and it is the only place a launch names its real major
/// without spawning `chrome --version`.
fn record_chrome_major(message: &[u8]) {
    if let Some(major) = major_from_version_reply(message) {
        crate::native::stealth::record_launched_chrome_major(major);
    }
}

/// Chrome major in a `Browser.getVersion` reply, or `None` for any other shape.
fn major_from_version_reply(message: &[u8]) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(message)
        .ok()?
        .pointer("/result/product")
        .and_then(serde_json::Value::as_str)
        .and_then(crate::native::stealth::chrome_major_from_product)
}

/// Write each message followed by the zero terminator Chrome splits on.
fn write_to_chrome(mut write: impl Write, mut input: tokio::sync::mpsc::Receiver<Vec<u8>>) {
    block_sigpipe_on_this_thread();
    while let Some(message) = input.blocking_recv() {
        let sent = write
            .write_all(&message)
            .and_then(|()| write.write_all(&[0]))
            .and_then(|()| write.flush());
        if sent.is_err() {
            break;
        }
    }
}

/// Turn a write to a dead Chrome into `EPIPE` instead of a process-wide kill.
///
/// `entry.rs` restores the default `SIGPIPE` disposition so a closed stdout
/// ends the CLI with exit 141. That same disposition killed the whole CLI when
/// Chrome exited before the writer's first write — measured in 26 of 40 runs
/// with `chrome_path=/usr/bin/false`, leaving no envelope and a profile on
/// disk. A pipe write raises `SIGPIPE` on the thread that wrote, so blocking
/// it here, and only here, keeps the exit-141 contract for everything else.
#[cfg(unix)]
fn block_sigpipe_on_this_thread() {
    // SAFETY:
    // - Contract: add SIGPIPE to this thread's signal mask.
    // - Invariant: `set` is a valid, initialised local sigset for both calls,
    //   and a null old-set pointer is permitted.
    // - See: `man 3 pthread_sigmask`, `man 7 signal` (thread-directed SIGPIPE).
    let mut set: libc::sigset_t = unsafe { std::mem::zeroed() };
    // SAFETY: as above; `set` is a valid local.
    unsafe { libc::sigemptyset(&mut set) };
    // SAFETY: as above; SIGPIPE is a valid signal number.
    unsafe { libc::sigaddset(&mut set, libc::SIGPIPE) };
    // SAFETY: as above; a null old-set pointer is permitted.
    unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut()) };
}

/// Windows has no `SIGPIPE`: a write to a closed pipe already returns an error.
#[cfg(not(unix))]
fn block_sigpipe_on_this_thread() {}

/// Byte-substring test without allocating.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// The pipe's own message ceiling, applied to the WebSocket half as well.
fn websocket_config() -> WebSocketConfig {
    let ceiling = crate::constants::CDP_PIPE_MAX_MESSAGE_BYTES;
    WebSocketConfig::default()
        .max_message_size(Some(ceiling))
        .max_frame_size(Some(ceiling))
}

/// Accept the one valid client, then relay until either side ends.
async fn relay(
    listener: tokio::net::TcpListener,
    path: String,
    accept_deadline: Duration,
    mut from_chrome: tokio::sync::mpsc::Receiver<Vec<u8>>,
    to_chrome: tokio::sync::mpsc::Sender<Vec<u8>>,
) {
    let accepted = tokio::time::timeout(accept_deadline, accept_one(&listener, &path)).await;
    // No second client, ever: the listener closes the moment one is chosen.
    drop(listener);
    let Ok(Some(socket)) = accepted else {
        return;
    };
    let (mut sink, mut stream) = socket.split();
    loop {
        tokio::select! {
            outgoing = from_chrome.recv() => match outgoing {
                Some(bytes) => match String::from_utf8(bytes) {
                    Ok(text) => {
                        if sink.send(Message::text(text)).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => tracing::warn!(
                        target: "browser_automation_cli::cdp_pipe",
                        "dropped a non-UTF-8 message from Chrome"
                    ),
                },
                None => {
                    let _ = sink.close(None).await;
                    break;
                }
            },
            incoming = stream.next() => match incoming {
                Some(Ok(frame)) if frame.is_text() || frame.is_binary() => {
                    if to_chrome.send(frame.into_data().to_vec()).await.is_err() {
                        break;
                    }
                }
                Some(Ok(frame)) if frame.is_close() => break,
                Some(Ok(_)) => {}
                Some(Err(_)) | None => break,
            },
        }
    }
}

/// Accept connections until one completes the handshake on `path`.
async fn accept_one(
    listener: &tokio::net::TcpListener,
    path: &str,
) -> Option<
    async_tungstenite::WebSocketStream<
        async_tungstenite::tokio::TokioAdapter<tokio::net::TcpStream>,
    >,
> {
    let handshake_budget = Duration::from_millis(crate::constants::CDP_PIPE_HANDSHAKE_TIMEOUT_MS);
    loop {
        let (stream, _) = listener.accept().await.ok()?;
        let expected = path.to_string();
        let check = move |request: &Request, response: Response| {
            if request.uri().path() == expected {
                Ok(response)
            } else {
                let mut refused = ErrorResponse::new(None);
                *refused.status_mut() = StatusCode::FORBIDDEN;
                Err(refused)
            }
        };
        let handshake = async_tungstenite::tokio::accept_hdr_async_with_config(
            stream,
            check,
            Some(websocket_config()),
        );
        if let Ok(Ok(socket)) = tokio::time::timeout(handshake_budget, handshake).await {
            return Some(socket);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::create;
    use super::*;

    #[test]
    fn the_readiness_reply_names_the_launched_major() {
        // Shape of a real `Browser.getVersion` reply on the pipe; the product
        // string is the one measured on this host.
        let reply = br#"{"id":2000000000,"result":{"protocolVersion":"1.3","product":"Chrome/152.0.7977.82","revision":"@0","userAgent":"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/152.0.0.0 Safari/537.36","jsVersion":"15.2"}}"#;
        assert_eq!(major_from_version_reply(reply).as_deref(), Some("152"));
        assert_eq!(
            major_from_version_reply(br#"{"id":2000000000,"result":{}}"#),
            None
        );
        assert_eq!(major_from_version_reply(b"not json"), None);
    }

    #[test]
    fn messages_are_split_on_the_zero_byte() {
        let input: &[u8] = b"{\"a\":1}\0{\"id\":2000000000,\"result\":{}}\0{\"b\":2}\0";
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        let (ready_tx, mut ready_rx) = tokio::sync::oneshot::channel();
        read_from_chrome(input, &tx, ready_tx);
        assert_eq!(rx.try_recv().ok(), Some(b"{\"a\":1}".to_vec()));
        assert_eq!(
            rx.try_recv().ok(),
            Some(b"{\"b\":2}".to_vec()),
            "the probe reply is consumed"
        );
        assert!(
            ready_rx.try_recv().is_ok(),
            "the probe reply signals readiness"
        );
    }

    #[test]
    fn a_message_cut_by_end_of_file_is_not_forwarded() {
        let input: &[u8] = b"{\"a\":1}\0{\"trunc";
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        let (ready_tx, _ready_rx) = tokio::sync::oneshot::channel();
        read_from_chrome(input, &tx, ready_tx);
        assert_eq!(rx.try_recv().ok(), Some(b"{\"a\":1}".to_vec()));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn written_messages_carry_the_terminator() {
        let (tx, rx) = tokio::sync::mpsc::channel(8);
        tx.try_send(b"x".to_vec()).expect("queued");
        tx.try_send(b"yz".to_vec()).expect("queued");
        drop(tx);
        let mut out = Vec::new();
        write_to_chrome(&mut out, rx);
        assert_eq!(out, b"x\0yz\0");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_bridge_refuses_a_wrong_path_and_serves_the_right_one() {
        let Ok((child, parent)) = create() else {
            crate::test_utils::skip_unit_test("cdp_pipe", "pipes could not be created.");
            return;
        };
        let mut bridge = PipeBridge::start(parent, Duration::from_secs(10))
            .await
            .expect("bridge starts");
        let url = bridge.ws_url().to_string();
        let wrong = url
            .rsplit_once('/')
            .map(|(base, _)| format!("{base}/guess"))
            .unwrap_or_default();
        assert!(
            async_tungstenite::tokio::connect_async(wrong.as_str())
                .await
                .is_err(),
            "a path without the token must be refused"
        );
        assert!(
            async_tungstenite::tokio::connect_async(url.as_str())
                .await
                .is_ok(),
            "the tokened path must be accepted"
        );
        // Closing Chrome's ends is what a dead Chrome looks like: the reader
        // sees end-of-file and the writer's next write fails, so both threads
        // can be joined instead of hanging the test.
        drop(child);
        bridge.shutdown();
    }

    /// Teardown is bounded even when something still holds Chrome's pipe ends.
    ///
    /// Reproduces the measured hang: a descendant that inherited descriptor 4
    /// keeps the reader from ever seeing end-of-file, and on a current-thread
    /// runtime the aborted relay cannot drop the sender the writer waits on.
    /// Both used to block `shutdown` for as long as the holder lived.
    #[tokio::test(flavor = "current_thread")]
    async fn shutdown_returns_within_its_grace_while_the_pipe_is_still_held() {
        let Ok((child, parent)) = create() else {
            crate::test_utils::skip_unit_test("cdp_pipe", "pipes could not be created.");
            return;
        };
        let mut bridge = PipeBridge::start(parent, Duration::from_secs(10))
            .await
            .expect("bridge starts");
        let grace = Duration::from_millis(crate::constants::CDP_PIPE_SHUTDOWN_GRACE_MS);
        let started = std::time::Instant::now();
        bridge.shutdown();
        let elapsed = started.elapsed();
        assert!(
            elapsed < grace + Duration::from_secs(1),
            "shutdown blocked for {elapsed:?} while the pipe was held open"
        );
        drop(child);
    }
}
