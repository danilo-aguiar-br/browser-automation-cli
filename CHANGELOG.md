[English](CHANGELOG.md) | [Português Brasileiro](CHANGELOG.pt-BR.md)

# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2026-07-17

### Fixed
- Public bilingual documentation and skills synchronized to the full v0.1.2 surface (print-pdf, monitor, qr, find-paths, parse PDF/DOCX/xlsx/ods, extract LLM, 13 XDG keys, browser scrape formats, fail-fast data.steps, scrape webhook-url)
- Public docs teach product settings only via flags and XDG `config path|init|show|set|get` (no product env catalogs)
- Live `schema --cmd` and static `docs/schemas/` regenerated for print-pdf/monitor/qr/find-paths and expanded scrape/config fragments (including scrape `webhook_url`)
- Browser scrape now captures `outerHTML` and applies `--format` (markdown/html/links/metadata/raw-html/screenshot/summary/product/branding) instead of silent text-only (GAP-001)
- `run` scroll accepts `dy`/`dx` aliases for `delta_y`/`delta_x` (GAP-002)
- `schema --cmd` expanded for goto/eval/type/scroll/assert tool-ref flags (GAP-003)
- Human suggestions localize for `pt-BR` via `--lang` and XDG `config set lang` (GAP-004)
- Product runtime no longer reads `RUST_LOG`, `CI`, `PUPPETEER_*`, or `PLAYWRIGHT_*`; logging uses flags + XDG `log_level`; Chrome via XDG `chrome_path` (GAP-005)
- `run` fail-fast returns partial `data.steps` on error envelopes (GAP-006/016)
- Lighthouse resolves XDG `lighthouse_path` and localized install suggestion (GAP-007)
- Search cleans SERP redirect wrappers (`uddg=`) to destination URLs (GAP-008)
- Scrape accepts `raw-html` / `rawHtml` aliases and `screenshot` format token (GAP-009/021)
- `exec` help describes full step surface (GAP-011)
- `assert` accepts `url_contains` / `text_contains` aliases (GAP-012)
- Clippy `manual_clamp` cleanups in MITM helpers (GAP-013)
- `attr` falls back to DOM properties when HTML attributes are null (GAP-018)
- Docs examples use `/tmp/browser-automation-cli-artifacts` instead of `bac-` prefix (GAP-019)
- Tool-reference fixture synced to 52 official tools from knowledge base (GAP-017/020)

### Added
- `print-pdf` one-shot CDP `Page.printToPDF` artifact command
- `monitor check` one-shot baseline hash compare with optional `--write-baseline`
- XDG config keys: `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model` (full key set also includes lang, timeout, artifacts_dir, ignore_robots, namespace, encryption_key, color)
- Error envelopes may include partial `data` for multi-step fail-fast recovery
- `parse` PDF (lopdf), DOCX, xlsx/ods (calamine), `--redact-pii`
- `extract --llm` / `--question` / `--schema-json` (XDG key only; fail-closed without key)
- `qr encode|decode` and `find-paths` (no Chrome)
- Scrape formats `summary`/`product`/`branding`; MITM `ws_count`
- Command inventory documents 56 top-level names (`commands --json`), including `print-pdf`, `monitor`, `qr`, `find-paths` beyond the 52 DevTools parity tools

### Changed
- clap feature set drops unused `env` (product settings stay XDG + argv)
- Version bumped to `0.1.2`

## [0.1.1] - 2026-07-17

### Added
- XDG config surface: `config path`, `config init`, `config show`, `config set`, and `config get` for resolved paths and `config.toml` keys (lang, timeout, artifacts_dir, ignore_robots, namespace)
- Local MITM surface on hudsucker: `mitm start` (bind `127.0.0.1` with ephemeral port, one-shot), `list`, `get`, `har`, `export`, `domains`, `apis`, and `init-ca`
- Workflow journal DAG (petgraph + SQLite): `workflow run`, `workflow resume`, and `workflow status`; resume skips steps already marked ok
- Local scrape/crawl/map/search/parse HTTP commands: `batch-scrape`, `crawl`, `map`, `search`, and `parse`
- `scrape` formats `text|markdown|html|links|metadata`, engines `http|browser`, and `--only-main-content`
- `wait` multi `--text` with OR semantics (any listed text resolves the wait)
- Doctor check for XDG `browsers_dir`
- Batch scrape bounded concurrency via Tokio `JoinSet`
- Public bilingual documentation framework for crates packaging (`docs/` guides, `docs/schemas/` index, dual-language skill packages)
- Dual license files `LICENSE-MIT` and `LICENSE-APACHE`
- Crate-level rustdoc with Overview, Features, Targets, MSRV, Safety, and Examples
- rustdoc lints on the crate root (`missing_docs`, broken/private links, invalid HTML/codeblocks)
- docs.rs `targets` and `default-target` for multiplatform builds
- README Features, Targets, and MSRV sections with local `cargo doc` formulas
- `aquamarine` Mermaid lifecycle diagram on `run()` rustdoc
- Vendored tool-ref fixture `tests/fixtures/tool-reference.md` (52 tools) for inventory/e2e parity
- English product lifecycle slogan **BORN EXECUTE FINALIZE DIE** in crate description, CLI about text, and agent docs

### Changed
- Product settings no longer use runtime product environment variables; configuration is XDG-backed (`config.toml` + flags)
- `run` gains scrape parity with standalone scrape options and enforces category gates (`category_memory`, `category_extensions`, `category_third_party`, `category_webmcp`) inside script steps
- `Cargo.toml` metadata now includes authors, repository, homepage, documentation, and MSRV
- License declared as `MIT OR Apache-2.0`
- README badge order now starts with docs.rs and crates.io
- Public API docs expanded for `error`, `envelope`, and `lifecycle`
- Release profile uses fat LTO (`lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`)
- Clap help shows zero product env suggestions (`BROWSER_AUTOMATION_CLI_*` no longer advertised on flags)
- Enabled crates packaging by removing `publish = false`

### Fixed
- Build blockers: `RunFlags.category_extensions` field wiring and `Selector` lifetime
- `run` + scrape parity end-to-end; multi-text wait OR; category gates in `run`
- XDG config/paths without product env for settings; doctor reports XDG `browsers_dir`
- MITM hudsucker one-shot bind on `127.0.0.1` with ephemeral port
- Workflow resume correctly skips completed ok steps
- Batch concurrency shutdown-friendly via `JoinSet`
- rustdoc broken intra-doc links in `emulate --viewport` help text
- `tests/parity_inventory.rs` reads vendored `tests/fixtures/tool-reference.md` (52 tools)
- Formatting drift under `cargo fmt`

### Removed
- GitHub Actions workflows under `.github/workflows/`
- Cargo `[profile.ci]` used only by removed CI
- Hosted CI and GitHub Actions integration guidance from public docs
- Product settings bound to `BROWSER_AUTOMATION_CLI_*` environment variables (settings live under XDG + CLI flags)

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
- Explicitly out of **0.1.0 only**: PRD local scrape crawl/map/search surface, MITM, and workflow SQLite journal (these landed in 0.1.1)
