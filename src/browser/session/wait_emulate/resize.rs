// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};

use super::super::OneShotSession;

impl OneShotSession {
    /// Resize the viewport (width, height, optional deviceScaleFactor).
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
        Ok(json!({
            "width": width,
            "height": height,
            "scale": scale,
            "mobile": mobile,
        }))
    }
}
