// SPDX-License-Identifier: MIT OR Apache-2.0
//! OneShotSession methods (componentized; single-responsibility impl blocks).

use rustc_hash::FxHashMap;
use serde_json::{json, Value};

use crate::error::{CliError, ErrorKind};
use crate::native::network;

use super::super::OneShotSession;

impl OneShotSession {
    // Mirrors the `emulate` argv surface 1:1 (UA, locale, timezone, geo, media,
    // network, CPU, color scheme, headers, viewport).
    #[allow(clippy::too_many_arguments)]
    /// Apply device/network/geolocation emulation presets via CDP.
    pub async fn emulate(
        &mut self,
        user_agent: Option<&str>,
        locale: Option<&str>,
        timezone: Option<&str>,
        offline: bool,
        latitude: Option<f64>,
        longitude: Option<f64>,
        media: Option<&str>,
        network_conditions: Option<&str>,
        cpu_throttling_rate: Option<f64>,
        color_scheme: Option<&str>,
        extra_headers_json: Option<&str>,
        viewport: Option<&str>,
    ) -> Result<Value, CliError> {
        self.drain_events();
        let session_id = self
            .manager
            .active_session_id()
            .map_err(|e| CliError::new(ErrorKind::Browser, e))?
            .to_string();
        if let Some(ua) = user_agent {
            if ua.is_empty() {
                // clear override with empty UA not portable; skip
            } else {
                self.manager
                    .set_user_agent(ua)
                    .await
                    .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate ua: {e}")))?;
            }
        }
        if let Some(loc) = locale {
            self.manager
                .set_locale(loc)
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate locale: {e}")))?;
        }
        if let Some(tz) = timezone {
            self.manager
                .set_timezone(tz)
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate timezone: {e}")))?;
        }

        let mut applied_network = None;
        if let Some(name) = network_conditions {
            let preset = crate::constants::network_preset_by_name(name).ok_or_else(|| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    format!("unknown network conditions: {name}"),
                    format!(
                        "Use one of: {}",
                        crate::constants::network_preset_names().join(", ")
                    ),
                )
            })?;
            network::set_network_conditions(
                &self.manager.client,
                &session_id,
                preset.offline,
                preset.latency_ms,
                preset.download_throughput,
                preset.upload_throughput,
            )
            .await
            .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate network: {e}")))?;
            applied_network = Some(preset.name);
        } else if offline {
            network::set_offline(&self.manager.client, &session_id, true)
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate offline: {e}")))?;
            applied_network = Some("Offline");
        }

        if let Some(rate) = cpu_throttling_rate {
            let rate = rate.clamp(1.0, 20.0);
            network::set_cpu_throttling_rate(&self.manager.client, &session_id, rate)
                .await
                .map_err(|e| {
                    CliError::new(ErrorKind::Browser, format!("emulate cpu throttle: {e}"))
                })?;
        }

        if let (Some(lat), Some(lon)) = (latitude, longitude) {
            self.manager
                .set_geolocation(lat, lon, Some(1.0))
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate geo: {e}")))?;
        }

        if let Some(scheme) = color_scheme {
            let value = match scheme.to_ascii_lowercase().as_str() {
                "dark" => "dark",
                "light" => "light",
                "auto" => "",
                other => {
                    return Err(CliError::with_suggestion(
                        ErrorKind::Usage,
                        format!("invalid color-scheme: {other}"),
                        crate::i18n::suggestion_key("use_listed_value", None),
                    ));
                }
            };
            self.manager
                .set_emulated_media(
                    media,
                    Some(vec![("prefers-color-scheme".into(), value.into())]),
                )
                .await
                .map_err(|e| {
                    CliError::new(ErrorKind::Browser, format!("emulate color-scheme: {e}"))
                })?;
        } else if let Some(m) = media {
            self.manager
                .set_emulated_media(Some(m), None)
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate media: {e}")))?;
        }

        if let Some(headers_raw) = extra_headers_json {
            let map: FxHashMap<String, String> = if headers_raw.trim().is_empty() {
                FxHashMap::default()
            } else {
                crate::json_util::from_str(headers_raw).map_err(|e| {
                    CliError::with_suggestion(
                        ErrorKind::Usage,
                        format!("invalid extra-headers JSON: {e}"),
                        r#"Pass object JSON e.g. {"X-Custom":"1"}"#,
                    )
                })?
            };
            network::set_extra_headers(&self.manager.client, &session_id, &map)
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate headers: {e}")))?;
        }

        let mut applied_viewport = None;
        if let Some(vp) = viewport {
            let spec = crate::constants::parse_viewport_spec(vp).map_err(|e| {
                CliError::with_suggestion(
                    ErrorKind::Usage,
                    e,
                    crate::i18n::suggestion_key("viewport_spec_format", None),
                )
            })?;
            self.manager
                .set_viewport(
                    spec.width,
                    spec.height,
                    spec.device_scale_factor,
                    spec.mobile,
                )
                .await
                .map_err(|e| CliError::new(ErrorKind::Browser, format!("emulate viewport: {e}")))?;
            applied_viewport = Some(json!({
                "width": spec.width,
                "height": spec.height,
                "device_scale_factor": spec.device_scale_factor,
                "mobile": spec.mobile,
                "has_touch": spec.has_touch,
                "is_landscape": spec.is_landscape,
            }));
        }

        Ok(json!({
            "emulated": true,
            "user_agent": user_agent,
            "locale": locale,
            "timezone": timezone,
            "offline": offline || applied_network == Some("Offline"),
            "latitude": latitude,
            "longitude": longitude,
            "media": media,
            "network_conditions": applied_network,
            "cpu_throttling_rate": cpu_throttling_rate,
            "color_scheme": color_scheme,
            "extra_headers": extra_headers_json.is_some(),
            "viewport": applied_viewport,
        }))
    }
}
