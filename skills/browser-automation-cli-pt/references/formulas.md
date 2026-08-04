# browser-automation-cli — Fórmulas de Argv

## Uso
- DEVE copiar fórmulas literalmente; trocar só placeholders; binário `browser-automation-cli` por extenso; `--json` SEMPRE
- DEVE parsear só stdout; checar exit antes de confiar; validar `.ok` com `jaq`; descobrir com `commands --json`, `schema <cmd> --json`, `config list-keys --json`
- NUNCA invente alias, env de produto ou flag ausente

## Contrato
- Após `dialog accept|dismiss` real leia `.data.dialog_settled`; se true NÃO wait artificial; multi-aba por `session_id`; settle via `config set dialog_settle_ms`
- run wait usa `wait_timeout_ms`; run scrape usa `format|formats` (text sem html monstro); grab só png|jpeg|webp; lighthouse `binary_source` real|mock
- select nativo → input+change e `via: native_select`; submit espera nav/request; storage `--path` obrigatório mode 0600 FORA de run
- mitm/storage/extension install|uninstall FORA de run; `exec` = passo único; multi-passo = `run --script`

## Globais
- DEVE executar `browser-automation-cli --json --json-steps --timeout 90 --step-timeout 20 --capture-console --capture-network run --script /tmp/steps.jsonl`
- DEVE executar `browser-automation-cli --json -q --plain --max-concurrency 4 --artifacts-dir /tmp/arts --correlation-id req-42 goto https://example.com`
- DEVE passar `--verbose|--debug` ou `config set log_level`; `--headed` só debug; `--lang en|pt-BR`
- DEVE passar `--category-memory` (heap), `--category-extensions` (extension), `--category-third-party` (devtools3p), `--category-webmcp` (webmcp), `--experimental-vision` (click-at), `--experimental-screencast` (screencast)
- DEVE passar `--mitm` + `--mitm-har|--mitm-hosts|--mitm-ca-dir|--mitm-ws|--mitm-max-body-bytes|--mitm-no-media-bodies|--mitm-redact-secrets` só quando intercepção exigir
- DEVE contornar robots só com ambas `--ignore-robots --i-accept-robots-risk`

## Meta
- DEVE executar `browser-automation-cli --json doctor --offline --quick` e `doctor --fix` só se reparo for necessário
- DEVE executar `browser-automation-cli --json commands`; `schema goto`; `schema --cmd wait`; `version`; `locale`
- DEVE executar `browser-automation-cli completions bash` (zsh|fish|elvish|powershell); `man --out /tmp/browser-automation-cli.1`

## Config XDG
- DEVE executar `browser-automation-cli --json config init|path|show|list-keys`; `config get`; `config get timeout`; `config set <k> <v>`
- DEVE descobrir a superfície viva de chaves com `config list-keys --json` antes de qualquer `config set`
- DEVE consultar `references/xdg-keys.md` para as 176 chaves com padrão e descrição
- DEVE setar binários com `chrome_path`, `lighthouse_path`, `ffmpeg_path`
- DEVE setar segredos com `encryption_key` e `openrouter_api_key`
- DEVE setar cache com `cache_backend sqlite|memory|redis` e Redis plain em `cache_redis_url`
- DEVE setar comportamento com `dialog_settle_ms`, `log_level`, `lang en|pt-BR`, `timeout`, `artifacts_dir`, `http_ssrf_mode strict|allow_loopback|off`, `log_rotation daily|hourly|never`
- NUNCA reintroduza lista inline de chaves nesta skill; a lista inline envelhece e trunca a superfície viva
- NUNCA `rediss://`; NUNCA logue segredos; NUNCA redis sem `cache_redis_url`

## Navegação / espera / snapshot / interação
- DEVE executar `browser-automation-cli --timeout 60 --json goto https://example.com --init-script 'window.__x=1' --handle-before-unload accept --navigation-timeout-ms 15000`
- DEVE executar `browser-automation-cli --json back`; `forward`; `reload --ignore-cache`
- DEVE executar `browser-automation-cli --json wait --ms 500`; `wait --selector "h1, main, #content" --wait-timeout-ms 10000 --include-snapshot`; `wait --text Example --wait-timeout-ms 5000`; `wait --state networkidle --wait-timeout-ms 15000`
- DEVE executar `browser-automation-cli --json view --detailed`; `view --path /tmp/view.txt --allow-empty` só se blank intencional
- DEVE executar `browser-automation-cli --json press @e1 --dblclick --include-snapshot`; `--experimental-vision click-at --x 10 --y 20`
- DEVE executar `browser-automation-cli --json write @e2 "olá"`; `keys Enter`; `type "olá" --target @e2 --clear --submit Enter`; `hover @e1`; `drag --from @e1 --to @e2`
- DEVE executar `browser-automation-cli --json fill-form --fields-json '[{"target":"@e3","value":"x"}]'`; `upload @e4 /tmp/a.txt`; `submit "#user" --timeout-ms 8000 --include-snapshot`
- DEVE executar `browser-automation-cli --json exec pick --target @e1 --option Anomalia`; `exec select-option --target @e2 --option Alta`; `scroll --delta-y 400 --delta-x 100`
- NUNCA `--ignore-cache` em goto; NUNCA `view --verbose`; NUNCA `fill-form --json` payload

## Leitura / artefatos
- DEVE executar `browser-automation-cli --json extract @e1 --attr href`; `--timeout 120 extract --llm --question "título?" --schema-json /tmp/s.json https://example.com`
- DEVE executar `browser-automation-cli --json text @e1`; `attr @e1 href`; `eval 'document.title' --file-path /tmp/eval.json`; `eval '(el)=>el.textContent' --args '["@e1"]' --dialog-action accept`
- DEVE executar `browser-automation-cli --category-extensions --json eval 'chrome.runtime.id' --service-worker-id <sw-id>`
- DEVE executar `browser-automation-cli --json grab --path /tmp/p.png --format png --full-page`; `grab --path /tmp/p.webp --format webp --quality 80 --element @e1`
- DEVE executar `browser-automation-cli --timeout 60 --json print-pdf --path /tmp/p.pdf --url https://example.com`
- NUNCA grab avif; NUNCA omita `--path` em grab/print-pdf; NUNCA omita `--url` em print-pdf one-shot

## Abas / cookies / storage / dialog / assert
- DEVE executar `browser-automation-cli --json page list`; `page info`; `page new --isolated-context s-a --url https://example.com`; `page select 0 --bring-to-front`; `page close --index 1`; `page tab-id`
- DEVE executar `browser-automation-cli --json cookie list --url https://example.com`; `cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'`; `cookie clear`
- DEVE executar `browser-automation-cli --json storage export --path /tmp/auth.json --url https://example.com`; `storage import --path /tmp/auth.json --url https://example.com`
- DEVE executar `browser-automation-cli --json dialog accept --text Ana --if-present`; `dialog dismiss --if-present`
- DEVE executar `browser-automation-cli --json assert url example.com --contains`; `assert text "Example" --target h1`; `--capture-console assert console-empty`; `--capture-console assert console-no-match --pattern TypeError`

## Console / rede
- DEVE executar `browser-automation-cli --capture-console --json console list --types log,warning,error`; `console get 0`; `console clear`; `console dump --path /tmp/console.json`
- DEVE executar `browser-automation-cli --capture-network --json net list --resource-types Document,XHR,Fetch`; `net get 0 --request-path /tmp/req.json --response-path /tmp/res.json`
- DEVE capturar console/rede no MESMO processo dos comandos

## Scrape / coleta / locais
- DEVE executar `browser-automation-cli -q --json scrape https://example.com --format markdown --select source_url,title,markdown --max-text-chars 800 --only-main-content`
- DEVE executar multi-format `scrape … --format markdown,jsonld --select source_url,title,markdown` · `--redact-pii --with-content-hash` · browser `--engine browser --wait-ms 500`
- DEVE executar `browser-automation-cli -q --json scrape https://example.com --format rawHtml --engine http`
- Formatos — os 14 valores de `--format` são text, markdown, html, rawHtml, links, metadata, screenshot, summary, product, branding, images, jsonld, json, feed (aliases md meta body shot); aceita CSV ou repetição da flag
- Engines — `--engine` aceita SOMENTE `http` (reqwest mais scraper) e `browser` (CDP); o default vem da chave XDG `scrape_default_engine`, hoje `http`
- NUNCA trate `rawHtml` como alias de `html`; `--format html` devolve a chave `html` e `--format rawHtml` devolve a chave `rawHtml`, com payloads distintos
- DEVE executar `batch-scrape --urls-file /tmp/u.txt --filter http_error=false --output-mode csv --select source_url,text` · `crawl … --dedup-key source_url --output-mode ndjson` · `map … --search docs --sitemap-only`
- DEVE executar `browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text --same-host`; `map https://example.com --limit 50`; `search "example domain" --limit 10`
- DEVE executar `browser-automation-cli --json parse /tmp/doc.pdf`; `parse /tmp/planilha.ods --redact-pii`
- DEVE executar `browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/b.baseline --write-baseline --engine http`
- DEVE executar `browser-automation-cli --json qr encode --text https://example.com --format png --path /tmp/qr.png`; `qr decode --path /tmp/qr.png`
- DEVE executar `browser-automation-cli --json image info --path /tmp/a.png --select format,width,height,sha256`; `image convert --path /tmp/a.png --format webp -o /tmp/a.webp`; `image download https://example.com/a.png -o /tmp/a.png`; `image resize --path /tmp/a.png --width 640 --keep-aspect -o /tmp/a-640.webp --format webp --quality 80`
- DEVE executar `browser-automation-cli --json video info --path /tmp/in.mp4 --select format,bytes,path` (aliases → container/size_bytes); `video convert --path /tmp/in.mp4 --format webm -o /tmp/out.webm --select path_out,auto_reencoded,bytes_out`; `video to-mp3 --path /tmp/in.mp4 -o /tmp/a.mp3`; `video trim --path /tmp/in.mp4 --start 0 --duration 0.5 -o /tmp/clip.mp4`; `video thumbnail --path /tmp/in.mp4 --at 0 -o /tmp/thumb.png`; `--timeout 120 video download https://example.com/v.mp4 -o /tmp/v.mp4 --max-bytes 52428800 --require-video`
- DEVE setar caps video via XDG após list-keys: `video_max_input_bytes` `video_download_max_bytes` `video_default_container` `video_default_crf` `video_default_audio_bitrate` `ffmpeg_path` `ffmpeg_timeout_secs`
- DEVE executar `browser-automation-cli --json audio info --path /tmp/in.wav --select format,codec,duration,bytes,sha256` · `audio convert --path /tmp/in.wav --format mp3 -o /tmp/a.mp3` · `audio convert --path /tmp/clip.mp4 --format m4a -o /tmp/a.m4a` · `audio trim --path /tmp/a.mp3 --start 1 --duration 5 -o /tmp/cut.mp3` · `audio download https://example.com/a.mp3 -o /tmp/a.mp3` · depois `upload @e1 /tmp/a.mp3`
- DEVE setar caps audio via XDG após list-keys: `audio_max_input_bytes` `audio_download_max_bytes` `audio_default_format` `audio_default_bitrate` `ffmpeg_path` `ffmpeg_timeout_secs`
- NUNCA despeje bytes/base64 de mídia no stdout; só path→path; NUNCA alegue HLS/yt-dlp/encode pure-Rust como produto
- DEVE executar `browser-automation-cli --json find-paths --glob '**/*.rs' . --type f --limit 200`; `sg-scan . --limit 100`; `sg-rewrite .`; `sg-rewrite . --apply`; `sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data`
- DEVE contornar robots só com `browser-automation-cli --ignore-robots --i-accept-robots-risk --json scrape https://example.com --format text --engine http`

## Emulação / perf / lighthouse / screencast / heap
- DEVE executar `browser-automation-cli --json emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G" --color-scheme dark`
- DEVE executar `browser-automation-cli --json resize --width 1280 --height 720`; `perf start --path /tmp/trace.json --reload --auto-stop`; `perf stop --path /tmp/trace.json`; `perf insight --name DocumentLatency`
- DEVE executar `browser-automation-cli --timeout 180 --json lighthouse https://example.com --out-dir /tmp/lh --device desktop --mode navigation` e ler `data.binary_source`
- DEVE executar `browser-automation-cli --experimental-screencast --json screencast start --path /tmp/cast`; `screencast stop --path /tmp/cast.webm`
- DEVE executar `browser-automation-cli --category-memory --json heap take --path /tmp/s.heapsnapshot`; `heap close --path /tmp/s.heapsnapshot`; `heap summary --path /tmp/s.heapsnapshot`
- DEVE executar `browser-automation-cli --category-memory --json heap compare --base /tmp/a.heapsnapshot --current /tmp/b.heapsnapshot`; `heap details --path /tmp/s.heapsnapshot`; `heap class-nodes --path /tmp/s.heapsnapshot --id 7`
- DEVE executar `browser-automation-cli --category-memory --json heap dominators --path /tmp/s.heapsnapshot --node 42`; `heap dup-strings --path /tmp/s.heapsnapshot`; `heap edges --path /tmp/s.heapsnapshot --node 42`; `heap retainers --path /tmp/s.heapsnapshot --node 42`; `heap paths --path /tmp/s.heapsnapshot --node 42`; `heap object-details --path /tmp/s.heapsnapshot --node 42`
- NUNCA `emulate --device`; NUNCA `--node-id` (use `--node`)

## Extensões / terceiros / MITM / workflow
- DEVE executar `browser-automation-cli --category-extensions --json extension list`; `extension install /tmp/ext`; `extension reload <id>`; `extension trigger <id>`; `extension uninstall <id>`
- DEVE executar `browser-automation-cli --category-third-party --json devtools3p list --url https://example.com`; `devtools3p exec Tool --params '{}'`
- DEVE executar `browser-automation-cli --category-webmcp --json webmcp list --url https://example.com`; `webmcp exec Tool --input '{}'`
- DEVE executar `browser-automation-cli --json mitm init-ca`; `mitm start --seconds 30`; `mitm status`; `mitm list --limit 50`; `mitm get 0`; `mitm har --out /tmp/c.har`; `mitm export --format ndjson --out /tmp/c.ndjson`
- DEVE executar `browser-automation-cli --json mitm domains`; `mitm apis`; `mitm graphql --limit 100`; `mitm ws list --limit 50`; `mitm ws get 0`
- DEVE executar `browser-automation-cli --json mitm block --host example.com --path /ads`; `mitm allow --host example.com`; `mitm redact --secrets true`
- DEVE executar `browser-automation-cli --timeout 60 --json mitm capture-url https://example.com --har /tmp/c.har`
- DEVE executar `browser-automation-cli --json workflow run --manifest /tmp/wf.json --journal /tmp/wf.journal`; `workflow resume --manifest /tmp/wf.json`; `workflow status --name demo`
- NUNCA mitm/extension install|uninstall em run


## Fórmulas de Rede e API
- DEVE gravar captura com `browser-automation-cli --timeout 60 --json mitm capture-url https://example.com --har /tmp/c.har --seconds 10 --hosts example.com`
- DEVE ler o caminho gravado em `data.capture_path` no envelope de `mitm capture-url`
- DEVE usar `--hosts` como allowlist que estreita a intercepção TLS
- DEVE reler a captura com `browser-automation-cli --json mitm domains --capture-path /tmp/capture.json`
- DEVE reler com `mitm apis --capture-path /tmp/capture.json`; `mitm graphql --capture-path /tmp/capture.json --limit 100`
- DEVE reler com `mitm list --capture-path /tmp/capture.json --limit 50`; `mitm get 0 --capture-path /tmp/capture.json`; `mitm ws list --capture-path /tmp/capture.json`
- `--capture-path` lê captura gravada por OUTRA invocação e é a ÚNICA ponte entre processos
- Medido em example.com: `capture_count` 37 e 9 hosts distintos
- Medido: a captura inclui ruído de fundo do Chrome como accounts.google.com e play.google.com
- DEVE filtrar esse ruído por host antes de concluir qualquer análise
- Medido: `mitm apis --capture-path` devolveu zero endpoints numa página estática
- Zero endpoints é resposta honesta e NUNCA é falha da captura
- DEVE executar `browser-automation-cli --capture-network --json net list --resource-types Document,XHR,Fetch --page-size 50`
- `net list` só enxerga tráfego com `--capture-network` no MESMO processo
- DEVE paginar com `--page-idx` e `--page-size`; incluir preservados com `--include-preserved`
- DEVE executar `browser-automation-cli --capture-network --json net get 0 --request-path /tmp/req.json --response-path /tmp/res.json`
- DEVE serializar `{"cmd":"net","action":"list","resource_types":"Document"}` em `run` com `--capture-network`
- DEVE navegar ANTES de chamar API por `eval` porque `eval` roda no contexto de origem da PÁGINA
- Medido A/B: sem `goto` antes, `fetch` devolve a string `Failed to fetch`
- Medido A/B: com `goto` antes para a mesma origem, devolve `ok:200`
- DEVE executar `browser-automation-cli --json eval 'fetch("/api").then(async r=>({ok:r.status})).catch(e=>({err:String(e)}))' --typed`
- `--typed` devolve `data.value` e `data.value_type` em vez do legado `data.result`
- Medido: `eval '({a:1,b:"x"})' --typed` devolve `value_type` igual a object
- Promise é resolvida automaticamente e NUNCA existe chave de await
- Medido: promise rejeitada sem try/catch devolve valor nulo com exit 0
- Essa falha é SILENCIOSA, portanto DEVE envolver toda chamada de API em try/catch ou `.catch`
- DEVE usar a chave `typed` no passo `eval` dentro de `run`
- Passo `eval` emite `refs_invalidated` true, portanto refs `@eN` morrem depois do eval
- DEVE recapturar refs com `view` depois de qualquer `eval` dentro de `run`
- Armadilha medida: chave desconhecida num passo de `run` é aceita em silêncio com ok true e exit 0
- DEVE conferir cada nome de chave com `schema <cmd> --json` antes de serializar o passo


## exec / run / residual
- DEVE executar `browser-automation-cli --json exec goto https://example.com`; `exec wait --selector h1 --wait-timeout-ms 2000`; `exec submit --target "#user" --timeout-ms 8000`; `exec pick --target @e1 --option Anomalia`; `exec select-option --target @e2 --option Alta`; `exec scrape --url https://example.com --format text`
- DEVE executar `browser-automation-cli --timeout 90 --json --json-steps run --script /tmp/steps.jsonl`
- DEVE executar `browser-automation-cli --json record --url https://example.com --path /tmp/rec.ndjson --seconds 30 --max-events 200`
- DEVE tratar `record` como gravador de interações da página que emite NDJSON reproduzível; `--url` e `--path` são OBRIGATÓRIOS; `--seconds` default 30 e `--max-events` default 200; o primeiro teto atingido vence
- DEVE fechar o ciclo record → replay com `browser-automation-cli --timeout 90 --json --json-steps run --script /tmp/rec.ndjson`
- DEVE serializar `{"cmd":"goto","url":"https://example.com","handle_before_unload":"accept","navigation_timeout_ms":15000}`, `{"cmd":"wait","selector":"h1, main, #content","wait_timeout_ms":10000}`, `{"cmd":"view","verbose":true}`, `{"cmd":"write","target":"@e1","value":"olá"}`, `{"cmd":"submit","target":"#user","timeout_ms":8000}`, `{"cmd":"scrape","url":"https://example.com","format":"text"}`, `{"cmd":"pick","target":"@e1","option":"Anomalia"}`, `{"cmd":"select-option","target":"@e2","option":"Alta"}`, `{"cmd":"dialog","action":"accept","if_present":true}`, `{"cmd":"grab","path":"/tmp/p.png","format":"png","full_page":true}`, `{"cmd":"print-pdf","path":"/tmp/p.pdf"}`, `{"cmd":"assert","kind":"url","url_contains":"example.com"}`, `{"cmd":"scroll","dy":400}`, `{"cmd":"page","action":"new","isolated_context":true}`
- DEVE validar residual com `browser-automation-cli -q --json doctor --offline --quick` — `residual_disk` não fail; zeros em `orphan_marker_dirs` e `ghost_marker_processes`; após DIE sozinho também zeros em `cli_marker_dirs` e `chromium_tmp_singleton_orphans` (`residual_disk` pass). NUNCA exigir zero `live_cli_marker_processes`; `sibling_live_processes>0` é concorrência saudável
- FORA de run: meta config mitm storage workflow crawl map batch-scrape search parse qr image video find-paths sg-scan sg-rewrite sheet-write monitor extension install/uninstall nested run/exec (mídia path-light é top-level)
- NUNCA trate exec como multi-passo; NUNCA divida `@eN` entre processos; NUNCA mate Chrome do usuário
- NUNCA apague em massa temporários genéricos do host; NUNCA mate Chrome Flatpak alheio
