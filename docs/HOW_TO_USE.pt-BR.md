[English](HOW_TO_USE.md) | [Português Brasileiro](HOW_TO_USE.pt-BR.md)

# Como Usar — browser-automation-cli

> Instale uma vez, lance o Chrome uma vez por processo, termine a tarefa e saia limpo. Ciclo de vida: BORN EXECUTE FINALIZE DIE.


## Pré-requisitos
- Rust 1.88.0 ou mais recente ao compilar a partir do source
- Chrome ou Chromium disponível no PATH (ou defina XDG `chrome_path`) para comandos com engine browser
- ffmpeg opcional para export de screencast experimental
- binário Lighthouse opcional para auditorias, ou passe `--lighthouse-path` / XDG `lighthouse_path` para um mock
- Um shell capaz de pipear stdout e inspecionar códigos de saída


## Primeiro Comando em 60 Segundos
```bash
cargo install --path . --locked
browser-automation-cli doctor --offline --quick --json
browser-automation-cli --timeout 60 --json goto https://example.com
browser-automation-cli --json view
```
- Doctor verifica descoberta do Chrome e prontidão one-shot sem sonda longa de rede
- Goto navega em um processo one-shot fresco (BORN → EXECUTE → FINALIZE → DIE)
- View imprime snapshot de acessibilidade com refs `@eN` apenas no processo atual
- Prefira `--json` desde a primeira chamada quando uma máquina for parsear o stdout


## Comandos Core
- Navegue com `goto`, `back`, `forward`, `reload`
- Faça snapshot da página com `view` (about:blank vazio recusa sucesso silencioso salvo `--allow-empty`)
- Clique com `press` usando seletor CSS ou ref `@eN`
- Preencha inputs com `write` e formulários multi-campo com `fill-form`
- Espere com `wait --ms`, `--text` repetível (OR), `--selector` (CSS multi-seletor OR) e `--state` opcional
- Capture screenshot com `grab --path /tmp/page.png` (flag, não caminho posicional; encode **png|jpeg|webp** apenas — **AVIF removido** na v0.1.6)
- Envie formulário com `submit <target>` (form ou qualquer campo dentro dele; espera navegação/requisição)
- Exporte/importe estado de auth portátil com `storage export|import --path <arquivo>` (cookies + localStorage + sessionStorage)
- Imprima a página em PDF com `print-pdf --url <url> --path /tmp/page.pdf` (também válido dentro de `run`)
- Extraia conteúdo com multi-formato `scrape --format markdown,html,links` quando precisar de várias formas de uma vez
- Parseie arquivos locais com `parse` (html/md/txt/pdf/docx/xlsx/ods; opcional `--redact-pii`)
- Codifique ou decodifique QR com `qr encode|decode` (sem Chrome)
- Processe imagens localmente com `image info|convert|resize|download|exif` (sem Chrome; sem base64 de pixels por padrão)
- Processe vídeos localmente com `video info|download|convert|to-mp3|trim|thumbnail|manifest` (sem Chrome; ffmpeg opcional via XDG `ffmpeg_path`; só path/meta; smart copy/re-encode)
- Resuma manifesto HLS/DASH sem baixar mídia com `video manifest`
- Processe áudio localmente com `audio info|download|convert|trim` (sem Chrome; ffmpeg opcional via XDG `ffmpeg_path`; só path/meta; smart copy/re-encode; upload via `upload` existente)
- Screenshot com `grab --format png|jpeg|webp` (opt-in `--include-base64`); receitas no COOKBOOK
- Descubra paths no filesystem com `find-paths` (pattern regex e/ou `--glob '**/*.rs'`; sem Chrome)
- Escreva XLSX a partir de CSV/JSON com `sheet-write <input> -o <out.xlsx>` (sem Chrome)
- Lint estrutural com `sg-scan [paths…]` e rewrite dry-run com `sg-rewrite [paths…]` (`--apply` para gravar)
- Verifique mudança de página contra baseline com `monitor check`
- Liste o inventário vivo (**69** nomes de agente) com `commands --json`
- Descubra formatos de argv com `schema <name> --json` ou `schema --cmd <name> --json`
- Imprima a versão do produto com `version`
- Inspecione o locale de UI resolvido com `locale --json` (só sugestões humanas)
- Gere uma página man com `man` (roff; sem Chrome)
- Descubra chaves XDG vivas com `config list-keys --json` (inclui `dialog_settle_ms`; nunca fixe uma contagem de chaves)

```bash
browser-automation-cli --timeout 60 --json goto https://example.com
browser-automation-cli --json view
browser-automation-cli --json wait --text "Example Domain" --ms 3000
browser-automation-cli --json grab --path /tmp/page.png --full-page
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/page.pdf
browser-automation-cli --json schema run
```


## Multi-passo com Run
- Use `run --script` quando refs `@eN` precisam sobreviver entre passos
- Launches de processos separados nunca compartilham refs nem a sessão do Chrome
- Um processo é um ciclo de vida: BORN EXECUTE FINALIZE DIE
- Não existe modo daemon de produto
- Em erro fail-fast, o envelope de erro pode incluir `data.steps` parcial para recuperação
- O corpo do script aceita **NDJSON** (um objeto JSON por linha) **ou** um **array JSON** de passos no topo
- `run --script -` lê os passos NDJSON do **stdin**, um por linha, contra uma única sessão viva
- Modo stdin continua one-shot: um BORN, um DIE, sem daemon; EOF no stdin dispara o FINALIZE
- Modo stdin valida cada linha ao chegar e reporta `validation: "per-line"`
- Modo arquivo, ao contrário, pré-valida o script inteiro antes do BORN, então typo nunca lança o Chrome
- Modo stdin mantém fail-fast: a primeira linha que falha para o loop e o envelope carrega os passos executados
- Modo stdin aceita somente NDJSON; array JSON no topo exige caminho de arquivo real
- Prefira stdin a process substitution do shell: `run --script <(printf ...)` é recusado, porque o caminho cai em `/proc/<pid>/fd/<n>` fora das raízes permitidas
- Envelope final `--json` inclui `ok` e `steps[].data` completo
- Global `--json-steps` streama uma linha NDJSON por passo (`step`, `cmd`, `ok`, `result`)
- Inventário de agente + multi-passo: `select-option` / `pick` com `target` + `option` (via run/exec; não clap standalone; `<select>` nativo despacha `input` e depois `change`, `via: native_select`)

```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500,"text":"Example Domain"}
{"cmd":"scroll","dy":1500}
{"cmd":"assert","url_contains":"example.com"}
{"cmd":"assert","text_contains":"Example Domain"}
{"cmd":"view"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.jsonl

# Mesmos passos como array JSON (GAP-A003)
cat > /tmp/demo.browser-automation.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"view"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.array.json

# Stream progressivo de passos (GAP-020)
browser-automation-cli --timeout 60 --json --json-steps run --script /tmp/demo.browser-automation.array.json

# Passos NDJSON direto do stdin — sem arquivo temporário (GAP-034)
printf '%s\n' \
  '{"cmd":"goto","url":"https://example.com"}' \
  '{"cmd":"view"}' \
  | browser-automation-cli --timeout 60 --json run --script -
```
- Linhas NDJSON e elementos de array usam o campo `cmd` com nome real de subcomando ou inventário run
- Scroll aceita `dy`/`dx` como aliases de `delta_y`/`delta_x`
- Assert aceita aliases `url_contains` / `text_contains` e kinds de console
- Wait aceita multi-seletor OR e campos run `url` / `url_contains` / `navigation: true` (booleano) e o prazo público **`wait_timeout_ms`** (GAP-053); sucesso multi-seletor pode incluir `matched_selector`
- Scrape em run honra `format` / `formats` (GAP-057): `{"cmd":"scrape","format":"text"}` não deve despejar campo `html` monstro quando só texto foi pedido
- Preencha formulários multi-campo em run: `{"cmd":"fill-form","fields":[{"target":"…","value":"…"}]}`
- Envie formulário em run: `{"cmd":"submit","target":"form"}` (ou um campo dentro do form)
- Envelope de dados de dialog accept/dismiss inclui **`dialog_settled`** (booleano). No happy path é `true` após `Page.javascriptDialogClosed`; **não invente wait artificial** antes do próximo passo de página quando settled for true (GAP-054)
- `view --allow-empty` / `allow_empty:true` só quando about:blank vazio for intencional
- Nota: `pick` / `select-option` **não** são subcomandos clap standalone
- Flags globais como `--timeout` e `--step-timeout` valem para o script inteiro
- Prefira caminhos HTTP de scrape quando só precisar de conteúdo e não de refs ao vivo


## Padrões Avançados
- Capture network no processo: `--capture-network` e depois `net list --json`
- Capture console no processo: `--capture-console` e depois `console list --json`
- Assert console limpo: `assert console-empty` / `assert console-no-match --pattern TypeError` (precisa capture)
- `console dump` sempre grava um array JSON válido (`[]` quando vazio)
- Emule sem perfil nomeado de device:
  - `emulate --user-agent "Mozilla/5.0 ..."`
  - `emulate --viewport 390x844x3,mobile,touch`
  - `emulate --network-conditions "Slow 3G"`
- Espere qualquer um de vários textos (semântica OR): `wait --text A --text B --ms 5000`
- Espere multi-seletor CSS OR: `wait --selector '#a, #b' --ms 5000`
- Formatos de scrape: `--format text|markdown|html|links|metadata|summary|product|branding|raw-html|screenshot` (CSV ou multi-formato repetível)
- Engines de scrape: `--engine http` (reqwest + scraper) ou `--engine browser` (CDP; formatos aplicam ao HTML capturado)
- Webhook opcional de operador com POST one-shot do resultado do scrape: `scrape ... --webhook-url https://127.0.0.1:9000/hook` (destino do operador, não telemetria de produto)
- Prefira heurística de conteúdo principal: `scrape ... --only-main-content`
- Batch scrape a partir de lista de URLs: `batch-scrape --urls-file urls.txt --format text --concurrency 2` (default `--engine http`; use `--engine browser` para páginas com JS)
- Descubra sites com `crawl` (`--engine http|browser`), `map`, `search` e arquivos locais com `parse`
- Extract LLM (fail-closed sem chaves): defina XDG `openrouter_api_key`, opcionais `llm_base_url` / `llm_model`, depois `extract <url> --llm --question '...'`
- Proxy MITM one-shot: `mitm start --seconds 30` (bind em `127.0.0.1`)
- MITM compose navega+captura: `mitm capture-url https://example.com --seconds 30 --har /tmp/cap.har`
- MITM export HAR: `mitm har --out /tmp/capture.har` (`--out` **obrigatório**)
- Superfície completa MITM: `status|list|get|har|export|domains|apis|init-ca|start|capture-url|graphql|ws|block|allow|redact`
- Flags globais MITM: `--mitm`, `--mitm-ca-dir`, `--mitm-har`, `--mitm-hosts`, `--mitm-ws`, `--mitm-max-body-bytes`, `--mitm-no-media-bodies`, `--mitm-redact-secrets`
- Journal de workflow em DAG: `workflow run|resume|status` (SQLite sob XDG state)
- Ferramentas profundas de heap exigem `--category-memory`
- Ferramentas de extension exigem `--category-extensions`
- Cliques por coordenada exigem `--experimental-vision`
- Ordem de resolve do binário Lighthouse: flag `--lighthouse-path` → XDG `lighthouse_path` → PATH
- Envelope Lighthouse reporta `binary_source` como `real` ou `mock` (mock é honesty para e2e/smoke, não produção)
- Lighthouse com caminho mock: `lighthouse https://example.com --lighthouse-path ./scripts/mock-lighthouse.sh --json`
- Cache backend só via XDG: `config set cache_backend sqlite|memory|redis` e opcional `config set cache_redis_url redis://127.0.0.1:6379`
- `rediss://` é fail-closed (somente TCP plain; não use URLs rediss)
- Doctor reporta Chrome, origem do lighthouse, `cache_redis` quando cache Redis está configurado, e higiene residual de disco
- Check do doctor `residual_disk` e JSON de topo `residual`: `scanned_roots`, `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legado), `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`
- Localize sugestões humanas: `--lang pt-BR` ou `config set lang pt-BR` (só flags + XDG)
- Verbosity: `--verbose` (info), `--debug` (máximo), `-q`/`--quiet` ou `config set log_level debug`
- Cor: `config set color true|false` (valores truthy: `true`, `1`, `yes`)
- Path do Chrome: `config set chrome_path /path/to/chrome` quando a descoberta por PATH não bastar
- Diálogo soft: `dialog accept --if-present` / `dialog dismiss --if-present` quando o diálogo pode estar ausente
- Settle de diálogo (GAP-054): accept/dismiss real devolve booleano `dialog_settled`; ajuste o orçamento com `config set dialog_settle_ms <ms>` (só XDG)
- Diálogos multi-aba isolam por `session_id` CDP (forwarders de página carimbam `Page::session_id`; browser-level `None` cai no active tab)
- Beforeunload: `goto` / `reload` com `--handle-before-unload accept|dismiss`
- Página isolada: `page new --isolated-context` (contexto isolado)


## Higiene Residual (disco + processo)
- O ciclo de vida é sempre BORN → EXECUTE → FINALIZE → DIE em um processo
- BORN executa GC cross-run de Singleton stale (`scavenge_stale_singleton_orphans`, age floor 60s)
- FINALIZE faz dual scavenge: orphans Chromium tmp da janela de invocação + GC Singleton stale
- Residual-zero significa: sem processo Chrome CLI vivo, sem markers `browser-automation-cli-chrome-*`, sem lixo Singleton-only de Chromium tmp owned
- Prefixos temp de Chrome Flatpak do host **nunca** são apagados pelo GC do produto
- Inspecione com doctor (relatório residual path-light; sem launch de Chrome para o relatório):

```bash
# O binário reduz o payload; nenhum processador JSON no prompt.
browser-automation-cli --json --fields residual doctor --offline --quick

# Só o veredito residual, de um envelope de 26 KB para uma linha
browser-automation-cli --json --fields checks --filter-rows 'id=residual_disk' \
  doctor --offline --quick
```

- Campos JSON de topo `residual`: `scanned_roots`, `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `scavenge_safe_candidates`, `live_cli_marker_processes` (legado), `sibling_live_processes`, `orphan_marker_dirs`, `foreign_root_orphans`, `ghost_marker_processes`, `process_table_unavailable`
- Check id `residual_disk`: `fail` em `orphan_marker_dirs` ou `ghost_marker_processes`; `warn` quando restam dirs marker ou orphans Singleton; senão `pass`. Uma invocação irmã viva é saudável e nunca reprova.
- Mantenedores podem rodar gates locais: `bash scripts/residual-check.sh` e `bash scripts/residual-stress.sh` (só scripts locais do mantenedor)


## Redução de Payload para Agentes (oito flags globais)
- Oito flags globais encolhem o envelope JSON antes de ele chegar ao seu prompt
- Os nomes são `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes`
- Elas se combinam livremente e rodam dentro do binário, sem processador JSON
- Medido aqui: `doctor --offline --quick` emite 26276 bytes sem redução
- A mesma chamada com `--fields residual.ghost_marker_processes` emite 79 bytes
- O envelope ganha um objeto `agent_ops` só quando alguma delas roda
- Rodar a flag é necessário mas não suficiente: `agent_ops` é omitido quando não há o que reportar
- Medido: `--fields commands commands` resolve limpo e não carrega `agent_ops`
- Medido: acrescentar `--limit-rows 3` à mesma chamada produz `total`, `matched` e `truncated`

### --fields
- Projeta o payload em caminhos pontilhados, passados como lista CSV
- O aninhamento que cada caminho implica é reconstruído, preservando a forma documentada
- Também desambigua comandos cujo data guarda mais de uma lista

```bash
# de 26276 bytes para 79 bytes
browser-automation-cli --json --fields residual.ghost_marker_processes \
  doctor --offline --quick
```

### --filter-rows
- Mantém só linhas que casam `key=value`, `key!=value` ou `key~substring`
- A flag é repetível e cada expressão entra com AND
- Campo ausente nunca casa, inclusive sob `!=`

```bash
# só a linha do veredito residual
browser-automation-cli --json --fields checks --filter-rows 'id=residual_disk' \
  doctor --offline --quick
```

### --limit-rows
- Emite no máximo N linhas da lista selecionada
- O corte roda depois de filtro, dedupe e ordenação

```bash
browser-automation-cli --json --fields checks --limit-rows 3 doctor --offline --quick
```

### --sort-rows
- Ordena linhas por um caminho pontilhado antes de qualquer corte
- Valores numéricos comparam numericamente, não como texto

```bash
browser-automation-cli --json --fields checks --sort-rows id --limit-rows 2 \
  doctor --offline --quick
```

### --dedupe-by
- Descarta linhas cujo valor no caminho pontilhado repete, mantendo a primeira
- Transforma uma lista longa em uma linha por valor distinto

```bash
browser-automation-cli --json --fields checks --dedupe-by status doctor --offline --quick
```

### --count-only
- Emite só `{"count": N}` no lugar das próprias linhas
- Use quando a resposta é uma quantidade, não o payload

```bash
browser-automation-cli --json --fields checks --count-only doctor --offline --quick
```
- O `--fields` dessa linha não é enfeite: ele nomeia qual lista contar
- Medido: `--count-only commands` sozinho sai com exit `2` e `data holds more than one list`
- O erro nomeia as listas concorrentes, então leia e estreite com `--fields`

### --truncate-content
- Corta toda string do payload em N caracteres
- O envelope marca `truncated`, então o corte nunca é silencioso

```bash
browser-automation-cli --json --fields checks --filter-rows 'id=browsers_dir' \
  --truncate-content 12 doctor --offline --quick
```

### --max-output-bytes
- Define um teto duro sobre os bytes emitidos
- Linhas caem a partir do fim e `omitted_rows` registra a perda
- Teto impossível devolve exit 2 com envelope de erro, nunca sucesso vazio silencioso

```bash
# exit 0, linhas cortadas a partir do fim
browser-automation-cli --json --fields checks --max-output-bytes 512 doctor --offline --quick

# exit 2: o payload não cabe
browser-automation-cli --json --fields checks --max-output-bytes 8 doctor --offline --quick
```

### Lendo agent_ops
- `agent_ops.total` conta linhas antes da redução e `matched` conta as mantidas
- `agent_ops.truncated` marca payload cortado por qualquer teto
- `agent_ops.omitted_rows` conta linhas descartadas por `--max-output-bytes`
- Caminho pedido que nenhuma linha carrega aparece em `agent_ops.unresolved_paths`
- Caminho não resolvido nunca reprova a chamada, então sempre leia esse array

```bash
browser-automation-cli --json --fields residual.ghost_marker_processes,residual.campo_inexistente \
  doctor --offline --quick
```

### Flags que não são globais
- `--select`, `--filter`, `--limit` e `--sort` não são flags globais
- Passá-las no escopo global devolve exit 2 com envelope de usage
- Elas existem por comando em scrape, crawl, map, search e batch-scrape
- Cada comando dá a elas um sentido próprio, como um seletor CSS
- Uma flag local e uma flag global ainda combinam numa só chamada

```bash
# exit 2: --select não é flag global
browser-automation-cli --json --select checks doctor --offline --quick

# exit 0: --limit é do map, --count-only é global
browser-automation-cli --json --count-only map https://example.com --limit 5
```


## Configuração (XDG)
- Prefira flags para chamadas pontuais de agente
- Prefira config XDG via comando `config` para defaults duráveis
- Settings de produto são só flags e CLI XDG: `config init`, `config path`, `config show`, `config set`, `config get`, `config list-keys` — **nunca** variáveis de ambiente de produto
- Resolva paths vivos de config/data/state com `config path --json`
- Logging de produto é controlado por `--verbose` / `--debug` / `-q` e XDG `log_level`
- Idioma das sugestões humanas: só `--lang` ou XDG `lang` (sem catálogos de env de produto)
- Leia a referência completa de chaves XDG em `docs/CONFIGURATION.pt-BR.md`, que documenta as 176 chaves
- Confirme o conjunto vivo com `config list-keys --json` antes de gravar chave desconhecida
- Nunca fixe uma contagem de chaves, porque o conjunto cresce a cada release
- Chaves comuns incluem: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`, `dialog_settle_ms`
- Valores truthy de color: `true`, `1`, `yes`
- Valores falsy ou outros resolvem para desligado salvo set truthy

```bash
browser-automation-cli --json config init
browser-automation-cli --json config path
browser-automation-cli --json config show
browser-automation-cli --json config list-keys
browser-automation-cli --json config set lang en
browser-automation-cli --json config set timeout 60
browser-automation-cli --json config set artifacts_dir /tmp/browser-automation-cli-artifacts
browser-automation-cli --json config set ignore_robots false
browser-automation-cli --json config set namespace demo
browser-automation-cli --json config set color true
browser-automation-cli --json config set log_level info
browser-automation-cli --json config set chrome_path /usr/bin/chromium
browser-automation-cli --json config set lighthouse_path ./scripts/mock-lighthouse.sh
browser-automation-cli --json config set openrouter_api_key YOUR_KEY
browser-automation-cli --json config set llm_base_url https://openrouter.ai/api/v1
browser-automation-cli --json config set llm_model openai/gpt-4o-mini
browser-automation-cli --json config set log_to_file false
browser-automation-cli --json config set cache_backend sqlite
browser-automation-cli --json config set cache_redis_url redis://127.0.0.1:6379
browser-automation-cli --json config set dialog_settle_ms 2000
browser-automation-cli --json config get lang
browser-automation-cli --json config get dialog_settle_ms
```
- Use apenas `redis://` para cache Redis; `rediss://` é rejeitado fail-closed
- Descubra chaves e defaults com `config list-keys --json` antes de gravar chaves desconhecidas
- Mantenha a política dual-flag de robots explícita ao contornar: `--ignore-robots` mais `--i-accept-robots-risk`
- O `ignore_robots` da config sozinho não substitui a exigência dual-flag na linha de comando


## Scrape, Crawl, Map, Search, Parse, PDF, QR, Paths
```bash
# Single page as markdown over HTTP (no Chrome)
browser-automation-cli --json scrape https://example.com --format markdown --engine http --only-main-content

# Browser engine formats apply to captured outerHTML (markdown, links, …)
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --timeout 60 --json scrape https://example.com --format links --engine browser

# Multi-format in one invocation (GAP-009)
browser-automation-cli --json scrape https://example.com --format markdown,html,links --engine http

# Optional one-shot operator webhook POST of scrape result data (not product telemetry)
browser-automation-cli --json scrape https://example.com --format markdown --engine http \
  --webhook-url https://127.0.0.1:9000/hook

# Many URLs: default HTTP engine; optional browser engine per URL (GAP-010)
printf '%s\n' 'https://example.com' 'https://example.org' > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format markdown --engine browser --concurrency 1

# Crawl / map / search / parse local files
browser-automation-cli --json crawl https://example.com --same-host --limit 20 --max-depth 2 --format text
browser-automation-cli --timeout 120 --json crawl https://example.com --same-host --limit 5 --engine browser
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
browser-automation-cli --json search "example domain" --limit 10
browser-automation-cli --json parse tests/fixtures/hello.pdf
browser-automation-cli --json parse tests/fixtures/hello.docx --redact-pii
# xlsx/ods spreadsheets are also supported:
# browser-automation-cli --json parse /tmp/sheet.xlsx
# browser-automation-cli --json parse /tmp/sheet.ods --redact-pii

# PDF print, monitor baseline, QR, path discovery
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/page.pdf
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/mon.base --write-baseline
browser-automation-cli --json qr encode --text 'hello' --format png --path /tmp/qr.png
browser-automation-cli --json qr decode --path /tmp/qr.png
browser-automation-cli --json find-paths 'Cargo.*' .
browser-automation-cli --json find-paths --glob '**/*.rs' .
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data
browser-automation-cli --json sg-scan . --limit 100
browser-automation-cli --json sg-rewrite .
# dry-run por padrão; grave só com --apply
# browser-automation-cli --json sg-rewrite . --apply
```
- Defaults de `scrape`: `--format text`, `--engine browser`
- A engine browser respeita `--format` (não fica só em text silencioso)
- Multi-formato devolve campos por format no envelope quando mais de um format é pedido
- `batch-scrape` default HTTP; passe `--engine browser` para CDP por URL
- `crawl` default HTTP BFS; passe `--engine browser` quando renderização JS for necessária
- `crawl` permanece no host da semente quando você passa `--same-host`
- `parse` extrai texto de paths locais `html`, `md`, `txt`, `pdf`, `docx`, `xlsx` e `ods`
- `--redact-pii` redige padrões comuns de PII na saída do parse
- `--webhook-url` em `scrape` faz POST one-shot dos dados do resultado para URL do operador (não telemetria de produto)
- Honre robots por default; bypass dual-flag quando pular a política de propósito


## Extract LLM (chaves XDG)
```bash
browser-automation-cli --json config set openrouter_api_key YOUR_KEY
browser-automation-cli --json config set llm_base_url https://openrouter.ai/api/v1
browser-automation-cli --json config set llm_model openai/gpt-4o-mini
browser-automation-cli --json extract https://example.com --llm --question 'What is the title?'
```
- Chaves ficam só sob XDG via `config set`
- Sem `openrouter_api_key`, `extract --llm` falha fechado com envelope de usage
- `--schema-json` opcional aponta para um arquivo JSON Schema local para respostas estruturadas


## i18n e Meta de Descoberta
```bash
browser-automation-cli --lang pt-BR --json click-at --x 1 --y 1
# usage error shows localized suggestion when lang is pt-BR (needs --experimental-vision for success)
browser-automation-cli --json config set lang pt-BR
browser-automation-cli --json locale
browser-automation-cli --json man
browser-automation-cli --json commands
browser-automation-cli --json schema locale
browser-automation-cli --json schema man
```
- Mensagens humanas e sugestões honram só `--lang` e XDG `lang`
- `locale` reporta diagnósticos do locale de UI resolvido (chaves de máquina permanecem em inglês)
- `man` emite página man em roff (sem Chrome)
- Envelopes de máquina mantêm campos estáveis em inglês: `kind` / `exit_code`


## MITM e Workflow
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list
browser-automation-cli --json mitm har --out /tmp/capture.har
browser-automation-cli --json mitm capture-url https://example.com --seconds 30 --har /tmp/cap.har
browser-automation-cli --json mitm redact --secrets
browser-automation-cli --json mitm graphql
browser-automation-cli --json mitm ws

cat > /tmp/wf.json <<'JSON'
{
  "name": "demo",
  "steps": [
    {"id": "a", "cmd": "echo", "args": {"message": "hello"}},
    {"id": "b", "cmd": "scrape", "args": {"url": "https://example.com", "engine": "http"}, "depends_on": ["a"]}
  ]
}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
```
- MITM faz bind só em loopback (`127.0.0.1`) com porta efêmera
- CA do MITM fica sob XDG data; capturas sob XDG state
- `mitm har` exige `--out <path>`
- `mitm capture-url` one-shot: proxy + Chrome + navega + captura
- Journals de workflow ficam sob XDG state (SQLite)
- Resume pula passos já marcados `ok` no journal
- Passos offline de workflow são só data-plane
- Trabalho browser multi-passo com refs `@eN` compartilhadas permanece em `run --script`


## Erros Comuns
### Chrome ausente
- Sintoma: exit `69`, kind `unavailable`, mensagem sobre chrome não encontrado
- Causa: Chrome ou Chromium não instalado ou fora do PATH / `chrome_path`
- Correção: instale Chromium ou Google Chrome, defina `config set chrome_path`, reexecute `doctor --offline --quick --json`

### Timeout
- Sintoma: exit `124`, kind `timeout`
- Causa: navegação ou passo excedeu `--timeout` / orçamento de wait
- Correção: eleve `--timeout`, use `wait --text` / `--selector` direcionados, ou prefira `--engine http` quando CDP for desnecessário

### Dual-flag de robots incompleto
- Sintoma: exit `2`, mensagem `--ignore-robots requires --i-accept-robots-risk`
- Causa: só uma flag de bypass de robots foi passada
- Correção: passe `--ignore-robots` e `--i-accept-robots-risk` juntos quando for intencional

### Broken pipe (exit 141)
- Sintoma: exit `141`, kind `broken-pipe` quando o consumidor fecha o stdout cedo
- Causa: pipe para um reader fechado (por exemplo um head que sai no meio do stream)
- Correção: leia o stdout completo antes de fechar, ou evite teardown precoce do pipe; trate `141` como semântica esperada de pipe

### Chave de config desconhecida
- Sintoma: exit `2`, mensagem `unknown config key: ...`
- Causa: `config set` recebeu chave fora do conjunto vivo suportado
- Correção: rode `config list-keys --json` e use só as chaves listadas (inclui `dialog_settle_ms` e outras além das 16 históricas)

### Chaves LLM ausentes
- Sintoma: exit `2`, mensagem `LLM extract requires XDG openrouter_api_key`
- Causa: `extract --llm` sem chave XDG
- Correção: `config set openrouter_api_key YOUR_KEY` (e opcionais `llm_base_url` / `llm_model`)

### URL Redis rediss rejeitada
- Sintoma: exit non-zero / erro de config ou cache quando `cache_redis_url` usa `rediss://`
- Causa: cliente Redis é só TCP plain; `rediss://` é fail-closed (GAP-A007)
- Correção: use `config set cache_redis_url redis://127.0.0.1:6379` para Redis local

### Scrape HTTP rejeita file://
- Sintoma: exit `2` usage quando `scrape --engine http` recebe URL `file://`
- Causa: engine HTTP é só rede (GAP-A004)
- Correção: use `--engine browser` para páginas file, ou `parse` para arquivos locais

### View vazio em about:blank
- Sintoma: exit non-zero / usage quando `view` roda sem navegação
- Causa: about:blank vazio recusa sucesso silencioso (GAP-012)
- Correção: navegue com `goto` primeiro, ou passe `--allow-empty` só quando for intencional

### Schema ou nome de comando errado
- Sintoma: exit `2`, mensagem `unknown command for schema: ...` ou clap `unrecognized subcommand`
- Causa: typo ou subcomando / nome de schema inventado
- Correção: rode `commands --json`, depois `schema <name> --json` com um nome listado
- Nota: `select-option` e `pick` são inventário run/schema only (não clap standalone)
- Com `--json` no argv, erros de usage clap emitem envelope JSON (GAP-002)

### Path de grab confundido com posicional
- Sintoma: erro de usage do clap em torno de argumentos inesperados
- Causa: destino do screenshot foi passado como posicional
- Correção: use `grab --path /tmp/page.png` (e opcional `--full-page`)

### Erros de usage do clap sob --json (GAP-002)
- Sintoma: falhas de usage ainda precisam de parse por máquina
- Causa: argv inválido com `--json` já na linha de comando
- Correção: leia o envelope JSON de erro no stdout (`ok: false`); não raspe só a prosa do clap

### print-pdf em branco (GAP-013)
- Sintoma: exit não zero ao imprimir sem conteúdo de página
- Causa: `print-pdf` recusa about:blank sem conteúdo navegado ou `url`
- Correção: passe `--url` / campo `url` do passo, ou faça `goto` antes no mesmo processo `run`

### Wait navigation é booleano
- Sintoma: passo wait ignorado ou recusado ao usar string como `"load"`
- Causa: o campo run `navigation` é booleano `true`, não nome de lifecycle em string
- Correção: use `{"cmd":"wait","navigation":true}`


## Padrões v0.1.7 (dialog, wait, scrape, grab, submit, storage)
```bash
# Dialog accept e próximo passo de página — leia dialog_settled; sem wait artificial quando true
cat > /tmp/dialog-settled.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"dialog","action":"accept"},
  {"cmd":"view"}
]
JSON
# Após resposta real, parseie steps[].data.dialog_settled (true no happy path)
# browser-automation-cli --timeout 60 --json run --script /tmp/dialog-settled.run.json | jaq '.data.steps[] | select(.cmd=="dialog") | .data.dialog_settled'

# Orçamento XDG de settle de diálogo (nunca env de produto)
browser-automation-cli --json config set dialog_settle_ms 2000

# Multi-aba conceitual: abra duas páginas; diálogo em aba não ativa isola por session_id
cat > /tmp/dialog-multitab.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"page","action":"new","url":"https://example.org"},
  {"cmd":"page","action":"select","index":0},
  {"cmd":"dialog","action":"accept","if_present":true},
  {"cmd":"page","action":"select","index":1},
  {"cmd":"view"}
]
JSON
# browser-automation-cli --timeout 90 --json run --script /tmp/dialog-multitab.run.json

# wait com prazo público wait_timeout_ms (GAP-053)
cat > /tmp/wait-timeout.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","selector":"h1","wait_timeout_ms":2000}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/wait-timeout.run.json

# passo scrape com format text em run — sem monstro HTML (GAP-057)
cat > /tmp/scrape-text.run.json <<'JSON'
[
  {"cmd":"scrape","url":"https://example.com","format":"text","engine":"http"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/scrape-text.run.json

# grab webp (não avif — encode AVIF removido)
cat > /tmp/grab-webp.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"grab","path":"/tmp/page.webp","format":"webp"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/grab-webp.run.json

# submit de formulário (target = form ou campo dentro dele)
# browser-automation-cli --timeout 60 --json submit "form#login" --timeout-ms 10000
# run: {"cmd":"submit","target":"form#login","timeout_ms":10000}

# storage export / import (--path explícito; nunca path XDG implícito)
# browser-automation-cli --timeout 60 --json storage export --path /tmp/auth-state.json --url https://example.com
# browser-automation-cli --timeout 60 --json storage import --path /tmp/auth-state.json --url https://example.com
```
- `dialog_settled: true` significa que a página está desbloqueada para o próximo passo de observação — não invente wait
- `select-option` / `pick` em `<select>` nativo despacham `input` e depois `change` e reportam `via: native_select` (GAP-055)
- A lei residual-zero de disco da v0.1.5 permanece corrente (GC Singleton em BORN + FINALIZE)


## Integração com Scripts de Shell
- Peça sempre stdout legível por máquina com `--json`
- Inspecione `$?` (ou `$LASTEXITCODE`) antes de confiar no payload
- Pipeie stdout em `jaq` / `jq` para extração de campos
- Mantenha diagnósticos no stderr com `--quiet` quando só quiser envelopes
- Em erros de `run`, inspecione `data.steps` parcial quando presente
- Use `--json-steps` quando linhas progressivas de passo forem mais fáceis de streamar que um único envelope final

```bash
browser-automation-cli --timeout 60 --json goto https://example.com \
  | jaq -e '.ok == true'

browser-automation-cli --json scrape https://example.com --format markdown --engine http \
  | jaq -r '.data // .'

printf '%s\n' 'https://example.com' > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2 \
  | jaq .
```
- Exit `141` de broken pipe significa que o reader fechou cedo, não necessariamente bug da CLI
- Prefira caminhos HTTP de scrape / batch / crawl em pipelines de shell puro que não precisam de CDP


## Integração com Agentes de IA
- Spawne `browser-automation-cli` como subprocesso one-shot por fronteira de tarefa
- Passe `--json` em toda chamada programática
- Parseie só envelopes do stdout; trate stderr como diagnóstico
- Ramifique no campo `ok` do envelope e no exit code do processo
- Descubra inventário com `commands --json` (**69** nomes de agente)
- Descubra argv com `schema <name> --json` ou `schema --cmd <name> --json`
- Após trabalho browser, confirme higiene residual com `doctor --json` → `residual` / check `residual_disk`
- Colapse trabalho browser multi-passo em um processo `run --script` quando refs importam (opcional `--json-steps`)
- Prefira flags para controle pontual; use `config` para defaults XDG duráveis
- Não invente daemon entre turns do agente
- Configure settings de produto só com flags e `config set` / `config get` / `config path`
- Logging de produto usa `--verbose` / `--debug` / `-q` ou `config set log_level`
- Cor usa `config set color`; path do Chrome usa `config set chrome_path`
- Editores e runners compatíveis incluem Claude Code, Codex, Cursor, Continue e Cline via shell ou subprocesso
- Contrato completo de agente: [docs/AGENTS.pt-BR.md](AGENTS.pt-BR.md) e [INTEGRATIONS.pt-BR.md](../INTEGRATIONS.pt-BR.md)


## Integração com Crates Rust
- Chame o binário com `std::process::Command`
- Capture stdout, cheque status, desserialise com `serde_json`
- Mantenha o nome do binário exato: `browser-automation-cli`

```rust
use serde_json::Value;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("browser-automation-cli")
        .args([
            "--json",
            "scrape",
            "https://example.com",
            "--format",
            "text",
            "--engine",
            "http",
        ])
        .output()?;

    if !output.status.success() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(output.status.code().unwrap_or(1));
    }

    let envelope: Value = serde_json::from_slice(&output.stdout)?;
    if envelope.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        eprintln!("envelope not ok: {envelope}");
        std::process::exit(1);
    }

    println!("{envelope}");
    Ok(())
}
```
- Prefira `scrape` HTTP em checks estilo unit que não devem lançar Chrome
- Use `run --script` quando o crate orquestra fluxos CDP multi-passo
- Veja notas orientadas a crates em [docs/AGENTS.pt-BR.md](AGENTS.pt-BR.md) e [INTEGRATIONS.pt-BR.md](../INTEGRATIONS.pt-BR.md)


## Inventário Completo de Comandos (69)
- Fonte viva: `browser-automation-cli commands --json` (**69** nomes voltados a agentes)
- Superfície clap de produto é **67** nomes (exclui `select-option` / `pick` de inventário de agente; esses dois usam-se via run/exec/schema)
- O e2e DevTools tool-ref cobre **53** tools (`scripts/e2e_all_52_tools.sh` é nome legado; a suite executa 53; lighthouse mock = **SKIP**, não PASS)
- Lista completa de comandos de agente (todos os **69** nomes):
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

## Próximos Passos
- Receitas e fluxos mais longos: [docs/COOKBOOK.pt-BR.md](COOKBOOK.pt-BR.md)
- Contrato de agente e regras de lifecycle: [docs/AGENTS.pt-BR.md](AGENTS.pt-BR.md)
- Contratos JSON: [docs/schemas/README.md](schemas/README.md)
- Catálogo de plataforma e agentes: [INTEGRATIONS.pt-BR.md](../INTEGRATIONS.pt-BR.md)
- Mudanças de versão: [docs/MIGRATION.pt-BR.md](MIGRATION.pt-BR.md)
- Espelho em inglês: [docs/HOW_TO_USE.md](HOW_TO_USE.md)
