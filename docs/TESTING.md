[English](TESTING.md) | [Português Brasileiro](TESTING.pt-BR.md)

# Testing — browser-automation-cli

> Run the right suite for the risk, not every browser path by default.

## Why Categorized Tests
- Browser runtime tests are slower and host-dependent
- Schema and inventory tests catch contract drift without Chrome
- Keeping categories explicit protects local iteration speed

## Test Categories
- Unit and library tests in `src/`
- CLI smoke tests such as `tests/doctor_cli.rs` and `tests/goto_smoke.rs`
- Envelope and schema gates such as `tests/envelope_schema.rs` and `tests/parity_toolref_schema.rs`
- Parity inventory and matrix tests
- Robots and pipe behaviour tests
- Optional e2e CDP event coverage when Chrome is available

## How to Run
```bash
timeout 300 cargo test --locked
timeout 120 cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
```
- Run a single file with `cargo test --test doctor_cli --locked`
- Use `-- --nocapture` only while debugging

## Local Validation Profiles
- Run fmt, clippy, and non-browser contract tests first on your machine
- Browser-backed tests require Chrome or Chromium installed locally
- Validation runs locally with cargo on the maintainer machine
- Keep crates.io publish blocked without explicit maintainer approval

## Environment Variables
- `RUST_LOG` for deeper tracing during failing tests
- `BROWSER_AUTOMATION_CLI_DEBUG` for maximum CLI stderr detail
- Host-specific Chrome path variables only when discovery needs override

## Troubleshooting
- Doctor fails on chrome: install Chromium or Google Chrome first
- Timeouts in goto smoke: raise process timeout or inspect network policy
- Schema gate failures: update both code and `docs/schemas/` in the same change
