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
- Faça snapshot da página com `view`
- Clique com `press` usando seletor CSS ou ref `@eN`
- Preencha inputs com `write` e formulários multi-campo com `fill-form`
- Espere com `wait --ms`, `--text` repetível (OR), `--selector` e `--state` opcional
- Capture screenshot com `grab --path /tmp/page.png` (flag, não caminho posicional)
- Imprima a página em PDF com `print-pdf --url <url> --path /tmp/page.pdf`
- Extraia conteúdo com `scrape` quando precisar de text, markdown, html, links, metadata ou formatos relacionados
- Parseie arquivos locais com `parse` (html/md/txt/pdf/docx/xlsx/ods; opcional `--redact-pii`)
- Codifique ou decodifique QR com `qr encode|decode` (sem Chrome)
- Descubra paths no filesystem com `find-paths` (sem Chrome)
- Verifique mudança de página contra baseline com `monitor check`
- Liste o inventário vivo (56 comandos) com `commands --json`
- Descubra formatos de argv com `schema --cmd <name> --json`
- Imprima a versão do produto com `version`

```bash
browser-automation-cli --timeout 60 --json goto https://example.com
browser-automation-cli --json view
browser-automation-cli --json wait --text "Example Domain" --ms 3000
browser-automation-cli --json grab --path /tmp/page.png --full-page
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --json print-pdf --url https://example.com --path /tmp/page.pdf
```


## Multi-passo com Run
- Use `run --script` quando refs `@eN` precisam sobreviver entre passos
- Launches de processos separados nunca compartilham refs nem a sessão do Chrome
- Um processo é um ciclo de vida: BORN EXECUTE FINALIZE DIE
- Não existe modo daemon de produto
- Em erro fail-fast, o envelope de erro pode incluir `data.steps` parcial para recuperação

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
```
- Linhas NDJSON usam o campo `cmd` com o nome real do subcomando
- Scroll aceita `dy`/`dx` como aliases de `delta_y`/`delta_x`
- Assert aceita aliases `url_contains` / `text_contains`
- Flags globais como `--timeout` e `--step-timeout` valem para o script inteiro
- Prefira caminhos HTTP de scrape quando só precisar de conteúdo e não de refs ao vivo


## Padrões Avançados
- Capture network no processo: `--capture-network` e depois `net list --json`
- Capture console no processo: `--capture-console` e depois `console list --json`
- Emule sem perfil nomeado de device:
  - `emulate --user-agent "Mozilla/5.0 ..."`
  - `emulate --viewport 390x844x3,mobile,touch`
  - `emulate --network-conditions "Slow 3G"`
- Espere qualquer um de vários textos (semântica OR): `wait --text A --text B --ms 5000`
- Formatos de scrape: `--format text|markdown|html|links|metadata|summary|product|branding|raw-html|screenshot`
- Engines de scrape: `--engine http` (reqwest + scraper) ou `--engine browser` (CDP; formatos aplicam ao HTML capturado)
- Webhook opcional de operador com POST one-shot do resultado do scrape: `scrape ... --webhook-url https://127.0.0.1:9000/hook` (destino do operador, não telemetria de produto)
- Prefira heurística de conteúdo principal: `scrape ... --only-main-content`
- Batch scrape a partir de lista de URLs: `batch-scrape --urls-file urls.txt --format text --concurrency 2`
- Descubra sites com `crawl`, `map`, `search` e arquivos locais com `parse`
- Extract LLM (fail-closed sem chaves): defina XDG `openrouter_api_key`, opcionais `llm_base_url` / `llm_model`, depois `extract <url> --llm --question '...'`
- Proxy MITM one-shot: `mitm start --seconds 30` (bind em `127.0.0.1`)
- Journal de workflow em DAG: `workflow run|resume|status` (SQLite sob XDG state)
- Ferramentas profundas de heap exigem `--category-memory`
- Ferramentas de extension exigem `--category-extensions`
- Cliques por coordenada exigem `--experimental-vision`
- Lighthouse com caminho mock: `lighthouse https://example.com --lighthouse-path mock --json`
- Localize sugestões humanas: `--lang pt-BR` ou `config set lang pt-BR`
- Verbosity: `--verbose` (info), `--debug` (máximo), `-q`/`--quiet` ou `config set log_level debug`
- Cor: `config set color true|false` (valores truthy: `true`, `1`, `yes`)
- Path do Chrome: `config set chrome_path /path/to/chrome` quando a descoberta por PATH não bastar


## Configuração (XDG)
- Prefira flags para chamadas pontuais de agente
- Prefira config XDG via comando `config` para defaults duráveis
- Settings de produto são só flags e CLI XDG: `config init`, `config path`, `config show`, `config set`, `config get`
- Resolva paths vivos de config/data/state com `config path --json`
- Logging de produto é controlado por `--verbose` / `--debug` / `-q` e XDG `log_level`
- Chaves suportadas (lista completa de 13): `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`
- Valores truthy de color: `true`, `1`, `yes`
- Valores falsy ou outros resolvem para desligado salvo set truthy

```bash
browser-automation-cli --json config init
browser-automation-cli --json config path
browser-automation-cli --json config show
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
browser-automation-cli --json config get lang
```
- Mantenha a política dual-flag de robots explícita ao contornar: `--ignore-robots` mais `--i-accept-robots-risk`
- O `ignore_robots` da config sozinho não substitui a exigência dual-flag na linha de comando


## Scrape, Crawl, Map, Search, Parse, PDF, QR, Paths
```bash
# Single page as markdown over HTTP (no Chrome)
browser-automation-cli --json scrape https://example.com --format markdown --engine http --only-main-content

# Browser engine formats apply to captured outerHTML (markdown, links, …)
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --timeout 60 --json scrape https://example.com --format links --engine browser

# Optional one-shot operator webhook POST of scrape result data (not product telemetry)
browser-automation-cli --json scrape https://example.com --format markdown --engine http \
  --webhook-url https://127.0.0.1:9000/hook

# Many URLs (HTTP engine, one-shot)
printf '%s\n' 'https://example.com' 'https://example.org' > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2

# Crawl / map / search / parse local files
browser-automation-cli --json crawl https://example.com --same-host --limit 20 --max-depth 2 --format text
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
```
- Defaults de `scrape`: `--format text`, `--engine browser`
- A engine browser respeita `--format` (não fica só em text silencioso)
- `batch-scrape` sempre usa a engine HTTP
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


## i18n
```bash
browser-automation-cli --lang pt-BR --json click-at --x 1 --y 1
# usage error shows localized suggestion when lang is pt-BR (needs --experimental-vision for success)
browser-automation-cli --json config set lang pt-BR
```
- Mensagens humanas e sugestões honram `--lang` e XDG `lang`
- Envelopes de máquina mantêm campos estáveis em inglês: `kind` / `exit_code`


## MITM e Workflow
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list
browser-automation-cli --json mitm har

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
- Causa: `config set` recebeu chave fora do conjunto suportado
- Correção: use só `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`

### Chaves LLM ausentes
- Sintoma: exit `2`, mensagem `LLM extract requires XDG openrouter_api_key`
- Causa: `extract --llm` sem chave XDG
- Correção: `config set openrouter_api_key YOUR_KEY` (e opcionais `llm_base_url` / `llm_model`)

### Schema ou nome de comando errado
- Sintoma: exit `2`, mensagem `unknown command for schema: ...` ou clap `unrecognized subcommand`
- Causa: typo ou subcomando / nome de schema inventado
- Correção: rode `commands --json`, depois `schema --cmd <name> --json` com um nome listado

### Path de grab confundido com posicional
- Sintoma: erro de usage do clap em torno de argumentos inesperados
- Causa: destino do screenshot foi passado como posicional
- Correção: use `grab --path /tmp/page.png` (e opcional `--full-page`)


## Integração com Scripts de Shell
- Peça sempre stdout legível por máquina com `--json`
- Inspecione `$?` (ou `$LASTEXITCODE`) antes de confiar no payload
- Pipeie stdout em `jaq` / `jq` para extração de campos
- Mantenha diagnósticos no stderr com `--quiet` quando só quiser envelopes
- Em erros de `run`, inspecione `data.steps` parcial quando presente

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
- Descubra inventário com `commands --json` (56 comandos)
- Descubra argv com `schema --cmd <name> --json`
- Colapse trabalho browser multi-passo em um processo `run --script` quando refs importam
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


## Próximos Passos
- Receitas e fluxos mais longos: [docs/COOKBOOK.pt-BR.md](COOKBOOK.pt-BR.md)
- Contrato de agente e regras de lifecycle: [docs/AGENTS.pt-BR.md](AGENTS.pt-BR.md)
- Contratos JSON: [docs/schemas/README.md](schemas/README.md)
- Catálogo de plataforma e agentes: [INTEGRATIONS.pt-BR.md](../INTEGRATIONS.pt-BR.md)
- Mudanças de versão: [docs/MIGRATION.pt-BR.md](MIGRATION.pt-BR.md)
- Espelho em inglês: [docs/HOW_TO_USE.md](HOW_TO_USE.md)
