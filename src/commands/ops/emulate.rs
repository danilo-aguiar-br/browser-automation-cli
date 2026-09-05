// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::browser::CaptureOpts;
use crate::commands::common::emit_ok;
use crate::commands::nav::with_session_blank;
use crate::error::CliError;
use crate::lifecycle::Lifecycle;

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_emulate(
    life: &Lifecycle,
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
    extra_headers: Option<&str>,
    viewport: Option<&str>,
    screen: Option<&str>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    if let Some(ua) = user_agent {
        if !ua.is_empty()
            && crate::native::stealth::ua_contradicts_profile(
                ua,
                crate::browser_policy::stealth_profile(),
            )
        {
            return Err(CliError::with_suggestion(
                crate::error::ErrorKind::Usage,
                format!(
                    "emulate --user-agent claims a platform that contradicts stealth profile `{}`",
                    crate::browser_policy::stealth_profile().as_str()
                ),
                crate::i18n::suggestion_key("stealth_profile_ua", None),
            ));
        }
    }
    let ua = user_agent.map(|s| s.to_string());
    let loc = locale.map(|s| s.to_string());
    let tz = timezone.map(|s| s.to_string());
    let media = media.map(|s| s.to_string());
    let net = network_conditions.map(|s| s.to_string());
    let scheme = color_scheme.map(|s| s.to_string());
    let headers = extra_headers.map(|s| s.to_string());
    let vp = viewport.map(|s| s.to_string());
    if let Some(raw) = screen {
        let size = crate::native::stealth::parse_screen_spec(raw).map_err(|e| {
            CliError::with_suggestion(
                crate::error::ErrorKind::Usage,
                e,
                crate::i18n::suggestion_key("screen_spec_format", None),
            )
        })?;
        crate::native::stealth::set_screen_override(
            Some(size),
            crate::native::stealth::ScreenSource::Argv,
        );
    }
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session
            .emulate(
                ua.as_deref(),
                loc.as_deref(),
                tz.as_deref(),
                offline,
                latitude,
                longitude,
                media.as_deref(),
                net.as_deref(),
                cpu_throttling_rate,
                scheme.as_deref(),
                headers.as_deref(),
                vp.as_deref(),
            )
            .await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |_| crate::output::writeln_stdout("ok emulate"))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_resize(
    life: &Lifecycle,
    width: i32,
    height: i32,
    scale: f64,
    mobile: bool,
    screen: Option<&str>,
    capture: CaptureOpts,
    timeout_secs: u64,
    json: bool,
) -> Result<(), CliError> {
    if let Some(raw) = screen {
        let size = crate::native::stealth::parse_screen_spec(raw).map_err(|e| {
            CliError::with_suggestion(
                crate::error::ErrorKind::Usage,
                e,
                crate::i18n::suggestion_key("screen_spec_format", None),
            )
        })?;
        crate::native::stealth::set_screen_override(
            Some(size),
            crate::native::stealth::ScreenSource::Argv,
        );
    }
    let data = with_session_blank(life, capture, timeout_secs, move |mut session| async move {
        let v = session.resize(width, height, scale, mobile).await?;
        Ok((session, v))
    })?;
    emit_ok(data, json, |_| {
        crate::output::writeln_stdout(format!("ok resize {width}x{height}"))
    })
}
