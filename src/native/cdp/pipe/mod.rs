// SPDX-License-Identifier: MIT OR Apache-2.0
//! DevTools over a pipe instead of a TCP port.
//!
//! # The defect this closes
//!
//! Chrome was launched with `--remote-debugging-port` on `127.0.0.1`. Measured
//! on 2026-09-13: while a one-shot command ran, `GET /json/version` on that port
//! answered `Chrome/152.0.7977.82` and a `webSocketDebuggerUrl`, with no
//! authentication. Loopback is not a user boundary, so any local process — of
//! any user — could take full control of the browser, logged-in sessions
//! included, for as long as the command lived.
//!
//! `--remote-debugging-pipe` opens no listener at all. Chrome reads commands
//! from descriptor 3 and writes replies to descriptor 4, each message a JSON
//! text followed by a zero byte (`content/browser/devtools/
//! devtools_agent_host_impl.cc` and `devtools_pipe_handler.cc`). On Windows the
//! two handles arrive as integers in `--remote-debugging-io-pipes`.
//!
//! # Why a bridge, and why it is still safe
//!
//! `chromiumoxide` 0.9 speaks WebSocket only. [`PipeBridge`] therefore serves
//! a WebSocket on loopback, at a path holding the 122 random bits of a v4
//! UUID, and relays exactly one valid client to the pipe. Unlike Chrome's port
//! it exposes no discovery endpoint, answers every other path with 403, and
//! closes its listener once that client completes the handshake.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

mod bridge;

pub use bridge::PipeBridge;

#[cfg(not(any(unix, windows)))]
pub use other::{create, ChildEnds, ParentEnds};
#[cfg(unix)]
pub use unix::{create, install_child_ends, ChildEnds, ParentEnds};
#[cfg(windows)]
pub use windows::{create, ChildEnds, ParentEnds};

/// Hosts with neither descriptors nor handles to pass: the pipe is refused.
#[cfg(not(any(unix, windows)))]
mod other {
    /// No child ends can exist on this host.
    pub enum ChildEnds {}
    /// No parent ends can exist on this host.
    pub enum ParentEnds {}

    impl ParentEnds {
        /// Uninhabited: no value of this type can be built here.
        pub fn into_files(self) -> (std::fs::File, std::fs::File) {
            match self {}
        }
    }

    /// The DevTools pipe needs POSIX descriptors or Windows handles.
    ///
    /// # Errors
    ///
    /// Always.
    pub fn create() -> std::io::Result<(ChildEnds, ParentEnds)> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "DevTools pipe needs POSIX descriptors or Windows handles",
        ))
    }
}

/// The Chrome switches that select the pipe transport for these child ends.
#[must_use]
pub fn chrome_switches(ends: &ChildEnds) -> Vec<String> {
    #[cfg(windows)]
    return vec![
        "--remote-debugging-pipe".to_string(),
        ends.io_pipes_switch(),
    ];
    #[cfg(not(windows))]
    {
        let _ = ends;
        vec!["--remote-debugging-pipe".to_string()]
    }
}

/// Replace any debugging port or address in `args` with the pipe switches.
pub fn use_pipe_transport(args: &mut Vec<String>, ends: &ChildEnds) {
    args.retain(|a| {
        !a.starts_with("--remote-debugging-port=")
            && !a.starts_with("--remote-debugging-address=")
            && !a.starts_with("--remote-debugging-pipe")
            && !a.starts_with("--remote-debugging-io-pipes=")
    });
    args.extend(chrome_switches(ends));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_port_switches_are_replaced_by_the_pipe() {
        let Ok((child, _parent)) = create() else {
            crate::test_utils::skip_unit_test("cdp_pipe", "pipes could not be created.");
            return;
        };
        let mut args = vec![
            "--remote-debugging-port=0".to_string(),
            "--remote-debugging-address=127.0.0.1".to_string(),
            "--no-first-run".to_string(),
        ];
        use_pipe_transport(&mut args, &child);
        assert!(!args
            .iter()
            .any(|a| a.starts_with("--remote-debugging-port")));
        assert!(!args
            .iter()
            .any(|a| a.starts_with("--remote-debugging-address")));
        assert_eq!(
            args.iter()
                .filter(|a| *a == "--remote-debugging-pipe")
                .count(),
            1
        );
        assert!(args.iter().any(|a| a == "--no-first-run"));
    }
}
