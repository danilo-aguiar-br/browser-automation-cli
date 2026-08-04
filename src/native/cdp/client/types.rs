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
    pub(crate) _event_forwarders: Vec<JoinHandle<()>>,
}
