// SPDX-License-Identifier: MIT OR Apache-2.0
//! Installing the anti-detection layer on a freshly attached session.
//!
//! Split out of `state.rs` for two reasons. The mechanical one is size: adding
//! it inline pushed that file past the 300-code-line ceiling. The real one is
//! that `state.rs` answers "which CDP domains does this session need", and this
//! answers "what does the page see before its own scripts run" — two questions
//! with no shared state and different reasons to change.

use super::super::OneShotSession;

impl OneShotSession {
    /// Install the pre-navigation patches and, when the profile is foreign, the
    /// matching User-Agent and Client Hints.
    ///
    /// # Best effort by design
    ///
    /// Neither step can fail the session. A page that refuses the script is
    /// still the page the caller asked for, and turning a hardening layer into
    /// an outage would trade a probabilistic risk for a certain one. The
    /// absence is visible through `doctor` and through a stderr warning, not
    /// through a dead command.
    pub(super) async fn install_stealth(&mut self) {
        // Anti-detection patches, installed at ATTACH and never later.
        //
        // `Page.addScriptToEvaluateOnNewDocument` runs before any page script on
        // every document, which is the only timing that works: a challenge
        // script reads `navigator.webdriver` and the plugin array during its own
        // first tick, so a patch applied after `goto` arrives to an audience
        // that already left with the answer.
        //
        // Best-effort by design. A page that cannot take the script is still a
        // page the caller asked for, and failing the whole session over a
        // hardening layer would turn a defence into an outage. The absence is
        // observable through `doctor`, not through a dead command.
        if let Some(script) = crate::native::stealth::script_for_process() {
            if let Err(e) = self.manager.add_script_to_evaluate(script).await {
                tracing::warn!(
                    target: "browser_automation_cli::stealth",
                    error = %e,
                    "stealth patches were not installed; automation markers stay visible"
                );
            }
        }

        // A foreign identity also needs the User-Agent and Client Hints moved,
        // and `Emulation.setUserAgentOverride` is the variant that carries
        // `userAgentMetadata`. The `Network` variant sets only the string, which
        // would leave `sec-ch-ua` announcing the host platform next to a
        // User-Agent claiming another one.
        if let Some(identity) = crate::native::stealth::user_agent_override() {
            if let Err(e) = self
                .manager
                .set_user_agent_with_client_hints(&identity)
                .await
            {
                tracing::warn!(
                    target: "browser_automation_cli::stealth",
                    error = %e,
                    "user agent override failed; the declared profile does not match what the page sees"
                );
            }
        }
    }
}
