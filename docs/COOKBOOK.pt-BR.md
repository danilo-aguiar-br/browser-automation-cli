[English](COOKBOOK.md) | [Português Brasileiro](COOKBOOK.pt-BR.md)

# Cookbook — browser-automation-cli

> Receitas práticas com comandos prontos para copiar em trabalho browser one-shot. Ciclo de vida: BORN EXECUTE FINALIZE DIE.


## Nota de Latência
- O launch do Chrome domina o cold start em comandos com engine browser
- Prefira um script `run` a muitos launches separados quando os passos compartilham estado
- Scrape HTTP, crawl, map, search, parse, qr, find-paths, sheet-write, sg-scan e sg-rewrite evitam Chrome quando só precisa de conteúdo ou IO local
- Cada processo é BORN, EXECUTE, FINALIZE, DIE sem browser compartilhado entre invocações


## Referência de Valores Default
- Timeout global default é `0` (sem wall budget de processo salvo flag ou config XDG)
- Step timeout default é `0` (herda o timeout global)
- Headless é default salvo `--headed`
- JSON fica off salvo `--json`
- Settings de produto vêm só de flags e `config` (CLI XDG)
- Logging: `--verbose` / `--debug` / `-q` ou XDG `log_level`
- Cor: `config set color`; path do Chrome: `config set chrome_path`
- Resolva paths com `config path --json`


## Como Inicializar Config XDG
```bash
browser-automation-cli --json config init
browser-automation-cli --json config path
browser-automation-cli --json config show
browser-automation-cli --json config set timeout 60
browser-automation-cli --json config set lang en
browser-automation-cli --json config set namespace demo
browser-automation-cli --json config set artifacts_dir /tmp/browser-automation-cli-artifacts
browser-automation-cli --json config set ignore_robots false
browser-automation-cli --json config set encryption_key "replace-me-with-a-secret"
browser-automation-cli --json config set color true
browser-automation-cli --json config set log_level info
browser-automation-cli --json config set chrome_path /usr/bin/chromium
browser-automation-cli --json config set lighthouse_path ./scripts/mock-lighthouse.sh
browser-automation-cli --json config get timeout
browser-automation-cli --json config get encryption_key
browser-automation-cli --json config get color
```
- `config init` cria dirs XDG e o `config.toml` default
- Descubra chaves e defaults com `config list-keys --json`
- Chaves suportadas (16): `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- Flags sempre sobrescrevem o arquivo de config naquela invocação
- Settings de produto usam só flags e `config path|init|show|set|get|list-keys`


## Como Configurar Chaves LLM no XDG
```bash
browser-automation-cli --json config set openrouter_api_key YOUR_KEY
browser-automation-cli --json config set llm_base_url https://openrouter.ai/api/v1
browser-automation-cli --json config set llm_model openai/gpt-4o-mini
browser-automation-cli --json config get openrouter_api_key
```
- Chaves ficam só no `config.toml` XDG
- `extract --llm` falha fechado quando `openrouter_api_key` está ausente


## Como Diagnosticar Saúde da Instalação
```bash
browser-automation-cli doctor --offline --quick --json
```
- Modo offline quick checa descoberta local do Chrome sem sondas de rede
- Use doctor completo sem `--quick` quando precisar de checks mais profundos


## Como Abrir uma Página e Fazer Snapshot
```bash
browser-automation-cli --timeout 60 --json goto https://example.com

cat > /tmp/goto-view.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/goto-view.browser-automation.jsonl
```
- `goto` standalone navega e encerra o processo
- Use `run` para o `view` ver a mesma página em um lifecycle
- Snapshot de acessibilidade emite refs `@eN` para passos posteriores de press e write


## Como Clicar e Preencher em Um Processo
```bash
cat > /tmp/form.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
{"cmd":"write","target":"input","value":"hello"}
{"cmd":"press","target":"button"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/form.browser-automation.jsonl
```
- Mantenha click e fill no mesmo processo para seletores e refs `@eN` permanecerem válidos
- Launches separados não compartilham refs de acessibilidade


## Como Usar fill-form
```bash
cat > /tmp/fill-form.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"fill-form","fields":[{"target":"input[name=q]","value":"hello"},{"target":"input[name=email]","value":"a@b.c"}]}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/fill-form.browser-automation.jsonl
```
- `fill-form` preenche vários campos no mesmo processo
- Prefira refs `@eN` do `view` quando o DOM for dinâmico


## Como Scrollar e Assertar em um Script Run
```bash
cat > /tmp/scroll-assert.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"scroll","dy":1500}
{"cmd":"assert","url_contains":"example.com"}
{"cmd":"assert","text_contains":"Example Domain"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/scroll-assert.browser-automation.jsonl
```
- `dy` / `dx` são aliases de `delta_y` / `delta_x`
- `url_contains` / `text_contains` são aliases de assert
- Em fail-fast, o envelope de erro pode incluir `data.steps` parcial


## Como Capturar Screenshot Full-page
```bash
cat > /tmp/grab.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"grab","path":"/tmp/page.png","full_page":true}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/grab.browser-automation.jsonl

# Same flags on the grab subcommand after a prior step in the same process:
# browser-automation-cli --timeout 60 --json grab --path /tmp/page.png --full-page
```
- Path é a flag `--path`, não argumento posicional
- `full_page` no NDJSON mapeia para `--full-page` na CLI


## Como Imprimir uma Página em PDF
```bash
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/page.pdf

# Dentro de multi-passo run (GAP-001 / GAP-017)
cat > /tmp/pdf.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"print-pdf","path":"/tmp/page-from-run.pdf"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/pdf.run.json
```
- Usa CDP `Page.printToPDF` em processo one-shot
- Passe `--url` para navegar antes do print, ou imprima a página atual dentro de um script `run` após `goto`
- PDF em about:blank vazio é recusado sem conteúdo navegado ou `url` no step/CLI (GAP-013); navegue com `goto` antes (não use `allow_empty` de `view` aqui)


## Como Monitorar Mudança de Página Contra Baseline
```bash
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/mon.base --write-baseline
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/mon.base
```
- Primeira chamada com `--write-baseline` grava o hash/texto baseline
- Chamadas posteriores comparam com o arquivo baseline sem gravar salvo nova solicitação


## Como Esperar Multi-texto (OR)
```bash
cat > /tmp/wait-or.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","text":["Example Domain","Example"],"ms":5000}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/wait-or.browser-automation.jsonl

# CLI form with repeatable --text (OR semantics):
# browser-automation-cli --timeout 60 --json wait --text "Example Domain" --text "Example" --ms 5000
```
- `--text` repetível resolve quando qualquer valor listado aparece
- Combine com `ms`, `selector`/`selectors`, `url`/`url_contains`/`navigation` ou `state` conforme necessário


## Como Esperar Multi-seletor ou URL (v0.1.4)
```bash
cat > /tmp/wait-multi.browser-automation.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","selector":"h1, body","ms":5000},
  {"cmd":"wait","url_contains":"example.com","ms":5000},
  {"cmd":"wait","url":"https://example.com/","ms":5000},
  {"cmd":"wait","navigation":true,"ms":5000}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/wait-multi.browser-automation.json

# CLI multi-selector CSS OR:
browser-automation-cli --timeout 60 --json wait --selector 'h1, body' --ms 5000
```
- Multi-seletor CSS OR: `#a, #b` ou arrays `selectors` no run
- Campos run: `url` (exato), `url_contains`, `navigation` (**boolean** `true` para ciclo de load — não string `"load"`)
- Ainda combina com multi-texto OR e `ms`


## Como Streamar Passos com --json-steps
```bash
cat > /tmp/steps.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","ms":200},
  {"cmd":"view"}
]
JSON
browser-automation-cli --timeout 60 --json --json-steps run --script /tmp/steps.array.json
```
- Global `--json-steps` streama uma linha NDJSON por passo: `step`, `cmd`, `ok`, `result`
- Envelope final `--json` ainda inclui `ok` e `steps[].data` completo
- Útil para feedback progressivo de agente sem re-spawnar Chrome


## Como Selecionar Opção HIG com select-option / pick (0.1.4)
```bash
cat > /tmp/pick.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"pick","target":"[role=combobox]","option":"Option label"},
  {"cmd":"select-option","target":"select#country","option":"BR"}
]
JSON
# browser-automation-cli --timeout 90 --json run --script /tmp/pick.run.json
browser-automation-cli --json schema select-option
browser-automation-cli --json schema pick
```
- `pick` / `select-option` são inventário multi-passo / schema only (não subcomandos clap standalone)
- Exigem `target` (trigger) e `option` (texto, seletor ou label de role)
- Descubra argv com `schema pick` ou `schema select-option`


## Como Assertar Console Limpo (0.1.4)
```bash
cat > /tmp/assert-console.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"assert","kind":"console_empty"},
  {"cmd":"assert","kind":"console_no_match","pattern":"TypeError"}
]
JSON
browser-automation-cli --capture-console --timeout 60 --json run --script /tmp/assert-console.run.json

# CLI forms (GAP-025):
# browser-automation-cli --capture-console --json assert console-empty
# browser-automation-cli --capture-console --json assert console-no-match --pattern TypeError
```
- Exige `--capture-console` no mesmo processo
- Kinds no run: `console_empty` / `console_no_match`; CLI: `console-empty` / `console-no-match`


## Como Usar Schema Posicional
```bash
browser-automation-cli --json schema run
browser-automation-cli --json schema wait
browser-automation-cli --json schema --cmd assert
```
- `schema <cmd>` posicional e `schema --cmd <cmd>` são ambos válidos (GAP-022)
- Prefira posicional para UX de agente


## Como View com --allow-empty
```bash
browser-automation-cli --json view --allow-empty

cat > /tmp/view-empty.run.json <<'JSON'
[
  {"cmd":"view","allow_empty":true}
]
JSON
browser-automation-cli --timeout 30 --json run --script /tmp/view-empty.run.json
```
- about:blank vazio recusa sucesso silencioso salvo `--allow-empty` / `allow_empty:true` (GAP-012)
- Prefira navegar com `goto` antes de `view` em fluxos normais


## Como Handle Beforeunload
```bash
browser-automation-cli --timeout 60 --json goto https://example.com --handle-before-unload accept
browser-automation-cli --timeout 60 --json reload --handle-before-unload dismiss

cat > /tmp/beforeunload.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com","handle_before_unload":"accept"},
  {"cmd":"reload","handle_before_unload":"dismiss"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/beforeunload.run.json
```
- `BeforeUnloadAction` accept|dismiss em `goto` / `reload` (GAP-003)
- Flag: `--handle-before-unload accept|dismiss`


## Como page new --isolated-context
```bash
browser-automation-cli --json page new --isolated-context

cat > /tmp/isolated.run.json <<'JSON'
[
  {"cmd":"page","action":"new","url":"about:blank","isolated_context":true},
  {"cmd":"goto","url":"https://example.com"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/isolated.run.json
```
- `page new --isolated-context` cria contexto isolado (GAP-004)
- No run, use `isolated_context:true` (ou nome quando suportado)


## Como Listar Requests de Network
```bash
cat > /tmp/nav.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"net","action":"list","resource_types":"Document,XHR"}
JSONL
browser-automation-cli --capture-network --timeout 60 --json run --script /tmp/nav.jsonl
```
- Crie o arquivo de script na receita antes do `run`
- Capture deve estar habilitado no mesmo processo que navega
- `net list` após processo separado não vê captura anterior


## Como Avaliar JavaScript
```bash
cat > /tmp/eval.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"eval","expression":"document.title"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/eval.browser-automation.jsonl

# Standalone eval runs against about:blank unless you already navigated in the same process
# browser-automation-cli --json eval 'document.title'
```
- Prefira `run` quando a expressão depende do conteúdo da página
- A expressão pode ser valor simples ou declaração de função `() => ...`


## Como Emular Viewport Mobile e Rede
```bash
cat > /tmp/emulate.browser-automation.jsonl <<'JSONL'
{"cmd":"emulate","user_agent":"Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)","viewport":"390x844x3,mobile,touch","network_conditions":"Slow 3G"}
{"cmd":"goto","url":"https://example.com"}
{"cmd":"resize","width":390,"height":844}
{"cmd":"view"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/emulate.browser-automation.jsonl

# Standalone compose (no --device preset flag):
# browser-automation-cli --json emulate \
#   --user-agent "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)" \
#   --viewport "390x844x3,mobile,touch" \
#   --network-conditions "Slow 3G"
```
- Não existe flag de preset `--device`
- Compose user agent, viewport e condições de rede você mesmo
- Presets de rede incluem Offline, No throttling, Slow 3G, Fast 3G, Slow 4G, Fast 4G


## Como Fazer Scrape com Markdown via HTTP
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http
```
- Formatos: `text`, `markdown`, `html`, `links`, `metadata`, `summary`, `product`, `branding`, `raw-html`, `screenshot`
- Engine `http` usa reqwest e pula o Chrome


## Como Scrape Multi-formato (GAP-009)
```bash
browser-automation-cli --json scrape https://example.com --format markdown,html,links --engine http
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --format links --engine browser
```
- CSV ou `--format` repetível devolve vários campos de format em uma invocação (GAP-009)
- Envelope inclui saída por format quando mais de um format é pedido


## Como Fazer Scrape com Engine Browser e Formatos
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --timeout 60 --json scrape https://example.com --format links --engine browser
```
- Engine `browser` usa CDP via Chrome
- A engine browser captura `outerHTML` e aplica `--format` (markdown/html/links/metadata/…)
- Use browser quando o conteúdo precisa de renderização JS


## Como Enviar Resultado de Scrape a um Webhook do Operador
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http \
  --webhook-url https://127.0.0.1:9000/hook
```
- `--webhook-url` é um POST one-shot do operador com os dados do resultado do scrape
- Não é telemetria de produto; o destino fica sob controle do operador


## Como Fazer Batch-scrape a Partir de Arquivo de URLs
```bash
cat > /tmp/urls.txt <<'URLS'
# one URL per line
https://example.com
https://example.org
URLS
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format markdown --engine browser --concurrency 1
```
- Default de `batch-scrape` é `--engine http`; em 0.1.4 aceita `--engine browser` (GAP-010)
- Crie o arquivo de URLs antes de invocar o comando


## Como Fazer Crawl com Same-host
```bash
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text --same-host
browser-automation-cli --timeout 120 --json crawl https://example.com --limit 5 --max-depth 1 --engine browser --same-host
```
- `--same-host` é flag booleana sem valor
- Não escreva `--same-host true`
- Default de crawl é HTTP BFS; `--engine browser` usa CDP por página (GAP-010)
- Com `--same-host` permanece no host da semente


## Como Mapear um Site
```bash
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
```
- Map descobre URLs a partir de uma semente sem extração completa de página
- Caminho HTTP; sem launch de Chrome


## Como Fazer Search
```bash
browser-automation-cli --json search "example domain" --limit 10
```
- Search local retorna links estilo SERP HTTP ou resultados de mapa de URLs
- Limit limita a contagem de resultados


## Como Parsear Arquivos Locais (HTML, PDF, DOCX, XLSX, ODS)
```bash
cat > /tmp/page.html <<'HTML'
<!doctype html>
<html><head><title>Demo</title></head>
<body><h1>Hello parse</h1><p>Local file text.</p></body></html>
HTML
browser-automation-cli --json parse /tmp/page.html
browser-automation-cli --json parse tests/fixtures/hello.pdf
browser-automation-cli --json parse tests/fixtures/hello.docx --redact-pii
# browser-automation-cli --json parse /tmp/sheet.xlsx
# browser-automation-cli --json parse /tmp/sheet.ods --redact-pii
```
- Parse extrai texto de html, md, txt, pdf, docx, xlsx ou ods local
- `--redact-pii` redige padrões comuns de PII no texto extraído
- Crie o HTML de exemplo antes do primeiro comando; use fixtures do repo para PDF/DOCX


## Como Extrair com LLM
```bash
browser-automation-cli --json config set openrouter_api_key YOUR_KEY
browser-automation-cli --json config set llm_base_url https://openrouter.ai/api/v1
browser-automation-cli --json config set llm_model openai/gpt-4o-mini
browser-automation-cli --json extract https://example.com --llm --question 'What is the title?'
```
- Sem a chave XDG, o comando falha fechado com envelope de usage
- `--schema-json` opcional para extração estruturada com schema local


## Como Codificar e Decodificar QR Codes
```bash
browser-automation-cli --json qr encode --text 'hello' --format png --path /tmp/qr.png
browser-automation-cli --json qr decode --path /tmp/qr.png
```
- Não exige Chrome
- Formatos de encode incluem `png`, `svg` e `terminal`


## Como Encontrar Paths no Disco
```bash
browser-automation-cli --json find-paths 'Cargo.*' .
browser-automation-cli --json find-paths --glob '**/*.rs' .
```
- Descoberta de paths estilo fd sob o nome do binário `browser-automation-cli`
- Use `--glob` para filtros estilo shell (GAP-A011)
- Sem launch de Chrome


## Como Localizar Sugestões (pt-BR)
```bash
browser-automation-cli --lang pt-BR --json click-at --x 1 --y 1
browser-automation-cli --json config set lang pt-BR
```
- Sugestões humanas localizam para `pt-BR` via `--lang` ou XDG `lang`
- Cliques por coordenada com sucesso ainda exigem `--experimental-vision`


## Como Capturar com MITM
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list --limit 100
browser-automation-cli --json mitm har --out /tmp/capture.har
browser-automation-cli --json mitm redact --secrets
browser-automation-cli --json mitm domains
browser-automation-cli --json mitm apis
browser-automation-cli --json mitm graphql
browser-automation-cli --json mitm ws
```
- Bind apenas em 127.0.0.1 com porta efêmera
- Material de CA fica sob XDG data (`mitm/ca`)
- `start` mantém o proxy one-shot vivo por `--seconds` e então sai
- Exporte HAR com `--out` **obrigatório**


## Como MITM capture-url One-shot
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm capture-url https://example.com --seconds 30 --har /tmp/cap.har
browser-automation-cli --json mitm list
browser-automation-cli --json mitm har --out /tmp/capture.har
```
- Compose one-shot: proxy local + Chrome + navega URL + captura (GAP-011)
- Allowlist opcional `--hosts` para intercept TLS
- Flags globais de rota-via-MITM também existem: `--mitm`, `--mitm-har`, `--mitm-redact-secrets`, …


## Como Rodar, Resumir e Ver Status de Workflow
```bash
cat > /tmp/wf.json <<'JSON'
{
  "name": "demo",
  "steps": [
    {"id": "ping", "cmd": "echo", "args": {"message": "start"}},
    {
      "id": "fetch",
      "cmd": "scrape",
      "args": {"url": "https://example.com", "engine": "http", "format": "text"},
      "depends_on": ["ping"]
    }
  ]
}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
```
- Resume pula passos já `ok` no journal SQLite
- Só passos offline; multi-passo browser com `@eN` permanece em `run --script`
- Comandos offline suportados incluem noop, echo, parse, scrape (http), batch-scrape


## Como Rodar Auditoria Lighthouse
```bash
# Requires a real lighthouse binary on PATH
browser-automation-cli --timeout 180 --json lighthouse https://example.com

# Mock binary for local smoke without a real lighthouse install
browser-automation-cli --timeout 60 --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh
```
- Ordem de resolve: flag `--lighthouse-path` → XDG `lighthouse_path` → PATH
- Envelope reporta `binary_source` como `real` ou `mock`
- Passe `--lighthouse-path` ou XDG `lighthouse_path` para binário externo ou script mock
- Lighthouse em si não está embutido na CLI


## Como Inspecionar Heap Snapshots
```bash
cat > /tmp/heap.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"heap","action":"take","path":"/tmp/snap.heapsnapshot"}
JSONL
browser-automation-cli --category-memory --timeout 120 --json run --script /tmp/heap.browser-automation.jsonl
browser-automation-cli --category-memory --json heap summary --path /tmp/snap.heapsnapshot
```
- Análise profunda de heap exige `--category-memory`
- Summary lê path de snapshot existente via `--path`


## Como Gerar Completions de Shell
```bash
browser-automation-cli completions bash
browser-automation-cli completions zsh
browser-automation-cli completions fish
```
- Caminho de completions é leve e não lança Chrome
- Redirecione stdout para o diretório de completions do shell conforme necessário



## Como Escrever Planilhas (sheet-write)
```bash
printf 'name,score\nalice,10\nbob,9\n' > /tmp/rows.csv
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data
```
- Escreve um XLSX simples a partir de CSV ou JSON array-of-objects
- Sem Chrome
- Use `--sheet` para nomear a planilha (padrão `Sheet1`)


## Como Fazer Lint Estrutural Com sg-scan
```bash
browser-automation-cli --json sg-scan . --limit 100
```
- Lint estrutural one-shot para padrões proibidos de produto
- Sem Chrome
- `--limit 0` significa findings ilimitados


## Como Dry-run e Aplicar sg-rewrite
```bash
browser-automation-cli --json sg-rewrite .
browser-automation-cli --json sg-rewrite . --apply
```
- Padrão é relatório dry-run
- Passe `--apply` para gravar correções known-safe
- Sem Chrome


## Como Encontrar Paths Com --glob
```bash
browser-automation-cli --json find-paths --glob '**/*.rs' .
browser-automation-cli --json find-paths 'Cargo.*' . --extension rs
```
- `--glob` é filtro glob estilo shell (GAP-A011)
- Pattern regex e `--glob` combinam com outros filtros
- Sem Chrome


## Como Rodar Script em Array JSON
```bash
cat > /tmp/demo.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"view"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/demo.array.json
```
- `run --script` aceita NDJSON **ou** um array JSON de objetos de passo
- Mesmo ciclo de vida: BORN EXECUTE FINALIZE DIE
- Erros fail-fast ainda podem incluir `data.steps` parcial
- Envelope final inclui `steps[].data` completo quando `--json` está set


## Como Ler binary_source do Lighthouse
```bash
browser-automation-cli --timeout 60 --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh \
  | jaq '.data.binary_source // .binary_source // .'
```
- Ordem de resolve: flag `--lighthouse-path` → XDG `lighthouse_path` → PATH
- Envelope reporta `binary_source` como `real` ou `mock`
- Mock é honesty para e2e/smoke, não auditoria de produção


## Como Configurar Cache Redis Com Honesty
```bash
browser-automation-cli --json config set cache_backend redis
browser-automation-cli --json config set cache_redis_url redis://127.0.0.1:6379
browser-automation-cli doctor --offline --quick --json
```
- Cache só via XDG com `config set` / `config get` / `config list-keys`
- Use apenas `redis://`; `rediss://` é fail-closed (cliente TCP plain)
- Doctor reporta `cache_redis` quando cache Redis está configurado


## Como Cobrir Demais Comandos de Interação e Página
```bash
# keys / type / hover / drag / upload (same process as navigation)
cat > /tmp/interact.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"keys","keys":"Tab"}
{"cmd":"type","text":"hello"}
{"cmd":"hover","target":"a"}
{"cmd":"text"}
{"cmd":"attr","selector":"a","name":"href"}
{"cmd":"page","action":"list"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/interact.browser-automation.jsonl

# dialog accept/dismiss subcommands (not --action); soft path when optional
browser-automation-cli --timeout 60 --json reload --ignore-cache
browser-automation-cli --json dialog accept --if-present
browser-automation-cli --json dialog dismiss --if-present
browser-automation-cli --json exec --help >/dev/null

# dialog inside run (NDJSON shape uses action + optional if_present)
cat > /tmp/dialog.run.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"dialog","action":"accept","if_present":true}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/dialog.run.json

# category-gated surfaces (explicit flags)
browser-automation-cli --category-extensions --json extension list
browser-automation-cli --category-third-party --json devtools3p list
browser-automation-cli --category-webmcp --json webmcp list
browser-automation-cli --experimental-screencast --json screencast --help >/dev/null
browser-automation-cli --category-memory --json heap --help >/dev/null
browser-automation-cli --json perf --help >/dev/null
browser-automation-cli --json resize --help >/dev/null
browser-automation-cli completions bash >/dev/null
```
- Cada nome de agente aparece em `commands --json` (61)
- `select-option` / `pick` aparecem no inventário e só em run/schema
- Prefira `schema <name>` antes de inventar argv em superfícies com gate


## Como Descobrir Schemas de Comando
```bash
browser-automation-cli commands --json
browser-automation-cli schema goto --json
browser-automation-cli schema --cmd scrape --json
browser-automation-cli schema print-pdf --json
browser-automation-cli schema monitor --json
browser-automation-cli schema qr --json
browser-automation-cli schema find-paths --json
browser-automation-cli schema sheet-write --json
browser-automation-cli schema sg-scan --json
browser-automation-cli schema sg-rewrite --json
browser-automation-cli schema run --json
browser-automation-cli schema pick --json
browser-automation-cli schema select-option --json
browser-automation-cli schema batch-scrape --json
browser-automation-cli schema config --json
browser-automation-cli schema mitm --json
browser-automation-cli schema workflow --json
```
- `commands` lista a superfície voltada a agentes (61 nomes)
- `schema <cmd>` ou `schema --cmd` imprime um fragmento JSON Schema de um comando
- Útil para registro de tools em frameworks de agentes


## Como Pipear JSON com jaq
```bash
browser-automation-cli doctor --offline --quick --json | jaq -e '.ok == true'
browser-automation-cli --json scrape https://example.com --format metadata --engine http \
  | jaq '.data // .'
browser-automation-cli commands --json | jaq '.data.commands // .commands // .'
```
- Prefira `--json` para stdout legível por máquina
- Filtros `jaq` mantêm a cola de agentes pequena e determinística


## Como Contornar robots.txt com Dual Flags
```bash
# Honor robots by default (no bypass flags)
browser-automation-cli --json scrape https://example.com --format text --engine http

# Bypass only when both flags are present together
browser-automation-cli --ignore-robots --i-accept-robots-risk --json \
  scrape https://example.com --format text --engine http
```
- Política default honra robots.txt
- `--ignore-robots` sozinho falha; `--i-accept-robots-risk` sozinho falha
- Ambas as flags são exigidas quando você aceita o risco de bypass


## Como Listar Cookies
```bash
cat > /tmp/cookie.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"cookie","action":"list"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/cookie.browser-automation.jsonl
```
- Helpers de cookie operam na página ativa no mesmo processo
- Filtro opcional de URL existe em `cookie list --url`


## Como Listar Mensagens de Console
```bash
cat > /tmp/console.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"eval","expression":"console.log('hello-cookbook')"}
{"cmd":"console","action":"list"}
JSONL
browser-automation-cli --capture-console --timeout 60 --json run --script /tmp/console.browser-automation.jsonl
```
- Habilite `--capture-console` no mesmo processo que produz as mensagens
- Filtre tipos com `--types log,warning,error,info,debug` na forma CLI


## Como Console Dump []
```bash
browser-automation-cli --capture-console --json console dump
# sempre array JSON válido; [] quando vazio (GAP-021)
```
- `console dump` sempre grava um array JSON válido (`[]` quando vazio)
- Capture precisa estar habilitada no mesmo processo para mensagens reais


## Como Fazer Assert de URL ou Texto
```bash
cat > /tmp/assert.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"assert","kind":"url","value":"example.com","contains":true}
{"cmd":"assert","kind":"text","value":"Example Domain"}
{"cmd":"assert","url_contains":"example.com"}
{"cmd":"assert","text_contains":"Example Domain"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/assert.browser-automation.jsonl
```
- Assert falha o processo quando a condição não é atendida
- Assert de URL suporta match exato ou semântica contains (`contains` ou `url_contains`)
- Assert de texto pode mirar seletor via `target` ou usar `text_contains`

## Inventário Completo de Comandos (61)
- Fonte viva: `browser-automation-cli commands --json` (61 nomes voltados a agentes)
- Help clap de topo lista 59 sem `select-option` e `pick` como subcomandos standalone
- O e2e DevTools tool-ref cobre **53** tools (`scripts/e2e_all_52_tools.sh` é nome legado; a suite executa 53)
- Lista completa de comandos de agente:
  - Meta: `doctor`, `commands`, `schema`, `version`, `completions`
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
