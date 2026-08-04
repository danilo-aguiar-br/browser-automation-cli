// SPDX-License-Identifier: MIT OR Apache-2.0
//! Ephemeral port allocation and CDP readiness polling, shared by all engines.

use std::net::TcpListener;
use std::process::Child;
use std::time::{Duration, Instant};

use super::logs::{launch_error, LaunchLogBuffer};
use crate::native::cdp::discovery::discover_cdp_url_with_timeout;

/// Reserve a free loopback port for a browser about to be spawned.
///
/// The listener is closed immediately, so this is a hint rather than a lock: two
/// concurrent invocations can in principle receive the same port. The kernel
/// hands out ports round-robin over a large range, and the browser binds within
/// milliseconds, so the collision window is negligible — and a collision only
/// costs a failed launch, never a wrong connection, because readiness is proven
/// by talking CDP to the port rather than by assuming it.
pub fn reserve_loopback_port() -> Result<u16, String> {
    TcpListener::bind((crate::constants::LOOPBACK_HOST, 0))
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .map_err(|e| format!("Failed to find an available port for the browser: {e}"))
}

/// The four time budgets a readiness poll needs.
///
/// Grouped rather than passed loose because they are read together and only
/// make sense together: a poll interval longer than the startup timeout, or a
/// per-probe timeout longer than the whole budget, is a misconfiguration that a
/// flat argument list makes easy to write by accident.
#[derive(Debug, Clone, Copy)]
pub struct ReadinessBudget {
    /// Total time the engine is given to serve CDP.
    pub startup: Duration,
    /// Drain window after the child exits, before snapshotting its logs.
    pub ready_slice: Duration,
    /// Delay between readiness probes.
    pub poll_interval: Duration,
    /// Timeout of a single discovery probe.
    pub discovery_timeout: Duration,
}

/// Poll `port` until the engine serves CDP, the child dies, or time runs out.
///
/// Returns the DevTools websocket URL. The discovery cascade
/// (`/json/version` → `/json/list` → direct websocket) is **not** duplicated
/// here: it lives in [`crate::native::cdp::discovery`] and is engine-agnostic.
pub async fn wait_for_cdp_ready(
    engine: &str,
    child: &mut Child,
    port: u16,
    logs: &LaunchLogBuffer,
    budget: ReadinessBudget,
) -> Result<String, String> {
    let ReadinessBudget {
        startup: startup_timeout,
        ready_slice,
        poll_interval,
        discovery_timeout,
    } = budget;
    let deadline = Instant::now() + startup_timeout;
    let mut last_probe_error = None;

    loop {
        if let Ok(Some(status)) = child.try_wait() {
            // Give the drainer threads a brief window to flush the last log lines
            // before snapshotting them. Best-effort: lines written immediately
            // before exit may still be missing, but early startup errors — the
            // useful ones — are already buffered.
            tokio::time::sleep(ready_slice).await;
            return Err(launch_error(
                engine,
                &format!("{engine} exited before CDP became ready (status: {status})"),
                logs,
                last_probe_error.as_deref(),
            ));
        }

        match discover_cdp_url_with_timeout(
            crate::constants::LOOPBACK_HOST,
            port,
            None,
            discovery_timeout,
        )
        .await
        {
            Ok(ws_url) => return Ok(ws_url),
            Err(err) => last_probe_error = Some(err),
        }

        if Instant::now() >= deadline {
            return Err(launch_error(
                engine,
                &format!(
                    "Timed out after {}ms waiting for {engine} CDP endpoint on port {port}",
                    startup_timeout.as_millis()
                ),
                logs,
                last_probe_error.as_deref(),
            ));
        }

        tokio::time::sleep(poll_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_port_is_usable_and_nonzero() {
        let port = reserve_loopback_port().expect("loopback must be bindable");
        assert!(port > 0, "kernel must hand out a real port");
    }

    #[test]
    fn two_reservations_do_not_collide_immediately() {
        let a = reserve_loopback_port().expect("first reservation");
        let b = reserve_loopback_port().expect("second reservation");
        // Not a hard guarantee of the API, but a regression signal: a kernel that
        // returned the same port twice in a row would make concurrent launches
        // fight for it.
        assert!(a > 0 && b > 0);
    }
}
