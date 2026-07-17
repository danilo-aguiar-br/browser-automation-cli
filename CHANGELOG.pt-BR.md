[English](CHANGELOG.md) | [Português Brasileiro](CHANGELOG.pt-BR.md)

# Changelog

Todas as mudanças notáveis deste projeto são documentadas neste arquivo.

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/),
e este projeto adere ao [Semantic Versioning](https://semver.org/lang/pt-BR/).

## [Unreleased]

### Added
- Framework público bilíngue de documentação para empacotamento crates
- Guias em `docs/`, índice `docs/schemas/` e pacotes de skill dual-idioma
- Arquivos de dual license `LICENSE-MIT` e `LICENSE-APACHE`
- rustdoc no nível do crate com Overview, Features, Targets, MSRV, Safety e Examples
- Lints rustdoc no crate root (`missing_docs`, links quebrados/privados, HTML/codeblocks inválidos)
- `targets` e `default-target` do docs.rs para builds multiplataforma após 2026-05-01
- Seções Features, Targets e MSRV no README com fórmulas locais de `cargo doc`
- Diagrama Mermaid de lifecycle via `aquamarine` no rustdoc de `run()`

### Changed
- Metadados do `Cargo.toml` agora incluem authors, repository, homepage, documentation e MSRV
- Licença declarada como `MIT OR Apache-2.0`
- Ordem de badges do README começa com docs.rs e crates.io
- Docs da API pública expandidas para `error`, `envelope` e `lifecycle`

### Fixed
- Links intra-doc quebrados no help de `emulate --viewport`

### Removed
- Workflows GitHub Actions em `.github/workflows/`
- Cargo `[profile.ci]` usado só pelo CI removido
- Orientação de CI hospedado e GitHub Actions da documentação pública

### Fixed
- `tests/parity_inventory.rs` lê `tests/fixtures/tool-reference.md` vendored (52 tools)
- Empacotamento crates liberado com remoção de `publish = false`
- Drift de formatação sob `cargo fmt`

## [0.1.0] - 2025-07-16

### Added
- Launch one-shot do Chrome via `chromiumoxide::Browser::launch`
- Flags de launch para proxy, webgpu, extensions e sandbox no path oxide
- Path FINALIZE com close, wait e kill fallback
- Comandos core: `doctor`, `open`/`goto`, `extract`, `scrape`, `run`, `grab`, `view`, `click`/`press`, `fill`/`write`, `robots`
- Captura opcional de console e network
- Política robots com dual-flag de aceite
- Superfície de paridade DevTools para navegação, input, snapshot, screenshot, eval, pages, wait, perf, lighthouse, screencast, heap, extensions
- Flags tool-ref como `--include-snapshot` em hover, drag, keys, upload e fill-form
- Filtros de `net` e `console` list com paginação
- `eval` com `--args`, `--dialog-action` e `--file-path`
- `perf start --auto-stop` e `perf insight`
- `screencast stop --path` com export webm ou mp4 via ffmpeg
- Análise profunda de heap sob `--category-memory`
- Gestão de páginas com `--background` e `--isolated-context`
- Descoberta de schema via `schema --cmd` e testes de inventário

### Changed
- `src/install.rs` reduzido a descoberta local
- Stack CDP 100 por cento chromiumoxide Chrome

### Removed
- Monólito dual-spawn `launch_chrome` / `ChromeProcess`
- Branding residual e dumps não-produto da árvore pública

### Fixed
- Histórico git público recriado sem commits de branding legado

### Notes
- Explicitamente fora deste release: PRD Firecrawl crawl/map/search, MITM e journal SQLite de workflow
