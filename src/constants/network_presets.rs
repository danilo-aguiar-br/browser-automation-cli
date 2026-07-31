// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chrome DevTools-style network throttling presets and lookup helpers.

/// Network throttling presets aligned with Chrome DevTools / Puppeteer PredefinedNetworkConditions.
#[derive(Debug, Clone, Copy)]
pub struct NetworkPreset {
    /// Human-readable preset name (`Slow 3G`, `Fast 4G`, ...).
    pub name: &'static str,
    /// When true, network is forced offline.
    pub offline: bool,
    /// Extra RTT latency in milliseconds.
    pub latency_ms: f64,
    /// Download throughput in bytes/sec (`-1.0` = unlimited).
    pub download_throughput: f64,
    /// Upload throughput in bytes/sec (`-1.0` = unlimited).
    pub upload_throughput: f64,
}

/// Throughput -1 means no throttle (unlimited).
///
/// Compile-time table of Chrome DevTools-style network conditions. Values are
/// pure `Copy` data inlined via `const` (no identity / no interior mutability).
pub const NETWORK_PRESETS: &[NetworkPreset] = &[
    NetworkPreset {
        name: "No throttling",
        offline: false,
        latency_ms: 0.0,
        download_throughput: -1.0,
        upload_throughput: -1.0,
    },
    NetworkPreset {
        name: "Offline",
        offline: true,
        latency_ms: 0.0,
        download_throughput: 0.0,
        upload_throughput: 0.0,
    },
    NetworkPreset {
        name: "Slow 3G",
        offline: false,
        latency_ms: 400.0,
        download_throughput: (500.0 * 1024.0) / 8.0 * 0.8,
        upload_throughput: (500.0 * 1024.0) / 8.0 * 0.8,
    },
    NetworkPreset {
        name: "Fast 3G",
        offline: false,
        latency_ms: 150.0,
        download_throughput: (1.6 * 1024.0 * 1024.0) / 8.0 * 0.9,
        upload_throughput: (750.0 * 1024.0) / 8.0 * 0.9,
    },
    NetworkPreset {
        name: "Slow 4G",
        offline: false,
        latency_ms: 20.0,
        download_throughput: (1.6 * 1024.0 * 1024.0) / 8.0,
        upload_throughput: (750.0 * 1024.0) / 8.0,
    },
    NetworkPreset {
        name: "Fast 4G",
        offline: false,
        latency_ms: 20.0,
        download_throughput: (9.0 * 1024.0 * 1024.0) / 8.0,
        upload_throughput: (1.5 * 1024.0 * 1024.0) / 8.0,
    },
];

// Build-time invariants for the network preset table.
const _: () = assert!(!NETWORK_PRESETS.is_empty());
const _: () = assert!(NETWORK_PRESETS.len() == 6);
const _: () = assert!(NETWORK_PRESETS[0].download_throughput < 0.0); // unlimited sentinel
const _: () = assert!(NETWORK_PRESETS[1].offline);

/// Lookup a network preset by case-insensitive name.
pub fn network_preset_by_name(name: &str) -> Option<&'static NetworkPreset> {
    let n = name.trim();
    NETWORK_PRESETS
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(n))
}

/// List known network preset names for help and validation.
pub fn network_preset_names() -> Vec<&'static str> {
    NETWORK_PRESETS.iter().map(|p| p.name).collect()
}
