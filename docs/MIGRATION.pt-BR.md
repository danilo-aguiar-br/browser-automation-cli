[English](MIGRATION.md) | [Português Brasileiro](MIGRATION.pt-BR.md)

# Migração — browser-automation-cli

> Migre para o modelo de processo one-shot sem adivinhar o mapa de comandos. Ciclo de vida: BORN EXECUTE FINALIZE DIE.


## O Que Muda
- `0.1.0` é a primeira linha pública do produto
- Nomes canônicos de comando são `view`, `press`, `write` e `grab`
- Automação multi-passo deve usar `run --script` em um processo
- Superfícies de categoria e experimental são opt-in
- Slogan de lifecycle é só em inglês: BORN EXECUTE FINALIZE DIE


## Baseline 0.1.0
- Launch one-shot do Chrome e cleanup FINALIZE em um único processo
- Navegação e interação centrais: `goto`, `view`, `press`, `write`, `grab`, `run`
- Superfície de paridade DevTools para input, snapshot, network, console, pages, wait, perf, lighthouse, screencast, heap, extensions
- Descoberta de schema via `schema --cmd` e inventário via `commands --json`
- Política dual-flag de robots para bypass explícito
- Gates de categoria como `--category-memory` e `--category-extensions`
- Gates experimentais como `--experimental-vision` e `--experimental-screencast`
- Explicitamente fora só de 0.1.0: MITM local, journal de workflow e superfície local de crawl/map/search


## 0.1.0 → 0.1.1
### Configuração e XDG
- Settings de produto usam só flags da CLI e XDG via `config init|set|get|path|show`
- `config path --json` reporta `config_dir`, `data_dir`, `state_dir`, `mitm_ca_dir`, `mitm_capture_dir`, `workflow_dir` e paths relacionados
- Chave de cifragem é definida com `config set encryption_key`
- Logging de produto é flags + XDG (`--verbose` / `--debug` / `-q` ou `config set log_level`)
- Cor é `config set color`; path do Chrome é `config set chrome_path`
- Doctor ganha check XDG de `browsers_dir`

### MITM
- Nova superfície MITM local em hudsucker
- `mitm start` faz bind em `127.0.0.1` com porta efêmera em modo one-shot
- Comandos relacionados: `status`, `init-ca`, `list`, `get`, `har`, `export`, `domains`, `apis`
- Material de CA fica sob XDG data; capturas sob XDG state

### Workflow
- Novo journal DAG de workflow (petgraph + SQLite)
- Comandos: `workflow run`, `workflow resume`, `workflow status`
- Journals ficam sob XDG state
- `workflow resume` pula passos já marcados `ok`

### Superfície local de scrape
- Novos comandos: `batch-scrape`, `crawl`, `map`, `search`, `parse`
- `scrape` ganha `--format` (`text|markdown|html|links|metadata`)
- `scrape` ganha `--engine` (`http|browser`) e `--only-main-content`
- Batch scrape usa concorrência limitada via Tokio `JoinSet`

### Flags de interação e captura
- `wait` aceita `--text` repetível com semântica OR (qualquer match resolve)
- `grab` usa `--path` (não path posicional)
- `emulate` usa `--user-agent`, `--viewport`, `--network-conditions` (sem preset `--device`)
- `run` ganha opções de paridade com scrape e aplica gates de categoria dentro dos passos do script

### Empacotamento e docs
- Documentação e skills bilíngues públicas para o pacote crates.io
- Dual license `MIT OR Apache-2.0`
- Validação local com cargo e scripts e2e


## 0.1.1 → 0.1.2
Correções de GAP e crescimento de superfície em alto nível no `0.1.2`:

### Scrape browser e formatos
- Scrape com engine browser captura `outerHTML` e aplica `--format` (markdown/html/links/metadata/…) em vez de text silencioso
- Tokens extras de format: `summary`, `product`, `branding`, além de aliases `raw-html` / `rawHtml` e token `screenshot`
- `--webhook-url` opcional no scrape: POST one-shot do operador com os dados do resultado (não telemetria de produto)

### Ergonomia do script run
- Scroll NDJSON aceita aliases `dy` / `dx` para `delta_y` / `delta_x`
- Assert aceita aliases `url_contains` / `text_contains`
- Erros fail-fast de `run` devolvem `data.steps` parcial no envelope de erro para recuperação
- `schema --cmd` expandido para flags tool-ref de goto/eval/type/scroll/assert
- Help de `exec` descreve a superfície completa de steps

### Logging, Chrome e paths de Lighthouse
- Settings de produto ficam só em flags + XDG
- Logging usa `--verbose` / `--debug` / `-q` e XDG `log_level`
- Path do Chrome via XDG `chrome_path`; Lighthouse via XDG `lighthouse_path` (mais flag)
- Cor via XDG `color`

### i18n
- Sugestões humanas localizam para `pt-BR` via `--lang` e XDG `config set lang`

### Search e attr
- Search limpa wrappers de redirect SERP (`uddg=`) para URLs de destino
- `attr` faz fallback para propriedades DOM quando atributos HTML são null

### Novos comandos e parse/LLM
- `print-pdf` — artefato one-shot CDP `Page.printToPDF`
- `monitor check` — comparação de hash/texto com baseline e `--write-baseline` opcional
- `qr encode|decode` — sem Chrome
- `find-paths` — descoberta de paths estilo fd (sem Chrome)
- `parse` — PDF (lopdf), DOCX, xlsx/ods (calamine), mais `--redact-pii`
- `extract --llm` / `--question` / `--schema-json` com chaves só XDG: `openrouter_api_key`, `llm_base_url`, `llm_model` (fail-closed sem chave)
- MITM reporta `ws_count`

### Chaves de config (lista completa em 0.1.2)
- `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`
- Mais: `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`

### Inventário
- Inventário vivo é **59 comandos** (`commands --json`)
- Suite e2e de tool-ref DevTools permanece **53 tools** (`scripts/e2e_all_52_tools.sh`)
- Schemas estáticos regeneram via `bash scripts/generate_command_schemas.sh`



## 0.1.2 → 0.1.3
Hard-close residual-zero, honesty Redis/Lighthouse e superfície PRD de write/lint no `0.1.3`:

### Residual e2e e scavenger (A001–A002)
- Medição residual do e2e sem self-match; harness residual seguro com pipefail
- FINALIZE faz scavenge de orphans Chromium em `/tmp` owned

### Contrato `run` (A003)
- `run --script` aceita **NDJSON** (um objeto por linha) **ou** um **array JSON** de passos
- Erros fail-fast ainda devolvem `data.steps` parcial quando presente

### Navegação / CDP honesty (A004–A006, A009, A012)
- `scrape --engine http` rejeita `file://` com Usage + suggestion (engine `browser` ou `parse`)
- `reload` usa CDP `Page.reload` com `ignoreCache` quando `--ignore-cache`
- `init_script` é removido após navigation/reload
- `handle_before_unload` auto-aceita via pump de dialog CDP (sem inject `preventDefault`)
- Eventos CDP desconhecidos são ignorados para a captura de rede continuar

### Redis / cache (A007–A008)
- Novas chaves XDG: `cache_backend`, `cache_redis_url`, além de `log_to_file`
- `rediss://` é fail-closed (somente TCP plain)
- Doctor reporta `cache_redis` quando cache Redis está configurado
- Unit RESP mock always-on; redis-server real opcional se presente no host

### Lighthouse honesty (A010)
- Ordem de resolve: flag `--lighthouse-path` → XDG `lighthouse_path` → PATH
- Envelope reporta `binary_source` como `real` ou `mock`
- Doctor reporta a origem do lighthouse com honesty

### Superfície PRD write/lint (A011)
- `find-paths --glob` filtro glob estilo shell
- `sheet-write` CSV/JSON → XLSX (sem Chrome)
- `sg-scan` lint estrutural; `sg-rewrite` dry-run padrão com `--apply`

### Outra superfície 0.1.3
- `page tab-id` (tool-ref `get_tab_id`) expande e2e para **53** tools
- `config list-keys` lista chaves e defaults
- Inventário vivo é **59 comandos** (`commands --json`)
- E2e DevTools tool-ref é **53 tools** (`scripts/e2e_all_52_tools.sh` nome legado)

### Chaves de config (lista completa em 0.1.3)
- `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`

## Migração Passo a Passo
### De qualquer tree antiga para 0.1.1
- Instale ou rebuild o binário para pelo menos `0.1.1`
- Substitua chamadas de session-daemon por invocações one-shot
- Reescreva planos multi-passo de agente em scripts NDJSON para `run`
- Mude consumidores de output para envelopes `--json`
- Mova defaults duráveis para `config set` ou mantenha-os como flags explícitas
- Mova material de cifragem para `config set encryption_key <secret>`
- Mapeie nomes antigos de tools via `commands --json` e o tool map DevTools
- Atualize callers de screenshot para `grab --path <file>`
- Atualize waits que precisam de textos alternativos para `--text` repetível (OR)
- Atualize callers de scrape para passar `--format` e `--engine` de forma explícita quando necessário

### De 0.1.1 para 0.1.2
- Rebuild/instale `0.1.2`
- Use `--verbose`, `--debug`, `-q` ou `config set log_level` para logging de produto
- Prefira XDG `chrome_path` / `lighthouse_path` quando a descoberta por PATH for frágil
- Prefira `config set color` para defaults de cor ANSI
- Espere que formatos de scrape browser funcionem (`--engine browser --format markdown|links|…`)
- Prefira aliases de scroll `dy`/`dx` e de assert `url_contains`/`text_contains` no NDJSON
- Em falhas de `run`, parseie `data.steps` parcial quando presente
- Descubra novos comandos: `print-pdf`, `monitor`, `qr`, `find-paths`
- Para webhooks de scrape do operador, passe `--webhook-url` em `scrape`
- Para extract LLM, defina só chaves XDG via `config set`:
```bash
browser-automation-cli --json config set openrouter_api_key YOUR_KEY
browser-automation-cli --json config set llm_base_url https://openrouter.ai/api/v1
browser-automation-cli --json config set llm_model openai/gpt-4o-mini
browser-automation-cli --json extract https://example.com --llm --question 'What is the title?'
```
- Use `--lang pt-BR` ou `config set lang pt-BR` para sugestões humanas localizadas
- Confirme inventário com `commands --json` (59) e regenere schemas se empacotar docs
- Reexecute validação local com cargo e scripts e2e: `cargo test --lib`, script e2e de 53 tools, smokes residuais que importam



### De 0.1.2 para 0.1.3
- Rebuild/instale `0.1.3`
- Atualize agentes: `run --script` pode usar array JSON de passos além de NDJSON
- Não passe `file://` para `scrape --engine http`
- Descubra novos comandos: `sheet-write`, `sg-scan`, `sg-rewrite` e `find-paths --glob`
- Configure Redis só via XDG: `config set cache_backend redis` e `config set cache_redis_url redis://…`
- Nunca use `rediss://` (fail-closed)
- Espere envelopes lighthouse com `binary_source`
- Confirme inventário com `commands --json` (59) e regenere schemas se empacotar docs
- Reexecute validação local: `cargo test --lib`, script e2e de 53 tools, smokes residuais de PRD

## Mudanças de JSON Schema
- Antes: prosa livre ou JSON ad-hoc sem `schema_version`
- Depois no sucesso:
```json
{"schema_version":1,"ok":true,"data":{}}
```
- Depois no erro com `--json`:
```json
{"schema_version":1,"ok":false,"error":{"message":"..."}}
```
- Envelopes de erro também carregam `kind` e `exit_code` para ramificação programática
- Erros multi-passo fail-fast podem incluir `data` parcial (por exemplo `data.steps`)
- Fragments vivos de input por comando vêm de `schema --cmd`
- Snapshots estáticos em `docs/schemas/` são um índice de conveniência e podem atrasar o binário
- Adições estáticas de v0.1.1 incluem `config`, `mitm`, `workflow`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse` e `wait`
- Adições estáticas de v0.1.2 incluem `print-pdf`, `monitor`, `qr`, `find-paths` (regenere com o gerador)
- Adições estáticas de v0.1.3 incluem `sheet-write`, `sg-scan`, `sg-rewrite`; `find-paths` ganha `glob`; chaves de config incluem cache/log_to_file
- Prefira `schema --cmd` ao vivo após upgrades para confirmar o binário instalado


## Notas de Compatibilidade
- Não existe linha estável prévia no crates.io para este repositório antes de `0.1.0`
- Limpeza de branding e histórico recriou um root commit público limpo
- O primeiro publish no crates.io ainda exige aprovação explícita do mantenedor
- Agentes que hardcoded settings fora de flags/`config` devem migrar para flags + `config set`
- Agentes que controlavam verbosity de produto fora de flags/`log_level` devem migrar para `--verbose` / `--debug` / `config set log_level`
- Integração por subprocesso permanece o único path de agente suportado
- Exit codes permanecem no estilo sysexits: `0`, `2`, `65`, `66`, `69`, `70`, `74`, `78`, `124`, `130`, `141`


## Rollback
- Fixe o commit local anterior ou o path do binário instalado
- Mantenha scripts compatíveis com os campos `ok` e `schema_version` do envelope
- Se reverter de `0.1.3` para `0.1.2`, remova o uso de `sheet-write`, `sg-scan`, `sg-rewrite`, `find-paths --glob`, scripts `run` só em array JSON, chaves XDG de cache e premissas de `binary_source`
- Se reverter de `0.1.2` para `0.1.1`, remova o uso de `print-pdf`, `monitor`, `qr`, `find-paths`, `parse --redact-pii`, `extract --llm` e as novas chaves de config
- Se reverter de `0.1.2`, também remova premissas de que formatos de scrape browser, scroll `dy`/`dx`, aliases contains de assert, `data.steps` fail-fast, scrape `--webhook-url` ou logging via flags/XDG sempre se aplicam
- Se reverter de `0.1.1` para `0.1.0`, remova o uso de config, mitm, workflow, batch-scrape, crawl, map, search, parse
- Se reverter, também remova premissas de scrape `--format`/`--engine` que dependem de `0.1.1`
- Se reverter, restaure wrappers de wait ou grab que assumiam argv antigo só se o seu fork os tinha
- Mantenha settings em flags e `config` mesmo ao mirar trees antigas


## Veja Também
- [CHANGELOG.pt-BR.md](../CHANGELOG.pt-BR.md)
- [docs/AGENTS.pt-BR.md](AGENTS.pt-BR.md)
- [docs/CROSS_PLATFORM.pt-BR.md](CROSS_PLATFORM.pt-BR.md)
- [docs/TESTING.pt-BR.md](TESTING.pt-BR.md)
- [docs/schemas/README.md](schemas/README.md)
