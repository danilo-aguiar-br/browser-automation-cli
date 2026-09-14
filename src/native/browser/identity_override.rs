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
    ///
    /// # Errors
    ///
    /// Fails with `"No active page"` when no tab is attached, or with the CDP
    /// error raised by `Emulation.setUserAgentOverride`. A `user_agent` with
    /// no parseable `Chrome/<major>` segment is **not** an error: the brand
    /// versions go out empty, which is itself a detectable identity.
    pub async fn set_user_agent_with_client_hints(
        &self,
        identity: &crate::native::stealth::Identity,
    ) -> Result<(), String> {
        let session_id = self.active_session_id()?;
        self.client
            .send_command(
                "Emulation.setUserAgentOverride",
                Some(user_agent_override_params(identity)),
                Some(session_id),
            )
            .await?;
        Ok(())
    }
}

/// CDP params that move UA, `navigator.platform`, and Client Hints together.
///
/// `platform` (not `userAgentMetadata.platform`) is what Chrome writes to
/// `navigator.platform`. Omitting it is how a Windows UA kept reporting
/// `Linux x86_64` on this host.
///
/// # Every high-entropy field is sent, not only the low-entropy ones
///
/// Chrome fills any `userAgentMetadata` field left out with the REAL binary's
/// value. Measured headless before the fields below were added: under a
/// User-Agent claiming 153, `sec-ch-ua-full-version` said `"152.0.7977.82"`
/// and `sec-ch-ua-full-version-list` named `Chromium` 152 — the host build,
/// leaking through the override. `fullVersionList` mirrors `brands` with the
/// full version, so the GREASE entry is the same one `sec-ch-ua` carries.
pub fn user_agent_override_params(
    identity: &crate::native::stealth::Identity,
) -> serde_json::Value {
    let major = crate::native::stealth::ua_chrome_major(&identity.user_agent).unwrap_or_default();
    let full_version = identity.full_version();
    // The same GREASE entry `Identity::brands` puts in `sec-ch-ua`, so the
    // header, `navigator.userAgentData.brands` and the full list agree.
    let brands = json!([
        { "brand": "Chromium", "version": major },
        { "brand": "Google Chrome", "version": major },
        { "brand": "Not?A_Brand", "version": "24" },
    ]);
    let full_version_list = json!([
        { "brand": "Chromium", "version": full_version },
        { "brand": "Google Chrome", "version": full_version },
        { "brand": "Not?A_Brand", "version": "24.0.0.0" },
    ]);
    json!({
        "userAgent": identity.user_agent,
        "platform": identity.navigator_platform,
        "userAgentMetadata": {
            "brands": brands,
            "fullVersionList": full_version_list,
            "fullVersion": full_version,
            // Unquoted here: the quotes in `identity.platform` are
            // header syntax, and CDP takes the bare token.
            "platform": identity.platform.trim_matches('"'),
            "platformVersion": identity.platform_version(),
            "architecture": "x86",
            // Every template this identity emits is a 64-bit x86 build
            // (`Win64; x64`, `Linux x86_64`, `Intel Mac OS X`), so the
            // bitness is 64 and never WOW64.
            "bitness": "64",
            "wow64": false,
            "model": "",
            "mobile": identity.mobile == "?1",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser_policy::StealthProfile;
    use crate::native::stealth::Identity;

    #[test]
    fn chrome_win_override_drags_navigator_platform() {
        let identity = Identity::for_profile(StealthProfile::ChromeWindows);
        let params = user_agent_override_params(&identity);
        assert_eq!(params["platform"], "Win32");
        assert_eq!(params["userAgentMetadata"]["platform"], "Windows");
        assert!(params["userAgent"].as_str().unwrap().contains("Windows NT"));
    }

    #[test]
    fn override_metadata_is_complete_and_matches_the_identity_major() {
        // Measured headless before this: the override sent no fullVersionList,
        // so Chrome filled `sec-ch-ua-full-version-list` and
        // `sec-ch-ua-full-version` from the REAL binary (152.0.7977.82) under a
        // User-Agent claiming 153.
        for profile in [StealthProfile::ChromeLinux, StealthProfile::ChromeWindows] {
            let identity = Identity::for_profile(profile);
            let major = identity
                .user_agent
                .split("Chrome/")
                .nth(1)
                .and_then(|s| s.split('.').next())
                .expect("major")
                .to_string();
            let params = user_agent_override_params(&identity);
            let meta = &params["userAgentMetadata"];
            let full = meta["fullVersion"].as_str().expect("fullVersion");
            assert!(full.starts_with(&format!("{major}.")), "{meta}");
            let list = meta["fullVersionList"].as_array().expect("fullVersionList");
            for brand in ["Chromium", "Google Chrome"] {
                assert!(
                    list.iter()
                        .any(|b| b["brand"] == brand && b["version"] == full),
                    "{brand} missing from {meta}"
                );
            }
            assert_eq!(meta["bitness"], "64", "{meta}");
            assert_eq!(meta["wow64"], false, "{meta}");
        }
    }

    #[test]
    fn chrome_mac_override_drags_navigator_platform() {
        let identity = Identity::for_profile(StealthProfile::ChromeMac);
        let params = user_agent_override_params(&identity);
        assert_eq!(params["platform"], "MacIntel");
        assert_eq!(params["userAgentMetadata"]["platform"], "macOS");
    }
}
