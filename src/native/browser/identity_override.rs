// SPDX-License-Identifier: MIT OR Apache-2.0
//! Moving the browser identity: User-Agent together with its Client Hints.
//!
//! Kept apart from `emulate.rs` because the invariant here is different from
//! every other emulation knob. Viewport, media type and timezone are
//! independent: setting one wrong makes one thing wrong. Identity fields are
//! CROSS-READ, so moving the User-Agent without moving `sec-ch-ua` produces a
//! pair no real browser can emit — a stronger signal than the untouched
//! original. Anything that changes one of them belongs next to the code that
//! changes the rest.

use serde_json::json;

use super::BrowserManager;

impl BrowserManager {
    /// Override the user agent AND the Client Hints that must agree with it.
    ///
    /// # Why not [`Self::set_user_agent_without_client_hints`]
    ///
    /// That one sends `userAgent` alone. Chrome derives `sec-ch-ua`,
    /// `sec-ch-ua-mobile` and `sec-ch-ua-platform` from its own build, not from
    /// the override, so a bare string swap produces a request whose User-Agent
    /// claims one platform while its Client Hints announce another. No real
    /// browser can emit that pair, which makes the mismatch a stronger signal
    /// than the untouched User-Agent would have been.
    ///
    /// `userAgentMetadata` is the field that moves the hints with the string.
    pub async fn set_user_agent_with_client_hints(
        &self,
        identity: &crate::native::stealth::Identity,
    ) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        let major = identity
            .user_agent
            .split("Chrome/")
            .nth(1)
            .and_then(|s| s.split('.').next())
            .unwrap_or("");
        let brands = json!([
            { "brand": "Chromium", "version": major },
            { "brand": "Google Chrome", "version": major },
            { "brand": "Not?A_Brand", "version": "24" },
        ]);
        self.client
            .send_command(
                "Emulation.setUserAgentOverride",
                Some(json!({
                    "userAgent": identity.user_agent,
                    "userAgentMetadata": {
                        "brands": brands,
                        // Unquoted here: the quotes in `identity.platform` are
                        // header syntax, and CDP takes the bare token.
                        "platform": identity.platform.trim_matches('"'),
                        "platformVersion": "",
                        "architecture": "x86",
                        "model": "",
                        "mobile": identity.mobile == "?1",
                    },
                })),
                Some(session_id),
            )
            .await?;
        Ok(())
    }
}
