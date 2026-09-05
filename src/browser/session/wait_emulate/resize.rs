// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Resize the viewport (width, height, optional deviceScaleFactor).
    ///
    /// # Errors
    ///
    /// Fails with [`ErrorKind::Browser`] —
    /// `"resize failed: …"` — when no page is active or
    /// `Emulation.setDeviceMetricsOverride` is refused, which is what a
    /// non-positive or absurd dimension produces.
    ///
    /// The content-area resize that follows is best-effort: it is experimental
    /// CDP, and its refusal is logged rather than returned, because the CSS
    /// viewport override the caller asked for already landed.
    pub async fn resize(
        &mut self,
        width: i32,
        height: i32,
        scale: f64,
        mobile: bool,
    ) -> Result<Value, CliError> {
        self.drain_events();
        self.manager
            .set_viewport(width, height, scale, mobile)
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("resize failed: {e}")))?;
        self.last_device_metrics = (width, height, scale, mobile);
        let (sw, sh) = crate::native::stealth::resolve_screen(width, height);
        Ok(json!({
            "width": width,
            "height": height,
            "scale": scale,
            "mobile": mobile,
            "screen": { "width": sw, "height": sh },
            // Same reason as the emulate path: the number and the request are
            // two different things whenever the floor wins.
            "screen_source": crate::native::stealth::resolved_screen_source(width, height).as_str(),
        }))
    }
}
