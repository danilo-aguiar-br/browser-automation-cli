[English](CHANGELOG.md) | [Português Brasileiro](CHANGELOG.pt-BR.md)

# Changelog

Todas as mudanças notáveis deste projeto são documentadas neste arquivo.

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/),
e este projeto adere ao [Semantic Versioning](https://semver.org/lang/pt-BR/).

## [Unreleased]

## [0.1.4] - 2026-07-18

### Adicionado
- `run --json-steps` (global `--json-steps`): stream de uma linha NDJSON por passo (`step`, `cmd`, `ok`, `result`) para observabilidade agent-first (GAP-020)
- `wait` suporta multi-seletor CSS OR (`#a, #b`), arrays `selectors`, `url` / `url_contains` / `navigation` (GAP-019, GAP-024)
- Comandos multi-passo `select-option` / `pick` para badge/popover HIG / `role=option` (GAP-023)
- Kinds de assert `console_empty` e `console_no_match` (GAP-025)
- `schema <cmd>` posicional além de `schema --cmd` (GAP-022)
- `BeforeUnloadAction` accept|dismiss em `goto` / `reload` (GAP-003)
- MITM `capture-url` one-shot compose + flags globais `--mitm*` (GAP-011)
- `print-pdf` no multi-passo `run` + gate de inventário do run (GAP-001, GAP-017)
- Scrape multi-formato e batch/crawl `--engine browser` (GAP-009, GAP-010)

### Corrigido
- `console dump` sempre grava um array JSON válido (`[]` quando vazio; nunca 0-byte) (GAP-021)
- Envelope final de `run --json` inclui `ok` + `steps[].data` completo (GAP-020)
- Erros de usage do Clap emitem envelope JSON quando `--json` está no argv (GAP-002)
- `view` em about:blank vazio recusa sucesso silencioso salvo `--allow-empty` (GAP-012)
- `print-pdf` recusa PDF em branco sem conteúdo navegado (GAP-013)
- Caminho soft de diálogo com `--if-present` (GAP-006)
- Flags de privacy no launch do Chrome; sem `metrics-recording-only` (GAP-016)

### Alterado
- Versão `0.1.4`
- Teste `parity_run_inventory` impõe `RUN_DISPATCHED_CMDS` ∪ exclusões intencionais
- Auditoria de superfície Clap (`rules_rust_cli_com_clap`): `GlobalOpts` usa `Args` + flatten; `ArgAction::SetTrue` explícito; `value_hint` em paths/URLs; help headings; exemplos `after_help`; alias `-v`; metadata `author`
- `CliError` deriva `thiserror::Error`; o binário instala `human-panic` para relatórios de panic em release
- Gate de integração `tests/clap_command_debug_assert.rs` roda `Cli::command().debug_assert()`


### Documentação
- Docs públicos bilíngues (README, INTEGRATIONS, llms*, HOW_TO_USE, AGENTS, COOKBOOK, MIGRATION, TESTING, SECURITY, CONTRIBUTING) sincronizados com a superfície v0.1.4
- Inventário documentado como 61 nomes de agente via `commands --json` (inclui `select-option` e `pick` só em run/schema; clap top-level lista 59 sem eles como subcomandos standalone)
- Skills EN/PT reescritas como playbooks imperativos com fórmulas para os 61 comandos (somente XDG + flags; sem catálogo de env de produto)
- `docs/schemas` regenerados; fragmentos live de `schema` para `batch-scrape`/`crawl`/`scrape` documentam `--engine browser` e multi-formato
- Banner em `gaps.md` marca GAP-001…025 Closed e preserva o histórico da auditoria pré-fix

## [0.1.3] - 2026-07-17


### Documentação
- Docs públicas da raiz (README, INTEGRATIONS, llms*, SECURITY, CONTRIBUTING) sincronizadas com a superfície v0.1.3 (59 comandos, honestidade Redis/Lighthouse, A001–A012)
- `CHANGELOG.pt-BR.md` espelha o hard-close 0.1.3 completo; adicionado `llms-full.pt-BR.txt`
### Corrigido (polish Redis live + Lighthouse real)
- Cache Redis: roundtrip RESP sempre ativo via mock TCP (sem `#[ignore]`, sem env de produto); spawn opcional de `redis-server` real quando estiver no PATH; doctor `cache_redis` a partir do XDG
- Lighthouse: resolve flag → XDG → PATH; envelope `binary_source`/`binary_present`; doctor reporta a origem; e2e rotula `source=real|mock`

### Corrigido (fechamento duro GAP-A001…A012)
- Assert residual do e2e sem self-match de scanners; empty match seguro com pipefail (GAP-A001)
- FINALIZE faz scavenge de órfãos Chromium em `/tmp` de propriedade da CLI (GAP-A002)
- `run --script` aceita NDJSON ou array JSON de passos (GAP-A003)
- `scrape --engine http` rejeita `file://` com Usage + sugestão browser/parse (GAP-A004)
- `reload` usa CDP `Page.reload` + `ignoreCache` (GAP-A005)
- `init_script` removido após navegação/reload (GAP-A006)
- Redis `rediss://` fail-closed (GAP-A007); roundtrip mock sempre ativo + live opcional se houver binário (GAP-A008)
- `handle_before_unload` auto-aceita via diálogo CDP sem inject de `preventDefault` (GAP-A009)
- Doctor lighthouse reporta sugestão de path XDG com honestidade (GAP-A010)
- Eventos CDP modernos desconhecidos são ignorados para a captura continuar (GAP-A012)

### Adicionado (pilares PRD GAP-A011)
- `find-paths --glob` com filtro estilo shell
- `sheet-write` CSV/JSON → XLSX via `rust_xlsxwriter`
- `sg-scan` / `sg-rewrite` lint estrutural one-shot (dry-run por padrão)

### Corrigido
- `goto` aplica `--init-script`, `--handle-before-unload` e `--navigation-timeout-ms` (sem descarte silencioso) via CDP `Page.addScriptToEvaluateOnNewDocument`
- Doctor nunca sugere `npm`; `--fix` / `--offline` com efeito; correção lighthouse aponta para `config set lighthouse_path`
- `console list` / `net list` `--include-preserved` usa ring buffer de navegações no processo com `include_preserved_mode` honesto
- Lighthouse `--mode snapshot` mapeia para `--gather-mode=snapshot` (mock ecoa argv)
- `reload --init-script` single-shot rejeita sessão em branco; multi-step `run` aplica init no reload
- Extension uninstall descarrega targets in-process com `effect` explícito (`unloaded` | `metadata_only`)
- Residual ledger preenche `profile_dir` + side-channels Singleton; FINALIZE limpa só paths owned
- Helpers Job Object no Windows para reap residual-zero (`win_job`)
- i18n pt-BR com acentos corretos em sugestões críticas (invocação, propósito, obrigatórios, não)
- Parse path usa cache HTTP/parse sob XDG (sem dir de cache descartado)

### Adicionado
- `page tab-id` (tool-ref `get_tab_id`) — inventário 53 tools
- `eval --service-worker-id` avalia em targets de service worker de extensão
- `config list-keys` para descoberta de chaves XDG
- Módulo `RetryConfig` com backoff/jitter; parsers proptest offline
- Cache HTTP em camadas (memória L1 + SQLite L2 sob XDG); logs rotacionados opcionais (`log_to_file`)
- Script `scripts/inventory_diff_base.sh` como gate local de inventário; e2e limpa `/tmp/ba-e2e-*` em sucesso
- Inventário de comandos de topo: 59 nomes (`commands --json`), incluindo `sheet-write`, `sg-scan`, `sg-rewrite`

## [0.1.2] - 2026-07-17

### Corrigido
- Documentação pública bilíngue e skills sincronizadas com a superfície completa v0.1.2 (print-pdf, monitor, qr, find-paths, parse PDF/DOCX/xlsx/ods, extract LLM, 13 chaves XDG, formatos scrape browser, fail-fast data.steps, scrape webhook-url)
- Documentação pública ensina settings de produto só via flags e XDG `config path|init|show|set|get` (sem catálogos de env de produto)
- `schema --cmd` ao vivo e `docs/schemas/` estáticos regenerados para print-pdf/monitor/qr/find-paths e fragmentos scrape/config expandidos (incluindo scrape `webhook_url`)
- Scrape com engine browser aplica `--format` (markdown/html/links/metadata/raw-html/screenshot/summary/product/branding) via outerHTML em vez de texto silencioso (GAP-001)
- `run` scroll aceita aliases `dy`/`dx` para `delta_y`/`delta_x` (GAP-002)
- `schema --cmd` expandido para flags tool-ref de goto/eval/type/scroll/assert (GAP-003)
- Sugestões humanas em `pt-BR` via `--lang` e `config set lang` (GAP-004)
- Runtime de produto sem `RUST_LOG`/`CI`/`PUPPETEER_*`/`PLAYWRIGHT_*`; logging via flags + XDG `log_level`; Chrome via XDG `chrome_path` (GAP-005)
- `run` fail-fast devolve `data.steps` parciais no envelope de erro (GAP-006/016)
- Lighthouse resolve XDG `lighthouse_path` e sugestão localizada de install (GAP-007)
- `search` limpa wrappers de redirect SERP (`uddg=`) para URLs de destino (GAP-008)
- Scrape aceita aliases `raw-html` / `rawHtml` e token de format `screenshot` (GAP-009/021)
- Help do `exec` descreve a superfície completa de steps (GAP-011)
- `assert` aceita aliases `url_contains`/`text_contains` (GAP-012)
- Ajustes clippy `manual_clamp` no MITM (GAP-013)
- `attr` faz fallback para properties DOM quando atributos HTML são null (GAP-018)
- Exemplos de docs usam `/tmp/browser-automation-cli-artifacts` em vez do prefixo `bac-` (GAP-019)
- Fixture tool-ref sincronizado com 52 tools oficiais da base de conhecimento (GAP-017/020)

### Adicionado
- Comando one-shot `print-pdf` (CDP `Page.printToPDF`)
- `monitor check` one-shot com comparação de baseline hash e `--write-baseline` opcional
- Chaves XDG: `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model` (conjunto completo também inclui lang, timeout, artifacts_dir, ignore_robots, namespace, encryption_key, color)
- Envelopes de erro podem incluir `data` parcial para recuperação fail-fast multi-passo
- `parse` PDF (lopdf), DOCX, xlsx/ods (calamine), `--redact-pii`
- `extract --llm` / `--question` / `--schema-json` (somente chave XDG; fail-closed sem chave)
- `qr encode|decode` e `find-paths` (sem Chrome)
- Formatos de scrape `summary`/`product`/`branding`; MITM `ws_count`
- Inventário de comandos documenta 56 nomes de topo (`commands --json`), incluindo `print-pdf`, `monitor`, `qr`, `find-paths` além das 52 tools de paridade DevTools

### Alterado
- Feature set do clap remove `env` não usado (settings de produto ficam XDG + argv)
- Versão elevada para `0.1.2`

## [0.1.1] - 2026-07-17

### Adicionado
- Superfície de config XDG: `config path`, `config init`, `config show`, `config set` e `config get` para paths resolvidos e chaves de `config.toml` (lang, timeout, artifacts_dir, ignore_robots, namespace)
- Superfície MITM local com hudsucker: `mitm start` (bind em `127.0.0.1` com porta efêmera, one-shot), `list`, `get`, `har`, `export`, `domains`, `apis` e `init-ca`
- Journal de workflow em DAG (petgraph + SQLite): `workflow run`, `workflow resume` e `workflow status`; o resume pula passos já marcados como ok
- Comandos HTTP locais scrape/crawl/map/search/parse: `batch-scrape`, `crawl`, `map`, `search` e `parse`
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
- Explicitamente fora **apenas de 0.1.0**: PRD superfície local scrape crawl/map/search, MITM e journal SQLite de workflow (esses itens entraram em 0.1.1)
