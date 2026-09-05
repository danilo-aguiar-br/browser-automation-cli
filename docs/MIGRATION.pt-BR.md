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
- Nota histórica: essa afirmação de alias descreve o `0.1.2` e não vale mais
- No `0.1.7` `rawHtml` é formato distinto com chave própria `rawHtml` no envelope
- Veja a seção `0.1.6 → 0.1.7` para a separação medida entre `html` e `rawHtml`
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
- O inventário em 0.1.2 era de **59 comandos** (`commands --json`)
- Suite e2e de tool-ref DevTools permanece **53 tools** (`scripts/e2e_all_52_tools.sh`)
- Schemas estáticos regeneram via `bash scripts/generate_command_schemas.sh`



## 0.1.2 → 0.1.3
Hard-close do scavenger residual de processo/tmp (A001–A002), honesty Redis/Lighthouse e superfície PRD de write/lint no `0.1.3`.
A lei canônica residual-zero de disco (GC Singleton em BORN com age ≥ 60s, dual scavenge em FINALIZE, doctor `residual_disk`) é `0.1.5` — veja a seção 0.1.4 → 0.1.5.

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
- O inventário em 0.1.3 era de **59 comandos** (`commands --json`)
- O e2e DevTools tool-ref em 0.1.3 era de **53 tools** (`scripts/e2e_all_52_tools.sh` nome legado)

### Chaves de config (lista completa em 0.1.3)
- `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`



## 0.1.3 → 0.1.4
Hard-close GAP-001…025 para observabilidade agent-first, profundidade de wait/assert, compose MITM e honesty clap:

### Observabilidade de run (GAP-020)
- Global `--json-steps`: stream de uma linha NDJSON por passo (`step`, `cmd`, `ok`, `result`)
- Envelope final de `run --json` inclui `ok` e `steps[].data` completo
- Fail-fast ainda devolve `data.steps` parcial nos envelopes de erro

### Wait multi-seletor e URL (GAP-019, GAP-024)
- Multi-seletor CSS OR: `#a, #b` e arrays `selectors` no run
- Campos wait no run: `url` (exato), `url_contains`, `navigation` (**boolean** `true` para ciclo de load — não string `"load"`)
- Wait multi-seletor bem-sucedido pode incluir `matched_selector` nos dados do resultado
- Multi `--text` OR existente permanece

### Select / pick multi-passo (GAP-023)
- Novos nomes de inventário: `select-option`, `pick` (HIG badge/popover / `role=option`)
- Disponíveis em `run` / `exec` / descoberta schema com `target` + `option`
- Não são subcomandos clap standalone (o help clap de topo em 0.1.4 listava 59 sem eles)

### Assert console kinds (GAP-025)
- Kinds no run: `console_empty`, `console_no_match` (exige `--capture-console`)
- CLI: `assert console-empty`, `assert console-no-match --pattern <re>`

### Schema posicional (GAP-022)
- `schema <cmd>` posicional além de `schema --cmd <cmd>`
- Prefira `schema <cmd>` posicional na UX de agente

### Navegação / diálogo / view / PDF honesty (GAP-003, GAP-006, GAP-012, GAP-013, GAP-001, GAP-017)
- `BeforeUnloadAction` accept|dismiss em `goto` / `reload` (`--handle-before-unload accept|dismiss`)
- Soft path de diálogo: `dialog accept --if-present` / run `if_present:true`
- `view` recusa about:blank vazio salvo `--allow-empty` / `allow_empty:true` (só GAP-012, não print-pdf)
- `print-pdf` no multi-passo `run`; recusa PDF em branco sem conteúdo navegado ou `url` de step/CLI (GAP-013)
- `parity_run_inventory` enforce `print-pdf` em `RUN_DISPATCHED_CMDS`

### Contexto isolado (GAP-004)
- `page new --isolated-context` (flag sozinha resolve para `default-isolated`) ou `--isolated-context <nome>`
- Run: `{"cmd":"page","action":"new","isolated_context":true}` ou string nomeada

### Extension install/uninstall fora do run (GAP-007)
- `extension install` / `extension uninstall` ficam intencionalmente fora do dispatch de `run`
- Use os comandos `extension` de topo; descubra via `schema extension` / `commands --json`

### Superfície dual de assert (GAP-014)
- Subcomandos CLI: `assert url|text|console|console-empty|console-no-match`
- Kinds no run: `url` / `text` / `console` / `console_empty` / `console_no_match` (mais aliases)

### MITM capture-url e globais (GAP-011)
- Superfície completa MITM: `status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact`
- `mitm capture-url <url> [--seconds N] [--har path] [--hosts …]` compose one-shot
- Flags globais: `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`
- `mitm har --out <path>` **obrigatório** para export HAR

### Scrape multi-formato e batch/crawl browser (GAP-009, GAP-010, GAP-018)
- `scrape --format` aceita CSV ou multi-formato repetível em uma invocação
- Alias `--formats` aceito onde houver suporte (GAP-018)
- Aliases de format (GAP-018): `raw-html` / `rawHtml` e tokens relacionados
- Nota histórica: esse agrupamento de aliases descreve o `0.1.4` e não vale mais
- No `0.1.7` `rawHtml` deixou de ser alias de `html` e ganhou chave própria
- `batch-scrape --engine http|browser` (default http)
- `crawl --engine http|browser` (default http)

### Clap / console / privacy (GAP-002, GAP-021, GAP-016)
- Erros de usage clap emitem envelope JSON quando `--json` está no argv
- `console dump` sempre grava um array JSON válido (`[]` quando vazio)
- Flags de privacy no launch do Chrome; sem `metrics-recording-only`

### Inventário e gates de contrato
- O inventário em 0.1.4 era de **61** nomes de agente via `commands --json` (inclui `select-option`, `pick`)
- Honesty carregada (fechada antes, ainda obrigatória em 0.1.4): lighthouse `binary_source` real|mock (GAP-008); `extract --llm` fail-closed só com chaves XDG (GAP-015)
- O help clap de topo em 0.1.4 listava **59** nomes (exclui `select-option` / `pick` de inventário de agente)
- E2e DevTools tool-ref permanece **53 tools**
- Gates: `tests/parity_run_inventory.rs`, `tests/clap_command_debug_assert.rs`
- Auditoria clap: `GlobalOpts` usa `Args` + flatten; `ArgAction::SetTrue` explícito; `value_hint`; help headings; `after_help` examples; alias `-v`

### Chaves de config (lista completa de 16 inalterada em 0.1.4)
- `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`


## 0.1.4 → 0.1.5
Hard-close de higiene residual-zero em **disco** (RES-01…12, Pass 27) e superfície meta de descoberta:

### Residual-zero em disco (processo + GC Singleton)
- Lei de produto residual-zero estende de processo/marker para higiene de disco Chromium tmp
- **BORN** GC automático cross-run: `scavenge_stale_singleton_orphans` apaga owned Singleton-only `/tmp/org.chromium.Chromium.*` (e ocultos `.org.chromium.Chromium.*`) com age **≥ 60s** e sem holder vivo
- **FINALIZE** dual scavenge: side-channels da janela de invocação + GC Singleton stale
- Prefixos temp de Chrome Flatpak do host (`com.google.Chrome.*`) **nunca** são apagados pelo GC do produto
- Constantes públicas residual (prefixo marker, age floor, caps de tamanho) anti-hardcode

### Superfície residual do doctor
- Novo check id: `residual_disk` (path-light; sem launch de Chrome para o relatório em si)
- Campo JSON de topo do doctor: `residual` (`ResidualDiskReport`)
- Campos em 0.1.5: `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes`
- Status em 0.1.5: `fail` se processos marker vivos; `warn` se restam dirs marker ou orphans Singleton; senão `pass`
- As duas linhas acima descrevem a **0.1.5** e ficam como registro histórico; o tip é diferente
- O tip 0.1.9 carrega os seis campos acrescentados na 0.1.7: `scanned_roots`, `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`
- Status no tip 0.1.9: `fail` em `orphan_marker_dirs` ou `ghost_marker_processes`; irmã viva é saudável e nunca reprova
- Contrato de agente no tip 0.1.9: **não** exija zero `live_cli_marker_processes` — veja `docs/AGENTS.pt-BR.md`

### Inventário e comandos meta
- O inventário em 0.1.5 era de `63` nomes de agente; o tip 0.1.9 é **71** (0.1.7 acrescentou `image`+`video`+`audio`+`record`, 0.1.9 acrescentou `sitemap`+`feed`) via `commands --json`
A superfície clap de produto no tip 0.1.9 é de **69** nomes (exclui `select-option` / `pick` de inventário de agente)
- Meta já no binário e no inventário: `locale` (diagnósticos de locale de UI), `man` (roff via clap_mangen; sem Chrome)
- E2e DevTools tool-ref permanece **53 tools**

### Gates locais residual (só scripts locais do mantenedor)
- Integração: `tests/residual_one_shot.rs`
- Scripts de mantenedor: `scripts/residual-check.sh`, `scripts/residual-stress.sh`
- Cobertura unit residual sob `cargo test --lib residual::`

### Chaves de config (lista completa de 16 inalterada em 0.1.5)
- `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- Idioma continua só flags + XDG: `--lang` ou `config set lang` (sem catálogos de env de produto)
- A lei residual-zero de disco introduzida aqui permanece **corrente** até a 0.1.6 (GC Singleton em BORN + FINALIZE, doctor `residual`)


## 0.1.5 → 0.1.6
Settle de diálogo agent-first, eventos nativos de select, format de scrape em run, honesty de prazo de wait, crescimento de inventário e notas residuais intencionais:

### O Que Muda
- **`dialog_settled` (GAP-054):** envelope de dados de `dialog accept|dismiss` real inclui booleano `dialog_settled`. Happy path é `true` após `Page.javascriptDialogClosed`. Agentes **não** devem inventar wait artificial antes do próximo passo de página quando settled for true
- **`dialog_settle_ms` (XDG):** `config set dialog_settle_ms <ms>` limita a espera por Closed após responder um diálogo JS (só flags + XDG; nunca env de produto)
- **Isolamento multi-aba de diálogo:** forwarders de evento de página carimbam `Page::session_id`; chaves do mapa de diálogo isolam por aba; browser-level `None` cai na sessão ativa
- **Select nativo (GAP-055):** `pick` / `select-option` em `<select>` nativo despacham `input` e depois `change` e reportam `via: native_select` (`DISPATCH_INPUT_AND_CHANGE` compartilhado)
- **`wait_timeout_ms` (GAP-053):** passos wait de run honram a chave pública de prazo (parser não descarta mais em silêncio)
- **Formats de scrape em run (GAP-057):** passos de run aceitam `format` / `formats`; pedidos só de texto não devem despejar campos `html` grandes
- **Encode AVIF removido (breaking):** `grab` suporta só **png | jpeg | webp** (features do crate `image` sem avif / core2 yanked)
- **Inventário (0.1.6 → 0.1.7):** `commands --json` listava 69 nomes (0.1.6: **`submit`**, **`storage`**; 0.1.7: **`image`**, **`video`**, **`audio`**, **`record`**); clap de topo **67** sem `select-option`/`pick` standalone
- **`submit`:** envio de formulário por form ou campo; espera navegação/requisição
- **`storage`:** `export|import --path` para cookies + localStorage + sessionStorage (path explícito)
- **Descoberta de chaves de config:** **não** alegue contagem fixa de “16 chaves” — sempre descubra com `config list-keys --json` (inclui `dialog_settle_ms` e mais)
- **Lighthouse (GAP-021 parcial↑):** fixtures unit `minimal_lhr.json` + `chrome_captured_lhr.json` (LHR real); e2e mock permanece **SKIP** — nunca alegue PASS completo do parser lighthouse em e2e
- **GAP-022 dups residuais:** ~53 multi-versão medidas; poda barata esgotada; residual aceito na 0.1.6
- **GAP-023/024 intencionais:** flags/comandos wishlist do PRD permanecem divergências — não paridade PRD completa
- **Residual-zero de disco:** lei de produto da 0.1.5 (RES-01…12) **ainda corrente**

### Inventário completo de agente (71) — tip 0.1.9 (base 0.1.7 + `record`, depois 0.1.9 `sitemap` + `feed`)

Descubra ao vivo: `browser-automation-cli commands --json`

```
assert attr back batch-scrape click-at commands completions config console cookie
crawl devtools3p dialog doctor drag emulate eval exec extension extract fill-form
find-paths forward goto grab heap hover image video audio keys lighthouse locale man map mitm monitor
net page parse perf pick press print-pdf qr record reload resize run schema scrape screencast
scroll search select-option sg-rewrite sg-scan sheet-write storage submit text type
upload version view wait webmcp workflow write
```

Nota: `pick` e `select-option` são nomes multi-passo de inventário usados em scripts `run`, então a contagem de subcomandos clap de produto é `69` no tip 0.1.9 (**71** nomes de agente menos os dois só-run). Nomes frequentemente ausentes em docs antigos: `back`, `click-at`, `completions`, `cookie`, `devtools3p`, `drag`, `fill-form`, `forward`, `hover`, `net`, `resize`, `upload`, `webmcp`.

### Migração passo a passo para agentes
1. Rebuild/instale `0.1.6` (`cargo install --path . --force --locked`)
2. Confirme versão e inventário:
```bash
browser-automation-cli --version   # 0.1.6
browser-automation-cli --json commands | jaq '.data.commands | length'  # 71
```
3. Após respostas reais de diálogo, parseie `dialog_settled`; remova waits inventados pós-diálogo quando true
4. Se o host precisar de orçamento Closed maior: `config set dialog_settle_ms <ms>` (XDG)
5. Atualize scripts wait de run para usar `wait_timeout_ms` público quando um prazo for intencional
6. Atualize passos scrape de run para passar `format` / `formats` (espere sem monstro HTML para `text`)
7. Substitua qualquer `grab --format avif` por `png`, `jpeg` ou `webp`
8. Adote `submit` / `storage` quando envio de form ou estado de auth portátil for necessário
9. Re-descubra chaves de config: `config list-keys --json` (não fixe contagens)
10. Trate lighthouse e2e mock como SKIP honesto; confie nas fixtures unit LHR para confiança do parser
11. Mantenha checks residual-zero da 0.1.5 (`doctor residual`, scripts residual locais)
12. Não assuma que flags/comandos PRD de GAP-023/024 existam salvo listados por `commands --json`

### Descoberta de chaves de config
```bash
browser-automation-cli --json config list-keys
browser-automation-cli --json config set dialog_settle_ms 2000
browser-automation-cli --json config get dialog_settle_ms
```

### Notas de rollback
- Se reverter de `0.1.6` para `0.1.5`, remova premissas de que:
  - `dialog_settled` está sempre presente após respostas de diálogo
  - `dialog_settle_ms` é chave de config
  - wait de run honra `wait_timeout_ms` como chave pública de passo
  - scrape de run honra `format`/`formats`
  - inventário é `65` (`submit` e `storage`), porque `image`, `video`, `audio` e `record` só chegam na 0.1.7
  - o inventário do tip é **71** na 0.1.9, e nenhuma tree 0.1.5 reporta isso
  - grab recusa AVIF (0.1.5 pode ainda aceitar conforme features do build)
- Campos residual-zero de disco permanecem válidos ao reverter só se permanecer em 0.1.5+

## 0.1.6 → 0.1.7
### Breaking: `rawHtml` deixou de ser alias de `html`
- Antes do `0.1.7` as grafias `html`, `rawHtml`, `raw-html`, `rawhtml` e `raw_html` colapsavam
- As cinco resolviam para um único formato interno, então pedido bruto devolvia markup processado
- No `0.1.7` `--format rawHtml` devolve o corpo da resposta intocado
- Esse corpo bruto sai sob a chave de envelope `rawHtml`
- No `0.1.7` `--format html` devolve o corpo após extração de conteúdo principal e filtros de seletor
- Esse corpo processado sai sob a chave de envelope `html`
- As grafias `raw-html`, `rawhtml` e `raw_html` continuam grafias de `rawHtml`
- Medido contra `https://doc.rust-lang.org/std/index.html` com `--engine http --only-main-content`
- `--format rawHtml` devolveu 54441 caracteres sob a chave `rawHtml`
- `--format html` devolveu 32769 caracteres sob a chave `html`
- Quem já usava `rawHtml` passa a receber bytes diferentes sob chave diferente
- Atualize todo parser que lia `data.html` enquanto pedia `rawHtml`

### Operadores de envelope: oito flags globais novas
- `--fields <PATHS>` projeta o envelope para caminhos pontilhados
- `--filter-rows <EXPR>` mantém somente as linhas que casam com a expressão
- `--limit-rows <N>` limita quantas linhas o envelope emite
- `--sort-rows <PATH>` ordena linhas por um caminho pontilhado
- `--dedupe-by <PATH>` descarta linhas que repetem um valor de caminho
- `--count-only` troca o payload por uma contagem
- `--truncate-content <CHARS>` encurta campos de conteúdo longos
- `--max-output-bytes <BYTES>` limita o tamanho do payload serializado
- O sufixo `-rows` existe porque `--select`, `--filter`, `--limit` e `--sort` já eram flags por comando
- Essas quatro não são globais e devolvem exit `2` no escopo global
- Medido: `--select`, `--filter`, `--limit` e `--sort` saem com exit `2` antes do subcomando

### Campo novo `agent_ops` no envelope
- Envelopes de sucesso ganham `agent_ops` somente quando uma dessas oito flags roda
- Não trate isso como garantia: flag que resolve limpo deixa `agent_ops` ausente
- Parseie como opcional, exatamente como `docs/schemas/envelope-success.schema.json` declara
- `agent_ops.unresolved_paths` nomeia caminho pedido que nenhuma linha carrega
- Cada entrada reporta a `flag` de origem e o `path` não resolvido
- Ausência de `agent_ops` significa que nenhum operador de envelope foi aplicado

### Honestidade de budget no doctor
- Antes, `doctor` devolvia exit `0` com stdout vazio quando o payload estourava `--max-output-bytes`
- Agora `doctor` devolve exit `2` com envelope de erro de kind `usage`
- A mensagem nomeia o budget violado e sugere `--fields` para estreitar a saída

### `scrape --format metadata` mais rico
- O formato metadata cresceu além dos cinco campos anteriores
- Ele agora colhe propriedades Open Graph, Dublin Core e `article:`
- Ele também colhe tags de Twitter card, URL canonical e favicon
- Ele também reporta `charset` do documento e `html_lang`
- Os campos seguem condicionais, então página sem a tag simplesmente a omite

### Chaves XDG novas e teto de `--urls-file`
- `batch-scrape --urls-file` agora tem teto de tamanho em vez de leitura ilimitada
- `max_urls_file_bytes` governa esse teto, default `8388608`
- `run_max_include_depth` limita includes aninhados de `run`, default `16`
- `mitm_rebind_attempts` limita retentativas de rebind do listener MITM, default `3`
- `network_idle_window_ms` define a janela de network idle, default `500`
- `dom_stable_window_ms` define a janela de estabilidade do DOM, default `500`
- `chrome_default_timeout_ms` define o timeout padrão do Chrome, default `25000`
- `drag_move_steps` define os passos intermediários de `drag`, default `6`
- `drag_move_gap_ms` define o intervalo entre movimentos de drag, default `16`
- `robots_fetch_timeout_secs` limitava o fetch de robots, default `30`. REMOVIDA em 0.1.9: era publicada pela tabela de knobs e lida por ninguém, e a única requisição de robots é governada por `robots_probe_timeout_secs`
- O total de chaves em 0.1.7 era `176`, documentado em `docs/CONFIGURATION.md`
- Descubra a lista viva com `config list-keys --json`


## 0.1.7 → 0.1.8
### Breaking nos defaults: anti-detecção e input humano ligados
- `stealth` tem default `true`, então a `0.1.8` aplica patches anti-detecção antes da primeira navegação
- Passe `--no-stealth` numa execução, ou `config set stealth false`, quando quiser o navegador intocado
- `input_profile` tem default `human`, então clique e digitação interpolam a trajetória do ponteiro
- Um gesto `human` também aplica dwell entre press e release e ritma cada tecla
- Isso custa tempo de parede por gesto contra o input instantâneo anterior
- Passe `--input-profile direct`, ou `config set input_profile direct`, para restaurar o comportamento anterior
- O custo do ritmo `human` cresce de forma superlinear com o tamanho digitado, medido em 2026-09-04 como `2281 ms` para 1 caractere, `14236 ms` para 2 e `95781 ms` para 4
- Cada dobra do tamanho digitado multiplica o tempo decorrido por cerca de 6,5, então um `type` longo esgota o `--timeout` e devolve exit `124`
- A contramedida é passar `--input-profile direct` antes de qualquer `type` longo
- Este é um defeito ABERTO rastreado em `gaps.md`, e a contramedida NUNCA é recurso de projeto
- `http2_enabled` tem default `true`
- `cdp_proxy_bypass_loopback` tem default `true` para o canal de controle CDP sobreviver a um proxy configurado

### Doze flags globais novas
- `--no-stealth` desliga os patches anti-detecção numa execução
- `--stealth-profile <PROFILE>` escolhe a identidade personificada: `auto`, `chrome-linux`, `chrome-win`, `chrome-mac`
- `--stealth-seed <SEED>` fixa essa identidade entre processos
- `--proxy <URL>` roteia a saída por `http`, `https` ou `socks5`
- `--proxy-bypass <HOSTS>` lista os hosts que ignoram o proxy
- `--input-profile <PROFILE>` seleciona `human` (default) ou `direct`
- `--input-seed <SEED>` torna uma execução `human` reproduzível
- `--warmup` visita a raiz da origem antes da URL alvo
- `--warmup-url <URL>` aquece essa URL em vez da raiz da origem
- `--no-xvfb` pula o display virtual privado no Linux
- `--expect <EXPR>` asserta o payload emitido contra uma expressão
- `--expect-exit-code` transforma um `--expect` não atendido em exit `65`

### Subcomando novo `config unset`
- `config unset <KEY>` restaura uma chave ao default embutido
- É o inverso de `config set`, o que `config set <key> ""` nunca foi
- Em chave string a grafia vazia grava um valor que o caminho normal nunca produz
- Em chave numérica a grafia vazia é erro de parse
- Desfazer chave já ausente tem sucesso, então um script não precisa saber o estado anterior
- Migre para `config unset` toda edição manual do arquivo XDG que removia chave

### Uma forma única de envelope no `scrape`
- Antes, o envelope mudava de forma conforme a aridade de `--format`
- Com um formato devolvia o conteúdo mais todo o diagnóstico no topo
- Com dois formatos devolvia quatro chaves, movia o conteúdo para `formats` e o diagnóstico sumia
- Nessa segunda forma `--fields markdown` devolvia `data` vazio com `ok: true` e exit `0`
- Na `0.1.8` `formats` e `format_list` estão sempre presentes, qualquer que seja a aridade
- Cada formato também é espelhado no topo, então a grafia de formato único continua funcionando
- O campo vindo do transporte vence o derivado de mesmo nome, então o topo continua significando o que voltou no fio
- Pare de ramificar na contagem de chaves do envelope

### Chaves XDG novas por família
- A superfície cresceu de `176` chaves na `0.1.7` para `204` na `0.1.8`
- Nenhuma chave foi removida, então a migração é aditiva e nenhuma configuração existente quebra
- Anti-detecção: `stealth`, `stealth_profile`, `stealth_seed`
- Janela: `browser_mode` aceita `auto|headed|headless`, `auto` resolve para headless, e o `doctor` reporta o modo efetivo
- Proxy de saída: `proxy_url`, `proxy_bypass`, `proxy_username`, `proxy_password`, `cdp_proxy_bypass_loopback`
- Fingerprint HTTP/2: `http2_enabled`, `http2_initial_stream_window_size`, `http2_initial_connection_window_size`, `http2_max_header_list_size`, `http2_max_frame_size`, `http2_adaptive_window`
- Input: `input_profile`, `input_move_steps`, `input_move_gap_ms`, `input_click_dwell_ms`, `input_key_dwell_ms`, `input_type_delay_ms`, `input_scroll_tick_px`, `input_scroll_max_ticks`, `input_target_jitter_px`, `input_scroll_settle_rounds`
- Avulsas: `robots_user_agent`, `scrape_no_cache`, `monitor_diff_max_bytes`
- Descubra a lista viva com `config list-keys --json`

### Credenciais de proxy pertencem ao arquivo XDG
- Defina `proxy_username` e `proxy_password` somente com `config set`
- NUNCA passe credencial de proxy em argv: a tabela de processos expõe argv a qualquer usuário da máquina

### Inventário
- A contagem de comandos permanece `69`, sem comando novo e sem comando removido


## 0.1.8 → 0.1.9
### Breaking: `cookie clear` exige `--all`
- `cookie clear` não tomava argumento nenhum e limpava o jar inteiro
- O escopo vinha da AUSÊNCIA de uma flag, e não de algo que o chamador escreveu
- Isso é autoridade ambiente sobre um verbo irreversível
- Na `0.1.9` um `cookie clear` pelado é erro de uso
- O parser recusa com exit `2`, antes de qualquer lançamento de navegador
- O CDP não oferece limpeza parcial, então `--all` NÃO restringe o escopo
- A flag obriga o chamador a DECLARAR o escopo que antes ficava implícito
- `target_source` no envelope passa de `ambient` para `argv`
- A migração é mecânica: acrescente `--all` a toda invocação existente
- Nada mais no verbo mudou, então o jar é limpo exatamente como antes

### Breaking: `mitm block` exige o alvo em argv
- Antes da `0.1.9`, `mitm block` não aceitava nem `--host` nem `--path`
- Ele produzia uma regra que não nomeia alvo nenhum
- Agora pelo menos um entre `--host` e `--path` é obrigatório na linha de comando
- Um `mitm block` sem nenhum dos dois é erro de uso com exit `2`
- Acrescente o alvo que você já pretendia a todo script que dependia da forma pelada

### Breaking: `mitm block` agora bloqueia de verdade
- Antes da `0.1.9` o verbo escrevia a regra em `block_rules.json` e respondia `{"ok": true}`
- Nada jamais lia esse arquivo de volta
- O tráfego que o operador mandou recusar passava intacto, atrás de um envelope de sucesso
- Agora a requisição que casa uma regra é curto-circuitada com `204 No Content`
- A recusa acontece antes de qualquer resolução DNS e antes de qualquer conexão
- A recusa fica registrada na captura, então requisição bloqueada se distingue de requisição que nunca aconteceu
- O casamento de host é insensível a maiúsculas
- O casamento de path é prefixo ancorado
- Uma regra que traz AMBOS `--host` e `--path` exige que os DOIS casem
- Lida como OU, essa regra recusaria tráfego que o operador nunca nomeou
- O custo da migração cai sobre quem dependia de o verbo ser inócuo
- O tráfego que antes fluía agora para
- Audite toda regra que você mantiver antes de apontar a `0.1.9` para tráfego vivo

### Breaking: `sg-rewrite --apply` exige a raiz em argv
- Antes da `0.1.9`, `--apply` assumia o diretório corrente como raiz padrão enquanto gravava no disco
- Agora a raiz precisa ser nomeada na linha de comando
- Passe `.` explicitamente para manter o comportamento anterior

### Buffer de corpo do MITM é limitado na LEITURA
- Antes da `0.1.9`, corpo sem `content-length` declarado era admitido inteiro
- Esse é o caso `chunked`, que é a norma e não a exceção
- Os leitores coletavam o corpo sem guarda, então o par remoto decidia quanta memória este processo alocava
- As duas direções agora leem através de um leitor limitado
- Um corpo `chunked` acima de 8 MiB chega VAZIO em vez de esgotar a memória
- Isso é distinto de `--mitm-max-body-bytes`
- Aquela flag corta o que fica RETIDO, depois de o corpo inteiro já estar residente
- Um agente que parseava corpos chunked grandes de uma captura agora precisa esperar corpo vazio acima desse teto

### Chave XDG `robots_fetch_timeout_secs` removida
- `robots_fetch_timeout_secs` foi REMOVIDA na `0.1.9`
- Ela era publicada pela tabela de knobs e lida por ninguém
- A única requisição de robots é governada por `robots_probe_timeout_secs`
- Remova a chave de qualquer arquivo XDG que a definia
- Mova o valor pretendido para `robots_probe_timeout_secs`

### Superfície XDG na 0.1.9
- A superfície tem `217` chaves na `0.1.9`, medida com `config list-keys --json`
- Eram `204` chaves na `0.1.8`
- Não fixe esse número no código: descubra a lista viva no host em que você executa

### Inventário
- O inventário cresce de `69` nomes na `0.1.8` para **71** na `0.1.9`
- Os dois nomes novos são `sitemap` e `feed`
- Nenhum dos dois acrescenta capacidade, os dois acrescentam descoberta

### Migração passo a passo para agentes
- Acrescente `--all` a toda invocação de `cookie clear`
- Nomeie um alvo em toda invocação de `mitm block`
- Audite as regras de bloqueio que você mantiver antes de apontar a `0.1.9` para tráfego vivo
- Passe a raiz explicitamente para `sg-rewrite --apply`
- Retire `robots_fetch_timeout_secs` do arquivo XDG e defina `robots_probe_timeout_secs` no lugar
- Redescubra a lista de chaves em vez de confiar numa contagem copiada

```bash
browser-automation-cli --json cookie clear --all
browser-automation-cli --json mitm block --host example.com --path /ads
browser-automation-cli --json sg-rewrite . --apply
browser-automation-cli --json config set robots_probe_timeout_secs 10
browser-automation-cli --json config list-keys
```


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


### De 0.1.3 para 0.1.4
- Rebuild/instale `0.1.4`
- Prefira feedback progressivo de agente com global `--json-steps` em `run`
- Espere envelopes de sucesso `run --json` com `steps[].data` completo e `ok`
- Atualize scripts wait para multi-seletor OR e `url` / `url_contains` / `navigation: true`
- Use `page new --isolated-context` / `isolated_context` no run para contextos isolados nomeados (GAP-004)
- Mantenha `extension install|uninstall` só no topo, fora do `run` (GAP-007)
- Prefira as superfícies duais de assert: CLI `assert console-empty` e run `kind: console_empty` (GAP-014)
- Prefira scrape `--format` multi/CSV ou o alias `--formats` (GAP-018)
- Use `select-option` / `pick` só dentro de `run` / `exec` (não como cmds clap standalone)
- Adote assert console kinds: `console_empty` / `console_no_match` (CLI `console-empty` / `console-no-match`)
- Prefira `schema run` posicional; `schema --cmd run` ainda funciona
- Para MITM one-shot navega+captura: `mitm capture-url <url>`; exporte com `mitm har --out <path>`
- Flags globais MITM opcionais ao rotear Chrome: `--mitm`, `--mitm-har`, `--mitm-redact-secrets`, …
- Passe scrape multi-formato: `--format markdown,html,links`
- Prefira `batch-scrape --engine browser` / `crawl --engine browser` quando render JS for necessário (default continua http)
- Trate `view` vazio / `print-pdf` em branco com honesty (`--allow-empty` só quando intencional)
- Confirme inventário com `commands --json` (61) e regenere schemas se empacotar docs
- Reexecute validação local: `cargo test --lib`, `parity_run_inventory`, `clap_command_debug_assert`, e2e 53 tools, smokes residuais

### De 0.1.4 para 0.1.5
- Rebuild/instale `0.1.5`
- Espere residual-zero em disco após cada one-shot: BORN + FINALIZE scavenge Singleton-only Chromium tmp
- Parseie JSON do doctor para topo `residual` e check `residual_disk` ao diagnosticar leaks
- Não dependa do GC residual apagar temp de Chrome Flatpak do host (nunca é alvo)
- Descubra comandos meta: `locale`, `man` (já no inventário; confirme com `commands --json`)
- Confirme inventário com `commands --json` (`69`) e regenere schemas se empacotar docs
- Prefira gates residual ao validar paths browser:
```bash
cargo test --lib residual:: --locked
cargo test --test residual_one_shot --locked
bash scripts/residual-check.sh
# opcional: bash scripts/residual-stress.sh
```
- Idioma e todos os settings de produto permanecem só flags + XDG (`--lang` / `config set lang`)
- Reexecute validação local: `cargo test --lib`, suite residual acima, `parity_run_inventory`, `clap_command_debug_assert`, script e2e 53 tools

### De 0.1.5 para 0.1.6
- Rebuild/instale `0.1.6`
- Leia `dialog_settled` após respostas reais de diálogo; remova waits inventados quando true
- Defina `dialog_settle_ms` só via XDG quando necessário
- Use `wait_timeout_ms` em passos wait de run para prazos públicos
- Passe `format`/`formats` em passos scrape de run
- Pare de usar `grab --format avif` (só png|jpeg|webp)
- Descubra `submit` e `storage` via `commands --json` / `schema`
- Confirme inventário **71**; regenere schemas se empacotar docs
- Espere lighthouse e2e mock **SKIP** (não PASS)
- Mantenha a lei residual-zero de disco da 0.1.5
- Reexecute gates locais: `dialog_multitab_gate`, `option_pick_gate`, `wait_conditions_gate`, suite residual, script e2e 53 tools

### De 0.1.6 para 0.1.7
- Rebuild/instale o `0.1.7`
- Audite todo chamador de `scrape --format rawHtml` e leia `data.rawHtml`, não `data.html`
- Mantenha `--format html` só quando quiser a extração de conteúdo principal aplicada
- Adote `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows` para moldar o envelope
- Adote `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes` para budget de payload
- Não passe `--select`, `--filter`, `--limit` ou `--sort` no escopo global: exit `2`
- Parseie `agent_ops.unresolved_paths` quando uma projeção devolver menos campos que o esperado
- Pare de tratar estouro de budget do `doctor` como exit `0` com stdout vazio
- Espere `scrape --format metadata` mais rico e trate cada campo como condicional
- Dimensione a entrada de `batch-scrape --urls-file` contra `max_urls_file_bytes`
- Redescubra chaves de config com `config list-keys --json`

### De 0.1.7 para 0.1.8
- Rebuild/instale o `0.1.8`
- Decida a postura de anti-detecção: mantenha o default de `stealth` ou passe `--no-stealth` / `config set stealth false`
- Recalcule qualquer budget de latência, porque `input_profile` tem default `human` e ritma cada gesto
- Passe `--input-profile direct` onde o input instantâneo anterior for obrigatório
- Substitua por `config unset <KEY>` toda edição manual do arquivo XDG que removia chave
- Leia `formats` / `format_list` em todo envelope de `scrape` e pare de ramificar na contagem de chaves
- Mova `proxy_username` e `proxy_password` para `config set`, nunca para argv
- Redescubra chaves de config com `config list-keys --json` (total vivo `217`)
- Confirme que o inventário continua `69` com `commands --json`

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
- Fragments vivos de input por comando vêm de `schema <cmd>` ou `schema --cmd`
- Prefira `schema <cmd>` posicional após upgrades para confirmar o binário instalado
- Snapshots estáticos em `docs/schemas/` são um índice de conveniência e podem atrasar o binário
- Adições estáticas de v0.1.1 incluem `config`, `mitm`, `workflow`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse` e `wait`
- Adições estáticas de v0.1.2 incluem `print-pdf`, `monitor`, `qr`, `find-paths` (regenere com o gerador)
- Adições estáticas de v0.1.3 incluem `sheet-write`, `sg-scan`, `sg-rewrite`; `find-paths` ganha `glob`; chaves de config incluem cache/log_to_file
- v0.1.4: fragments wait/assert/schema/run expandem multi-seletor, wait url, console asserts, json-steps; inventário adiciona `select-option`/`pick` como nomes run/schema
- v0.1.5: campos residual do doctor; inventário adiciona `locale` / `man` (meta); contrato residual-zero em disco
- v0.1.6: settle de diálogo / `dialog_settled`; `dialog_settle_ms`; run `wait_timeout_ms` + scrape `format`/`formats`; inventário **65** (`submit`, `storage`); grab remove AVIF; fixtures unit LHR de lighthouse; e2e lighthouse mock SKIP
- 0.1.7: inventário 69 adiciona pipelines locais `image` + `video` + `audio` (image info|convert|resize|download|exif; video info|download|convert|to-mp3|trim|thumbnail|manifest)
- 0.1.7: envelopes de sucesso podem carregar `agent_ops` com `unresolved_paths` quando operadores de envelope rodam
- 0.1.7: `scrape --format rawHtml` emite a chave `rawHtml`; `--format html` emite a chave `html`


## Notas de Compatibilidade
- Não existe linha estável prévia no crates.io para este repositório antes de `0.1.0`
- Limpeza de branding e histórico recriou um root commit público limpo
- O primeiro publish no crates.io ainda exige aprovação explícita do mantenedor
- Agentes que hardcoded settings fora de flags/`config` devem migrar para flags + `config set`
- Agentes que controlavam verbosity de produto fora de flags/`log_level` devem migrar para `--verbose` / `--debug` / `config set log_level`
- Integração por subprocesso permanece o único path de agente suportado
- Exit codes permanecem no estilo sysexits: `0`, `2`, `65`, `66`, `69`, `70`, `74`, `78`, `124`, `130`, `141`
- Agentes que assumiam `batch-scrape` só HTTP devem aceitar `--engine browser` opcional em 0.1.4
- Agentes que só checavam residual de processo em 0.1.3/0.1.4 devem também parsear campos de disco `residual` do doctor em 0.1.5
- Tamanho do inventário move 61 → **63** (`locale`, `man`) em 0.1.5, depois **63 → 65** (`submit`, `storage`) em 0.1.6, depois **65 → 66** (`image`), depois **66 → 67** (`video`), depois **67 → 68** (`audio`), depois **68 → 69** (`record`) na 0.1.7, depois **69 → 71** (`sitemap`, `feed`) na 0.1.9
- Agentes que tratavam `select-option`/`pick` como subcomandos clap devem usar passos `run`/`exec`
- Agentes que hardcoded “16 chaves de config” devem migrar para `config list-keys --json`
- Duplicatas residuais GAP-022 e divergências wishlist PRD GAP-023/024 são intencionais na 0.1.6 (não paridade PRD completa)
- Agentes que chamavam `image ocr` devem parar: a ação foi REMOVIDA na 0.1.7
- O agente que consome a CLI lê imagens nativamente, então OCR embutido era middleware redundante
- O OCR também arrastava o binário C externo `tesseract` para dentro de uma ferramenta rust-native
- As chaves XDG `ocr_engine`, `ocr_lang` e `tesseract_path` foram REMOVIDAS junto
- Um `config.toml` legado com essas três chaves continua carregando sem erro
- O modelo de config usa `#[serde(default)]` e NÃO define `deny_unknown_fields`
- Chaves desconhecidas são ignoradas na carga, então nenhuma edição manual é exigida
- `image` agora expõe 5 ações: `info`, `convert`, `resize`, `download`, `exif`
- `video` ganha `manifest` e passa a expor 7 ações, com `manifest` resumindo playlists HLS/DASH
- Agentes que tratavam `rawHtml` como alias de `html` precisam migrar na 0.1.7
- `rawHtml` agora devolve o corpo intocado sob a própria chave `rawHtml` do envelope
- Agentes que contavam com `doctor` saindo `0` e stdout vazio sob budget precisam tratar exit `2`


## Rollback
- Fixe o commit local anterior ou o path do binário instalado
- Mantenha scripts compatíveis com os campos `ok` e `schema_version` do envelope
- Se reverter de `0.1.6` para `0.1.5`, remova premissas sobre `dialog_settled`, `dialog_settle_ms`, honesty de `wait_timeout_ms` / scrape `format` em run, inventário 65 / `submit`+`storage` e remoção de AVIF
- Se reverter de `0.1.5` para `0.1.4`, remova premissas de que o doctor sempre emite topo `residual` / check `residual_disk`, de que BORN faz GC automático de Singleton tmp stale com age ≥ 60s, de que o inventário é 63, e de que `locale`/`man` estão sempre presentes em trees antigas sem esses cmds
- Se reverter de `0.1.4` para `0.1.3`, remova o uso de `--json-steps`, wait `url`/`url_contains`/`navigation`, arrays multi-seletor de wait, passos `select-option`/`pick`, assert `console_empty`/`console_no_match`, fluxos só-posicionais de `schema <cmd>`, `mitm capture-url` / `graphql` / `ws` / `block` / `allow` / `redact`, flags globais `--mitm*`, premissas de scrape multi-formato, `batch-scrape`/`crawl` `--engine browser`, `view --allow-empty`, recusa de PDF em branco, `page new --isolated-context`, `--handle-before-unload accept|dismiss`, e premissas de erro de usage clap em JSON
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
