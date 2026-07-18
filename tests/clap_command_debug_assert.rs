//! Clap command tree integrity (rules_rust_cli_com_clap).
//!
//! Ensures the derive surface has no developer-definition bugs
//! (`Cli::command().debug_assert()`).

#[test]
fn clap_command_factory_debug_assert_passes() {
    browser_automation_cli::command_factory_debug_assert();
}
