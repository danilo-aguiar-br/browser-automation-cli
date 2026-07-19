// SPDX-License-Identifier: MIT OR Apache-2.0
//! Named constants for `browser-automation-cli` (anti-hardcode).

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

/// Parsed viewport string: `WxHxDPR[,mobile][,touch][,landscape]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportSpec {
    /// CSS width in pixels.
    pub width: i32,
    /// CSS height in pixels.
    pub height: i32,
    /// Device pixel ratio.
    pub device_scale_factor: f64,
    /// Mobile metric emulation.
    pub mobile: bool,
    /// Touch support flag.
    pub has_touch: bool,
    /// Landscape orientation flag.
    pub is_landscape: bool,
}

/// Parse `WxHxDPR` with optional `,mobile`, `,touch`, `,landscape` flags.
pub fn parse_viewport_spec(raw: &str) -> Result<ViewportSpec, String> {
    let mut parts = raw.split(',').map(|s| s.trim()).filter(|s| !s.is_empty());
    let dims = parts
        .next()
        .ok_or_else(|| "viewport empty; expected WxHxDPR".to_string())?;
    let mut nums = dims.split('x').map(|s| s.trim());
    let width: i32 = nums
        .next()
        .ok_or("viewport missing width")?
        .parse()
        .map_err(|_| "viewport width must be integer")?;
    let height: i32 = nums
        .next()
        .ok_or("viewport missing height")?
        .parse()
        .map_err(|_| "viewport height must be integer")?;
    let device_scale_factor: f64 = nums
        .next()
        .map(|s| {
            s.parse()
                .map_err(|_| "viewport dpr must be number".to_string())
        })
        .transpose()?
        .unwrap_or(1.0);
    let mut mobile = false;
    let mut has_touch = false;
    let mut is_landscape = false;
    for flag in parts {
        match flag.to_ascii_lowercase().as_str() {
            "mobile" => mobile = true,
            "touch" => has_touch = true,
            "landscape" => is_landscape = true,
            other => return Err(format!("unknown viewport flag: {other}")),
        }
    }
    Ok(ViewportSpec {
        width,
        height,
        device_scale_factor,
        mobile,
        has_touch,
        is_landscape,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_lookup() {
        assert!(network_preset_by_name("Slow 3G").is_some());
        assert!(network_preset_by_name("offline").is_some());
        assert!(network_preset_by_name("nope").is_none());
    }

    #[test]
    fn viewport_parse() {
        let v = parse_viewport_spec("412x823x1.75,mobile,touch").unwrap();
        assert_eq!(v.width, 412);
        assert_eq!(v.height, 823);
        assert!((v.device_scale_factor - 1.75).abs() < 0.001);
        assert!(v.mobile && v.has_touch);
    }
}
