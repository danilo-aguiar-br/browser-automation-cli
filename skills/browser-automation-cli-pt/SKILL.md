---
name: browser-automation-cli
description: Esta skill DEVE ser usada para automação one-shot de browser com browser-automation-cli, Chrome CDP, Chrome headless, refs a11y, formulário, grab screenshot, print-pdf, scrape multi-formato, captura network ou console, heap, lighthouse binary_source, run multi-passo NDJSON ou array JSON com fail-fast data.steps, config XDG 16 chaves incluindo cache e log_to_file, config list-keys, Redis URL plain com rediss fail-closed, MITM, workflow, batch-scrape, crawl, map, search, parse PDF DOCX xlsx ods, extract --llm schema-json, monitor check, qr encode decode, find-paths --glob, sheet-write, sg-scan, sg-rewrite, goto init-script. Auto-invocar em controle de browser, scraping, CDP, PDF, QR, caminhos, planilhas, scan estrutural ou extract LLM mesmo sem nomear esta skill. Esta skill DEVE ensinar BORN EXECUTE FINALIZE DIE, envelopes --json, só XDG mais flags (NUNCA env vars de produto), 59 comandos, formulas em references/formulas.md, exit codes, dual-flag robots e playbooks de ação.
---

# browser-automation-cli

## Regra Zero
### REQUIRED
- DEVE invocar esta skill em controle de browser, CDP, Chrome headless, scrape, crawl, formulário, grab, print-pdf, QR, find-paths, sheet-write, sg-scan, sg-rewrite, monitor, captura network/console, heap, lighthouse, MITM, workflow, parse PDF/DOCX/xlsx/ods, extract --llm, cache Redis XDG, ou `browser-automation-cli`
- DEVE SEMPRE executar somente o binário `browser-automation-cli` (NUNCA invente wrappers de servidor de protocolo, daemons, sticky sessions ou alias `bac`)
- DEVE SEMPRE tratar um processo como um ciclo BORN EXECUTE FINALIZE DIE
- DEVE SEMPRE passar `--json` para consumidores máquina e validar envelope `ok` antes de `data`
- DEVE SEMPRE manter trabalho multi-passo com `@eN` dentro de um único `run --script` (NDJSON ou array JSON)
- DEVE SEMPRE rodar `schema --cmd <name> --json` antes de inventar argv desconhecido
- DEVE SEMPRE carregar fórmulas executáveis de `references/formulas.md` para argv completo
### FORBIDDEN
- NUNCA invente variáveis de ambiente de produto para settings ou logging
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
- DEVE usar Chrome/Chromium de sistema descoberto pela CLI (ou XDG `chrome_path`)
- DEVE configurar defaults de produto só via flags CLI ou XDG `config set` / `config.toml`
- DEVE instalar exatamente com `cargo install --path . --locked` ou `cargo install browser-automation-cli --locked`
### FORBIDDEN
- NUNCA mantenha daemon de browser de longa duração entre processos
- NUNCA espere empacotamento npm ou `.env` como config de produto em runtime
- NUNCA invente variáveis de ambiente de produto para logging (SEMPRE use `--verbose`/`--debug`/`-q` ou `config set log_level` / `log_to_file`)
### Correct Pattern
```bash
cargo install browser-automation-cli --locked
browser-automation-cli doctor --offline --quick --json
```

## Quando Invocar
### REQUIRED
- DEVE auto-invocar em automação de browser, Chrome headless, CDP, refs a11y, formulário, grab, print-pdf, scrape, crawl, map, search, parse, extract --llm, monitor, qr, find-paths, sheet-write, sg-scan, sg-rewrite, captura network/console, heap, lighthouse, MITM, workflow, batch-scrape, multi-passo run, Redis XDG/cache, ou o nome do binário
- DEVE auto-invocar mesmo quando o usuário NÃO nomear esta skill
- DEVE usar scrape/crawl/map/search/parse/qr/find-paths/sheet-write/sg-scan/sg-rewrite HTTP/local quando Chrome for desnecessário
### FORBIDDEN
- NUNCA recuse tarefas de browser alegando que só GUI ou servidores de protocolo externos resolvem
- NUNCA invente SaaS cloud de scrape ou servidores remotos de workflow para este produto
### Correct Pattern
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http
browser-automation-cli --timeout 90 --json run --script /tmp/steps.jsonl
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx
```

## Identidade
### REQUIRED
- DEVE tratar o nome do binário como exatamente `browser-automation-cli`
- DEVE tratar um processo como BORN, EXECUTE, FINALIZE, DIE
- DEVE manter multi-passo dependente de `@eN` dentro de `run --script` quando refs DEVEM sobreviver
- DEVE passar `--json` em todo consumidor programático
- DEVE configurar defaults só via flags ou XDG `config set` / `config.toml`
- DEVE tratar a superfície viva como exatamente 59 nomes de comando de topo (incluindo `sheet-write`, `sg-scan`, `sg-rewrite` e a paridade DevTools)
### FORBIDDEN
- NUNCA invente alias `bac`, sticky sessions, npm packaging ou variáveis de ambiente de produto para settings
- NUNCA reutilize refs `@eN` entre launches de processo
- NUNCA omita comandos do inventário de 59 nomes
### Correct Pattern
```bash
browser-automation-cli -q --timeout 60 --json goto https://example.com
browser-automation-cli --timeout 90 --json run --script /tmp/steps.jsonl
browser-automation-cli commands --json
```

## Flags Globais
### REQUIRED
- DEVE passar `--json` para envelopes legíveis por máquina
- DEVE passar `-q`/`--quiet` quando prosa de stderr NÃO DEVE poluir transcripts
- DEVE passar `--verbose` ou `--debug` para detalhe de logging de produto (ou definir XDG `log_level`)
- DEVE passar `--timeout <seconds>` para orçamento wall-clock do processo quando o trabalho puder travar
- DEVE passar `--step-timeout <seconds>` para orçamento por passo em todo `run` multi-passo
- DEVE passar `--headed` só para debug interativo
- DEVE passar `--capture-console` antes de qualquer `console` no mesmo processo
- DEVE passar `--capture-network` antes de qualquer `net` no mesmo processo
- DEVE passar gates de categoria antes das tools gated: `--category-memory`, `--category-extensions`, `--category-third-party`, `--category-webmcp`
- DEVE passar gates experimentais: `--experimental-vision` para `click-at`, `--experimental-screencast` para `screencast`
### FORBIDDEN
- NUNCA assuma que flags de captura persistem entre launches
- NUNCA habilite categorias/experimentais em silêncio nos defaults do agente
- NUNCA invente variáveis de ambiente de produto para settings ou logging
### Correct Pattern
```bash
browser-automation-cli --json --timeout 90 --capture-network run --script steps.jsonl
browser-automation-cli --category-memory heap summary --path snap.heapsnapshot --json
browser-automation-cli --experimental-vision click-at --x 10 --y 20 --json
browser-automation-cli --debug --json doctor --offline --quick
```

## Config XDG
### REQUIRED
- DEVE tratar settings de produto como flags mais config XDG apenas
- DEVE usar `config init|path|show|get|set|list-keys`
- DEVE usar somente estas 16 chaves: `lang`, `timeout`, `artifacts_dir`, `ignore_robots`, `namespace`, `encryption_key`, `color`, `log_level`, `log_to_file`, `chrome_path`, `lighthouse_path`, `openrouter_api_key`, `llm_base_url`, `llm_model`, `cache_backend`, `cache_redis_url`
- DEVE descobrir chaves e defaults com `config list-keys --json`
- DEVE definir encryption com `config set encryption_key <secret>`
- DEVE definir credenciais de extract LLM com `config set openrouter_api_key <key>` e DEVE setar `llm_base_url` / `llm_model` quando endpoint/modelo DEVEM diferir dos defaults
- DEVE definir default de log com `config set log_level <error|warn|info|debug|trace>`
- DEVE definir rotação de logs sob XDG state com `config set log_to_file true|false`
- DEVE definir cor com `config set color true|false` e path do Chrome com `config set chrome_path <path>`
- DEVE definir path do Lighthouse com `config set lighthouse_path <path>` (resolve flag → XDG → PATH; envelope reporta `binary_source`)
- DEVE definir cache com `config set cache_backend sqlite|memory|redis` e, se redis, `config set cache_redis_url redis://127.0.0.1:6379`
- DEVE usar somente URL plain `redis://` (NUNCA `rediss://` — fail-closed)
- DEVE resolver paths de config/data/cache/state via `config path --json`
### FORBIDDEN
- NUNCA invente env vars de produto para settings/encryption/chaves LLM/logging/cache
- NUNCA use `.env` como config de produto em runtime
- NUNCA logue valores de `encryption_key`, `openrouter_api_key` ou `cache_redis_url`
- NUNCA invente chaves fora das 16 suportadas
- NUNCA use `rediss://` (TLS) — o cliente embutido é plain TCP fail-closed
### Correct Pattern
```bash
browser-automation-cli config init --json
browser-automation-cli config path --json
browser-automation-cli config list-keys --json
browser-automation-cli config set timeout 90 --json
browser-automation-cli config set log_level info --json
browser-automation-cli config set log_to_file true --json
browser-automation-cli config set chrome_path /usr/bin/google-chrome --json
browser-automation-cli config set lighthouse_path /usr/bin/lighthouse --json
browser-automation-cli config set openrouter_api_key "replace-me" --json
browser-automation-cli config set cache_backend redis --json
browser-automation-cli config set cache_redis_url redis://127.0.0.1:6379 --json
browser-automation-cli config show --json
browser-automation-cli config get timeout --json
```

## Regras de Contrato
### REQUIRED
- DEVE usar `doctor` para saúde Chrome/XDG/lighthouse/`cache_redis`; `commands --json` para inventário; `schema --cmd` antes de inventar argv; `version --json` para fixar identidade
- DEVE usar `wait` multi `--text` como OR (qualquer match resolve); NUNCA como AND
- DEVE usar `grab --path <file>` (NUNCA path posicional bare); `type` com TEXT posicional mais `--target` OU `--focus-only`
- DEVE passar fill-form como comando `--json '[{"target":"@eN","value":"x"}]'` + `--json` global; upload exige target+path
- DEVE usar `click-at` só com `--experimental-vision`; `screencast` só com `--experimental-screencast`
- DEVE usar `console` só após `--capture-console` no mesmo processo; `net` só após `--capture-network` no mesmo processo
- DEVE compor `emulate` com `--user-agent`/`--viewport`/`--network-conditions` (NUNCA `--device`)
- DEVE gatear `heap` com `--category-memory`; `extension` com `--category-extensions`; `devtools3p` com `--category-third-party`; `webmcp` com `--category-webmcp`
- DEVE bindar MITM em `127.0.0.1` apenas; tratar CA/capturas como material sensível do host
- DEVE usar workflow só com manifests JSON; resume pula steps ok no journal sob XDG state
- DEVE tratar `exec` como single-step inline apenas (NÃO engine multi-passo)
- DEVE usar `run --script` com arquivo NDJSON (um objeto por linha) **ou** array JSON de objetos de passo
- DEVE em erros fail-fast de `run` inspecionar `data.steps` parcial no envelope de erro quando presente
- DEVE usar formatos de scrape `text|markdown|html|raw-html|links|metadata|screenshot|summary|product|branding` e engines `http|browser`; engine HTTP NÃO DEVE lançar Chrome
- DEVE tratar scrape `--webhook-url` como POST one-shot do operador (NÃO telemetria de produto)
- DEVE usar scrape `--only-main-content` quando o conteúdo principal for OBRIGATÓRIO
- DEVE usar `goto` com `--init-script`, `--handle-before-unload` e `--navigation-timeout-ms` quando a tarefa exigir
- DEVE usar `print-pdf --path` para artefatos PDF; DEVE passar `--url` quando navegação no mesmo one-shot for OBRIGATÓRIA
- DEVE usar `monitor check` com `--url` e `--baseline`
- DEVE usar `qr encode --text` / `qr decode --path`
- DEVE usar `find-paths` só em filesystem (sem Chrome); DEVE usar `--glob`, `--extension`, `--type`, `--limit`, `--max-depth`, `--hidden`, `--no-ignore` conforme a tarefa
- DEVE usar `sheet-write <input.csv|json> -o <out.xlsx>` (sem Chrome); opcional `--sheet`
- DEVE usar `sg-scan [paths…]` para lint estrutural; `sg-rewrite [paths…]` dry-run por padrão e `--apply` para gravar
- DEVE usar `parse` para html/md/txt/pdf/docx/xlsx/ods; passar `--redact-pii` quando mascarar PII for OBRIGATÓRIO
- DEVE setar XDG `openrouter_api_key` antes de `extract --llm`; DEVE usar `--question`; DEVE passar `--schema-json` quando schema estruturado for OBRIGATÓRIO
- DEVE esperar que `attr` faça fallback para propriedades DOM quando o atributo HTML for null
- DEVE usar `scroll --delta-y`/`--delta-x` (NDJSON DEVE usar `dy` ou `delta_y`); `assert url … --contains` (NDJSON DEVE usar `url_contains` quando contains for OBRIGATÓRIO)
- DEVE esperar envelope `lighthouse` com `binary_source` `real|mock` (resolve flag `--lighthouse-path` → XDG → PATH)
- DEVE copiar argv completo de `references/formulas.md` ao montar one-shots
### FORBIDDEN
- NUNCA invente aliases `snapshot`, `click`, `fill`, `screenshot` ou `bac`
- NUNCA espere estado de página ou `@eN` sobreviver FINALIZE DIE em novo processo
- NUNCA invente SaaS cloud de scrape ou servidores remotos sticky de workflow
- NUNCA substitua multi-passo browser `run --script` com `@eN` por workflow
- NUNCA use `rediss://` para cache Redis
### Correct Pattern
```bash
browser-automation-cli schema --cmd goto --json
browser-automation-cli --json grab --path /tmp/page.png --full-page
browser-automation-cli --json assert url https://example.com --contains
browser-automation-cli --json find-paths --glob '**/*.rs' .
```

## Inventário
### REQUIRED
- DEVE tratar esta superfície exata de 59 nomes como inventário OBRIGATÓRIO
- DEVE carregar ao menos uma linha executável por nome em `references/formulas.md`

doctor commands schema version goto view press click-at write keys type wait hover drag fill-form upload back forward reload eval grab print-pdf monitor run exec extract text scroll cookie attr assert console net page dialog scrape batch-scrape crawl map search parse qr find-paths sg-scan sg-rewrite sheet-write mitm workflow config emulate resize perf lighthouse screencast heap extension devtools3p webmcp completions

### FORBIDDEN
- NUNCA invente nomes de alias fora deste inventário
- NUNCA omita comandos só-PRD quando forem a tool correta
### Correct Pattern
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd print-pdf --json
browser-automation-cli schema --cmd sheet-write --json
```

## Playbooks de Ação
### REQUIRED
- DEVE executar estas fórmulas as-is salvo quando `schema --cmd` forçar mudança de flag
- DEVE manter multi-passo `@eN` dentro de um único `run --script`
- DEVE validar envelope `ok` após cada invocação
- DEVE usar `references/formulas.md` para a superfície restante
### FORBIDDEN
- NUNCA invente `bac`, env vars de produto, path bare em `grab`, `emulate --device`, `rediss://` ou manifest workflow não-JSON

#### A. Goto com init-script, beforeunload, navigation timeout
```bash
browser-automation-cli --timeout 60 --json goto https://example.com \
  --init-script 'window.__ready=true' \
  --handle-before-unload \
  --navigation-timeout-ms 15000
```

#### B. HTTP scrape main content + webhook
```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http --only-main-content
browser-automation-cli --json scrape https://example.com --format text --engine http --webhook-url https://127.0.0.1:9000/hook
```

#### C. Browser scrape multi-formato
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --timeout 60 --json scrape https://example.com --format links --engine browser
browser-automation-cli --timeout 60 --json scrape https://example.com --format screenshot --engine browser
```

#### D. Extract LLM com schema-json
```bash
browser-automation-cli config set openrouter_api_key "replace-me" --json
browser-automation-cli config set llm_base_url "https://openrouter.ai/api/v1" --json
browser-automation-cli config set llm_model "openai/gpt-4o-mini" --json
printf '%s\n' '{"type":"object","properties":{"title":{"type":"string"}},"required":["title"]}' > /tmp/extract.schema.json
browser-automation-cli --timeout 120 --json extract --llm --question "Qual é o título principal?" --schema-json /tmp/extract.schema.json https://example.com
```

#### E. Form fill via run NDJSON
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

#### F. Run array JSON multi-passo
```bash
cat > /tmp/demo.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","ms":300},
  {"cmd":"view"},
  {"cmd":"grab","path":"/tmp/array-run.png"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/demo.array.json
```

#### G. Run fail-fast inspeciona data.steps
```bash
cat > /tmp/failfast.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":200}
{"cmd":"view"}
{"cmd":"assert","kind":"url","url_contains":"this-must-not-match.invalid"}
{"cmd":"grab","path":"/tmp/never.png"}
JSONL
set +e
out=$(browser-automation-cli -q --timeout 60 --json run --script /tmp/failfast.browser-automation.jsonl 2>/dev/null)
code=$?
set -e
echo "$out" | jaq -e '.ok == false'
echo "$out" | jaq -e '(.data.steps | type) == "array"'
echo "$out" | jaq -r '.data.steps | length'
echo "exit=$code"
```

#### H. print-pdf
```bash
browser-automation-cli --timeout 60 --json print-pdf --path /tmp/page.pdf --url https://example.com
```

#### I. monitor check
```bash
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/example.baseline --write-baseline --engine http
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/example.baseline --engine http
```

#### J. QR encode/decode
```bash
browser-automation-cli --json qr encode --text "https://example.com" --format png --path /tmp/qr.png
browser-automation-cli --json qr decode --path /tmp/qr.png
```

#### K. find-paths com --glob e flags completas
```bash
browser-automation-cli --json find-paths --glob '**/*.rs' .
browser-automation-cli --json find-paths '\.rs$' . --hidden --no-ignore --max-depth 4 --extension rs --type f --limit 100
```

#### L. sheet-write CSV/JSON → XLSX
```bash
printf 'name,score\nalice,10\nbob,9\n' > /tmp/rows.csv
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data
printf '%s\n' '[{"name":"alice","score":10},{"name":"bob","score":9}]' > /tmp/rows.json
browser-automation-cli --json sheet-write /tmp/rows.json --out /tmp/out2.xlsx
```

#### M. sg-scan e sg-rewrite
```bash
browser-automation-cli --json sg-scan . --limit 100
browser-automation-cli --json sg-rewrite .
browser-automation-cli --json sg-rewrite . --apply
```

#### N. parse PDF/DOCX/xlsx/ods
```bash
browser-automation-cli --json parse /tmp/doc.pdf
browser-automation-cli --json parse /tmp/doc.docx --redact-pii
browser-automation-cli --json parse /tmp/sheet.xlsx
browser-automation-cli --json parse /tmp/sheet.ods --redact-pii
```

#### O. Config XDG 16 chaves + list-keys + Redis plain
```bash
browser-automation-cli config init --json
browser-automation-cli config path --json
browser-automation-cli config list-keys --json
browser-automation-cli config set lang pt-BR --json
browser-automation-cli config set timeout 90 --json
browser-automation-cli config set artifacts_dir /tmp/browser-automation-cli-artifacts --json
browser-automation-cli config set ignore_robots false --json
browser-automation-cli config set namespace demo --json
browser-automation-cli config set encryption_key "replace-me" --json
browser-automation-cli config set color true --json
browser-automation-cli config set log_level info --json
browser-automation-cli config set log_to_file true --json
browser-automation-cli config set chrome_path /usr/bin/google-chrome --json
browser-automation-cli config set lighthouse_path /usr/bin/lighthouse --json
browser-automation-cli config set openrouter_api_key "replace-me" --json
browser-automation-cli config set llm_base_url "https://openrouter.ai/api/v1" --json
browser-automation-cli config set llm_model "openai/gpt-4o-mini" --json
browser-automation-cli config set cache_backend redis --json
browser-automation-cli config set cache_redis_url redis://127.0.0.1:6379 --json
browser-automation-cli config show --json
browser-automation-cli doctor --offline --quick --json
```

#### P. Lighthouse binary_source
```bash
browser-automation-cli --timeout 60 --json lighthouse https://example.com \
  --lighthouse-path ./scripts/mock-lighthouse.sh \
  | jaq '.data.binary_source // .binary_source // .'
browser-automation-cli config set lighthouse_path /usr/bin/lighthouse --json
browser-automation-cli --timeout 90 --json lighthouse https://example.com
```

#### Q. scroll dy + attr property fallback + i18n lang
```bash
browser-automation-cli --json scroll --delta-y 400
browser-automation-cli --json attr @e1 href
browser-automation-cli --json attr @e1 value
browser-automation-cli config set lang pt-BR --json
browser-automation-cli --lang pt-BR --json version
```

#### R. MITM 127.0.0.1
```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
```

#### S. Network capture list (mesmo processo)
```bash
cat > /tmp/net.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"net","action":"list","resource_types":"Document,XHR"}
JSONL
browser-automation-cli --capture-network --timeout 60 --json run --script /tmp/net.browser-automation.jsonl
```

#### T. batch-scrape / crawl / map / search
```bash
printf '%s\n' https://example.com https://example.org > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
browser-automation-cli --json search "example domain" --limit 10
```

## Scripts Multi-passo NDJSON + array JSON
### REQUIRED
- DEVE usar `run --script <path>` para passos multi-passo em um processo
- DEVE aceitar NDJSON (um objeto JSON por linha com campo `cmd`) **ou** array JSON de objetos de passo
- DEVE manter estado de página e refs `@eN` dentro desse único processo
- DEVE definir `--timeout` grande o bastante para o script inteiro
- DEVE serializar grab como `{"cmd":"grab","path":"/tmp/example.png"}`
- DEVE serializar scroll dy como `{"cmd":"scroll","dy":400}` ou `"delta_y":400`
- DEVE serializar assert de URL com contains como `{"cmd":"assert","kind":"url","url_contains":"example.com"}`
- DEVE em erros fail-fast de `run` inspecionar `data.steps` parcial no envelope de erro quando presente
### FORBIDDEN
- NUNCA divida passos dependentes de ref entre processos
- NUNCA trate `exec` como engine multi-passo
- NUNCA espere `@eN` sobreviver ao DIE do processo
### Correct Pattern
```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com","init_script":"window.__x=1","handle_before_unload":true,"navigation_timeout_ms":15000}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"scroll","dy":400}
{"cmd":"assert","kind":"url","url_contains":"example.com"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.jsonl

cat > /tmp/demo.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"view"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/demo.array.json
```

## Manifest Workflow
### REQUIRED
- DEVE usar `workflow run --manifest <path>` com path JSON
- DEVE usar `workflow resume --manifest <path>`; `workflow status`; passar `--journal` quando path não-default for OBRIGATÓRIO
### FORBIDDEN
- NUNCA use manifests workflow não-JSON
### Correct Pattern
```bash
cat > /tmp/wf.json <<'JSON'
{"name":"demo","steps":[{"id":"ping","cmd":"echo","args":{"message":"start"}}]}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
```

## Envelope JSON
### REQUIRED
- DEVE esperar sucesso: `{"schema_version":1,"ok":true,"data":...}`
- DEVE esperar erro sob `--json`: `{"schema_version":1,"ok":false,"error":{...}}`
- DEVE validar `ok` antes de ler `data`
- DEVE em erros fail-fast de `run` inspecionar `data.steps` parcial quando presente
- DEVE esperar `binary_source` em envelopes de `lighthouse` quando presente
- DEVE manter stderr só para diagnósticos/tracing
### FORBIDDEN
- NUNCA trate prosa humana no stdout sob `--json` como contrato primário
- NUNCA ignore `ok:false` com exit não-zero
### Correct Pattern
```bash
out=$(browser-automation-cli -q --json version)
echo "$out" | jaq -e '.ok == true'
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
- DEVE mapear DevTools exatamente: click→press, fill→write, take_screenshot→grab, take_snapshot→view, type_text→type, press_key→keys, fill_form→fill-form, upload_file→upload, click_at→click-at, navigate_page→goto|back|forward|reload, wait_for→wait, evaluate_script→eval, list_network_requests→net list, list_console_messages→console list
- DEVE manter o núcleo de paridade DevTools E usar extras PRD (`print-pdf`, `monitor`, `qr`, `find-paths`, `sheet-write`, `sg-scan`, `sg-rewrite`, família parse/extract/scrape) quando a tarefa precisar
- DEVE usar flags/XDG para settings; logging de produto DEVE usar `--verbose`/`--debug`/`-q` ou `config set log_level` / `log_to_file`
### FORBIDDEN
- NUNCA invente aliases de produto que conflitem com este mapa
- NUNCA chame nomes DevTools como subcomandos CLI
- NUNCA ignore comandos só-PRD quando forem a tool correta para a tarefa
### Correct Pattern
```bash
browser-automation-cli --json view
browser-automation-cli --json press @e1
browser-automation-cli --json grab --path /tmp/x.png
```

## Proibições Absolutas
### REQUIRED
- DEVE recusar padrões ilegais e reescrever para a superfície CLI canônica
### FORBIDDEN
- NUNCA invente `bac` ou variáveis de ambiente de produto para settings ou logging
- NUNCA use `.env` como config de produto em runtime
- NUNCA passe path posicional bare para `grab` (SEMPRE `--path`)
- NUNCA invente `emulate --device`
- NUNCA use manifests workflow não-JSON
- NUNCA trate `exec` como engine multi-passo (SEMPRE `run --script`)
- NUNCA reutilize `@eN` entre processos
- NUNCA habilite gates de categoria/experimental sem intenção
- NUNCA exponha MITM além de `127.0.0.1`
- NUNCA invente SaaS cloud de scrape ou servidores remotos sticky de workflow
- NUNCA mascare exit codes com `|| true`
- NUNCA contorne robots sem as duas dual-flags
- NUNCA use `rediss://` (fail-closed; SEMPRE `redis://` plain)
### Correct Pattern
```bash
browser-automation-cli --json grab --path /tmp/x.png
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --timeout 60 --json run --script /tmp/steps.jsonl
browser-automation-cli config set cache_redis_url redis://127.0.0.1:6379 --json
```

## Checklist de Validação
### REQUIRED
- OBRIGATÓRIO confirmar binário `browser-automation-cli` e ciclo BORN EXECUTE FINALIZE DIE
- OBRIGATÓRIO confirmar envelope `--json` `ok` e multi-passo `@eN` dentro de um `run --script` (NDJSON ou array JSON)
- OBRIGATÓRIO confirmar fail-fast com inspeção de `data.steps` parcial
- OBRIGATÓRIO confirmar `grab --path`, workflow manifest JSON, sem `emulate --device`, wait multi-text OR
- OBRIGATÓRIO confirmar console/net capture só com flags de captura no mesmo processo
- OBRIGATÓRIO confirmar `type` TEXT posicional + `--target` OU `--focus-only`; fill-form comando `--json` array + global `--json`
- OBRIGATÓRIO confirmar todas as 16 chaves de config + `config list-keys`; NUNCA invente env de produto
- OBRIGATÓRIO confirmar logging via `--verbose`/`--debug`/`-q`/`log_level`/`log_to_file` apenas
- OBRIGATÓRIO confirmar Redis plain `redis://` e fail-closed de `rediss://`; `cache_backend` + `cache_redis_url`
- OBRIGATÓRIO confirmar lighthouse `binary_source` (flag → XDG → PATH)
- OBRIGATÓRIO confirmar `find-paths --glob`, `sheet-write`, `sg-scan`, `sg-rewrite`
- OBRIGATÓRIO confirmar exit codes 0,2,65,66,69,70,74,78,124,130,141; robots dual-flag; gates categoria/experimental; schema discovery
- OBRIGATÓRIO confirmar inventário completo de 59 comandos e fórmulas em `references/formulas.md`
- OBRIGATÓRIO confirmar playbooks de goto flags, print-pdf, QR, find-paths, monitor, parse, extract LLM, scrape main-content/webhook, run NDJSON|array JSON, fail-fast `data.steps`, scroll dy, attr fallback, i18n lang, sheet-write, sg-scan/sg-rewrite, Redis, lighthouse
### FORBIDDEN
- NUNCA entregue glue de agente que viole este checklist
### Correct Pattern
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd run --json
browser-automation-cli schema --cmd sheet-write --json
browser-automation-cli config list-keys --json
browser-automation-cli doctor --offline --quick --json
```
