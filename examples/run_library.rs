//! Library entry example: parse argv and run the one-shot CLI.
//!
//! ```bash
//! cargo run --example run_library -- doctor --offline --quick --json
//! ```

fn main() -> std::process::ExitCode {
    browser_automation_cli::run()
}
