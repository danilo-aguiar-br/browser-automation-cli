---
name: browser-automation-cli
description: Aciona quando o usuário pede automação one-shot de browser, CLI de agente Chrome CDP, snapshot de acessibilidade com refs, preencher formulário, screenshot, scrape de página, captura de network ou console, heap snapshot, lighthouse, scripts multi-passo run, headless chrome ou browser-automation-cli. Esta skill DEVE ser usada para invocar a CLI Rust browser-automation-cli em trabalho de browser não interativo com envelope JSON e ciclo NASCE EXECUTA FINALIZE MORRE. Cobre flags globais, inventário de comandos, scripts NDJSON de run, exit codes, envelopes, gates de categoria, vision e screencast experimentais, política robots e variáveis de ambiente.
---

# browser-automation-cli


## Identidade e Arquitetura
### REQUIRED
- Trate o nome do binário como sempre `browser-automation-cli`
- Trate um processo como um ciclo de vida de Chrome: NASCE, EXECUTA, FINALIZE, MORRE
- Use Chrome ou Chromium de sistema descoberto pela CLI
- Mantenha trabalho multi-passo dentro de `run --script` quando refs `@eN` precisarem sobreviver
- Prefira `--json` para todo consumidor programático
- Instale com `cargo install --path . --locked` enquanto `publish = false`
- Após release no crates.io use `cargo install browser-automation-cli --locked`
- MSRV é Rust 1.88.0

### FORBIDDEN
- Não invente alias curto como `bac`
- Não mantenha daemon ou sessão sticky de browser entre processos
- Não espere empacotamento npm para este produto
- Não reutilize refs `@eN` entre launches de processo separados
- Não habilite telemetria remota

### Correct Pattern
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli -q --timeout 60 --json goto https://example.com
```


## Flags Globais
### REQUIRED
- Passe `--json` para envelopes legíveis por máquina
- Passe `-q` ou `--quiet` quando prosa de stderr poluir transcripts de agente
- Passe `--timeout <seconds>` para orçamento wall-clock do processo
- Passe `--step-timeout <seconds>` para orçamento por passo dentro de `run`
- Passe `--headed` só para debug interativo
- Passe `--capture-console` quando comandos `console` posteriores precisarem ver mensagens
- Passe `--capture-network` quando comandos `net` posteriores precisarem ver requests
- Passe gates de categoria só quando tools profundas forem necessárias
- Passe gates experimentais só quando essas superfícies forem intencionais

### FORBIDDEN
- Não assuma que flags de captura persistem entre launches separados
- Não habilite superfícies de categoria ou experimental em silêncio nos defaults do agente

### Correct Pattern
```bash
browser-automation-cli --json --timeout 90 --capture-network run --script steps.jsonl
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --experimental-vision click-at --x 10 --y 20 --json
```


## Comandos de Descoberta
### REQUIRED
- Use `doctor` para diagnosticar prontidão do Chrome
- Use `commands --json` para inventário vivo e tool map DevTools
- Use `schema --cmd <name> --json` antes de inventar argv de um comando
- Use `version --json` para fixar identidade do binário
- Use `completions <shell>` para setup humano de shell

### Correct Pattern
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd goto --json
browser-automation-cli version --json
```


## Navegação e Páginas
### REQUIRED
- Use `goto <url>` para navegar
- Use `back`, `forward` e `reload` para controle de histórico
- Use `page list|new|select|close` para multi-tab
- Use `scrape <url>` quando só body text for necessário
- Use `wait` para ms, text, selector ou load state

### Correct Pattern
```bash
browser-automation-cli --json goto https://example.com
browser-automation-cli --json page list
browser-automation-cli --json wait --text Example --ms 1000
```


## Snapshot e Input
### REQUIRED
- Use `view` para snapshots de acessibilidade com refs `@eN`
- Use `press` para cliques, não um nome de produto fictício `click`
- Use `write` para fill inteligente, não um nome de produto fictício `fill`
- Use `type` para digitação com `--target` ou `--focus-only` opcional
- Use `keys` para teclas únicas
- Use `hover`, `drag`, `fill-form` e `upload` para as ações tool-ref correspondentes
- Use `click-at` somente com `--experimental-vision`

### FORBIDDEN
- Não invente aliases de produto `snapshot`, `click`, `fill` ou `screenshot`
- Não chame `click-at` sem o gate experimental de vision

### Correct Pattern
```bash
browser-automation-cli --json view
browser-automation-cli --json press @e1 --include-snapshot
browser-automation-cli --json write @e2 "text"
browser-automation-cli --json fill-form --json '[{"target":"@e3","value":"x"}]'
```


## Observação e Artefatos
### REQUIRED
- Use `grab <path>` para screenshots
- Use `extract`, `text`, `attr`, `scroll` e `assert` para checagens de conteúdo
- Use `cookie list|set|clear` para helpers de jar
- Use `dialog` para aceitar ou dismissar diálogos

### Correct Pattern
```bash
browser-automation-cli --json grab /tmp/page.png --full-page
browser-automation-cli --json text @e2
browser-automation-cli --json cookie list
```


## Captura, Eval e Profundidade DevTools
### REQUIRED
- Use `console list|get` só após `--capture-console` no mesmo processo
- Use `net list|get` só após `--capture-network` no mesmo processo
- Use `eval` para avaliação JavaScript
- Use `emulate` e `resize` para device e viewport
- Use `perf start|stop|insight` para trabalho de performance
- Use `lighthouse` quando o binário externo estiver disponível
- Use `screencast` só com `--experimental-screencast`
- Use análise profunda de `heap` só com `--category-memory`
- Use `extension` só com `--category-extensions`
- Use `devtools3p` só com `--category-third-party`
- Use `webmcp` só com `--category-webmcp`

### Correct Pattern
```bash
browser-automation-cli --capture-console --json console list
browser-automation-cli --json eval 'document.title'
browser-automation-cli --category-memory --json heap summary --path snap.heapsnapshot
```


## Scripts Multi-passo Run
### REQUIRED
- Use `run --script <path>` para passos NDJSON em um processo
- Coloque um objeto JSON por linha com campo `cmd`
- Mantenha estado de página e refs `@eN` dentro desse único processo
- Defina `--timeout` grande o bastante para o script inteiro

### FORBIDDEN
- Não divida passos dependentes de ref entre múltiplos processos da CLI
- Não trate `exec` como engine multi-passo completo

### Correct Pattern
```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.jsonl
```


## Envelope JSON
### REQUIRED
- Espere sucesso no stdout como `{"schema_version":1,"ok":true,"data":...}`
- Espere erro no stdout com `--json` como `{"schema_version":1,"ok":false,"error":{...}}`
- Valide `ok` antes de ler `data`
- Mantenha stderr só para diagnósticos e tracing

### Correct Pattern
```bash
out=$(browser-automation-cli -q --json version)
echo "$out" | jaq -e '.ok == true'
echo "$out" | jaq -r '.data.version'
```


## Exit Codes e Retry
### REQUIRED
- Ramifique no exit code antes de confiar no stdout
- Trate `0` como sucesso
- Trate `2` como usage e corrija argv
- Trate `65` como erro de dados
- Trate `66` como no input
- Trate `69` como unavailable e repare install do Chrome
- Trate `70` como falha de software, browser ou protocolo
- Trate `74` como falha de I/O
- Trate `78` como falha de config
- Trate `124` como timeout e eleve orçamento ou encurte trabalho
- Trate `130` como cancelamento
- Trate `141` como broken pipe
- Retente só falhas transitórias de host ou launch de browser com backoff

### FORBIDDEN
- Não retente falhas puras de usage sem mudar argv
- Não mascare exit codes com `|| true` em pipelines de agente


## Variáveis de Ambiente
### REQUIRED
- Honre `BROWSER_AUTOMATION_CLI_JSON`, `QUIET`, `VERBOSE`, `DEBUG`, `TIMEOUT` e `STEP_TIMEOUT`
- Honre variáveis de captura, categoria, experimental, headed, artifacts, lang, robots, encryption, namespace e color documentadas no README
- Mantenha `BROWSER_AUTOMATION_CLI_ENCRYPTION_KEY` fora de logs duráveis

### FORBIDDEN
- Não logue encryption keys ou valores de cookie


## Política Robots
### REQUIRED
- Respeite defaults de robots
- Ao contornar, satisfaça a dual-flag com `--ignore-robots` e `--i-accept-robots-risk`

### FORBIDDEN
- Não contorne robots com uma flag casual única em automação de agente


## Nomes Canônicos
### REQUIRED
- Use `view` e não snapshot
- Use `press` e não click
- Use `write` e não fill
- Use `grab` e não screenshot
- Use somente o binário `browser-automation-cli`
