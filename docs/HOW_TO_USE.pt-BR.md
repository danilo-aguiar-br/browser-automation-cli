[English](HOW_TO_USE.md) | [Português Brasileiro](HOW_TO_USE.pt-BR.md)

# Como Usar — browser-automation-cli

> Instale uma vez, lance o Chrome uma vez por processo, termine a tarefa e saia limpo. Ciclo de vida: BORN EXECUTE FINALIZE DIE.


## Pré-requisitos
- Rust 1.88.0 ou mais recente ao compilar a partir do source
- Chrome ou Chromium disponível no PATH para comandos com engine browser
- ffmpeg opcional para export de screencast experimental
- binário Lighthouse opcional para auditorias, ou passe `--lighthouse-path` para um mock
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
- Extraia conteúdo com `scrape` quando precisar de text, markdown, html, links ou metadata
- Liste o inventário ao vivo com `commands --json`
- Descubra formatos de argv com `schema --cmd <name> --json`
- Imprima a versão do produto com `version`

```bash
browser-automation-cli --timeout 60 --json goto https://example.com
browser-automation-cli --json view
browser-automation-cli --json wait --text "Example Domain" --ms 3000
browser-automation-cli --json grab --path /tmp/page.png --full-page
browser-automation-cli --json scrape https://example.com --format text --engine browser
```


## Multi-passo com Run
- Use `run --script` quando refs `@eN` precisam sobreviver entre passos
- Launches de processos separados nunca compartilham refs nem a sessão do Chrome
- Um processo é um ciclo de vida: BORN EXECUTE FINALIZE DIE
- Não existe modo daemon de produto

```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500,"text":"Example Domain"}
{"cmd":"view"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.jsonl
```
- Linhas NDJSON usam o campo `cmd` com o nome real do subcomando
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
- Formatos de scrape: `--format text|markdown|html|links|metadata`
- Engines de scrape: `--engine http` (reqwest + scraper) ou `--engine browser` (CDP)
- Prefira heurística de conteúdo principal: `scrape ... --only-main-content`
- Batch scrape a partir de lista de URLs: `batch-scrape --urls-file urls.txt --format text --concurrency 2`
- Descubra sites com `crawl`, `map`, `search` e arquivos locais com `parse`
- Proxy MITM one-shot: `mitm start --seconds 30` (bind em `127.0.0.1`)
- Journal de workflow em DAG: `workflow run|resume|status` (SQLite sob XDG state)
- Ferramentas profundas de heap exigem `--category-memory`
- Ferramentas de extension exigem `--category-extensions`
- Cliques por coordenada exigem `--experimental-vision`
- Lighthouse com caminho mock em CI: `lighthouse https://example.com --lighthouse-path mock --json`


## Configuração (XDG)
- Prefira flags para chamadas pontuais de agente
- Prefira config XDG via comando `config` para defaults duráveis
- Não existem variáveis de ambiente de produto `BROWSER_AUTOMATION_CLI_*`
- Convenções de SO apenas: `RUST_LOG` para detalhe de tracing, `NO_COLOR` para desligar cor ANSI
- Comandos de layout: `config init`, `config path`, `config show`, `config set`, `config get`
- Chaves suportadas: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`
- Valores truthy de color: `true`, `1`, `yes`
- Valores falsy ou outros resolvem para desligado salvo set truthy

```bash
browser-automation-cli --json config init
browser-automation-cli --json config path
browser-automation-cli --json config show
browser-automation-cli --json config set lang en
browser-automation-cli --json config set timeout 60
browser-automation-cli --json config set artifacts_dir /tmp/bac-artifacts
browser-automation-cli --json config set color true
browser-automation-cli --json config get lang
```
- Mantenha a política dual-flag de robots explícita ao contornar: `--ignore-robots` mais `--i-accept-robots-risk`
- O `ignore_robots` da config sozinho não substitui a exigência dual-flag na linha de comando


## Scrape, Crawl, Map, Search, Parse
```bash
# Single page as markdown over HTTP (no Chrome)
browser-automation-cli --json scrape https://example.com --format markdown --engine http --only-main-content

# Browser engine when JS rendering is required
browser-automation-cli --timeout 60 --json scrape https://example.com --format text --engine browser

# Many URLs (HTTP engine, one-shot)
printf '%s\n' 'https://example.com' 'https://example.org' > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2

# Crawl / map / search / parse local files
browser-automation-cli --json crawl https://example.com --same-host --limit 20 --max-depth 2 --format text
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
browser-automation-cli --json search "example domain" --limit 10
browser-automation-cli --json parse /tmp/page.html
```
- Defaults de `scrape`: `--format text`, `--engine browser`
- `batch-scrape` sempre usa a engine HTTP
- `crawl` permanece no host da seed quando você passa `--same-host`
- `parse` extrai texto de caminhos locais `html`, `md`, `txt` e PDF
- Honre robots por padrão; bypass dual-flag quando contornar a política for intencional


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
- MITM faz bind apenas em loopback (`127.0.0.1`) com porta efêmera
- CA do MITM fica sob XDG data; capturas sob XDG state
- Journals de workflow ficam sob XDG state (SQLite)
- Resume pula passos já marcados `ok` no journal
- Passos offline de workflow são apenas data-plane
- Trabalho multi-passo com refs `@eN` compartilhadas permanece em `run --script`


## Erros Comuns
### Chrome ausente
- Sintoma: saída `69`, kind `unavailable`, mensagem sobre chrome not found
- Causa: Chrome ou Chromium não instalado ou fora do PATH
- Correção: instale Chromium ou Google Chrome, garanta o PATH, rode de novo `doctor --offline --quick --json`

### Timeout
- Sintoma: saída `124`, kind `timeout`
- Causa: navegação ou passo excedeu `--timeout` / orçamento de wait
- Correção: aumente `--timeout`, use `wait --text` / `--selector` alvo, ou prefira `--engine http` quando CDP for desnecessário

### Dual-flag de robots incompleta
- Sintoma: saída `2`, mensagem `--ignore-robots requires --i-accept-robots-risk`
- Causa: apenas uma flag de bypass de robots foi passada
- Correção: passe juntas `--ignore-robots` e `--i-accept-robots-risk` quando for intencional

### Broken pipe (saída 141)
- Sintoma: saída `141`, kind `broken-pipe` quando o consumidor fecha o stdout cedo
- Causa: pipe para leitor fechado (por exemplo um head que sai no meio do stream)
- Correção: leia o stdout completo antes de fechar, ou evite teardown precoce do pipe; trate `141` como semântica esperada de pipe

### Chave de config desconhecida
- Sintoma: saída `2`, mensagem `unknown config key: ...`
- Causa: `config set` recebeu chave fora do conjunto suportado
- Correção: use apenas `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`

### Schema ou nome de comando errado
- Sintoma: saída `2`, mensagem `unknown command for schema: ...` ou clap `unrecognized subcommand`
- Causa: typo ou subcomando / nome de schema inventado
- Correção: rode `commands --json` e depois `schema --cmd <name> --json` com um nome listado

### Caminho do grab confundido com posicional
- Sintoma: erro de usage do clap por argumentos inesperados
- Causa: destino do screenshot foi passado como posicional
- Correção: use `grab --path /tmp/page.png` (e opcional `--full-page`)


## Integração Com Scripts de Shell
- Sempre peça stdout legível por máquina com `--json`
- Inspecione `$?` (ou `$LASTEXITCODE`) antes de confiar no payload
- Pipe o stdout para `jaq` / `jq` para extrair campos
- Mantenha diagnósticos no stderr com `--quiet` quando quiser só envelopes

```bash
browser-automation-cli --timeout 60 --json goto https://example.com \
  | jaq -e '.ok == true'

browser-automation-cli --json scrape https://example.com --format markdown --engine http \
  | jaq -r '.data // .'

printf '%s\n' 'https://example.com' > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2 \
  | jaq .
```
- Saída broken pipe `141` significa que o leitor fechou cedo, não necessariamente bug da CLI
- Prefira scrape / batch / crawl HTTP em pipelines de shell que não precisam de CDP


## Integração Com Agentes de IA
- Lance `browser-automation-cli` como subprocesso one-shot por fronteira de tarefa
- Passe `--json` em toda chamada programática
- Parseie apenas envelopes de stdout; trate stderr como diagnóstico
- Ramifique pelo campo `ok` do envelope e pelo código de saída do processo
- Descubra inventário com `commands --json`
- Descubra argv com `schema --cmd <name> --json`
- Colapse trabalho multi-passo de browser em um processo `run --script` quando refs importam
- Prefira flags para controle pontual; use `config` para defaults XDG duráveis
- Não invente daemon entre turnos do agente
- Não invente variáveis de ambiente de produto como `BROWSER_AUTOMATION_CLI_*`
- Env de SO apenas quando necessário: `RUST_LOG`, `NO_COLOR`
- Editores e runners compatíveis incluem Claude Code, Codex, Cursor, Continue e Cline via shell ou subprocesso
- Contrato completo do agente: [docs/AGENTS.md](AGENTS.md) e [INTEGRATIONS.md](../INTEGRATIONS.md)


## Integração Com Crates Rust
- Chame o binário com `std::process::Command`
- Capture stdout, confira o status, deserialise com `serde_json`
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
- Prefira `scrape` HTTP em checagens estilo unitário que não devem lançar Chrome
- Use `run --script` quando a crate orquestra fluxos CDP multi-passo
- Veja notas orientadas a crates em [docs/AGENTS.md](AGENTS.md) e [INTEGRATIONS.md](../INTEGRATIONS.md)


## Próximos Passos
- Receitas e fluxos longos: [docs/COOKBOOK.md](COOKBOOK.md)
- Contrato de agente e regras de lifecycle: [docs/AGENTS.md](AGENTS.md)
- Contratos JSON: [docs/schemas/README.md](schemas/README.md)
- Catálogo de plataforma e agentes: [INTEGRATIONS.md](../INTEGRATIONS.md)
- Mudanças entre versões: [docs/MIGRATION.md](MIGRATION.md)
- Espelho em inglês: [docs/HOW_TO_USE.md](HOW_TO_USE.md)
