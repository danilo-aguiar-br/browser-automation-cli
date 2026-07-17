//! Binary entry for `browser-automation-cli`.
//!
//! Delegates to [`browser_automation_cli::run`].

fn main() -> std::process::ExitCode {
    browser_automation_cli::run()
}
