[English](AGENTS.md) | [Português Brasileiro](AGENTS.pt-BR.md)

# Guia de Agentes — browser-automation-cli

> Corte cola de browser-tool. Mantenha um ciclo de vida de Chrome sob seu agente. Ciclo de vida: BORN EXECUTE FINALIZE DIE.


## Por que Agentes Escolhem Esta CLI
- Ownership de subprocesso é explícito e de curta duração
- Envelopes JSON reduzem scraping frágil de stdout
- Scripts multi-passo preservam refs de acessibilidade sem daemon
- Gates de categoria mantêm superfícies experimentais opt-in
- Superfície local de scrape / crawl / map / search / parse embarca como subcomandos de primeira classe
- Helpers de artefato (`print-pdf`, `monitor`, `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`) e chaves LLM XDG estendem fluxos de agente sem daemons
- Defaults duráveis vivem em flags e XDG `config path|init|show|set|get`
- v0.1.5 residual-zero em disco: GC Singleton em BORN + FINALIZE, doctor `residual_disk` / JSON `residual`, meta `locale` e `man`, inventário 63
- Carry-forward dos contratos de agente da v0.1.4: `--json-steps`, wait multi/url, pick/select-option, assert console, schema posicional, MITM capture-url, erros de usage clap em JSON


## Economia
- Evite servers de browser long-lived que vazam entre turns do agente
- Pague o custo de launch do Chrome só quando a tarefa precisa de página real
- Prefira `scrape` / `batch-scrape` / `crawl` / `map` HTTP quando só conteúdo basta
- Colapse fluxos multi-passo em um processo `run` quando refs importam
- Stream de feedback progressivo com `--json-steps` em vez de re-spawnar para status
- Reutilize `schema <cmd>` uma vez por sessão em vez de adivinhar argv


## Soberania
- Sem dependência de runtime npm no binário do produto
- Sem caminho de telemetria remota na CLI
- Chrome do sistema permanece sob a política do host do operador
- Settings de produto vivem só em flags e `config` XDG
- Logging de produto usa `--verbose` / `--debug` / `-q` e XDG `log_level`
- Cor usa `config set color`; path do Chrome usa `config set chrome_path`


## Agentes e Orquestradores Compatíveis
- O modo de integração de cada entrada abaixo é subprocesso one-shot com `--json`
- Este projeto valida localmente com cargo e scripts e2e
- Claude Code
- Codex
- Gemini CLI
- Opencode
- Cursor
- Windsurf
- VS Code Copilot
- GitHub Copilot CLI
- Cline
- Continue
- Aider
- Zed AI assistant
- JetBrains AI Assistant
- Scripts de shell local e Makefiles
- Qualquer orquestrador que possa spawnar um processo e ler stdout e exit codes


## Detalhes de Integração de Agente
- Spawne `browser-automation-cli` como subprocesso one-shot
- Passe sempre `--json` para parsing por máquina
- Leia envelopes de sucesso e erro no stdout
- Mantenha stderr só para logs humanos ou debug
- Use `commands --json` para descobrir o inventário vivo (**63 nomes de agente**)
- O inventário inclui config, mitm, workflow, scrape, batch-scrape, crawl, map, search, parse, print-pdf, monitor, qr, find-paths, sheet-write, sg-scan, sg-rewrite, extract, select-option, pick, locale, man e tools de paridade DevTools (63 no total; e2e 53 tools)
- Nota: `select-option` e `pick` são superfície multi-passo/schema only (não subcomandos clap standalone; help clap de topo lista **61** sem eles)
- Use `schema <name> --json` ou `schema --cmd <name> --json` antes de gerar argv de comandos pouco familiares
- Prefira flags para controle pontual
- Use `config init|set|get|path|show|list-keys` para defaults XDG duráveis
- Chaves completas de config (16) via `config list-keys`: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- Resolva paths com `config path --json`
- Para multi-passo que precisa de refs `@eN` compartilhadas, use um processo `run --script` (NDJSON **ou** array JSON de passos)
- Envelope final de `run --json` inclui `ok` e `steps[].data` completo
- Stream por passo NDJSON com global `--json-steps` (`step`, `cmd`, `ok`, `result`)
- Wait com texto OR: `wait --text A --text B`
- Wait multi-seletor CSS OR e campos run `url` / `url_contains` / `navigation`
- Menus de opção: `{"cmd":"pick","target":"…","option":"…"}` ou `select-option`
- Aliases de scroll no NDJSON: `{"cmd":"scroll","dy":1500}`
- Aliases de assert: `{"cmd":"assert","url_contains":"example.com"}` / `text_contains`
- Assert console: `{"cmd":"assert","kind":"console_empty"}` ou `console_no_match` + `pattern` (precisa `--capture-console`)
- CLI assert: `assert console-empty` / `assert console-no-match --pattern …`
- Em erros fail-fast de `run`, inspecione `data.steps` parcial quando presente
- Scrape com multi-formato `--format text|markdown|html|links|metadata|summary|product|branding|raw-html|screenshot` e `--engine http|browser`
- Batch/crawl: opcional `--engine browser` (default http)
- Webhook opcional de operador no scrape: `--webhook-url` (POST one-shot, não telemetria de produto)
- Capture screenshots com `grab --path <file>` (não path posicional)
- Imprima PDF com `print-pdf --url … --path …` (também dentro de `run`)
- Páginas em branco no view: passe `--allow-empty` só quando for intencional
- Extract LLM falha fechado sem XDG `openrouter_api_key`
- Localize sugestões humanas com `--lang pt-BR` ou `config set lang pt-BR` (só flags + XDG)
- Inspecione locale resolvido com `locale --json`; gere man page com `man`
- Após trabalho browser, espere residual-zero em disco: check do doctor `residual_disk` e topo `residual` (`cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes`)
- Erros de usage clap emitem JSON quando `--json` já está no argv
- Diálogo soft: `dialog accept --if-present` / `dialog dismiss --if-present`
- Beforeunload: `goto` / `reload` com `--handle-before-unload accept|dismiss`
- Página isolada: `page new --isolated-context` (contexto isolado nomeado/anon)


## Integrações do Crate
- O nome do binário é sempre `browser-automation-cli`
- Instale com `cargo install browser-automation-cli --locked` após publish no crates.io
- Em desenvolvimento, instale por path ou git
- Qualquer crate Rust de agente integra via `std::process::Command`
- Crates de padrão compatível incluem `rig-core`, `genai`, `async-openai`, `ollama-rs`, `anthropic-sdk`, `agentai`, `autoagents`, `swarms-rs`, `graphbit`, `llm-agent-runtime`
- A CLI não é dependência de library Rust desses crates
- O contrato compartilhado é argv mais JSON no stdout mais exit codes no estilo sysexits

### Exemplo Mínimo em Rust com Command
```rust
use std::process::Command;

fn main() {
    let out = Command::new("browser-automation-cli")
        .args(["-q", "--json", "version"])
        .output()
        .expect("spawn browser-automation-cli");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
}
```


## Descoberta de Superfície para Agentes
- Inventário: `browser-automation-cli commands --json` (**63** nomes de agente)
- Fragments de input: `browser-automation-cli schema <name> --json` ou `schema --cmd <name> --json`
- Paths de config: `browser-automation-cli config path --json`
- Chaves de config: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- MITM: `mitm status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact`
- Globais MITM: `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`
- Workflow: `workflow run|resume|status`
- Superfície local de scrape: `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
- Artefatos e IO local: `print-pdf`, `monitor check`, `qr encode|decode`, `find-paths` (`--glob`), `sheet-write`, `sg-scan`, `sg-rewrite`
- Multi-passo only: `select-option`, `pick`
- Meta: `locale` (diagnósticos de locale de UI), `man` (página man roff; sem Chrome)
- Extract LLM: `extract --llm --question …` (só chaves XDG)
- Saúde: `doctor --json` (descoberta de Chrome, XDG browsers_dir, origem do lighthouse, `cache_redis` quando configurado, higiene residual de disco)
- Residual: topo `residual` + check `residual_disk` com campos `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes`
- Cache: XDG `cache_backend` (`sqlite|memory|redis`) e `cache_redis_url` (somente `redis://`; `rediss://` fail-closed)
- Lighthouse: flag → XDG `lighthouse_path` → PATH; envelope `binary_source` é `real` ou `mock`


## Inventário Completo de Comandos (63)
- Fonte viva: `browser-automation-cli commands --json` (**63** nomes voltados a agentes)
- Help clap de topo lista **61** sem `select-option` e `pick` como standalone
- O e2e DevTools tool-ref cobre **53** tools (`scripts/e2e_all_52_tools.sh` é nome legado; a suite executa 53)
- Lista completa de comandos de agente:
  - Meta / descoberta: `doctor`, `commands`, `schema`, `version`, `locale`, `completions`, `man`
  - Navegação: `goto`, `back`, `forward`, `reload`, `page`, `wait`, `dialog`
  - Interação: `press`, `click-at`, `write`, `keys`, `type`, `hover`, `drag`, `fill-form`, `upload`, `scroll`
  - Multi-passo / schema only: `select-option`, `pick`
  - Observação: `view`, `eval`, `text`, `attr`, `assert`, `cookie`, `console`, `net`
  - Captura: `grab`, `print-pdf`, `monitor`, `screencast`, `lighthouse`
  - Multi-passo: `run`, `exec`
  - Extract/scrape: `extract`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
  - IO local (sem Chrome): `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`
  - Infra: `config`, `mitm`, `workflow`
  - Emulação/perf: `emulate`, `resize`, `perf`, `heap`
  - Portões de categoria: `extension`, `devtools3p`, `webmcp`
- Descubra argv com `schema <name> --json` para qualquer nome acima

## Ciclo de Vida
- Slogan (English): BORN EXECUTE FINALIZE DIE
- Um processo possui uma sessão Chrome do launch até o FINALIZE
- BORN faz scavenge de Chromium tmp Singleton-only stale (age floor 60s)
- FINALIZE é idempotente (Browser.close, wait, kill fallback) e faz dual scavenge: janela de invocação + orphans Singleton stale
- Contrato residual para agentes: após DIE espere zero processos marker CLI vivos, zero dirs marker CLI, zero lixo Singleton-only de Chromium tmp owned
- Chrome Flatpak do host **nunca** é morto ou apagado pelo GC residual do produto
- Não espere sessão ou refs `@eN` sobreviverem ao exit do processo
- Verifique com `doctor --offline --quick --json` → `residual` / check `residual_disk`


## Contrato Técnico (v0.1.5)
### REQUIRED
- Passe `--json` para consumo programático
- Trate um processo como um ciclo de vida de Chrome (BORN EXECUTE FINALIZE DIE)
- Use `run --script` para multi-passo que precisa de refs `@eN` compartilhadas (NDJSON ou array JSON)
- Prefira `--json-steps` quando o agente precisar de feedback progressivo por passo
- Prefira `schema <cmd>` posicional (também válido: `schema --cmd`)
- Cheque exit code do processo antes de confiar no stdout
- Ramifique no campo `ok` do envelope
- Mantenha gates de categoria e experimental explícitos quando necessários
- Configure settings duráveis de produto só via `config` / flags (`--lang` + XDG para idioma)
- Descubra comandos desconhecidos com `commands --json` e `schema <cmd>` ou `schema --cmd`
- Após one-shots browser, trate residual-zero como parte do sucesso: inspecione `residual` do doctor ao diagnosticar leaks

### FORBIDDEN
- Não mantenha daemon entre turns do agente
- Não invente aliases de produto como `bac`, `click` ou `screenshot`
- Não reutilize refs `@eN` entre launches de processo separados
- Não parseie stderr como canal primário de sucesso
- Não peça à CLI que mate ou apague residual de Chrome Flatpak do host
- Não habilite bypass de robots sem a política dual-flag
- Use só flags e `config` para settings de produto — **nunca** variáveis de ambiente de produto
- Não passe path posicional para `grab`; use `--path`
- Não invente preset `--device` em `emulate`; use `--user-agent`, `--viewport`, `--network-conditions`
- Não trate `select-option` / `pick` como subcomandos clap standalone; use passos de `run` / `exec`
- Não assuma sucesso silencioso de `view` vazio em about:blank sem `--allow-empty`
- Não assuma sucesso de `print-pdf` sem página navegada ou `url` explícito (GAP-013); smokes residual podem usar `print-pdf --url about:blank` como one-shot leve quando `url` está presente

### Correct Pattern
```bash
browser-automation-cli -q --timeout 60 --json goto https://example.com
browser-automation-cli -q --json view
out=$(browser-automation-cli -q --json version)
echo "$out" | jaq -e '.ok == true'
browser-automation-cli -q --json commands
browser-automation-cli -q --json config path
browser-automation-cli -q --json wait --text Example --text Domain --ms 5000
browser-automation-cli -q --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli -q --json grab --path /tmp/page.png --full-page
browser-automation-cli -q --json print-pdf --url https://example.com --path /tmp/page.pdf
browser-automation-cli -q --json find-paths 'Cargo.*' .
browser-automation-cli -q --json find-paths --glob '**/*.rs' .
browser-automation-cli -q --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx
browser-automation-cli -q --json sg-scan . --limit 50
browser-automation-cli -q --json config list-keys
browser-automation-cli -q --json schema run
browser-automation-cli -q --json --json-steps run --script '[{"cmd":"goto","url":"https://example.com"},{"cmd":"view"}]'
browser-automation-cli -q --json mitm capture-url https://example.com --seconds 20
browser-automation-cli -q --capture-console --json assert console-empty
browser-automation-cli -q --json dialog accept --if-present
browser-automation-cli -q --json goto https://example.com --handle-before-unload accept
browser-automation-cli -q --json page new --isolated-context
browser-automation-cli -q --json locale
browser-automation-cli -q --json doctor --offline --quick
```


## Envelope JSON
- Sucesso: `{"schema_version":1,"ok":true,"data":...}`
- Erro: `{"schema_version":1,"ok":false,"error":{...}}`
- Objetos de erro incluem `kind`, `message` e `exit_code` quando `--json` está ativo
- Erros fail-fast multi-passo também podem incluir `data.steps` parcial
- Sucesso de `run --json` inclui `ok` e `steps[].data` completo
- `--json-steps` streama um objeto NDJSON por passo: `step`, `cmd`, `ok`, `result`
- Erros de usage clap com `--json` no argv emitem envelopes de erro JSON
- Índice de schemas: [docs/schemas/README.md](schemas/README.md)
- Fragments vivos de input sempre vêm de `schema <cmd>` / `schema --cmd`; arquivos estáticos podem atrasar


## Códigos de Saída
- `0` sucesso
- `2` usage
- `65` data
- `66` no input
- `69` unavailable
- `70` software, browser, protocol
- `74` I/O
- `78` config
- `124` timeout
- `130` cancelled
- `141` broken pipe
