[English](CHANGELOG.md) | [Português Brasileiro](CHANGELOG.pt-BR.md)

# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Public bilingual documentation framework for crates packaging
- `docs/` guides, `docs/schemas/` index, and dual-language skill packages
- Dual license files `LICENSE-MIT` and `LICENSE-APACHE`
- Crate-level rustdoc with Overview, Features, Targets, MSRV, Safety, and Examples
- rustdoc lints on the crate root (`missing_docs`, broken/private links, invalid HTML/codeblocks)
- docs.rs `targets` and `default-target` for multiplatform builds after 2026-05-01
- README Features, Targets, and MSRV sections with local `cargo doc` formulas
- `aquamarine` Mermaid lifecycle diagram on `run()` rustdoc

### Changed
- `Cargo.toml` metadata now includes authors, repository, homepage, documentation, and MSRV
- License declared as `MIT OR Apache-2.0`
- README badge order now starts with docs.rs and crates.io
- Public API docs expanded for `error`, `envelope`, and `lifecycle`

### Fixed
- rustdoc broken intra-doc links in `emulate --viewport` help text

### Removed
- GitHub Actions workflows under `.github/workflows/`
- Cargo `[profile.ci]` used only by removed CI
- Hosted CI and GitHub Actions integration guidance from public docs

### Fixed
- `tests/parity_inventory.rs` now reads vendored `tests/fixtures/tool-reference.md` (52 tools)
- Enabled crates packaging by removing `publish = false`
- Formatting drift under `cargo fmt`

## [0.1.0] - 2025-07-16

### Added
- One-shot Chrome launch via `chromiumoxide::Browser::launch`
- Launch flags for proxy, webgpu, extensions, and sandbox on the oxide path
- FINALIZE path with close, wait, and kill fallback
- Core commands: `doctor`, `open`/`goto`, `extract`, `scrape`, `run`, `grab`, `view`, `click`/`press`, `fill`/`write`, `robots`
- Optional console and network capture
- Robots policy with dual-flag acceptance
- DevTools parity surface for navigation, input, snapshot, screenshot, eval, pages, wait, perf, lighthouse, screencast, heap, extensions
- Tool-ref flags such as `--include-snapshot` on hover, drag, keys, upload, and fill-form
- `net` and `console` list filters with pagination
- `eval` with `--args`, `--dialog-action`, and `--file-path`
- `perf start --auto-stop` and `perf insight`
- `screencast stop --path` with ffmpeg-backed webm or mp4 export
- Heap deep analysis gated by `--category-memory`
- Page management with `--background` and `--isolated-context`
- Schema discovery via `schema --cmd` and inventory gate tests

### Changed
- `src/install.rs` slimmed to local discovery only
- CDP stack is 100 percent chromiumoxide Chrome

### Removed
- Dual-spawn monólito `launch_chrome` / `ChromeProcess`
- Residual branding and non-product dump artifacts from the public tree

### Fixed
- Clean public git history recreated without legacy branding commits

### Notes
- Explicitly out of this release: PRD Firecrawl crawl/map/search, MITM, and workflow SQLite journal
