// SPDX-License-Identifier: MIT OR Apache-2.0
//! On-disk shape of a portable session: cookies plus per-origin web storage.

use serde::{Deserialize, Serialize};

use crate::native::cookies::Cookie;

/// A whole browser session, serialized so another process can resume it.
///
/// This is the file `--storage-state` writes and reads. It carries live
/// credentials, so it is written with owner-only permissions and never logged.
#[derive(Debug, Serialize, Deserialize)]
pub struct StorageState {
    /// Every cookie in the profile, across origins.
    pub cookies: Vec<Cookie>,
    /// Web storage, one entry per origin that had any.
    pub origins: Vec<OriginStorage>,
}

/// Web storage belonging to one origin.
///
/// Storage is partitioned by origin in the browser, and restoring it has to
/// respect that: entries cannot be replayed into a page from a different origin.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginStorage {
    /// Origin these entries belong to, as `scheme://host[:port]`.
    pub origin: String,
    /// `localStorage` contents, which outlive the tab.
    pub local_storage: Vec<StorageEntry>,
    /// `sessionStorage` contents, which normally die with the tab.
    ///
    /// Defaulted so a state file written before this field existed still loads.
    #[serde(default)]
    pub session_storage: Vec<StorageEntry>,
}

/// One key/value pair of web storage.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageEntry {
    /// Storage key.
    pub name: String,
    /// Storage value. May be a credential; never logged.
    pub value: String,
}
