//! Named constants for `browser-automation-cli` (anti-hardcode).

/// Network throttling presets aligned with Chrome DevTools / Puppeteer PredefinedNetworkConditions.
#[derive(Debug, Clone, Copy)]
pub struct NetworkPreset {
    pub name: &'static str,
    pub offline: bool,
    pub latency_ms: f64,
    pub download_throughput: f64,
    pub upload_throughput: f64,
}

/// Throughput -1 means no throttle (unlimited).
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

pub fn network_preset_by_name(name: &str) -> Option<&'static NetworkPreset> {
    let n = name.trim();
    NETWORK_PRESETS
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(n))
}

pub fn network_preset_names() -> Vec<&'static str> {
    NETWORK_PRESETS.iter().map(|p| p.name).collect()
}

/// Parsed viewport string: `WxHxDPR[,mobile][,touch][,landscape]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportSpec {
    pub width: i32,
    pub height: i32,
    pub device_scale_factor: f64,
    pub mobile: bool,
    pub has_touch: bool,
    pub is_landscape: bool,
}

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
