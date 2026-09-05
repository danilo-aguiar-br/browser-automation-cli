// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`CdpClient`] ownership shell (tokio mutex + handler tasks).

use std::sync::Arc;

use chromiumoxide::browser::Browser;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

use super::super::types::CdpEvent;

/// CDP client wrapping a shared chromiumoxide [`Browser`].
///
/// # Interior mutability
///
/// `browser` uses **`tokio::sync::Mutex`** because guards are held across
/// `.await` points. A `std::sync::Mutex` here would block the async runtime.
///
/// # Ownership
///
/// Holds the event-handler task and shared browser mutex — do not discard
/// without FINALIZE (`#[must_use]`).
#[must_use = "CdpClient owns the CDP connection and handler tasks"]
pub struct CdpClient {
    pub(crate) browser: Arc<Mutex<Browser>>,
    pub(crate) event_tx: broadcast::Sender<CdpEvent>,
    /// Event pump. Named (not `_handler`) because FINALIZE aborts it explicitly
    /// via [`CdpClient::stop_event_pump`] before the transport is torn down.
    pub(crate) handler: JoinHandle<()>,
    /// Per-page event relays.
    ///
    /// The underscore is kept because the field is never read by name — the
    /// initializer lives in `connect.rs` and renaming it would touch a module
    /// this change does not own. [`Drop`] below is what actually ends them.
    pub(crate) _event_forwarders: Vec<JoinHandle<()>>,
}

/// Ends every task this client started, so none of them outlives the process.
///
/// # Why the forwarders needed this
///
/// FINALIZE aborts the event pump through
/// [`CdpClient::stop_event_pump`](CdpClient::stop_event_pump), but the per-page
/// forwarders were only ever parked in a field with an underscore and dropped.
/// Dropping a [`JoinHandle`] detaches the task, it does not stop it: each
/// forwarder stayed subscribed to its page's CDP stream until the runtime itself
/// went away. That is a residual by the product's own definition — a task that
/// survives the command that created it — and it is invisible to the residual
/// scanner, which looks for processes and files.
///
/// Abort is safe on a task that already finished, and safe to call from a
/// non-async context, so this holds whether teardown ran or the client was
/// dropped on an error path that never reached FINALIZE.
impl Drop for CdpClient {
    fn drop(&mut self) {
        self.handler.abort();
        for forwarder in &self._event_forwarders {
            forwarder.abort();
        }
    }
}
