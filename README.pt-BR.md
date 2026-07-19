[English](README.md) | [Português Brasileiro](README.pt-BR.md)

# browser-automation-cli

> Automação one-shot do Chrome CDP para agentes de IA. BORN, EXECUTE, FINALIZE, DIE.

[![docs.rs](https://img.shields.io/docsrs/browser-automation-cli)](https://docs.rs/browser-automation-cli)
[![crates.io](https://img.shields.io/crates/v/browser-automation-cli)](https://crates.io/crates/browser-automation-cli)
[![License](https://img.shields.io/crates/l/browser-automation-cli)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88.0-orange)](Cargo.toml)
[![Downloads](https://img.shields.io/crates/d/browser-automation-cli)](https://crates.io/crates/browser-automation-cli)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-blue)](https://www.rust-lang.org)
[![GitHub](https://img.shields.io/badge/github-browser--automation--cli-black.svg)](https://github.com/danilo-aguiar-br/browser-automation-cli)

## O que é
- CLI de automação de browser em um único processo para agentes de IA
- Fala com Chrome ou Chromium do sistema via chromiumoxide CDP
- Sem daemon, sem empacotamento npm e sem telemetria remota
- O ciclo de vida é sempre BORN, EXECUTE, FINALIZE, DIE
- Envelopes JSON no stdout para agentes programáticos
- Config e caminhos XDG só via comandos `config`

## A Dor
- Fluxos de agente precisam de browser multi-passo sem daemon sticky
- Stacks Node e npm adicionam peso de runtime e superfície de supply-chain
- Ferramentas baseadas em sessão deixam Chrome órfão e ownership obscuro
- Contratos JSON costumam divergir de flags e exit codes reais
- Settings de produto fora do `config` XDG tornam prompts de agente frágeis

## Por que browser-automation-cli
- Um processo é dono de um ciclo completo de Chrome do launch ao kill fallback
- Trabalho multi-passo usa `run --script` NDJSON ou um array JSON de passos no mesmo processo
- Refs de acessibilidade `@eN` só valem dentro daquele processo
- Envelopes `--json` estáveis para agentes programáticos
- Caminho de install é Rust puro via cargo
- v0.1.5 fecha residual-zero de disco (RES-01…12): GC de Singleton em BORN+FINALIZE, `doctor residual_disk`, honestidade de inventário com `locale`/`man` (63 nomes de agente); mantém a superfície agent-first completa de 0.1.4

## Superpoderes
- Navegação e ciclo de página: `goto`, `back`, `forward`, `reload`, `page`
- Input: `press`, `write`, `type`, `keys`, `hover`, `drag`, `fill-form`, `select-option`/`pick` (run/schema), `upload`
- Observação: `view`, `grab`, `extract`, `text`, `attr`, `scroll`, `assert`
- Wait: múltiplos `--text` resolvem como OR; multi-seletor CSS OR (`#a, #b` / `selectors`); `url` / `url_contains` / `navigation`
- Scrape: `scrape` com `--format` multi/CSV `text|markdown|html|links|metadata|raw-html|screenshot|summary|product|branding` e `--engine http|browser` (engine browser aplica formatos via outerHTML)
- Superfície local scrape/crawl/map/search/parse: `batch-scrape` (`--engine http|browser`), `crawl` (`--engine http|browser`), `map`, `search` (limpa redirects SERP `uddg=`), `parse` (PDF/DOCX/xlsx/ods + `--redact-pii`)
- Extract LLM: `extract --llm --question --schema-json` (XDG `openrouter_api_key`, `llm_base_url`, `llm_model`)
- Captura: `console` e `net` com flags globais opcionais
- Profundidade DevTools: `eval`, `emulate`, `resize`, `perf`, `lighthouse` (resolve flag → XDG `lighthouse_path` → PATH; envelope `binary_source` real|mock), `heap`
- Impressão PDF: `print-pdf` one-shot CDP `Page.printToPDF` (também no multi-passo `run`)
- Monitor: `monitor check --url --baseline [--write-baseline]`
- Utilitários (sem Chrome): `qr encode|decode`, `find-paths` (`--glob`), `sheet-write`, `sg-scan`, `sg-rewrite`
- Aliases de assert: `url_contains` / `text_contains`; kinds `console_empty` / `console_no_match`; `attr` faz fallback para properties DOM
- Aliases de scroll em `run`: `dy`/`dx` para `delta_y`/`delta_x`
- Categorias opcionais: memory, extensions, third-party, webmcp
- Experimental: vision `click-at`, screencast com export via ffmpeg
- MITM one-shot: `mitm start` / `mitm capture-url` escuta só em `127.0.0.1` (hudsucker); flags globais `--mitm*`
- Workflow DAG: `workflow run|resume|status` com journal SQLite (resume pula ok)
- Config XDG: `config path|init|show|set|get|list-keys` para config.toml
- Descoberta: `doctor` (browsers_dir, origem lighthouse, `cache_redis`, `residual_disk`), `commands` (63 nomes), `schema <cmd>` ou `schema --cmd` (goto/eval/type/scroll/assert expandidos), `version`, `locale`, `man`, `completions`
- Fail-fast multi-passo: `run` devolve `data.steps` parciais em envelopes de erro; `--json-steps` streama NDJSON por passo
- Residual-zero de disco: BORN faz auto-GC de dirs Chromium Singleton-only em `/tmp` com mais de 60s; FINALIZE dual scavenge + re-scan; nunca mata Chrome Flatpak do host; prefixo de marker `browser-automation-cli-chrome-`
- Ciclo de vida: BORN + FINALIZE fazem scavenge de órfãos Chromium em `/tmp` owned; product law é residual-zero de processo + disco
- Cache: XDG `cache_backend` (`sqlite|memory|redis`) e `cache_redis_url`; `rediss://` fail-closed

## Início Rápido
```bash
cargo install --path . --locked
browser-automation-cli --version
browser-automation-cli doctor --offline --quick --json
browser-automation-cli doctor --offline --quick --json | jaq '.residual // .data.residual // .'
browser-automation-cli locale --json
browser-automation-cli goto https://example.com --json
browser-automation-cli view --json
```

## Instalação
- Install de desenvolvimento local:
```bash
git clone https://github.com/danilo-aguiar-br/browser-automation-cli
cd browser-automation-cli
cargo install --path . --locked
```
- Do crates.io após o primeiro publish:
```bash
cargo install browser-automation-cli --locked
```
- Runtime exige Chrome ou Chromium no path do shell (ou `config set chrome_path`)
- Opcional: `ffmpeg` para export de screencast
- Opcional: binário `lighthouse` para auditorias lighthouse (ou `config set lighthouse_path`)

## Uso
- Passe sempre `--json` em pipelines de agente
- Mantenha diagnósticos humanos no stderr com `-q` ao pipar
- Use `--timeout` para orçamento wall-clock do processo em segundos
- Use `run --script` (linhas NDJSON ou um array JSON de passos) para sessões multi-passo que compartilham refs `@eN`; adicione `--json-steps` para stream por passo
- Prefira flags de CLI em chamadas one-off; use `config` para defaults XDG duráveis
- Detalhe de logging: `--verbose` / `--debug` / `-q`, ou `config set log_level`
- Localize sugestões humanas com `--lang pt-BR` ou `config set lang pt-BR`
- Opcional: scrape `--webhook-url` faz POST único do resultado para URL do operador (não é telemetria de produto)

```bash
browser-automation-cli config set openrouter_api_key sk-or-...
browser-automation-cli --json goto https://example.com
browser-automation-cli --json wait --text Hello --text Welcome --ms 5000
browser-automation-cli --json schema wait
browser-automation-cli --json-steps run --script '[{"cmd":"goto","url":"https://example.com"},{"cmd":"view"}]'
browser-automation-cli --json mitm capture-url https://example.com --seconds 30
browser-automation-cli --json scrape https://example.com --format markdown --engine http
browser-automation-cli --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --json scrape https://example.com --format markdown --engine http --webhook-url https://example.com/hook
browser-automation-cli --json extract --llm --question "What is the title?" https://example.com
browser-automation-cli --capture-network --json net list --resource-types Document,XHR
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json workflow resume --manifest workflow.toml
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/browser-automation-cli-artifacts/page.pdf
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/browser-automation-cli-artifacts/base.txt --write-baseline
browser-automation-cli --json parse ./doc.pdf --redact-pii
browser-automation-cli --json parse ./doc.ods
browser-automation-cli --json qr encode --text "hello" --path /tmp/browser-automation-cli-artifacts/qr.png
browser-automation-cli --json qr decode --path /tmp/browser-automation-cli-artifacts/qr.png
browser-automation-cli --json find-paths /path/to/tree --glob "**/*.rs"
browser-automation-cli --json sheet-write --input rows.csv --out /tmp/browser-automation-cli-artifacts/out.xlsx
browser-automation-cli --json sg-scan --paths src
browser-automation-cli --json run --script '[{"cmd":"goto","url":"https://example.com"},{"cmd":"view"}]'
```

## Comandos
- Descoberta: `doctor`, `commands`, `schema`, `version`, `locale`, `man`, `completions`
- Config: `config path`, `config init`, `config show`, `config set`, `config get`, `config list-keys`
- Navegação: `goto`, `back`, `forward`, `reload`
- Snapshot e input: `view`, `press`, `write`, `type`, `keys`, `wait`, `hover`, `drag`, `fill-form`, `upload`
- Multi-passo / inventário schema apenas: `select-option`, `pick` (não são subcomandos clap standalone; use dentro de `run` / `exec` / schema)
- Conteúdo: `extract`, `text`, `scroll`, `attr`, `assert`, `grab`
- Scrape e discovery: `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
- PDF e monitor: `print-pdf`, `monitor`
- Utilitários: `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`
- Abas e diálogos: `page`, `dialog`, `cookie`
- Captura: `console`, `net`
- MITM: `mitm status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact`
- Workflow: `workflow run|resume|status`
- Avançado: `eval`, `emulate`, `resize`, `perf`, `lighthouse`, `screencast`, `heap`
- Categorias: `extension`, `devtools3p`, `webmcp`
- Multi-passo: `run`, `exec`
- Inventário: **63** nomes de agente via `commands --json` (inclui `locale`, `man`, `select-option`, `pick`); help clap de topo lista **61** sem `select-option`/`pick` como subcomandos standalone; e2e DevTools cobre 53 tools

## Configuração
- Prefira flags de CLI para chamadas one-off de agente
- Use `config path|init|show|set|get|list-keys` para o config.toml XDG
- Settings de produto só via flags e `config set` (XDG)
- Logging: `--verbose` / `--debug` / `-q`, ou XDG `config set log_level` / `log_to_file`
- Color: `config set color true|false`
- Binário Chrome: path do shell ou XDG `config set chrome_path`
- Binário Lighthouse: flag `--lighthouse-path`, XDG `config set lighthouse_path`, ou PATH (envelope reporta `binary_source`)
- Cache: `config set cache_backend sqlite|memory|redis` e opcional `cache_redis_url` (somente `redis://`; `rediss://` fail-closed)
- Chaves de config (16): `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- `config init` cria o layout XDG e o config.toml padrão
- `config path` imprime paths resolvidos de config, data, cache, state e browsers_dir
- `config list-keys` lista cada chave suportada com defaults
- CLI flags sobrescrevem valores do config.toml
- Doctor reporta browsers_dir, origem lighthouse, `cache_redis` e `residual_disk` entre as checagens de readiness
- Campo JSON de topo `residual` do doctor reporta: `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes`

## Recursos
- Este crate não tem feature flags de Cargo
- Categorias opcionais são flags de processo, não features de compile-time
- `--category-memory` habilita ferramentas profundas de heap
- `--category-extensions` habilita ferramentas de extension
- `--category-third-party` habilita helpers DevTools de terceiros
- `--category-webmcp` habilita ferramentas webmcp
- `--experimental-vision` habilita `click-at`
- `--experimental-screencast` habilita export de screencast com ffmpeg

## Alvos
- Documentado para `x86_64-unknown-linux-gnu`
- Documentado para `x86_64-apple-darwin`
- Documentado para `aarch64-apple-darwin`
- Documentado para `x86_64-pc-windows-msvc`
- Documentado para `aarch64-unknown-linux-musl`
- Sem suporte em `wasm32-unknown-unknown` (CDP exige browser desktop)
- Metadados docs.rs declaram esses targets após a mudança multi-target de 2026-05-01

## MSRV (Rust mínimo)
- Minimum Supported Rust Version é 1.88.0
- Política: subir MSRV só em release minor ou major com nota no CHANGELOG
- Docs locais: `timeout 180 cargo doc --no-deps`

## Padrões de Integração
- Claude Code, Codex, Cursor e agentes de shell disparam um processo por ação
- Planos multi-passo de agentes devem usar `run --script` (NDJSON ou array JSON; opcional `--json-steps`) em vez de encadear processos separados
- Parseie stdout com `jaq` e ignore stderr salvo em diagnóstico
- Persista defaults duráveis com `config set` sob XDG
- Veja [INTEGRATIONS.pt-BR.md](INTEGRATIONS.pt-BR.md) e [docs/AGENTS.pt-BR.md](docs/AGENTS.pt-BR.md)

## Performance
- Cold start é dominado pelo launch do Chrome, não pelo tamanho do binário Rust
- Prefira `doctor --offline --quick` para checagens de install sem rede
- Reutilize scripts multi-passo para evitar launches repetidos do Chrome
- Prefira `scrape --engine http` quando CDP não for necessário
- Use concorrência de `batch-scrape` para fetches paralelos (`--engine http` default; `--engine browser` por URL)

## Requisitos de Memória
- Espere memória do processo Chrome muito acima do binário da CLI
- Tools de heap exigem `--category-memory` e snapshots maiores elevam RAM
- Screencast pode invocar ffmpeg como helper externo
- Journals de workflow e capturas MITM ficam sob paths XDG de state/data

## FAQ de Troubleshooting
- Chrome não encontrado: instale Chromium ou Google Chrome, garanta o path do shell, ou `config set chrome_path`, e rode `doctor`
- Config / XDG: rode `config init` e depois `config path` para inspecionar o layout; use `config set|get` para valores
- Settings de produto só via flags e `config set` (XDG)
- Exit 69 unavailable: binário do browser ausente, bloqueado ou não lançável
- Exit 124 timeout: eleve `--timeout` ou encurte o script
- Exit 2 usage: confira flags com `browser-automation-cli help <cmd>`
- Refs `@eN` inválidas entre comandos: mantenha passos dentro de um `run`; refs não atravessam processos
- Network vazio: passe `--capture-network` no mesmo processo que navega
- Wait multi-text: repita `--text` para semântica OR; multi-seletor CSS OR e `url`/`url_contains`/`navigation` no `run`
- Bind MITM: `mitm start` / `mitm capture-url` escuta só em `127.0.0.1` com porta efêmera; flags globais `--mitm*`
- Workflow resume: `workflow resume` pula passos já `ok` no journal
- Formatos scrape browser: `--engine browser` aplica `--format` (markdown/html/links/metadata/raw-html/screenshot/summary/product/branding) via outerHTML
- Aliases de scroll: em scripts `run` use `dy`/`dx` como aliases de `delta_y`/`delta_x`
- Descoberta de schema: `schema <cmd>` ou `schema --cmd goto|eval|type|scroll|assert` expõe flags tool-ref expandidas
- Lang: `--lang pt-BR` ou `config set lang pt-BR` localiza sugestões humanas
- Fail-fast com steps parciais: envelopes de erro de `run` podem incluir `data.steps` parciais; `--json-steps` streama NDJSON por passo
- Path do Lighthouse: flag, `config set lighthouse_path`, ou PATH; envelope `binary_source` é `real` ou `mock` (mock é honestidade de e2e, não produção)
- Redirects de search: `search` limpa wrappers `uddg=` para URLs de destino
- Parse de documentos: `parse` suporta PDF/DOCX/xlsx/ods e `--redact-pii`
- Extract LLM: exige XDG `openrouter_api_key` (opcionais `llm_base_url`, `llm_model`)
- Print PDF: `print-pdf --url <url> --path <file>` one-shot CDP
- Baseline de monitor: `monitor check --url <url> --baseline <file> [--write-baseline]`
- Aliases de assert: `url_contains` / `text_contains`; kinds `console_empty` / `console_no_match`; `attr` usa fallback de property DOM quando o atributo HTML é null
- Tamanho do inventário: `commands --json` lista **63** nomes de agente (inclui `locale`, `man`); clap de topo lista **61** sem `select-option`/`pick` como standalone; e2e cobre 53 tools de paridade DevTools
- Locale: `locale --json` diagnostica o idioma resolvido; defina com `--lang pt-BR` ou `config set lang pt-BR`
- `file://` + `scrape --engine http`: erro Usage — use engine browser ou `parse` para arquivos locais
- `reload --ignore-cache`: CDP `Page.reload` com `ignoreCache` (não é no-op em JS)
- Formatos de script `run`: NDJSON um objeto por linha, ou um único array JSON de passos; `print-pdf` e `select-option`/`pick` no multi-passo
- Cache Redis: defina `cache_backend redis` e `cache_redis_url`; nunca use `rediss://`
- Residual /tmp higiene de disco (v0.1.5 residual-zero):
  - BORN auto-GC: `scavenge_stale_singleton_orphans` remove dirs `/tmp` `org.chromium.Chromium.*` Singleton-only com mais de 60s
  - FINALIZE dual scavenge + re-scan de dirs marker owned (prefixo `browser-automation-cli-chrome-`)
  - Nunca mata Chrome Flatpak do host nem processos de browser fora da CLI
  - Checagem doctor `residual_disk` + campo JSON de topo `residual` (`cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes`)
  - Gates locais: `scripts/residual-check.sh`, `scripts/residual-stress.sh` (sem CI obrigatório)
- Utils de planilha/lint: `sheet-write`, `sg-scan`, `sg-rewrite`; `find-paths --glob` para globs shell

## Códigos de Saída
- `0` sucesso
- `2` usage ou falha de parse do clap
- `65` erro de dados
- `66` sem entrada
- `69` indisponível
- `70` falha de software, browser ou protocolo
- `74` falha de I/O
- `78` erro de config
- `124` timeout
- `130` cancelado por SIGINT
- `141` broken pipe
- `255` caminho fatal inesperado

## Mapa de Documentação
- [docs/HOW_TO_USE.pt-BR.md](docs/HOW_TO_USE.pt-BR.md) primeiro comando em 60 segundos
- [docs/AGENTS.pt-BR.md](docs/AGENTS.pt-BR.md) contrato de integração para agentes
- [docs/COOKBOOK.pt-BR.md](docs/COOKBOOK.pt-BR.md) receitas práticas
- [docs/CROSS_PLATFORM.pt-BR.md](docs/CROSS_PLATFORM.pt-BR.md) matriz de plataformas
- [docs/MIGRATION.pt-BR.md](docs/MIGRATION.pt-BR.md) notas de migração
- [docs/TESTING.pt-BR.md](docs/TESTING.pt-BR.md) categorias de teste
- [docs/schemas/README.md](docs/schemas/README.md) índice de JSON schemas
- [skills/browser-automation-cli-pt/SKILL.md](skills/browser-automation-cli-pt/SKILL.md) skill imperativa
- [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) histórico Keep a Changelog
- [SECURITY.pt-BR.md](SECURITY.pt-BR.md) reporte de vulnerabilidades
- [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md) fluxo do contribuidor
- [CODE_OF_CONDUCT.pt-BR.md](CODE_OF_CONDUCT.pt-BR.md) Contributor Covenant 2.1
- [llms.pt-BR.txt](llms.pt-BR.txt) mapa curto de descoberta para LLMs

## Contribuindo
- Leia [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md) antes de abrir um PR
- Siga o Código de Conduta em [CODE_OF_CONDUCT.pt-BR.md](CODE_OF_CONDUCT.pt-BR.md)

## Segurança
- Reporte vulnerabilidades em privado via [SECURITY.pt-BR.md](SECURITY.pt-BR.md)
- Contato do maintainer: daniloaguiarbr@proton.me

## Changelog
- O histórico de versões vive somente em [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md)

## Licença
- Dual license sob MIT OR Apache-2.0
- Veja [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT) e [LICENSE-APACHE](LICENSE-APACHE)
