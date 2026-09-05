// SPDX-License-Identifier: MIT OR Apache-2.0
//! One identity, three surfaces: User-Agent, Client Hints, and header order.
//!
//! **Why one source.** A bot check does not read these fields, it CROSS-READS
//! them. A User-Agent claiming Windows next to a `sec-ch-ua-platform` of
//! `"Linux"` is a stronger signal than either field alone, because no real
//! browser produces that pair. The same holds for the Canvas hash and the WebGL
//! renderer, which the host GPU decides and no header can override.
//!
//! So the identity is resolved once and every surface derives from it. There is
//! deliberately no way to move the User-Agent without moving the Client Hints.
//!
//! # What the browser engine does NOT get
//!
//! Under the default `auto` profile the browser keeps Chrome's OWN User-Agent.
//! Overriding it would buy nothing and risk everything: the real UA already
//! matches the real engine, the real Canvas hash and the real TLS fingerprint,
//! and any string we substitute can only introduce a contradiction. The
//! automation markers were always the problem, not the UA.
//!
//! An explicit `chrome-win` or `chrome-mac` profile does override it, and the
//! caller is accepting a known mismatch: this product impersonates neither TLS
//! nor HTTP/2, so the transport still says Linux.
//!
//! The HTTP engine is the opposite case. It has no browser to borrow an
//! identity from, so it needs a synthesized one — see [`Identity::user_agent`].

use spider_fingerprint::configs::AgentOs;

use crate::browser_policy::StealthProfile;

/// A coherent Chrome identity for one process.
#[derive(Debug, Clone)]
pub struct Identity {
    /// Full `User-Agent` header value.
    pub user_agent: String,
    /// `sec-ch-ua` brand list, matching the User-Agent major version.
    pub brands: String,
    /// `sec-ch-ua-platform`, quoted as the header requires.
    pub platform: String,
    /// `navigator.platform`, which is NOT spelled like the header.
    pub navigator_platform: &'static str,
    /// `sec-ch-ua-mobile`.
    pub mobile: &'static str,
    /// The operating system this identity claims.
    pub agent_os: AgentOs,
}

/// Per-platform pieces of the Chrome User-Agent template.
///
/// The tail (`AppleWebKit/537.36 (KHTML, like Gecko) Chrome/… Safari/537.36`)
/// is frozen across platforms in real Chrome, so only the parenthesised
/// platform token varies.
const fn platform_tokens(os: AgentOs) -> (&'static str, &'static str, &'static str) {
    match os {
        AgentOs::Windows => ("Windows NT 10.0; Win64; x64", "\"Windows\"", "Win32"),
        AgentOs::Mac => ("Macintosh; Intel Mac OS X 10_15_7", "\"macOS\"", "MacIntel"),
        _ => ("X11; Linux x86_64", "\"Linux\"", "Linux x86_64"),
    }
}

impl Identity {
    /// Build the identity for a resolved profile.
    ///
    /// The profile is resolved again here rather than assumed: the type cannot
    /// express "already resolved", and an unresolved `Auto` would leave the
    /// platform undecided and ship the mismatch this module exists to avoid.
    #[must_use]
    pub fn for_profile(profile: StealthProfile) -> Self {
        let agent_os = match profile.resolved() {
            StealthProfile::ChromeWindows => AgentOs::Windows,
            StealthProfile::ChromeMac => AgentOs::Mac,
            StealthProfile::ChromeLinux | StealthProfile::Auto => AgentOs::Linux,
        };
        let (platform_token, platform, navigator_platform) = platform_tokens(agent_os);

        // The version comes from the crate's table rather than from a literal
        // here, so a dependency bump moves it instead of leaving a frozen
        // major behind that every request then advertises.
        let version = spider_fingerprint::spoof_user_agent::get_default_version();
        let major = version.split('.').next().unwrap_or(version);

        let user_agent = format!(
            "Mozilla/5.0 ({platform_token}) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/{major}.0.0.0 Safari/537.36"
        );
        let brands = format!(
            "\"Chromium\";v=\"{major}\", \"Google Chrome\";v=\"{major}\", \
             \"Not?A_Brand\";v=\"24\""
        );

        Self {
            user_agent,
            brands,
            platform: platform.to_string(),
            navigator_platform,
            // Desktop only. Claiming mobile would need a matching viewport,
            // touch points and device pixel ratio; asserting it without those
            // is the same cross-field contradiction this module prevents.
            mobile: "?0",
            agent_os,
        }
    }

    /// This identity's User-Agent rebuilt around a different Chrome major.
    ///
    /// The platform token, the frozen WebKit tail and the `Safari/537.36`
    /// suffix are preserved: only the version moves. Used by
    /// `doctor --fingerprint --no-stealth`, which must describe the binary on
    /// this host rather than the version the dependency happens to ship.
    #[must_use]
    pub fn user_agent_with_major(&self, major: &str) -> String {
        let (platform_token, _, _) = platform_tokens(self.agent_os);
        format!(
            "Mozilla/5.0 ({platform_token}) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/{major}.0.0.0 Safari/537.36"
        )
    }

    /// Whether the browser engine should override Chrome's own User-Agent.
    ///
    /// # The rule this used to get wrong
    ///
    /// The original rule was "the host profile needs no override", on the
    /// premise that Chrome's real UA already agrees with the real renderer and
    /// the real transport. That premise holds for a headed Chrome and fails
    /// for a headless one, which announces itself in the UA as
    /// `HeadlessChrome/<version>`.
    ///
    /// Measured on the default path: profile `auto` on Linux produced
    /// `HeadlessChrome/151.0.0.0` in `navigator.userAgent` and in the header
    /// echo, while the envelope reported `stealth: true`. The single most
    /// obvious automation marker travelled intact under a flag that claimed to
    /// hide it. The workaround available to a caller was worse: switching to
    /// `chrome-win` cleaned the UA and introduced a platform mismatch instead,
    /// so both of the product's answers leaked something.
    ///
    /// # Why the headless override does not lie about the platform
    ///
    /// [`Identity::for_profile`] derives the platform from the *resolved*
    /// profile, so on a Linux host the override still says Linux. Only the
    /// `HeadlessChrome` product token is replaced — by the `Chrome` token a
    /// headed build of the same browser would send.
    ///
    /// `headless` is passed in rather than read here so this stays a pure
    /// function of its inputs, and so a caller that already knows the launch
    /// mode cannot disagree with it.
    #[must_use]
    pub fn overrides_browser_user_agent(profile: StealthProfile, headless: bool) -> bool {
        let host = StealthProfile::Auto.resolved();
        profile.resolved() != host || headless
    }

    /// Header pairs in the exact order Chrome emits them.
    ///
    /// Order is itself a signal. Chrome sends the three Client Hints before
    /// `user-agent`, then `accept`, then the four `sec-fetch-*`. A client that
    /// leads with `user-agent` is identifiable from order alone, even when
    /// every value is correct.
    ///
    /// The four HTTP/2 pseudo-headers come first in a real request and are NOT
    /// listed: `reqwest` owns them and exposes no way to order them. That gap
    /// is why the envelope reports `tls_impersonation: false` rather than
    /// implying the transport is indistinguishable.
    #[must_use]
    pub fn chrome_header_order(&self) -> Vec<(&'static str, String)> {
        vec![
            ("sec-ch-ua", self.brands.clone()),
            ("sec-ch-ua-mobile", self.mobile.to_string()),
            ("sec-ch-ua-platform", self.platform.clone()),
            ("upgrade-insecure-requests", "1".to_string()),
            ("user-agent", self.user_agent.clone()),
            (
                "accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,\
                 image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"
                    .to_string(),
            ),
            ("sec-fetch-site", "none".to_string()),
            ("sec-fetch-mode", "navigate".to_string()),
            ("sec-fetch-user", "?1".to_string()),
            ("sec-fetch-dest", "document".to_string()),
            // Every encoding named here MUST be one the client can actually
            // decode. Advertising an encoding without the matching decoder is
            // not a disguise flaw, it is a data-corruption bug: the server
            // honours the offer and the caller receives framed bytes as text.
            // The four below are backed by the reqwest features of the same
            // name — see the dependency note in `Cargo.toml`.
            ("accept-encoding", "gzip, deflate, br, zstd".to_string()),
            ("accept-language", "en-US,en;q=0.9".to_string()),
        ]
    }
}

/// Chrome major version from a `--version` line, or `None` when it names none.
///
/// The line is vendor prose, not a contract: this host answers
/// `Chromium 151.0.7922.137 Built from source for Fedora release 44`, upstream
/// answers `Google Chrome 152.0.1`, and a wrapper may answer something else
/// entirely. Rather than match a vendor name, take the first dotted numeric
/// token — the one shape all of them share. An unparseable line returns `None`
/// so the caller can fall back and SAY it fell back, instead of inventing a
/// version out of a partial match.
#[must_use]
pub fn chrome_major_from_version_line(line: &str) -> Option<String> {
    line.split_whitespace().find_map(|token| {
        let (major, rest) = token.split_once('.')?;
        if major.is_empty()
            || !major.bytes().all(|b| b.is_ascii_digit())
            || !rest.starts_with(|c: char| c.is_ascii_digit())
        {
            return None;
        }
        Some(major.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_line_major_survives_every_vendor_spelling() {
        for (line, want) in [
            (
                "Chromium 151.0.7922.137 Built from source for Fedora release 44 (Forty Four)",
                Some("151"),
            ),
            ("Google Chrome 152.0.1", Some("152")),
            ("Chromium 151.0.7922.137", Some("151")),
            ("Microsoft Edge 140.0.3485.14 stable", Some("140")),
        ] {
            assert_eq!(
                chrome_major_from_version_line(line).as_deref(),
                want,
                "line: {line}"
            );
        }
    }

    #[test]
    fn an_unparseable_version_line_refuses_instead_of_guessing() {
        for line in [
            "",
            "   ",
            "Chromium",
            "not a version at all",
            "1.",
            ".2",
            "v.x",
        ] {
            assert_eq!(
                chrome_major_from_version_line(line),
                None,
                "line: {line:?} must not yield a major"
            );
        }
    }

    #[test]
    fn user_agent_with_major_moves_only_the_version() {
        for profile in [
            StealthProfile::ChromeLinux,
            StealthProfile::ChromeWindows,
            StealthProfile::ChromeMac,
        ] {
            let id = Identity::for_profile(profile);
            let rebuilt = id.user_agent_with_major("151");
            assert!(rebuilt.contains("Chrome/151.0.0.0"), "{rebuilt}");
            assert!(rebuilt.ends_with("Safari/537.36"), "{rebuilt}");
            // The platform token is what a mismatch would leak; it must survive.
            let (token, _, _) = platform_tokens(id.agent_os);
            assert!(rebuilt.contains(token), "{rebuilt} lost {token}");
        }
    }

    #[test]
    fn user_agent_and_platform_never_contradict() {
        for (profile, ua_needle, platform) in [
            (StealthProfile::ChromeWindows, "Windows NT", "\"Windows\""),
            (StealthProfile::ChromeMac, "Mac OS X", "\"macOS\""),
            (StealthProfile::ChromeLinux, "Linux x86_64", "\"Linux\""),
        ] {
            let id = Identity::for_profile(profile);
            assert!(
                id.user_agent.contains(ua_needle),
                "{profile:?}: {}",
                id.user_agent
            );
            assert_eq!(id.platform, platform, "{profile:?}");
        }
    }

    #[test]
    fn navigator_platform_uses_the_js_spelling_not_the_header_one() {
        // `sec-ch-ua-platform` says "Windows"; `navigator.platform` says
        // "Win32". One spelling for both is a detectable mismatch.
        let id = Identity::for_profile(StealthProfile::ChromeWindows);
        assert_eq!(id.navigator_platform, "Win32");
        assert_ne!(id.navigator_platform, id.platform);
    }

    #[test]
    fn brand_major_matches_the_user_agent_major() {
        // A `sec-ch-ua` advertising a different major than the User-Agent is
        // the single cheapest mismatch to detect.
        let id = Identity::for_profile(StealthProfile::ChromeLinux);
        let ua_major = id
            .user_agent
            .split("Chrome/")
            .nth(1)
            .and_then(|s| s.split('.').next())
            .expect("chrome token");
        assert!(
            id.brands
                .contains(&format!("\"Google Chrome\";v=\"{ua_major}\"")),
            "brands {} disagree with UA major {ua_major}",
            id.brands
        );
    }

    #[test]
    fn header_order_matches_chrome_not_alphabetical() {
        let id = Identity::for_profile(StealthProfile::ChromeLinux);
        let names: Vec<&str> = id.chrome_header_order().iter().map(|(n, _)| *n).collect();
        let ua = names.iter().position(|n| *n == "user-agent").expect("ua");
        let ch = names.iter().position(|n| *n == "sec-ch-ua").expect("ch");
        assert!(ch < ua, "sec-ch-ua must precede user-agent: {names:?}");
        assert!(names.contains(&"sec-fetch-dest"));
    }

    #[test]
    fn a_headed_host_browser_keeps_its_own_user_agent() {
        // Chrome's own UA already agrees with its renderer and its transport,
        // so substituting an equivalent string can only lose.
        assert!(!Identity::overrides_browser_user_agent(
            StealthProfile::Auto,
            false
        ));
        assert!(!Identity::overrides_browser_user_agent(
            StealthProfile::Auto.resolved(),
            false
        ));
    }

    #[test]
    fn a_headless_host_browser_must_be_overridden() {
        // The regression this guards: on the default path Chrome announced
        // itself as `HeadlessChrome/<version>` while the envelope reported
        // `stealth: true`. The loudest automation marker rode along untouched.
        assert!(Identity::overrides_browser_user_agent(
            StealthProfile::Auto,
            true
        ));
    }

    #[test]
    fn the_headless_override_does_not_invent_a_platform() {
        // Cleaning the product token must not become a platform lie: the old
        // workaround for this leak was `chrome-win`, which swapped one leak
        // for another.
        let host = StealthProfile::Auto.resolved();
        let id = Identity::for_profile(host);
        assert!(
            !id.user_agent.contains("HeadlessChrome"),
            "{}",
            id.user_agent
        );
        assert_eq!(id.agent_os, Identity::for_profile(host).agent_os);
    }

    #[test]
    fn a_foreign_profile_is_overridden_regardless_of_launch_mode() {
        for headless in [false, true] {
            assert!(
                Identity::overrides_browser_user_agent(StealthProfile::ChromeMac, headless)
                    || StealthProfile::ChromeMac.resolved() == StealthProfile::Auto.resolved()
            );
        }
    }
}
