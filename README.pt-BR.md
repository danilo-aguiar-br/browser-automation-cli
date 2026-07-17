[English](README.md) | [Português Brasileiro](README.pt-BR.md)

# browser-automation-cli

> Automação one-shot do Chrome CDP para agentes de IA. NASCE, EXECUTA, FINALIZE, MORRE.

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
- O ciclo de vida é sempre NASCE, EXECUTA, FINALIZE, MORRE

## A Dor
- Fluxos de agente precisam de browser multi-passo sem daemon sticky
- Stacks Node e npm adicionam peso de runtime e superfície de supply-chain
- Ferramentas baseadas em sessão deixam Chrome órfão e ownership obscuro
- Contratos JSON costumam divergir de flags e exit codes reais

## Por que browser-automation-cli
- Um processo é dono de um ciclo completo de Chrome do launch ao kill fallback
- Trabalho multi-passo usa `run --script` NDJSON no mesmo processo
- Refs de acessibilidade `@eN` só valem dentro daquele processo
- Envelopes `--json` estáveis para agentes programáticos
- Caminho de install é Rust puro via cargo

## Superpoderes
- Navegação e ciclo de página: `goto`, `back`, `forward`, `reload`, `page`
- Input: `press`, `write`, `type`, `keys`, `hover`, `drag`, `fill-form`, `upload`
- Observação: `view`, `grab`, `extract`, `text`, `attr`, `scroll`, `assert`
- Captura: `console` e `net` com flags globais opcionais
- Profundidade DevTools: `eval`, `emulate`, `resize`, `perf`, `lighthouse`, `heap`
- Categorias opcionais: memory, extensions, third-party, webmcp
- Experimental: vision `click-at`, screencast com export via ffmpeg
- Descoberta: `doctor`, `commands`, `schema`, `completions`

## Início Rápido
```bash
cargo install --path . --locked
browser-automation-cli --version
browser-automation-cli doctor --offline --quick --json
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
- Runtime exige Chrome ou Chromium no PATH
- Opcional: `ffmpeg` para export de screencast
- Opcional: binário `lighthouse` para auditorias lighthouse

## Uso
- Passe sempre `--json` em pipelines de agente
- Mantenha diagnósticos humanos no stderr com `-q` ao pipar
- Use `--timeout` para orçamento wall-clock do processo em segundos
- Use `run --script` para sessões multi-passo que compartilham refs `@eN`

```bash
browser-automation-cli --json goto https://example.com
browser-automation-cli --capture-network --json net list --resource-types Document,XHR
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
```

## Comandos
- Descoberta: `doctor`, `commands`, `schema`, `version`, `completions`
- Navegação: `goto`, `back`, `forward`, `reload`, `scrape`
- Snapshot e input: `view`, `press`, `write`, `type`, `keys`, `wait`, `hover`, `drag`, `fill-form`, `upload`
- Conteúdo: `extract`, `text`, `scroll`, `attr`, `assert`, `grab`
- Abas e diálogos: `page`, `dialog`, `cookie`
- Captura: `console`, `net`
- Avançado: `eval`, `emulate`, `resize`, `perf`, `lighthouse`, `screencast`, `heap`
- Categorias: `extension`, `devtools3p`, `webmcp`
- Multi-passo: `run`, `exec`

## Variáveis de Ambiente
- `BROWSER_AUTOMATION_CLI_JSON` ativa envelopes JSON
- `BROWSER_AUTOMATION_CLI_QUIET` suprime prosa não-erro no stderr
- `BROWSER_AUTOMATION_CLI_VERBOSE` eleva tracing para info
- `BROWSER_AUTOMATION_CLI_DEBUG` eleva tracing para debug
- `BROWSER_AUTOMATION_CLI_TIMEOUT` define timeout global em segundos
- `BROWSER_AUTOMATION_CLI_STEP_TIMEOUT` define timeout por passo do `run`
- `BROWSER_AUTOMATION_CLI_HEADED` lança Chrome visível
- `BROWSER_AUTOMATION_CLI_ARTIFACTS_DIR` armazena artefatos
- `BROWSER_AUTOMATION_CLI_LANG` seleciona locale de mensagens
- `BROWSER_AUTOMATION_CLI_CAPTURE_CONSOLE` ativa captura de console
- `BROWSER_AUTOMATION_CLI_CAPTURE_NETWORK` ativa captura de rede
- `BROWSER_AUTOMATION_CLI_IGNORE_ROBOTS` e `BROWSER_AUTOMATION_CLI_I_ACCEPT_ROBOTS_RISK` controlam política robots
- `BROWSER_AUTOMATION_CLI_CATEGORY_MEMORY` habilita heap profundo
- `BROWSER_AUTOMATION_CLI_CATEGORY_EXTENSIONS` habilita tools de extensão
- `BROWSER_AUTOMATION_CLI_CATEGORY_THIRD_PARTY` habilita tools third-party
- `BROWSER_AUTOMATION_CLI_CATEGORY_WEBMCP` habilita tools webmcp
- `BROWSER_AUTOMATION_CLI_EXPERIMENTAL_VISION` habilita `click-at`
- `BROWSER_AUTOMATION_CLI_EXPERIMENTAL_SCREENCAST` habilita screencast
- `BROWSER_AUTOMATION_CLI_ENCRYPTION_KEY` cifra state nativo opcional
- `BROWSER_AUTOMATION_CLI_NAMESPACE` escopa namespaces de state
- `BROWSER_AUTOMATION_CLI_COLOR` e `NO_COLOR` controlam cor no stderr
- `RUST_LOG` sobrescreve filtros de tracing quando necessário

## Features
- Este crate não tem feature flags de Cargo
- Categorias opcionais são flags de processo, não features de compile-time
- `--category-memory` habilita ferramentas profundas de heap
- `--category-extensions` habilita ferramentas de extension
- `--category-third-party` habilita helpers DevTools de terceiros
- `--category-webmcp` habilita ferramentas webmcp
- `--experimental-vision` habilita `click-at`
- `--experimental-screencast` habilita export de screencast com ffmpeg

## Targets
- Documentado para `x86_64-unknown-linux-gnu`
- Documentado para `x86_64-apple-darwin`
- Documentado para `aarch64-apple-darwin`
- Documentado para `x86_64-pc-windows-msvc`
- Documentado para `aarch64-unknown-linux-musl`
- Sem suporte em `wasm32-unknown-unknown` (CDP exige browser desktop)
- Metadados docs.rs declaram esses targets após a mudança multi-target de 2026-05-01

## MSRV
- Minimum Supported Rust Version é 1.88.0
- Política: subir MSRV só em release minor ou major com nota no CHANGELOG
- Docs locais: `timeout 180 cargo doc --no-deps`
- Nightly com cfg docsrs: `RUSTDOCFLAGS="--cfg docsrs" timeout 180 cargo +nightly doc --no-deps`

## Padrões de Integração
- Claude Code, Codex, Cursor e agentes de shell disparam um processo por ação
- Planos multi-passo devem usar `run --script` em vez de encadear processos
- Parseie stdout com `jaq` e ignore stderr salvo em diagnóstico
- Veja [INTEGRATIONS.pt-BR.md](INTEGRATIONS.pt-BR.md) e [docs/AGENTS.pt-BR.md](docs/AGENTS.pt-BR.md)

## Performance
- Cold start é dominado pelo launch do Chrome, não pelo tamanho do binário Rust
- Prefira `doctor --offline --quick` para checagens de install sem rede
- Reutilize scripts multi-passo para evitar launches repetidos do Chrome

## Requisitos de Memória
- Espere memória do processo Chrome muito acima do binário da CLI
- Tools de heap exigem `--category-memory` e snapshots maiores elevam RAM
- Screencast pode invocar ffmpeg como helper externo

## FAQ de Troubleshooting
- Chrome não encontrado: instale Chromium ou Google Chrome e rode `doctor`
- Exit 69 unavailable: binário do browser ausente, bloqueado ou não lançável
- Exit 124 timeout: eleve `--timeout` ou encurte o script
- Exit 2 usage: confira flags com `browser-automation-cli help <cmd>`
- Refs `@eN` inválidas entre comandos: mantenha passos dentro de um `run`
- Network vazio: passe `--capture-network` no mesmo processo que navega

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
- [skill/browser-automation-cli-pt/SKILL.md](skill/browser-automation-cli-pt/SKILL.md) skill imperativa
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
