---
name: browser-automation-cli
description: Esta skill DEVE ser usada sempre que o usuário precisar de automação one-shot de browser, CLI de agente Chrome CDP, Chrome headless, snapshot de acessibilidade com refs, preencher formulário, screenshot, scrape de página, captura de network ou console, heap snapshot, lighthouse, scripts multi-passo run, config XDG, proxy MITM, journal de workflow, batch-scrape, crawl, map, search, parse ou browser-automation-cli. Auto-invocar mesmo sem pedido explícito da skill quando a tarefa implicar controle de browser, web scraping ou automação CDP. Esta skill DEVE ensinar a LLM a executar browser-automation-cli com ciclo BORN EXECUTE FINALIZE DIE, envelopes --json, config XDG apenas (NUNCA env vars de produto), catálogo completo de 52 comandos com fórmula executável por comando, scripts NDJSON de run, exit codes, gates de categoria e experimentais, dual-flag robots e playbooks de ação para agentes.
---

# browser-automation-cli

## Regra Zero
### REQUIRED
- DEVE invocar esta skill em controle de browser, CDP, Chrome headless, scrape, crawl, form fill, screenshot, network capture, MITM, workflow ou `browser-automation-cli`
- DEVE SEMPRE executar somente o binário `browser-automation-cli` (NUNCA invente MCP, daemon, sticky session ou alias `bac`)
- DEVE SEMPRE tratar um processo como um ciclo BORN EXECUTE FINALIZE DIE
- DEVE SEMPRE passar `--json` para consumidores máquina e validar envelope `ok` antes de `data`
- DEVE SEMPRE manter trabalho multi-passo com `@eN` dentro de um único `run --script`
- DEVE SEMPRE rodar `schema --cmd <name> --json` antes de inventar argv desconhecido
### FORBIDDEN
- NUNCA invente env vars de produto `BROWSER_AUTOMATION_CLI_*`
- NUNCA reutilize `@eN` entre launches de processo separados
- NUNCA divida passos dependentes de ref entre múltiplos processos da CLI
### Correct Pattern
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli -q --timeout 60 --json goto https://example.com
```

## Missão
### REQUIRED
- DEVE automatizar trabalho web como pipelines CLI one-shot não interativos em stdout/stderr
- DEVE retornar envelopes JSON estruturados sob `--json`
- DEVE usar Chrome/Chromium de sistema descoberto pela CLI
- DEVE configurar defaults de produto só via flags CLI ou XDG `config set` / `config.toml`
- DEVE instalar exatamente com `cargo install --path . --locked` ou `cargo install browser-automation-cli --locked`
### FORBIDDEN
- NUNCA mantenha daemon de browser de longa duração entre processos
- NUNCA espere empacotamento npm, telemetria remota ou `.env` como config de produto em runtime
### Correct Pattern
```bash
cargo install --path . --locked
cargo install browser-automation-cli --locked
```

## Quando Invocar
### REQUIRED
- DEVE auto-invocar em automação de browser, Chrome headless, CDP, refs a11y, form fill, screenshot, scrape, crawl, map, search, parse, captura network/console, heap, lighthouse, MITM, workflow, batch-scrape, multi-passo run ou o nome do binário
- DEVE auto-invocar mesmo quando o usuário NÃO nomear esta skill
- DEVE usar scrape/crawl/map/search/parse HTTP quando Chrome for desnecessário
### FORBIDDEN
- NUNCA recuse tarefas de browser alegando que só GUI ou MCP resolvem
- NUNCA invente Firecrawl cloud ou servidores remotos de workflow para este produto
### Correct Pattern
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http
browser-automation-cli --timeout 90 --json run --script /tmp/steps.jsonl
```

## Identidade
### REQUIRED
- DEVE tratar o nome do binário como exatamente `browser-automation-cli`
- DEVE tratar um processo como BORN, EXECUTE, FINALIZE, DIE
- DEVE usar Chrome/Chromium de sistema descoberto pela CLI
- DEVE manter multi-passo dependente de `@eN` dentro de `run --script`
- DEVE passar `--json` em todo consumidor programático
- DEVE configurar defaults só via flags ou XDG `config set` / `config.toml`
- DEVE usar o inventário completo de 52 comandos como superfície viva
### FORBIDDEN
- NUNCA invente alias `bac`, sticky sessions, npm packaging ou env vars `BROWSER_AUTOMATION_CLI_*`
- NUNCA reutilize refs `@eN` entre launches de processo
### Correct Pattern
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli -q --timeout 60 --json goto https://example.com
```
Inventário 52 (OBRIGATÓRIO):

doctor commands schema version goto view press click-at write keys type wait hover drag fill-form upload back forward reload eval grab run exec extract text scroll cookie attr assert console net page dialog scrape batch-scrape crawl map search parse mitm workflow config emulate resize perf lighthouse screencast heap extension devtools3p webmcp completions

## Flags Globais
### REQUIRED
- DEVE passar `--json` para envelopes legíveis por máquina
- DEVE passar `-q`/`--quiet` quando prosa de stderr NÃO DEVE poluir transcripts
- DEVE passar `--timeout <seconds>` para orçamento wall-clock do processo
- DEVE passar `--step-timeout <seconds>` para orçamento por passo em todo `run` multi-passo
- DEVE passar `--headed` só para debug interativo
- DEVE passar `--capture-console` antes de qualquer `console` no mesmo processo
- DEVE passar `--capture-network` antes de qualquer `net` no mesmo processo
- DEVE passar gates de categoria antes das tools gated: `--category-memory`, `--category-extensions`, `--category-third-party`, `--category-webmcp`
- DEVE passar gates experimentais: `--experimental-vision` para `click-at`, `--experimental-screencast` para `screencast`
- DEVE usar só vars de SO fora da config de produto: `RUST_LOG`, `NO_COLOR`
### FORBIDDEN
- NUNCA assuma que flags de captura persistem entre launches
- NUNCA habilite categorias/experimentais em silêncio nos defaults do agente
- NUNCA invente env vars `BROWSER_AUTOMATION_CLI_*`
### Correct Pattern
```bash
browser-automation-cli --json --timeout 90 --capture-network run --script steps.jsonl
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --experimental-vision click-at --x 10 --y 20 --json
RUST_LOG=debug browser-automation-cli --debug --json doctor --offline --quick
```

## Config XDG
### REQUIRED
- DEVE tratar settings de produto como flags mais config XDG apenas
- DEVE usar `config init|path|show|get|set`
- DEVE usar somente as 7 chaves: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`
- DEVE definir encryption com `config set encryption_key <secret>`
- DEVE esperar layout Linux sob `$XDG_CONFIG_HOME/browser-automation-cli` (e data/cache/state correspondentes)
### FORBIDDEN
- NUNCA invente env vars de produto para settings/encryption
- NUNCA use `.env` como config de produto em runtime
- NUNCA logue valores de `encryption_key`
- NUNCA invente chaves fora das sete suportadas
### Correct Pattern
```bash
browser-automation-cli config init --json
browser-automation-cli config path --json
browser-automation-cli config show --json
browser-automation-cli config get timeout --json
browser-automation-cli config set timeout 90 --json
browser-automation-cli config set encryption_key "replace-me" --json
```

## Catálogo Completo de Comandos (52)
### REQUIRED
- DEVE executar as fórmulas abaixo como superfície canônica viva
- DEVE copiar argv exatamente; só substitua placeholders de path/URL/texto
- DEVE chamar `schema --cmd <name> --json` antes de inventar flags extras
- DEVE manter `grab` SEMPRE com `--path`; `emulate` NUNCA com `--device`
- DEVE tratar multi `--text` em `wait` como OR; `type` com TEXT + `--target` OU `--focus-only`
- DEVE passar `fill-form` com `--json` array de comando + `--json` global de envelope
- DEVE tratar `exec` como single-step; MITM só em `127.0.0.1`; workflow só manifest JSON
### FORBIDDEN
- NUNCA invente alias `bac`, path posicional em `grab`, `emulate --device` ou manifest workflow não-JSON
- NUNCA omita gates de categoria/experimental exigidos pela fórmula
### Correct Pattern
```bash
# meta / descoberta
browser-automation-cli doctor --offline --quick --json
browser-automation-cli commands --json
browser-automation-cli schema --cmd goto --json
browser-automation-cli version --json
browser-automation-cli completions bash

# navegação e páginas
browser-automation-cli --json goto https://example.com
browser-automation-cli --json back
browser-automation-cli --json forward
browser-automation-cli --json reload --ignore-cache
browser-automation-cli --json page info
browser-automation-cli --json page list
browser-automation-cli --json page new --url https://example.com
browser-automation-cli --json page select 0 --bring-to-front
browser-automation-cli --json page close --index 0
browser-automation-cli --json wait --ms 500
browser-automation-cli --json wait --text Example --text Demo --ms 1000
browser-automation-cli --json wait --selector "h1" --state load

# snapshot e input
browser-automation-cli --json view
browser-automation-cli --json press @e1 --include-snapshot
browser-automation-cli --experimental-vision --json click-at --x 10 --y 20
browser-automation-cli --json write @e2 "hello"
browser-automation-cli --json keys Enter
browser-automation-cli --json type "hello" --target @e2 --clear --submit Enter
browser-automation-cli --json type "world" --focus-only
browser-automation-cli --json hover @e1
browser-automation-cli --json drag --from @e1 --to @e2
browser-automation-cli --json fill-form --json '[{"target":"@e3","value":"x"}]'
browser-automation-cli --json upload @e4 /tmp/file.txt

# observação e artefatos
browser-automation-cli --json grab --path /tmp/page.png --full-page
browser-automation-cli --json extract @e1
browser-automation-cli --json extract @e1 --attr href
browser-automation-cli --json text @e2
browser-automation-cli --json scroll --delta-y 400
browser-automation-cli --json attr @e1 href
browser-automation-cli --json assert url https://example.com --contains
browser-automation-cli --json assert text "Example"
browser-automation-cli --capture-console --json assert console --level error
browser-automation-cli --json cookie list
browser-automation-cli --json cookie set --json '[{"name":"a","value":"b","url":"https://example.com"}]'
browser-automation-cli --json cookie clear
browser-automation-cli --json dialog accept
browser-automation-cli --json dialog dismiss
browser-automation-cli --json eval 'document.title'

# captura console/net (mesmo processo)
browser-automation-cli --capture-console --json console list
browser-automation-cli --capture-console --json console get 0
browser-automation-cli --capture-console --json console clear
browser-automation-cli --capture-console --json console dump --path /tmp/console.json
browser-automation-cli --capture-network --json net list
browser-automation-cli --capture-network --json net get 0

# scrape local
browser-automation-cli --json scrape https://example.com --format markdown --engine http
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
browser-automation-cli --json search "example domain" --limit 10
browser-automation-cli --json parse /tmp/page.html

# emulate / resize / perf / lighthouse / screencast / heap / extension / third-party / webmcp
browser-automation-cli --json emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G"
browser-automation-cli --json resize --width 1280 --height 720
browser-automation-cli --json perf start
browser-automation-cli --json perf stop --path /tmp/trace.json
browser-automation-cli --json perf insight --name DocumentLatency
browser-automation-cli --json lighthouse https://example.com
browser-automation-cli --experimental-screencast --json screencast start --path /tmp/cast
browser-automation-cli --experimental-screencast --json screencast stop
browser-automation-cli --category-memory --json heap take --path /tmp/snap.heapsnapshot
browser-automation-cli --category-memory --json heap summary --path /tmp/snap.heapsnapshot
browser-automation-cli --category-memory --json heap compare --base /tmp/a.heapsnapshot --current /tmp/b.heapsnapshot
browser-automation-cli --category-extensions --json extension list
browser-automation-cli --category-extensions --json extension install --path /tmp/ext
browser-automation-cli --category-third-party --json devtools3p list
browser-automation-cli --category-third-party --json devtools3p exec SomeTool --params '{}'
browser-automation-cli --category-webmcp --json webmcp list
browser-automation-cli --category-webmcp --json webmcp exec SomeTool --input '{}'

# MITM 127.0.0.1
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list --limit 50
browser-automation-cli --json mitm get 0
browser-automation-cli --json mitm har --out /tmp/capture.har
browser-automation-cli --json mitm export --out /tmp/capture.json
browser-automation-cli --json mitm domains
browser-automation-cli --json mitm apis

# workflow JSON + run multi-passo + exec single-step
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
browser-automation-cli --timeout 60 --json run --script /tmp/steps.jsonl
browser-automation-cli --json exec goto https://example.com
```

## Scripts Multi-passo
### REQUIRED
- DEVE usar `run --script <path>` para passos NDJSON em um processo
- DEVE colocar um objeto JSON por linha com campo `cmd`
- DEVE manter estado de página e refs `@eN` dentro desse único processo
- DEVE definir `--timeout` grande o bastante para o script inteiro
- DEVE serializar grab como `{"cmd":"grab","path":"/tmp/example.png"}` no NDJSON
- DEVE tratar `exec` como single-step inline apenas (NÃO engine multi-passo)
- DEVE usar workflow só com manifest JSON path (ex. `/tmp/wf.json`)
### FORBIDDEN
- NUNCA divida passos dependentes de ref entre processos
- NUNCA trate `exec` como engine multi-passo
- NUNCA espere `@eN` sobreviver ao DIE do processo
- NUNCA use manifest workflow não-JSON
### Correct Pattern
```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.jsonl

cat > /tmp/wf.json <<'JSON'
{"name":"demo","steps":[{"id":"ping","cmd":"echo","args":{"message":"start"}}]}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
```

## Playbooks
### REQUIRED
- DEVE executar estas fórmulas as-is salvo quando `schema --cmd` forçar mudança de flag
- DEVE manter multi-passo `@eN` dentro de um único `run --script`
- DEVE validar envelope `ok` após cada invocação
### FORBIDDEN
- NUNCA invente `bac`, env vars de produto, path bare em `grab`, `emulate --device` ou manifest workflow não-JSON
### Correct Pattern
#### A. Doctor e version
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli version --json
```
#### B. HTTP scrape markdown (sem Chrome com --engine http)
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http
```
#### C. Form fill browser via run NDJSON
```bash
cat > /tmp/form.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"write","target":"@e1","value":"hello"}
{"cmd":"press","target":"@e2"}
{"cmd":"grab","path":"/tmp/form.png"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/form.browser-automation.jsonl
```
#### D. Wait OR multi-text
```bash
browser-automation-cli --timeout 60 --json wait --text "Example Domain" --text "Example" --ms 5000
```
#### E. Network capture list
```bash
cat > /tmp/net.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"net","action":"list","resource_types":"Document,XHR"}
JSONL
browser-automation-cli --capture-network --timeout 60 --json run --script /tmp/net.browser-automation.jsonl
```
#### F. MITM init-ca + start
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
```
#### G. Workflow run manifest JSON offline
```bash
cat > /tmp/wf.json <<'JSON'
{"name":"demo","steps":[{"id":"ping","cmd":"echo","args":{"message":"start"}}]}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
```
#### H. batch-scrape / crawl / map / search / parse
```bash
printf '%s\n' https://example.com https://example.org > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
browser-automation-cli --json search "example domain" --limit 10
browser-automation-cli --json parse /tmp/page.html
```
#### I. Config XDG set timeout encryption_key
```bash
browser-automation-cli config init --json
browser-automation-cli config set timeout 90 --json
browser-automation-cli config set encryption_key "replace-me" --json
browser-automation-cli config show --json
```
#### J. Schema discovery antes de argv desconhecido
```bash
browser-automation-cli schema --cmd scrape --json
browser-automation-cli schema --cmd fill-form --json
browser-automation-cli schema --cmd workflow --json
browser-automation-cli commands --json
```

## Envelope JSON
### REQUIRED
- DEVE esperar sucesso: `{"schema_version":1,"ok":true,"data":...}`
- DEVE esperar erro sob `--json`: `{"schema_version":1,"ok":false,"error":{...}}`
- DEVE validar `ok` antes de ler `data`
- DEVE manter stderr só para diagnósticos/tracing
### FORBIDDEN
- NUNCA trate prosa humana no stdout sob `--json` como contrato primário
- NUNCA ignore `ok:false` com exit não-zero
### Correct Pattern
```bash
out=$(browser-automation-cli -q --json version)
echo "$out" | jaq -e '.ok == true'
echo "$out" | jaq -r '.data.version'
```

## Exit Codes
### REQUIRED
- DEVE ramificar no exit code antes de confiar no stdout
- DEVE tratar códigos: `0` sucesso; `2` usage/fix argv; `65` data; `66` no input; `69` unavailable/reparar Chrome; `70` software/browser/protocol; `74` I/O; `78` config; `124` timeout/elevar orçamento; `130` cancel; `141` broken pipe
- DEVE retentar só falhas transitórias de host/launch de browser com backoff
### FORBIDDEN
- NUNCA retente falhas puras de usage sem mudar argv
- NUNCA mascare exit codes com `|| true`
### Correct Pattern
```bash
set +e; browser-automation-cli -q --timeout 60 --json goto https://example.com; code=$?; set -e
case "$code" in 0) echo ok;; 2) echo fix_argv;; 69) echo repair_chrome;; 124) echo raise_timeout;; *) echo fail_$code;; esac
```

## Robots
### REQUIRED
- DEVE respeitar defaults de robots
- DEVE contornar só com dual-flag juntas: `--ignore-robots` E `--i-accept-robots-risk`
### FORBIDDEN
- NUNCA contorne robots com uma flag única
- NUNCA invente env vars de bypass de robots
### Correct Pattern
```bash
browser-automation-cli --json scrape https://example.com --format text --engine http
browser-automation-cli --ignore-robots --i-accept-robots-risk --json scrape https://example.com --format text --engine http
```

## Mapa DevTools
### REQUIRED
- DEVE usar somente o binário `browser-automation-cli`
- DEVE usar `view` não snapshot; `press` não click; `write` não fill; `grab` não screenshot
- DEVE mapear DevTools tools para comandos de produto exatamente como abaixo
- DEVE usar flags/XDG para settings de produto; fora disso só vars de SO `RUST_LOG` e `NO_COLOR`
Mapa DevTools (OBRIGATÓRIO): click→press, fill→write, take_screenshot→grab, take_snapshot→view, type_text→type, press_key→keys, fill_form→fill-form, upload_file→upload, click_at→click-at, navigate_page→goto|back|forward|reload, wait_for→wait, evaluate_script→eval, list_network_requests→net list, list_console_messages→console list
### FORBIDDEN
- NUNCA invente aliases de produto que conflitem com este mapa
- NUNCA chame nomes DevTools como subcomandos CLI
### Correct Pattern
```bash
browser-automation-cli --timeout 60 --json run --script /tmp/goto-view-press.jsonl
```

## Proibições
### REQUIRED
- DEVE recusar padrões ilegais e reescrever para a superfície CLI canônica
### FORBIDDEN
- NUNCA invente `bac` ou `BROWSER_AUTOMATION_CLI_*`
- NUNCA use `.env` como config de produto em runtime
- NUNCA passe path posicional bare para `grab` (SEMPRE `--path`)
- NUNCA invente `emulate --device`
- NUNCA use manifests workflow não-JSON (SEMPRE path JSON, ex. `/tmp/wf.json`)
- NUNCA trate `exec` como engine multi-passo (SEMPRE `run --script`)
- NUNCA reutilize `@eN` entre processos
- NUNCA habilite gates de categoria/experimental sem intenção
- NUNCA exponha MITM além de `127.0.0.1`
- NUNCA invente Firecrawl cloud ou servidores remotos sticky de workflow
- NUNCA mascare exit codes com `|| true`
- NUNCA contorne robots sem as duas dual-flags
### Correct Pattern
```bash
browser-automation-cli --json grab --path /tmp/x.png
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --timeout 60 --json run --script /tmp/steps.jsonl
```

## Checklist
### REQUIRED
- OBRIGATÓRIO confirmar binário `browser-automation-cli` e ciclo BORN EXECUTE FINALIZE DIE
- OBRIGATÓRIO confirmar envelope `--json` `ok` e multi-passo `@eN` dentro de um `run --script`
- OBRIGATÓRIO confirmar `grab --path`, workflow manifest JSON, sem `emulate --device`, wait multi-text OR
- OBRIGATÓRIO confirmar console `list|get|clear|dump` + `--capture-console` no mesmo processo
- OBRIGATÓRIO confirmar net `list|get` + `--capture-network` no mesmo processo
- OBRIGATÓRIO confirmar `type` TEXT posicional + `--target` OU `--focus-only`
- OBRIGATÓRIO confirmar fill-form `--json` array de comando + `--json` global; upload target+path; `exec` single-step only
- OBRIGATÓRIO confirmar só sete chaves de config; NUNCA invente env de produto (só `RUST_LOG`/`NO_COLOR`)
- OBRIGATÓRIO confirmar exit codes 0,2,65,66,69,70,74,78,124,130,141; robots dual-flag; gates categoria/experimental; schema discovery
- OBRIGATÓRIO confirmar cobertura de todos os 52 comandos no Catálogo Completo de Comandos
### FORBIDDEN
- NUNCA entregue glue de agente que viole este checklist
### Correct Pattern
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd run --json
browser-automation-cli doctor --offline --quick --json
```
