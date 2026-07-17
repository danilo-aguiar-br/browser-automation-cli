[English](CONTRIBUTING.md) | [Português Brasileiro](CONTRIBUTING.pt-BR.md)

# Contributing to browser-automation-cli

## Welcome
- Thank you for helping improve one-shot browser automation for agents
- This guide covers setup, branching, commits, PRs, and release hygiene

## Quick Start
```bash
git clone https://github.com/danilo-aguiar-br/browser-automation-cli
cd browser-automation-cli
cargo build --locked
cargo test --locked
browser-automation-cli doctor --offline --quick --json
```

## Development Setup
- Install Rust 1.88.0 or newer
- Install Chrome or Chromium for runtime commands
- Optional tools: `ffmpeg`, `lighthouse`, `cargo-deny`, `cargo-audit`
- Prefer `cargo run -q -- <args>` during local development

## Branching Strategy
- Branch from `main`
- Use short topic names such as `fix/goto-timeout` or `docs/agents-guide`
- Keep each PR focused on one concern

## Commit Convention
- Prefer imperative subjects: `fix doctor offline path`
- Keep commits small and reviewable
- Never add `Co-authored-by` trailers unless the user explicitly requests them
- Never commit secrets, cookies, or encrypted state keys

## PR Process
- Open a PR against `main`
- Describe what changed, why, and how you validated
- Link related issues when they exist
- Keep the diff free of drive-by reformatting

## Testing
- Run unit and integration suites with `timeout 300 cargo test --locked`
- Run clippy with `timeout 120 cargo clippy --all-targets --locked -- -D warnings`
- Run format check with `cargo fmt --check`
- Add regression coverage for every bug fix
- See [docs/TESTING.md](docs/TESTING.md)

## Documentation
- Update English and Portuguese public docs in the same PR
- Keep technical identifiers untranslated
- Refresh `docs/schemas/` when JSON contracts change
- Refresh skill packages under `skills/` when command surface changes
- Keep product settings documented as flags plus `config` XDG only
- Do not document product `BROWSER_AUTOMATION_CLI_*` settings (none exist)
- When adding commands, update README Commands, INTEGRATIONS New Flags, llms-full Command Surface, COOKBOOK recipes, skills, and MIGRATION

## Report Bugs
- Open a GitHub issue with `browser-automation-cli --version`
- Include the exact command line and redacted URL when needed
- Attach `--json` envelopes when the failure is structured

## Request Features
- Describe the user problem before proposing API surface
- Prefer extending existing subcommands over inventing aliases
- Keep one-shot process ownership as a hard constraint

## Release Process
- Bump SemVer in `Cargo.toml`
- Update both CHANGELOG files under `[Unreleased]` then cut a version section
- Keep Keep a Changelog order: Unreleased first, then versions descending
- Sync public docs with the shippable command surface before tagging
- Confirm `cargo package --list` includes `docs/`, `skills/`, and root public docs
- Keep crates.io publish and GitHub release blocked until explicit maintainer approval
- Validate with build, clippy, fmt, and tests before tagging

## Recognition
- Contributors are credited through the Git history and release notes
- Security reporters are listed in SECURITY after coordinated disclosure

## Questions
- Open a discussion or issue on GitHub
- Contact the maintainer at daniloaguiarbr@proton.me for private topics
