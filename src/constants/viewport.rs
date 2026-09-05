// SPDX-License-Identifier: MIT OR Apache-2.0
//! Viewport defaults and the `WxHxDPR[,flags]` spec parser.

/// Default headless Chrome window width when launch options omit viewport.
///
/// Matches [`crate::constants::DEFAULT_XVFB_WIDTH`]: the process `--window-size`
/// and the launch device-metrics default are the same desktop size. Operators
/// override per-command via `--viewport WxH…` / run JSON — not a product env var.
/// XDG `screen` is a separate knob (`screen.width`/`height`, never smaller).
pub const DEFAULT_VIEWPORT_WIDTH: u32 = 1920;

/// Default headless Chrome window height when launch options omit viewport.
pub const DEFAULT_VIEWPORT_HEIGHT: u32 = 1080;

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
