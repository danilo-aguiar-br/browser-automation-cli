---
name: browser-automation-cli
description: Esta skill DEVE ser usada quando a tarefa exigir operar a CLI browser-automation-cli para automação Chrome via CDP, scraping local e diagnóstico de páginas. DEVE ativar em navegar, clicar, digitar, submit de formulário, fill-form, storage export/import, snapshot de acessibilidade com refs @eN, screenshot png jpeg webp, PDF, extract com LLM, scrape multi-formato, batch-scrape, crawl, map, parse de PDF DOCX XLSX ODS, monitor, QR, sheet-write, sg-scan, sg-rewrite, find-paths, console e rede, MITM em loopback, emulate, perf, lighthouse com binary_source, screencast, heap, extension, webmcp, workflow JSON, scripts multi-passo run com wait_timeout_ms e scrape format, diálogos com dialog_settled e multi-aba. Entrega fórmulas de argv, leis de sintaxe, envelope JSON, exit codes, config XDG sem variáveis de ambiente de produto, robots e residual-zero em disco. Entrega comandos executáveis corretos na primeira tentativa.
---

# browser-automation-cli

## Regra Zero
### REQUIRED
- DEVE usar SEMPRE o binário `browser-automation-cli` por extenso; NUNCA alias `bac`
- DEVE passar `--json` em TODA invocação programática; parsear SOMENTE stdout; silenciar stderr com `-q`
- DEVE checar exit code ANTES de confiar no stdout; validar `.ok == true` antes de `.data`; parsear com `jaq` e NUNCA `jq`
- DEVE consultar `references/formulas.md` para superfície argv exaustiva
### FORBIDDEN
- NUNCA invente variável de ambiente de produto; NUNCA use `.env` como runtime; NUNCA mascare exit com `|| true`; NUNCA parseie stderr como JSON

## Descoberta Obrigatória
### REQUIRED
- DEVE resolver superfície viva por descoberta — `browser-automation-cli --json commands`, `schema <cmd>`, `schema --cmd <cmd>`, `config list-keys`, `config path`
- DEVE rodar `<cmd> --help` quando schema não bastar; `doctor --offline --quick` quando host parecer errado
### FORBIDDEN
- NUNCA invente flag ausente do schema/help nem flags wishlist do PRD; NUNCA recuse chave de config por memória

## Identidade e Ciclo de Vida
### REQUIRED
- DEVE tratar cada processo como BORN → EXECUTE → FINALIZE → DIE; Chrome nasce e morre no mesmo processo
- DEVE manter multi-passo com refs `@eN` em UM `run --script`; `@eN` morre com o processo
- DEVE usar Chrome de sistema ou `config set chrome_path`
- DEVE mapear DevTools→produto — click→`press`, fill→`write`, take_screenshot→`grab`, take_snapshot→`view`, type_text→`type`, press_key→`keys`, navigate_page→`goto|back|forward|reload`, evaluate_script→`eval`, list_network_requests→`net list`, list_console_messages→`console list`
- DEVE tratar `exec` como passo único; multi-passo DEVE usar `run --script`
### FORBIDDEN
- NUNCA reutilize `@eN` entre processos; NUNCA assuma daemon/sessão sticky/remota/telemetria; NUNCA chame nomes DevTools como subcomando

## Flags Globais
### REQUIRED
- DEVE aceitar flags globais antes ou depois do subcomando
- DEVE passar `--json`; `--json-steps` em `run`; `--timeout`; `--step-timeout` em `run`; `--max-concurrency`; `--artifacts-dir`; `--correlation-id`; `--plain`
- DEVE passar `--capture-console` no MESMO processo de `console`/assert console; `--capture-network` no MESMO processo de `net`
- DEVE passar `--headed` somente para debug interativo; `--lang en` ou `--lang pt-BR`
- DEVE elevar tracing com `--verbose` ou `--debug` ou `config set log_level` — NUNCA env
- DEVE passar gates só quando a família exigir — `--category-memory` (`heap`), `--category-extensions` (`extension`), `--category-third-party` (`devtools3p`), `--category-webmcp` (`webmcp`), `--experimental-vision` (`click-at`), `--experimental-screencast` (`screencast`)
- DEVE passar `--mitm` e combinar com `--mitm-har|--mitm-hosts|--mitm-ca-dir|--mitm-ws|--mitm-max-body-bytes|--mitm-no-media-bodies|--mitm-redact-secrets` somente quando a intercepção exigir
### FORBIDDEN
- NUNCA espere captura sobreviver ao DIE; NUNCA ligue gate por padrão; NUNCA omita `--json` em pipeline de agente

## Config XDG
### REQUIRED
- DEVE configurar SOMENTE por flags CLI e `config init|path|show|get|set|list-keys`
- DEVE descobrir chaves com `config list-keys --json`; resolver caminhos com `config path --json` — NUNCA inventar paths XDG
- DEVE tratar flag CLI como override; setar segredos com `config set encryption_key` e `openrouter_api_key`
- DEVE setar binários com `chrome_path`, `lighthouse_path`, `ffmpeg_path`; cache com `cache_backend sqlite|memory|redis` e Redis plain em `cache_redis_url`
- DEVE setar `dialog_settle_ms` e `log_level` via config set
### FORBIDDEN
- NUNCA invente env de produto; NUNCA logue segredos/cookies; NUNCA use `rediss://`; NUNCA configure redis sem `cache_redis_url`

## Contrato Argv e Superfície
### REQUIRED
- DEVE passar `grab --path` (nunca posicional); `grab --format png|jpeg|webp`; `--quality`/`--element` somente quando necessário
- DEVE passar `print-pdf --path` SEMPRE; `print-pdf --url` em one-shot (recusa blank)
- DEVE passar `view --detailed` para árvore completa; `view --allow-empty` somente se blank for intencional
- DEVE passar `type <TEXTO>` com `--target` OU `--focus-only`; `fill-form --fields-json`; `cookie set --cookies-json`
- DEVE passar `submit <ALVO>` (form ou campo dono); `submit --timeout-ms` e `--include-snapshot` somente quando necessário
- DEVE passar `storage export|import --path` obrigatório; `--url` no mesmo processo quando origem autenticada for necessária
- DEVE passar `mitm block --host`; `mitm allow --host` (host obrigatório); `mitm ws list|get`
- DEVE passar `reload --ignore-cache` (NUNCA em `goto`); `goto --handle-before-unload accept|dismiss`
- DEVE passar `sheet-write <in> -o <out.xlsx>`; `emulate` por flags (NUNCA `--device`); `assert url <v> --contains`
- DEVE passar `workflow run --manifest`; `--journal` somente quando necessário; `eval --file-path`; `--service-worker-id` para SW
- DEVE descobrir `pick`/`select-option` via schema e invocar na superfície reportada; usar `locale`/`man` sem Chrome
### FORBIDDEN
- NUNCA use avif em grab; NUNCA embuta mitm/storage/extension install|uninstall em `run`
- NUNCA use `fill-form --json` nem `cookie set --json` como payload; NUNCA use `view --verbose` (correto `--detailed`)

## Envelope JSON e Exit Codes
### REQUIRED
- DEVE esperar sucesso `schema_version`+`ok` true+`data`; falha `ok` false+`error`; usage → `error.kind=usage` exit 2
- DEVE ler `data.steps` parciais em falha de `run`; `data.binary_source` real|mock em lighthouse; `.data.dialog_settled` após dialog; `matched_selector` em wait multi-seletor
- DEVE ramificar exit `0` ok, `2` usage, `65` data, `66` no-input, `69` unavailable, `70` software, `74` io, `78` config, `124` timeout, `130` cancel, `141` broken-pipe
- DEVE retentar somente falha transitória de host/launch
### FORBIDDEN
- NUNCA retente usage sem corrigir argv; NUNCA trate prosa stdout como contrato; NUNCA ignore `ok` falso; NUNCA trate mock lighthouse como validação de parser LHR

## Scripts Multi-passo run
### REQUIRED
- DEVE usar `run --script` (NDJSON ou array JSON); cada passo com `cmd`; `--timeout` cobre o script inteiro
- DEVE serializar grab/print-pdf com `path`; print-pdf com `url` ou `goto` prévio
- DEVE serializar scroll com `delta_y`/`dy` e `delta_x`/`dx`; wait com `selector` CSV ou `selectors` array (OR) e `wait_timeout_ms`
- DEVE serializar wait pós-nav com `url`/`url_contains`/`navigation`; scrape com `url`+`format|formats`
- DEVE serializar submit com `target` (+`timeout_ms` se diferir); dialog com `if_present` se puder faltar
- DEVE serializar view blank com `allow_empty`; view detalhado em run com `verbose` ou `detailed`; aba isolada com `isolated_context`
- DEVE serializar assert com `kind` em `url|text|console|console_empty|console_no_match`
- DEVE manter fora de `run` — meta, config, mitm, storage, workflow, crawl, map, batch-scrape, search, parse, qr, find-paths, sg-scan, sg-rewrite, sheet-write, monitor, extension install|uninstall
### FORBIDDEN
- NUNCA divida passos `@eN` entre processos; NUNCA ignore `data.steps` parciais; NUNCA use `exec` como multi-passo

## Leis Agent-First
### REQUIRED
- DEVE ler `.data.dialog_settled` após `dialog accept|dismiss` real; quando true, NÃO inserir wait artificial
- DEVE tratar diálogos multi-aba por `session_id`; troca de aba sob diálogo = domain enable best-effort
- DEVE setar settle com `config set dialog_settle_ms` (XDG), nunca env
- DEVE usar `wait_timeout_ms` em wait de run; scrape de run com `format|formats` (text NÃO despeja html monstro)
- DEVE usar grab só png|jpeg|webp; lighthouse `binary_source` real|mock (mock ≠ validação parser)
- DEVE esperar `select-option`/`pick` nativo despachar input→change e reportar `via: native_select`
- DEVE usar `submit` para form e esperar nav/request; storage exige `--path`, export mode 0600, FORA de run
- DEVE descobrir superfície com `commands --json` e `schema`; NUNCA invente flags ausentes
### FORBIDDEN
- NUNCA wait artificial com `dialog_settled` true; NUNCA avif; NUNCA storage/mitm/extension install|uninstall em run

## Residual-Zero e Robots
### REQUIRED
- DEVE tratar residual-zero como parte do sucesso de one-shot de browser
- DEVE validar com `doctor --offline --quick --json` — zeros em `residual.cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `live_cli_marker_processes`; check `residual_disk` pass
- DEVE respeitar robots por padrão; contornar SOMENTE com ambas `--ignore-robots` e `--i-accept-robots-risk`
### FORBIDDEN
- NUNCA declare residual-zero sem ler `data.residual`; NUNCA apague temporários genéricos; NUNCA mate Chrome/perfil alheio; NUNCA contorne robots com uma flag só

## Inventário Completo de Comandos
### REQUIRED
- DEVE conhecer estes 65 — doctor, commands, schema, version, locale, goto, view, press, click-at, write, keys, type, wait, hover, drag, submit, fill-form, select-option, pick, upload, back, forward, reload, eval, grab, print-pdf, monitor, run, exec, extract, text, scroll, cookie, storage, attr, assert, console, net, page, dialog, scrape, batch-scrape, crawl, map, search, parse, qr, find-paths, sg-scan, sg-rewrite, sheet-write, mitm, workflow, config, emulate, resize, perf, lighthouse, screencast, heap, extension, devtools3p, webmcp, completions, man
- DEVE confirmar inventário vivo com `commands --json`

## Playbooks de Execução
### REQUIRED
- DEVE executar fórmulas literalmente; validar envelope após cada invocação; consultar `references/formulas.md`
- DEVE executar `browser-automation-cli --json doctor --offline --quick`; `commands`; `schema <cmd>`; `version`; `locale`; `completions bash`; `man --out /tmp/browser-automation-cli.1`
- DEVE executar `browser-automation-cli --timeout 60 --json goto https://example.com --handle-before-unload accept --navigation-timeout-ms 15000`; `view --detailed`; `back`; `forward`; `reload --ignore-cache`
- DEVE executar `press @e1 --include-snapshot`; `--experimental-vision click-at --x 10 --y 20`; `write @e2 "texto"`; `type "olá" --target @e2 --clear --submit Enter`; `keys Enter`; `hover @e1`; `drag --from @e1 --to @e2`; `upload @e4 /tmp/a.txt`
- DEVE executar `submit "#user" --timeout-ms 8000`; `fill-form --fields-json '[{"target":"@e3","value":"x"}]'`; `exec pick --target @e1 --option Anomalia`; `exec select-option --target @e2 --option Alta`
- DEVE executar `wait --selector "h1, main, #content" --wait-timeout-ms 10000`; `scroll --delta-y 400`; `text @e1`; `attr @e1 href`; `eval 'document.title' --file-path /tmp/eval.json`
- DEVE executar `grab --path /tmp/p.png --format webp --quality 80 --full-page`; `--timeout 60 print-pdf --path /tmp/p.pdf --url https://example.com`
- DEVE executar `--timeout 90 --json --json-steps run --script /tmp/steps.jsonl`; `exec goto https://example.com`
- DEVE serializar `{"cmd":"wait","selector":"h1","wait_timeout_ms":10000}`, `{"cmd":"scrape","url":"https://example.com","format":"text"}`, `{"cmd":"submit","target":"#user","timeout_ms":8000}`, `{"cmd":"dialog","action":"accept","if_present":true}`, `{"cmd":"grab","path":"/tmp/p.png","format":"png"}`, `{"cmd":"print-pdf","path":"/tmp/p.pdf"}`, `{"cmd":"pick","target":"@e1","option":"Anomalia"}`, `{"cmd":"select-option","target":"@e2","option":"Alta"}`
- DEVE executar `scrape https://example.com --format markdown,links,metadata --engine http --only-main-content`; `batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2`; `crawl https://example.com --limit 20 --max-depth 2 --format text`; `map https://example.com --limit 50`; `search "example domain" --limit 10`
- DEVE executar `parse /tmp/doc.pdf`; `parse /tmp/planilha.ods --redact-pii`; `extract --llm --question "título?" --schema-json /tmp/s.json https://example.com`
- DEVE executar `--capture-console console list`; `--capture-console assert console-empty`; `--capture-network net list`; `net get 0`
- DEVE executar `page new --isolated-context s-a --url https://example.com`; `page list`; `page select 0 --bring-to-front`; `cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'`; `cookie list`
- DEVE executar `storage export --path /tmp/auth.json --url https://example.com`; `storage import --path /tmp/auth.json --url https://example.com`; `dialog accept --if-present` e ler `.data.dialog_settled`; `assert url example.com --contains`
- DEVE executar `mitm init-ca`; `mitm capture-url https://example.com --har /tmp/c.har`; `mitm block --host example.com --path /ads`; `mitm allow --host example.com`; `mitm ws list --limit 50`
- DEVE executar `emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G"`; `resize --width 1280 --height 720`; `perf start`; `perf stop --path /tmp/trace.json`
- DEVE executar `--timeout 180 lighthouse https://example.com --out-dir /tmp/lh --device desktop` e ler `data.binary_source`; `--experimental-screencast screencast start --path /tmp/cast`; `screencast stop --path /tmp/cast.webm`
- DEVE executar `--category-memory heap take --path /tmp/s.heapsnapshot`; `heap summary --path /tmp/s.heapsnapshot`; `heap retainers --path /tmp/s.heapsnapshot --node 42`
- DEVE executar `--category-extensions extension list`; `extension install /tmp/ext`; `extension reload <id>`; `extension uninstall <id>`; `--category-third-party devtools3p list`; `--category-webmcp webmcp list`
- DEVE executar `monitor check --url https://example.com --baseline /tmp/b.baseline --write-baseline --engine http`; `qr encode --text "https://example.com" --format png --path /tmp/qr.png`; `qr decode --path /tmp/qr.png`
- DEVE executar `find-paths --glob '**/*.rs' .`; `sg-scan . --limit 100`; `sg-rewrite .`; `sg-rewrite . --apply`; `sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data`
- DEVE executar `workflow run --manifest /tmp/wf.json --journal /tmp/wf.journal`; `workflow resume --manifest /tmp/wf.json`; `workflow status --name demo`
- DEVE executar `config init`; `config path`; `config show`; `config get timeout`; `config set dialog_settle_ms 2000`; `config list-keys`
- DEVE contornar robots somente com `--ignore-robots --i-accept-robots-risk --json scrape https://example.com --format text --engine http`
### FORBIDDEN
- NUNCA adapte fórmula sem `schema <cmd> --json`

## Proibições Absolutas
### FORBIDDEN
- NUNCA invente alias `bac` nem env de produto nem `export` de chaves de produto
- NUNCA use `jq` no lugar de `jaq`; NUNCA confie em stdout sem exit+`.ok`
- NUNCA reutilize `@eN` entre processos; NUNCA embuta mitm/storage/extension install|uninstall em `run`
- NUNCA use avif; NUNCA trate lighthouse mock como parser válido; NUNCA contorne robots com uma flag
- NUNCA trate `exec` como multi-passo; NUNCA use `view --verbose` nem `fill-form --json`/`cookie set --json` como payload
- NUNCA declare residual-zero sem doctor; NUNCA invente flags ausentes do schema vivo
