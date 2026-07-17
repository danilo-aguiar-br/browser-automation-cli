[English](CHANGELOG.md) | [Português Brasileiro](CHANGELOG.pt-BR.md)

# Changelog

Todas as mudanças notáveis deste projeto são documentadas neste arquivo.

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/),
e este projeto adere ao [Semantic Versioning](https://semver.org/lang/pt-BR/).

## [Unreleased]

## [0.1.1] - 2026-07-17

### Adicionado
- Superfície de config XDG: `config path`, `config init`, `config show`, `config set` e `config get` para paths resolvidos e chaves de `config.toml` (lang, timeout, artifacts_dir, ignore_robots, namespace)
- Superfície MITM local com hudsucker: `mitm start` (bind em `127.0.0.1` com porta efêmera, one-shot), `list`, `get`, `har`, `export`, `domains`, `apis` e `init-ca`
- Journal de workflow em DAG (petgraph + SQLite): `workflow run`, `workflow resume` e `workflow status`; o resume pula passos já marcados como ok
- Comandos HTTP locais de paridade Firecrawl: `batch-scrape`, `crawl`, `map`, `search` e `parse`
- Formatos de `scrape` `text|markdown|html|links|metadata`, engines `http|browser` e `--only-main-content`
- `wait` multi `--text` com semântica OR (qualquer texto listado resolve a espera)
- Check do doctor para `browsers_dir` XDG
- Concorrência limitada em batch scrape via Tokio `JoinSet`
- Framework público bilíngue de documentação para empacotamento crates (guias em `docs/`, índice `docs/schemas/`, pacotes de skill dual-idioma)
- Arquivos de dual license `LICENSE-MIT` e `LICENSE-APACHE`
- rustdoc no nível do crate com Overview, Features, Targets, MSRV, Safety e Examples
- Lints rustdoc no crate root (`missing_docs`, links quebrados/privados, HTML/codeblocks inválidos)
- `targets` e `default-target` do docs.rs para builds multiplataforma
- Seções Features, Targets e MSRV no README com fórmulas locais de `cargo doc`
- Diagrama Mermaid de lifecycle via `aquamarine` no rustdoc de `run()`
- Fixture tool-ref vendored em `tests/fixtures/tool-reference.md` (52 tools) para inventário/e2e de paridade
- Slogan inglês de lifecycle do produto **BORN EXECUTE FINALIZE DIE** na description do crate, no about da CLI e na documentação de agentes

### Alterado
- Configurações de produto deixam de usar variáveis de ambiente de produto em runtime; configuração é XDG (`config.toml` + flags)
- `run` ganha paridade de scrape com as opções standalone e aplica gates de categoria (`category_memory`, `category_extensions`, `category_third_party`, `category_webmcp`) nos passos do script
- Metadados do `Cargo.toml` agora incluem authors, repository, homepage, documentation e MSRV
- Licença declarada como `MIT OR Apache-2.0`
- Ordem de badges do README começa com docs.rs e crates.io
- Docs da API pública expandidas para `error`, `envelope` e `lifecycle`
- Profile de release com LTO fat (`lto = "fat"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`)
- Help do clap sem sugestões de env de produto (`BROWSER_AUTOMATION_CLI_*` não anunciado nas flags)
- Empacotamento crates liberado com remoção de `publish = false`

### Corrigido
- Bloqueios de build: wiring do campo `RunFlags.category_extensions` e lifetime de `Selector`
- Paridade `run` + scrape ponta a ponta; wait multi-text OR; gates de categoria no `run`
- Config/paths XDG sem env de produto para settings; doctor reporta `browsers_dir` XDG
- MITM hudsucker one-shot em bind `127.0.0.1` com porta efêmera
- Resume de workflow pula corretamente passos ok já concluídos
- Concorrência de batch amigável a shutdown via `JoinSet`
- Links intra-doc quebrados no help de `emulate --viewport`
- `tests/parity_inventory.rs` lê `tests/fixtures/tool-reference.md` vendored (52 tools)
- Drift de formatação sob `cargo fmt`

### Removido
- Workflows GitHub Actions em `.github/workflows/`
- Cargo `[profile.ci]` usado só pelo CI removido
- Orientação de CI hospedado e GitHub Actions da documentação pública
- Settings de produto amarrados a variáveis de ambiente `BROWSER_AUTOMATION_CLI_*` (settings ficam sob XDG + flags da CLI)

## [0.1.0] - 2025-07-16

### Adicionado
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

### Alterado
- `src/install.rs` reduzido a descoberta local
- Stack CDP 100 por cento chromiumoxide Chrome

### Removido
- Monólito dual-spawn `launch_chrome` / `ChromeProcess`
- Branding residual e dumps não-produto da árvore pública

### Corrigido
- Histórico git público recriado sem commits de branding legado

### Notas
- Explicitamente fora **apenas de 0.1.0**: PRD Firecrawl crawl/map/search, MITM e journal SQLite de workflow (esses itens entraram em 0.1.1)
