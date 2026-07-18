//! Binary entry for `browser-automation-cli`.
//!
//! Delegates to [`browser_automation_cli::run`].

fn main() -> std::process::ExitCode {
    // Friendly panic reports in release builds (rules_rust_cli_com_clap).
    // With `panic = "abort"` the hook still runs before process abort.
    human_panic::setup_panic!(human_panic::Metadata::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
    .authors(env!("CARGO_PKG_AUTHORS"))
    .homepage(env!("CARGO_PKG_HOMEPAGE")));

    browser_automation_cli::run()
}
