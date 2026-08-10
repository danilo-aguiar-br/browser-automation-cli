[English](AGENTS.md) | [Português Brasileiro](AGENTS.pt-BR.md)

# Guia de Agentes — browser-automation-cli

> Corte cola de browser-tool. Mantenha um ciclo de vida de Chrome sob seu agente. Ciclo de vida: BORN EXECUTE FINALIZE DIE.


## Por que Agentes Escolhem Esta CLI
- Ownership de subprocesso é explícito e de curta duração
- Envelopes JSON reduzem scraping frágil de stdout
- Scripts multi-passo preservam refs de acessibilidade sem daemon
- Gates de categoria mantêm superfícies experimentais opt-in
- Superfície local de scrape / crawl / map / search / parse embarca como subcomandos de primeira classe
- Helpers de artefato (`print-pdf`, `monitor`, `qr`, `image`, `video`, `audio`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`) e chaves LLM XDG estendem fluxos de agente sem daemons
- Defaults duráveis vivem em flags e XDG `config path|init|show|set|get`
- v0.1.8 agent-first: família anti-detecção entregue; envelope de scrape unificado; inventário **69** vivo; **204** chaves XDG
- Herdado das versões anteriores: booleano `dialog_settled` após resposta real de diálogo; XDG `dialog_settle_ms`; grab só **png|jpeg|webp** (AVIF removido); run `wait_timeout_ms` + scrape `format`/`formats` (0.1.6 adicionou `submit`/`storage`; 0.1.7 adicionou `image`+`video`+`audio`+`record`)
- Isolamento multi-aba de diálogo via `Page::session_id` / `dialog_map_key`; select nativo `via: native_select` (input e depois change)
- Lei residual-zero de disco da v0.1.5 permanece corrente: GC Singleton em BORN + FINALIZE, doctor `residual_disk` / JSON `residual`, cmds meta `locale` e `man`
- Config de produto: só flags + XDG (nunca env de produto); descubra chaves via `config list-keys --json`
- GAP-021 parcial: fixtures unit LHR; e2e lighthouse mock **SKIP**. GAP-022 residual ~53 dups multi-versão aceitos. GAP-023/024 divergências intencionais de PRD em `parity_intentional_divergences.json`
- Carry-forward dos contratos de agente da v0.1.4: `--json-steps`, wait multi/url, pick/select-option, assert console, schema posicional, MITM capture-url, erros de usage clap em JSON


## Economia
- Evite servers de browser long-lived que vazam entre turns do agente
- Pague o custo de launch do Chrome só quando a tarefa precisa de página real
- Prefira `scrape` / `batch-scrape` / `crawl` / `map` HTTP quando só conteúdo basta
- CLEAN STDOUT scrape: use sempre `--select` (ex. `source_url,title,markdown,status_code`); engine default `http`; prefira `--format markdown` + `--only-main-content`; `--max-text-chars` / XDG; opcional `--include-selector`/`--exclude-selector`, `--redact-pii`, `--with-content-hash`, `--header "Nome: valor"`, browser `--wait-ms`; multi-format + `--select` promove campos aninhados; format `json` + `--schema-json`/`--question` (OpenRouter XDG, fail-closed); batch/crawl: `--filter http_error=false`, `--sort`, `--dedup-key`, `--output-mode json|ndjson|csv`
- Map/crawl: `--use-sitemap`, `--sitemap-only`, `--include-path` / `--exclude-path`, map `--search`; batch/crawl `--filter http_error=false`, opcional `--output-mode ndjson|csv`
- Superfície local scraping-oriented one-shot — não é a hosted scraping SaaS (CAPTCHA/proxy/async SaaS TREATED fora do produto)
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
- Use `commands --json` para descobrir o inventário vivo (**69 nomes de agente**)
- O inventário inclui config, mitm, workflow, scrape, batch-scrape, crawl, map, search, parse, print-pdf, monitor, qr, find-paths, sheet-write, sg-scan, sg-rewrite, extract, submit, storage, select-option, pick, locale, man e tools de paridade DevTools (**69** no total, inclui `image`, `video`, `audio`; e2e 53 tools com lighthouse mock SKIP)
- Nota: `select-option` e `pick` estão no inventário de agente **69** (`commands --json`) e usam-se via `run` / `exec` / `schema`; **não** são subcomandos clap standalone (superfície clap de produto é **67** nomes, excluindo `help`)
- Use `schema <name> --json` ou `schema --cmd <name> --json` antes de gerar argv de comandos pouco familiares
- Prefira flags para controle pontual
- Use `config init|set|get|path|show|list-keys` para defaults XDG duráveis
- Descubra chaves vivas de config via `config list-keys --json` (não fixe contagem; inclui `dialog_settle_ms` e mais)
- Resolva paths com `config path --json`
- Para multi-passo que precisa de refs `@eN` compartilhadas, use um processo `run --script` (NDJSON **ou** array JSON de passos)
- `run --script -` lê os passos NDJSON do **stdin**, um por linha, contra uma única sessão viva
- Prefira stdin a process substitution do shell: `run --script <(printf ...)` é recusado, porque o caminho cai em `/proc/<pid>/fd/<n>` e o jail de arquivos recusa leitura fora dos roots permitidos
- Envelope final de `run --json` inclui `ok` e `steps[].data` completo
- Stream por passo NDJSON com global `--json-steps` (`step`, `cmd`, `ok`, `result`)
- Wait com texto OR: `wait --text A --text B`
- Wait multi-seletor CSS OR e campos run `url` / `url_contains` / `navigation: true` (booleano) e o prazo público **`wait_timeout_ms`**; pode devolver `matched_selector`
- Após `dialog accept|dismiss` real, leia **`dialog_settled`** (booleano). Quando true, **não** insira wait artificial antes do próximo passo de página
- Configure o orçamento de settle de diálogo só com `config set dialog_settle_ms` (XDG; nunca env de produto)
- Menus de opção: `{"cmd":"pick","target":"…","option":"…"}` ou `select-option` (`<select>` nativo → `input`+`change`, `via: native_select`)
- Envie formulário: `submit <target>` ou `{"cmd":"submit","target":"…"}`
- Storage de auth portátil: `storage export|import --path <arquivo>` (cookies + localStorage + sessionStorage)
- Formatos de encode do grab: **png | jpeg | webp** apenas — nunca `avif`
- Aliases de scroll no NDJSON: `{"cmd":"scroll","dy":1500}`
- Aliases de assert: `{"cmd":"assert","url_contains":"example.com"}` / `text_contains`
- Assert console: `{"cmd":"assert","kind":"console_empty"}` ou `console_no_match` + `pattern` (precisa `--capture-console`)
- CLI assert: `assert console-empty` / `assert console-no-match --pattern …`
- Em erros fail-fast de `run`, inspecione `data.steps` parcial quando presente
- Scrape com multi-formato `--format text|markdown|html|rawHtml|links|metadata|summary|product|branding|screenshot` e `--engine http|browser`
- `html` é o corpo processado (extração de main-content e filtros de seletor aplicados); `rawHtml` é o corpo da resposta intacto, sob a própria chave `rawHtml`
- `metadata` colhe o que o documento declara: `og_*`, `dc_*`, `article_*`, `twitter_*`, `canonical`, `favicon`, `charset`, `html_lang`, mais title/description/status_code/source_url/link_count
- Campos de metadata ausentes são omitidos, nunca emitidos como null
- Open Graph chega como `og_title`, `og_description`, `og_image`, `og_site_name`, `og_type`, `og_url`
- Dublin Core chega como `dc_creator`, `dc_title`, `dc_subject`, `dc_publisher`, `dc_date`
- Datas de artigo chegam como `article_published_time`, `article_modified_time`, `article_author`, `article_section`
- Twitter card chega como `twitter_card`, `twitter_title`, `twitter_description`, `twitter_image`, `twitter_site`
- Nunca indexe uma chave de metadata às cegas; leia a chave só após checar presença
- Passos scrape em run honram `format` / `formats` sem despejar HTML quando só texto foi pedido
- A forma do envelope de scrape é unificada desde a v0.1.8: um formato e vários produzem as mesmas chaves
- `formats` existe sempre e mapeia cada formato pedido ao seu conteúdo
- Cada formato é espelhado na própria chave de topo, então leitores de formato único seguem funcionando
- Campos de diagnóstico como `status_code` e `source_url` sobrevivem a pedido multi-formato
- Antes da v0.1.8 o pedido multi-formato descartava esses campos; nunca dependa daquela forma
- Batch/crawl: opcional `--engine browser` (default http)
- Webhook opcional de operador no scrape: `--webhook-url` (POST one-shot, não telemetria de produto)
- Capture screenshots com `grab --path <file>` (não path posicional)
- Imprima PDF com `print-pdf --url … --path …` (também dentro de `run`)
- Páginas em branco no view: passe `--allow-empty` só quando for intencional
- Extract LLM falha fechado sem XDG `openrouter_api_key`
- Localize sugestões humanas com `--lang pt-BR` ou `config set lang pt-BR` (só flags + XDG)
- Inspecione locale resolvido com `locale --json`; gere man page com `man`
- Após trabalho browser, espere residual-zero em disco quando sozinho: check do doctor `residual_disk` não `fail` e topo `residual` com zeros em `orphan_marker_dirs`, `ghost_marker_processes`, e (após DIE sozinho) `cli_marker_dirs` + `chromium_tmp_singleton_orphans`; `sibling_live_processes` é concorrência informativa; **não** exija zero `live_cli_marker_processes`
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
- Inventário: `browser-automation-cli commands --json` (**69** nomes de agente)
- Fragments de input: `browser-automation-cli schema <name> --json` ou `schema --cmd <name> --json`
- Paths de config: `browser-automation-cli config path --json`
- Chaves de config: descubra com `config list-keys --json` (inclui `dialog_settle_ms`; nunca invente env de produto)
- MITM: `mitm status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact`
- Globais MITM: `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`, `--mitm-no-redact-secrets`
- Workflow: `workflow run|resume|status`
- Superfície local de scrape: `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
- Artefatos e IO local: `print-pdf`, `monitor check`, `qr encode|decode`, `image info|convert|resize|download|exif`, `video info|download|convert|to-mp3|trim|thumbnail|manifest`, `audio info|download|convert|trim`, `find-paths` (`--glob`), `sheet-write`, `sg-scan`, `sg-rewrite`
- Forms / estado: `submit`, `storage export|import`, `select-option` / `pick` (inventário + run/exec; não clap standalone)
- Meta: `locale` (diagnósticos de locale de UI), `man` (página man roff; sem Chrome)
- Extract LLM: `extract --llm --question …` (só chaves XDG)
- Saúde: `doctor --json` (descoberta de Chrome, XDG browsers_dir, origem do lighthouse, `cache_redis` quando configurado, higiene residual de disco)
- Residual: topo `residual` + check `residual_disk` com campos `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legado), `sibling_live_processes`, `orphan_marker_dirs`, `ghost_marker_processes`, `foreign_root_orphans`, `scanned_roots`
- Cache: XDG `cache_backend` (`sqlite|memory|redis`) e `cache_redis_url` (somente `redis://`; `rediss://` fail-closed)
- Lighthouse: flag → XDG `lighthouse_path` → PATH; envelope `binary_source` é `real` ou `mock`; e2e mock é SKIP (nunca alegue PASS completo do parser lighthouse em e2e)


## Inventário Completo de Comandos (69)
- Fonte viva: `browser-automation-cli commands --json` (**69** nomes voltados a agentes)
- Superfície clap de produto é **67** nomes (exclui `select-option` / `pick` de inventário de agente)
- O e2e DevTools tool-ref cobre **53** tools (`scripts/e2e_all_52_tools.sh` é nome legado; a suite executa 53; lighthouse mock SKIP)
- Lista completa de comandos de agente (todos os **69**):
  - Meta / descoberta: `doctor`, `commands`, `schema`, `version`, `locale`, `completions`, `man`
  - Navegação: `goto`, `back`, `forward`, `reload`, `page`, `wait`, `dialog`
  - Interação: `press`, `click-at`, `write`, `keys`, `type`, `hover`, `drag`, `submit`, `fill-form`, `upload`, `scroll`
  - Agent inventory + run/exec/schema (not clap standalone): `select-option`, `pick`
  - Observação: `view`, `eval`, `text`, `attr`, `assert`, `cookie`, `storage`, `console`, `net`
  - Captura: `grab`, `print-pdf`, `monitor`, `screencast`, `lighthouse`
  - Multi-passo: `run`, `exec`, `record`
  - Extract/scrape: `extract`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`
  - IO local (sem Chrome): `qr`, `image`, `video`, `audio`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`
  - Infra: `config`, `mitm`, `workflow`
  - Emulação/perf: `emulate`, `resize`, `perf`, `heap`
  - Portões de categoria: `extension`, `devtools3p`, `webmcp`
- Lista plana completa: `doctor`, `commands`, `schema`, `version`, `locale`, `goto`, `view`, `press`, `click-at`, `write`, `keys`, `type`, `wait`, `hover`, `drag`, `submit`, `fill-form`, `select-option`, `pick`, `upload`, `back`, `forward`, `reload`, `eval`, `grab`, `print-pdf`, `monitor`, `run`, `exec`, `record`, `extract`, `text`, `scroll`, `cookie`, `storage`, `attr`, `assert`, `console`, `net`, `page`, `dialog`, `scrape`, `batch-scrape`, `crawl`, `map`, `search`, `parse`, `qr`, `image`, `video`, `audio`, `find-paths`, `sg-scan`, `sg-rewrite`, `sheet-write`, `mitm`, `workflow`, `config`, `emulate`, `resize`, `perf`, `lighthouse`, `screencast`, `heap`, `extension`, `devtools3p`, `webmcp`, `completions`, `man`
- Descubra argv com `schema <name> --json` para qualquer nome acima

## Ciclo de Vida
- Slogan (English): BORN EXECUTE FINALIZE DIE
- Um processo possui uma sessão Chrome do launch até o FINALIZE
- BORN faz scavenge de Chromium tmp Singleton-only stale (age floor 60s)
- FINALIZE é idempotente (Browser.close, wait, kill fallback) e faz dual scavenge: janela de invocação + orphans Singleton stale
- Contrato residual para agentes: após DIE sozinho espere zero `orphan_marker_dirs`, zero `ghost_marker_processes`, zero dirs marker CLI, zero lixo Singleton-only de Chromium tmp owned; `sibling_live_processes>0` é concorrência saudável; não exija zero `live_cli_marker_processes`
- Chrome Flatpak do host **nunca** é morto ou apagado pelo GC residual do produto
- Não espere sessão ou refs `@eN` sobreviverem ao exit do processo
- Verifique com `doctor --offline --quick --json` → `residual` / check `residual_disk`


## Contrato Técnico (v0.1.8)
### REQUIRED
- Passe `--json` para consumo programático
- Trate um processo como um ciclo de vida de Chrome (BORN EXECUTE FINALIZE DIE)
- Use `run --script` para multi-passo que precisa de refs `@eN` compartilhadas (NDJSON ou array JSON)
- Prefira `--json-steps` quando o agente precisar de feedback progressivo por passo
- Prefira `schema <cmd>` posicional (também válido: `schema --cmd`)
- Use caminho soft de diálogo quando opcional: `dialog accept --if-present` / `dialog dismiss --if-present`
- Após resposta real de diálogo, leia `dialog_settled`; quando true, siga para o próximo passo de página sem inventar wait
- Configure settle de diálogo só via XDG `config set dialog_settle_ms` (flags + XDG; sem env de produto)
- Honre `wait_timeout_ms` nos passos wait de run como chave pública de prazo
- Honre scrape `format` / `formats` nos passos de run (só texto não deve emitir monstro HTML)
- Leia `formats` nos envelopes de scrape; a forma não muda mais com a contagem de formatos
- Trate stealth, fingerprint HTTP/2 e ritmo humano de input como LIGADOS por padrão
- Mantenha a mascaração de segredos do MITM ligada salvo pedido explícito de `--mitm-no-redact-secrets`
- Cheque exit code do processo antes de confiar no stdout
- Ramifique no campo `ok` do envelope
- Mantenha gates de categoria e experimental explícitos quando necessários
- Configure settings duráveis de produto só via `config` / flags (`--lang` + XDG para idioma)
- Descubra comandos desconhecidos com `commands --json` (**69**) e `schema <cmd>` ou `schema --cmd`
- Descubra chaves de config com `config list-keys --json` (nunca fixe contagem de chaves)
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
- Não passe `grab --format avif` — encode AVIF foi removido (só png|jpeg|webp)
- Não invente preset `--device` em `emulate`; use `--user-agent`, `--viewport`, `--network-conditions`
- Não invoque `select-option` / `pick` como subcomandos clap de topo; use passos de `run` / `exec` (permanecem no inventário `commands --json`)
- Não invente wait artificial após `dialog_settled: true`
- Não assuma sucesso silencioso de `view` vazio em about:blank sem `--allow-empty`
- Não assuma sucesso de `print-pdf` sem página navegada ou `url` explícito (GAP-013); smokes residual podem usar `print-pdf --url about:blank` como one-shot leve quando `url` está presente
- Não alegue PASS completo do parser lighthouse em e2e quando a suite faz SKIP do caminho mock
- Não desligue a mascaração de segredos do MITM só para ler a captura com mais conforto
- Não ajuste chaves de HTTP/2 ou de ritmo de input sem decisão explícita do operador
- Não assuma que scrape multi-formato descarta diagnóstico; a forma do envelope é unificada

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
browser-automation-cli -q --json config set dialog_settle_ms 2000
browser-automation-cli -q --json goto https://example.com --handle-before-unload accept
browser-automation-cli -q --json page new --isolated-context
browser-automation-cli -q --json schema submit
browser-automation-cli -q --json schema storage
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


## Reduzindo o Payload (nunca canalize por um processador JSON)
- Estas flags são GLOBAIS e funcionam em todos os 69 comandos
- O binário as aplica sobre `data` antes de escrever, então o modelo nunca recebe o que descartaria
- `--fields PATHS` projeta caminhos pontilhados (CSV) e mantém o aninhamento documentado
- `--filter-rows EXPR` mantém linhas que casam `key=value`, `key!=value` ou `key~substring`; repetível e com AND
- `--limit-rows N` limita linhas depois de filtro, dedupe e ordenação
- `--sort-rows PATH` ordena linhas; números comparam numericamente, não como texto
- `--dedupe-by PATH` descarta repetidos, mantendo o primeiro
- `--count-only` devolve `{"count": N}` em vez das linhas
- `--truncate-content CHARS` corta toda string do payload
- `--max-output-bytes BYTES` é teto duro que descarta linhas do fim
- Medido: `doctor --offline --quick` tem 26.277 bytes; com `--fields residual.ghost_marker_processes` tem 80
- Quando uma operação de linha rodou, o envelope ganha `agent_ops` com `total`, `matched`, `truncated`, `omitted_rows`
- `--fields` opera em CAMINHOS, não em linhas, então não reporta contadores de linha — um `total` ali seria falso
- `agent_ops` é omitido inteiro quando não há o que reportar, então projeção limpa mantém o envelope anterior
- `unresolved_paths` lista todo caminho pontilhado que não resolveu, cada um com a `flag` que o pediu
- Leia sempre: `--fields typo` devolve `data:{}` e `--sort-rows typo` devolve as linhas intactas com `matched == total`
- Os dois são indistinguíveis de sucesso sem esse campo
- `truncated` é a única forma de distinguir payload curto de payload cortado — sempre leia
- Envelopes intocados nunca ganham o campo, então parsers existentes não são afetados
- Filtro que não casa nada devolve lista vazia com `ok: true`, nunca erro e nunca a lista sem filtro
- Campo ausente nunca casa, nem sob `!=`: ausência não é diferença
- Operações de linha exigem uma lista; quando `data` tem várias, o erro as nomeia e `--fields` estreita antes
- NÃO DEVE canalizar stdout por `jaq` para encolher payload — esse trabalho é do binário
- `--select`, `--filter`, `--limit` e `--sort` são flags LOCAIS de certos comandos, nunca estas globais
- Passar uma flag local como global falha no argv com erro de argumento inesperado


## Asserções sobre o Payload
- `--expect EXPR` declara o que o payload emitido precisa conter, na gramática de `--filter-rows`
- Repetível e conjugada com AND, então várias asserções valem ao mesmo tempo
- Avaliada por ÚLTIMO, sobre o payload que você recebe, então projeção e truncagem não escondem falha
- Uma expectativa vale quando ao menos UMA linha a satisfaz: `--expect status=200` pergunta "existe um 200 aqui?"
- Filtre antes quando toda linha precisa casar — `--filter-rows` estreita e `--expect` então afirma
- Expectativas não atendidas chegam em `agent_ops.expectation_unmet`, ecoadas como você as escreveu
- O exit code segue `0` por padrão, porque mudá-lo por conteúdo quebraria pipelines que ramificam nele
- `--expect-exit-code` faz o opt-in para sair com `65` quando alguma expectativa falha
- O envelope ainda é escrito primeiro: o payload é o que explica a falha
- Expressão malformada falha no argv com exit `2`, nunca como casamento vazio silencioso


## Acréscimos ao Scrape
### Ler atributos exatos
- `--format attributes` com `--attribute-selector CSS` e `--attribute-name NAME` pareadas, ambas repetíveis
- Responde "o que está exatamente nestes lugares?", pergunta que nenhum outro formato faz
- Sem ela, ler um atributo de uma lista exigia puxar `rawHtml` e parsear fora do binário
- O pareamento é posicional; contagens diferentes falham no argv em vez de descartar uma pergunta em silêncio
- Cada linha traz `selector`, `attribute`, `values` e `count`; seletor inválido acrescenta `error` e as demais linhas sobrevivem
- Lido do documento completo, então `--only-main-content` não remove os elementos que você nomeou
### Agir antes de raspar
- `--action JSON`, repetível, executa um passo de `run --script` antes da extração
- Exemplo: `--action '{"cmd":"press","target":"#load-more"}'`
- Mesma gramática de `run --script` de propósito, para uma gravação de `record` continuar reproduzível aqui
- Roda nesta sessão, entre a navegação e a extração — outra invocação perderia o efeito
- Somente no motor browser; com `--engine http` é rejeitada com exit `2` em vez de ignorada em silêncio
- Ação que falha reprova o scrape: ela era uma pré-condição que você declarou para a extração
### Ver o que mudou
- `monitor check --diff-mode git|json` reporta O QUE mudou, não apenas que mudou
- `git` emite um diff unificado como texto; `json` emite `added` e `removed` como listas que o agente lê direto
- Um diff precisa do conteúdo anterior, e o arquivo de baseline guarda só um hash
- Por isso o conteúdo fica em `<baseline>.content`, gravado sempre que a flag está ligada
- A primeira execução com a flag não tem com o que comparar e declara isso em `diff_available: false`
- `added_count` e `removed_count` reportam o tamanho real mesmo quando `diff_truncated` está marcado
- `config set monitor_diff_max_bytes` move o teto


## Demais Flags Globais
- Toda flag abaixo vale para os 69 comandos e é aceita antes ou depois do subcomando
### Saída e diagnóstico
- `--json` emite o envelope de máquina; `--json-steps` acrescenta um envelope por passo dentro de `run`
- `-q` / `--quiet` silencia a prosa no stderr; `--plain` remove ANSI da saída humana
- `--verbose` e `--debug` elevam o tracing no stderr, nunca no stdout
- `--correlation-id ID` carimba o envelope para rastrear uma execução entre ferramentas
- `--artifacts-dir DIR` escolhe onde os arquivos caem; `--dump-on-failure` grava evidência de console e rede ali
- Combine `--dump-on-failure` com `--capture-console` ou `--capture-network`, porque a captura morre com o processo
### Tempo e concorrência
- `--timeout SEGS` limita a execução inteira; `--step-timeout SEGS` limita um passo de `run`
- `--max-concurrency N` limita o fan-out dos comandos que têm algum
### Modo do browser e anti-detecção
- `--headed` renderiza uma janela real; no Linux ela vai para um display virtual privado quando há `Xvfb`
- `--no-xvfb` mantém o lançamento headed no display do próprio operador
- O `doctor` reporta `xvfb` com o comando de instalação da distribuição detectada; a CLI nunca instala nada
- `--no-stealth` desliga o disfarce; `--stealth-profile` escolhe `auto`, `chrome-linux`, `chrome-win` ou `chrome-mac`
- `auto` segue a plataforma do host, e um lançamento headless ainda recebe override de User-Agent para não anunciar `HeadlessChrome`
- `--stealth-seed SEED` fixa a identidade para que ela seja estável entre processos
- `--input-profile human|direct` e `--input-seed SEED` governam o ritmo de ponteiro e teclado
- `--warmup` visita a raiz da origem antes; `--warmup-url URL` nomeia outra porta de entrada e implica `--warmup`
- O cookie jar vive por um processo só; o envelope de scrape declara isso como `cookie_jar_persistent: false`
- O `doctor` repete esse escopo no check `cookie_jar_scope`, então o limite é descoberto sem rodar scrape
- Use `storage export` e `storage import` para levar uma sessão entre invocações
- O envelope reporta `profile_contradicts_host: true` quando o profile de stealth alega outra plataforma
- Leia esse campo antes de culpar um bloqueio: TLS e HTTP/2 carregam a pilha real, diga o User-Agent o que disser
- Defaults da família anti-detecção, todos definidos por `config set` e nunca por variável de ambiente
- `stealth` é `true`, `stealth_profile` é `auto` e `browser_mode` é `auto`
- `stealth_seed` não tem default; defina só quando precisar de identidade estável
- `http2_enabled` é `true`, então o fingerprint HTTP/2 já está ligado antes de você pedir
- `http2_initial_stream_window_size` é 6291456 e `http2_initial_connection_window_size` é 15663105
- `http2_max_header_list_size` é 262144 e `http2_max_frame_size` é 16384
- `http2_adaptive_window` governa o crescimento da janela durante a conexão viva
- Toda mudança de HTTP/2 move o fingerprint observável, então exige decisão explícita do operador
- `input_profile` é `human`, `input_move_steps` é 24 e `input_move_gap_ms` é 12
- `input_click_dwell_ms` é 65, `input_key_dwell_ms` é 45 e `input_type_delay_ms` é 95
- `input_scroll_tick_px` é 100, `input_scroll_max_ticks` é 40 e `input_scroll_settle_rounds` é 3
- `input_target_jitter_px` é 3 e espalha o ponto de pouso do ponteiro
- `input_profile direct` remove esse ritmo e é observável pela origem
- `robots_user_agent` nomeia a identidade usada ao buscar o robots
- `scrape_no_cache` faz o opt-out do cache de resposta do scrape
- `monitor_diff_max_bytes` é 65536 e limita o conteúdo de diff armazenado
### Rede
- `--proxy URL` roteia os dois motores; credenciais ficam no XDG por `config set proxy_url`, nunca no argv
- `--proxy-bypass HOSTS` acrescenta hosts que pulam o proxy
- O loopback é ignorado automaticamente sob `--proxy`, porque o canal de controle CDP é loopback
- Sem isso, uma falha de proxy aparece como timeout de inicialização do Chrome e culpa o componente errado
- `config set cdp_proxy_bypass_loopback false` faz o opt-out
- `proxy_url`, `proxy_bypass`, `proxy_username` e `proxy_password` são chaves XDG, nunca segredos no argv
- `cdp_proxy_bypass_loopback` é `true`, então o canal de controle CDP fica fora do proxy
- `--mitm` e suas companheiras `--mitm-*` interceptam tráfego; `--allow-outside-roots` permite ler e gravar fora das raízes permitidas
- A mascaração de segredos do MITM vem LIGADA por padrão, então nenhuma captura carrega segredo cru por acidente
- `--mitm-redact-secrets` apenas reafirma esse padrão de forma explícita e não muda nada
- `--mitm-no-redact-secrets` é a única forma de desligar a mascaração
- Pedir as duas ao mesmo tempo resolve para MASCARAR, porque a leitura segura de uma contradição sobre segredos é mascarar
- `--mitm-max-body-bytes` limita o corpo capturado; o teto padrão é 65536 bytes
- `--mitm-no-media-bodies` descarta corpo de imagem, vídeo e áudio da captura
- `--ignore-robots` exige também `--i-accept-robots-risk`; uma flag sozinha não contorna o robots
### Gates de recurso
- `--category-memory` para `heap`, `--category-extensions` para `extension`
- `--category-third-party` para `devtools3p`, `--category-webmcp` para `webmcp`
- `--experimental-vision` para `click-at`, `--experimental-screencast` para `screencast`
- `--lang en|pt-BR` seleciona o idioma das mensagens


## Códigos de Saída
- `0` sucesso
- `2` usage
- `6` blocked — a origem devolveu uma verificação antibot no lugar do conteúdo. O transporte teve sucesso (HTTP 200, HTML válido), então `status_code` e `http_error` relatam sucesso enquanto o corpo carrega o desafio. Leia `error.suggestion`; repetir a mesma requisição escala em direção a um banimento
- `65` data
- `66` no input
- `69` unavailable
- `70` software, browser, protocol
- `74` I/O
- `78` config
- `124` timeout
- `130` cancelled
- `141` broken pipe
