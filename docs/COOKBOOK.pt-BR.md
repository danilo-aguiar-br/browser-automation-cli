[English](COOKBOOK.md) | [Português Brasileiro](COOKBOOK.pt-BR.md)

# Cookbook — browser-automation-cli

> Receitas práticas com comandos prontos para trabalho one-shot de browser. Ciclo de vida: BORN EXECUTE FINALIZE DIE.


## Nota de Latência
- O launch do Chrome domina o cold start em comandos com engine browser
- Prefira um script `run` a vários launches separados quando os passos compartilham estado
- HTTP scrape, crawl, map, search e parse evitam Chrome quando só precisa de conteúdo
- Cada processo é BORN, EXECUTE, FINALIZE, DIE sem browser compartilhado entre invocações


## Referência de Defaults
- Timeout global default é `0` e significa sem orçamento wall-clock até ser setado por flag ou config XDG
- Step timeout default é `0` e herda o timeout global
- Headless é o default salvo `--headed`
- JSON fica off até `--json`
- Settings de produto vêm de flags e `config` (XDG), não de env vars de produto
- Não existem settings de produto `BROWSER_AUTOMATION_CLI_*`
- Env de SO apenas: `RUST_LOG`, `NO_COLOR`


## Como Inicializar Config XDG
```bash
browser-automation-cli --json config init
browser-automation-cli --json config path
browser-automation-cli --json config show
browser-automation-cli --json config set timeout 60
browser-automation-cli --json config set lang en
browser-automation-cli --json config set namespace demo
browser-automation-cli --json config set artifacts_dir /tmp/bac-artifacts
browser-automation-cli --json config set ignore_robots false
browser-automation-cli --json config set encryption_key "replace-me-with-a-secret"
browser-automation-cli --json config set color true
browser-automation-cli --json config get timeout
browser-automation-cli --json config get encryption_key
browser-automation-cli --json config get color
```
- `config init` cria dirs XDG e o `config.toml` default
- Chaves suportadas incluem `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`
- Flags sempre sobrescrevem o arquivo de config naquela invocação
- Settings de produto não usam env vars de produto


## Como Diagnosticar Saúde do Install
```bash
browser-automation-cli doctor --offline --quick --json
```
- Modo offline quick checa descoberta local do Chrome sem probes de rede
- Use doctor completo sem `--quick` para checagens de readiness mais profundas


## Como Abrir uma Página e Fazer Snapshot
```bash
browser-automation-cli --timeout 60 --json goto https://example.com

cat > /tmp/goto-view.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/goto-view.browser-automation.jsonl
```
- `goto` isolado navega e encerra o processo
- Use `run` para o `view` ver a mesma página em um ciclo de vida
- Snapshot de acessibilidade emite refs `@eN` para press e write posteriores


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
- Mantenha clique e preenchimento no mesmo processo para selectors e refs `@eN` válidos
- Launches separados não compartilham refs de acessibilidade


## Como Capturar Screenshot Full-page
```bash
cat > /tmp/grab.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"grab","path":"/tmp/page.png","full_page":true}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/grab.browser-automation.jsonl

# Mesmas flags no subcomando grab após um passo anterior no mesmo processo:
# browser-automation-cli --timeout 60 --json grab --path /tmp/page.png --full-page
```
- Path é a flag `--path`, não um argumento posicional
- `full_page` no NDJSON mapeia para `--full-page` na CLI


## Como Esperar Multi-texto (OR)
```bash
cat > /tmp/wait-or.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","text":["Example Domain","Example"],"ms":5000}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/wait-or.browser-automation.jsonl

# Forma CLI com --text repetível (semântica OR):
# browser-automation-cli --timeout 60 --json wait --text "Example Domain" --text "Example" --ms 5000
```
- `--text` repetível resolve quando qualquer valor listado aparece
- Combine com `ms` ou `selector` ou `state` da página conforme precisar


## Como Listar Requisições de Rede
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
- `net list` em processo separado não vê capture anterior


## Como Avaliar JavaScript
```bash
cat > /tmp/eval.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"eval","expression":"document.title"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/eval.browser-automation.jsonl

# Eval isolado roda em about:blank a menos que já tenha navegado no mesmo processo
# browser-automation-cli --json eval 'document.title'
```
- Prefira `run` quando a expressão depende do conteúdo da página
- Expressão pode ser valor simples ou declaração de função `() => ...`


## Como Emular Viewport Mobile e Rede
```bash
cat > /tmp/emulate.browser-automation.jsonl <<'JSONL'
{"cmd":"emulate","user_agent":"Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)","viewport":"390x844x3,mobile,touch","network_conditions":"Slow 3G"}
{"cmd":"goto","url":"https://example.com"}
{"cmd":"resize","width":390,"height":844}
{"cmd":"view"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/emulate.browser-automation.jsonl

# Compose isolado (sem flag preset --device):
# browser-automation-cli --json emulate \
#   --user-agent "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X)" \
#   --viewport "390x844x3,mobile,touch" \
#   --network-conditions "Slow 3G"
```
- Não existe flag preset `--device`
- Compose user agent, viewport e condições de rede você mesmo
- Presets de rede incluem Offline, No throttling, Slow 3G, Fast 3G, Slow 4G, Fast 4G


## Como Fazer Scrape com Markdown via HTTP
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http
```
- Formatos: `text`, `markdown`, `html`, `links`, `metadata`
- Engine `http` usa reqwest e não lança Chrome


## Como Fazer Scrape com Engine Browser
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com --format text --engine browser
```
- Engine `browser` usa CDP via Chrome
- Use browser quando o conteúdo exige renderização JS


## Como Fazer Batch-scrape a Partir de Arquivo de URLs
```bash
cat > /tmp/urls.txt <<'EOF'
# one URL per line
https://example.com
https://example.org
EOF
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
```
- Batch-scrape usa apenas engine HTTP
- Crie o arquivo de URLs antes de invocar o comando


## Como Fazer Crawl com Same-host
```bash
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text --same-host
```
- `--same-host` é flag booleana sem valor
- Não escreva `--same-host true`
- Crawl HTTP BFS permanece no host da seed quando a flag está setada


## Como Mapear um Site
```bash
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
```
- Map descobre URLs a partir de uma seed sem extração completa de página
- Path HTTP; sem launch de Chrome


## Como Buscar
```bash
browser-automation-cli --json search "example domain" --limit 10
```
- Search local retorna links estilo SERP HTTP ou resultados de URL map
- Limit limita a contagem de resultados


## Como Fazer Parse de HTML Local
```bash
cat > /tmp/page.html <<'HTML'
<!doctype html>
<html><head><title>Demo</title></head>
<body><h1>Hello parse</h1><p>Local file text.</p></body></html>
HTML
browser-automation-cli --json parse /tmp/page.html
```
- Parse extrai texto de html, md, txt ou pdf locais
- Crie o arquivo de exemplo antes do comando


## Como Fazer Captura MITM
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list --limit 100
browser-automation-cli --json mitm har --out /tmp/capture.har
```
- Faz bind apenas em 127.0.0.1 com porta efêmera
- Material de CA fica sob XDG data (`mitm/ca`)
- `start` mantém o proxy one-shot vivo por `--seconds` e depois sai
- Exporte HAR com `--out` obrigatório


## Como Rodar, Retomar e Ver Status de Workflow
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
- Resume pula steps já `ok` no journal SQLite
- Apenas steps offline; multi-step com refs `@eN` de browser permanece em `run --script`
- Comandos offline suportados incluem noop, echo, parse, scrape (http), batch-scrape


## Como Rodar Auditoria Lighthouse
```bash
# Requer binário lighthouse real no PATH
browser-automation-cli --timeout 180 --json lighthouse https://example.com

# Binário mock para smoke local sem instalar lighthouse real
browser-automation-cli --timeout 60 --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh
```
- Passe `--lighthouse-path` para binário externo ou script mock
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
- Summary lê um snapshot existente via `--path`


## Como Gerar Completions de Shell
```bash
browser-automation-cli completions bash
browser-automation-cli completions zsh
browser-automation-cli completions fish
```
- Path de completions é leve e não lança Chrome
- Redirecione stdout para o diretório de completion do seu shell conforme necessário


## Como Descobrir Schemas de Comandos
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd goto --json
browser-automation-cli schema --cmd scrape --json
browser-automation-cli schema --cmd batch-scrape --json
browser-automation-cli schema --cmd config --json
browser-automation-cli schema --cmd mitm --json
browser-automation-cli schema --cmd workflow --json
```
- `commands` lista a superfície voltada a agentes
- `schema --cmd` imprime um fragmento JSON Schema de um comando
- Útil para registro de tools em frameworks de agentes


## Como Encadear JSON com jaq
```bash
browser-automation-cli doctor --offline --quick --json | jaq -e '.ok == true'
browser-automation-cli --json scrape https://example.com --format metadata --engine http \
  | jaq '.data // .'
browser-automation-cli commands --json | jaq '.data.commands // .commands // .'
```
- Prefira `--json` para stdout legível por máquina
- Filtros `jaq` mantêm o glue de agentes pequeno e determinístico


## Como Contornar robots.txt com Dual Flags
```bash
# Honra robots por default (sem flags de bypass)
browser-automation-cli --json scrape https://example.com --format text --engine http

# Bypass só quando as duas flags estão presentes juntas
browser-automation-cli --ignore-robots --i-accept-robots-risk --json \
  scrape https://example.com --format text --engine http
```
- Política default honra robots.txt
- `--ignore-robots` sozinho falha; `--i-accept-robots-risk` sozinho falha
- Ambas as flags são obrigatórias quando você aceita o risco do bypass


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


## Como Fazer Assert de URL ou Texto
```bash
cat > /tmp/assert.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"assert","kind":"url","value":"example.com","contains":true}
{"cmd":"assert","kind":"text","value":"Example Domain"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/assert.browser-automation.jsonl
```
- Assert falha o processo quando a condição não é atendida
- Assert de URL suporta match exato ou semântica contains
- Assert de texto pode mirar um seletor via `target` ou `ref` no step
