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
- DEVE setar (após list-keys) — lang, timeout, artifacts_dir, ignore_robots, namespace, encryption_key, color, log_level, log_to_file, max_log_files, log_rotation, chrome_path, lighthouse_path, ffmpeg_path, lighthouse_timeout_secs, ffmpeg_timeout_secs, openrouter_api_key, llm_base_url, llm_model, cache_backend, cache_redis_url, search_base_url, lightpanda_startup_timeout_secs, lightpanda_session_timeout_secs, max_json_file_bytes, max_ndjson_line_bytes, max_cli_json_payload_bytes, default_jpeg_quality, event_pump_slice_ms, screencast_jpeg_quality, interact_settle_ms, dialog_settle_ms, cdp_connection_probe_timeout_secs, http_ssrf_mode, http_timeout_secs, http_connect_timeout_secs, scrape_max_body_bytes, llm_http_timeout_secs, redis_allow_remote, redis_connect_timeout_secs, robots_loopback_exempt, allowed_roots, chrome_search_paths, redis_io_timeout_secs, cache_max_resp_bulk_bytes, cache_max_resp_line_bytes, scrape_http_cache_ttl_secs, file_parse_cache_ttl_secs, cdp_discovery_max_body_bytes, cdp_event_broadcast_capacity, cdp_event_drain_poll_ms, cdp_network_idle_settle_ms, cdp_target_event_wait_ms, cdp_discovery_timeout_secs, event_tracker_max_entries, extension_attach_poll_ms, screencast_ffmpeg_framerate, robots_probe_timeout_secs, robots_max_body_bytes, browser_scrape_max_body_bytes, http_redirect_max, http_pool_max_idle_per_host, webhook_post_timeout_secs, webhook_retry_base_delay_ms, webhook_max_attempts, heap_snapshot_max_bytes, heap_max_retainers, heap_max_edges, heap_max_paths, heap_max_path_depth, heap_max_class_nodes, heap_dominator_max_states, heap_outer_iters, heap_inner_iters, heap_final_iters, browser_close_wait_secs, platform_child_wait_secs, shutdown_poll_ms, shutdown_deadline_secs, lightpanda_poll_interval_ms, lightpanda_discovery_timeout_ms, lightpanda_max_log_lines, lightpanda_ready_slice_ms, lightpanda_cdp_connect_timeout_secs, lightpanda_target_init_timeout_secs, screencast_start_pump_iters, screencast_stop_pump_iters, mitm_list_limit_max, mitm_proxy_seconds_max, mitm_chrome_settle_ms, mitm_capture_wait_min_ms, mitm_capture_wait_max_ms, mitm_ws_frames_cap, mitm_ws_preview_chars, mitm_ca_cache_size, max_sg_file_bytes, scrape_crawl_limit_max, scrape_crawl_max_depth, scrape_search_limit_max, scrape_max_parse_bytes, retry_default_max_attempts, retry_base_delay_ms, retry_max_delay_secs, retry_budget_secs, retry_cdp_max_attempts, retry_cdp_base_delay_ms, retry_cdp_max_delay_secs, retry_cdp_budget_secs, retry_http_max_attempts, retry_http_base_delay_ms, retry_http_max_delay_secs, retry_http_budget_secs, retry_llm_max_attempts, retry_llm_base_delay_ms, retry_llm_max_delay_secs, retry_llm_budget_secs, eval_drain_slice_ms, support_settle_ms, nav_micro_settle_ms, perf_autostop_settle_ms, perf_trace_inner_slice_ms, perf_trace_outer_slice_ms, perf_trace_outer_iters, perf_trace_inner_iters, state_collect_deadline_secs, state_event_recv_secs, state_load_settle_ms, default_viewport_width, default_viewport_height
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
- DEVE executar `browser-automation-cli --json scrape https://example.com --format markdown,links,metadata --engine http --only-main-content`
- DEVE executar `browser-automation-cli --json scrape https://example.com --format summary --format product --format branding --engine browser`
- DEVE executar `browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2`
- DEVE executar `browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text --same-host`; `map https://example.com --limit 50`; `search "example domain" --limit 10`
- DEVE executar `browser-automation-cli --json parse /tmp/doc.pdf`; `parse /tmp/planilha.ods --redact-pii`
- DEVE executar `browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/b.baseline --write-baseline --engine http`
- DEVE executar `browser-automation-cli --json qr encode --text https://example.com --format png --path /tmp/qr.png`; `qr decode --path /tmp/qr.png`
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

## exec / run / residual
- DEVE executar `browser-automation-cli --json exec goto https://example.com`; `exec wait --selector h1 --wait-timeout-ms 2000`; `exec submit --target "#user" --timeout-ms 8000`; `exec pick --target @e1 --option Anomalia`; `exec select-option --target @e2 --option Alta`; `exec scrape --url https://example.com --format text`
- DEVE executar `browser-automation-cli --timeout 90 --json --json-steps run --script /tmp/steps.jsonl`
- DEVE serializar `{"cmd":"goto","url":"https://example.com","handle_before_unload":"accept","navigation_timeout_ms":15000}`, `{"cmd":"wait","selector":"h1, main, #content","wait_timeout_ms":10000}`, `{"cmd":"view","verbose":true}`, `{"cmd":"write","target":"@e1","value":"olá"}`, `{"cmd":"submit","target":"#user","timeout_ms":8000}`, `{"cmd":"scrape","url":"https://example.com","format":"text"}`, `{"cmd":"pick","target":"@e1","option":"Anomalia"}`, `{"cmd":"select-option","target":"@e2","option":"Alta"}`, `{"cmd":"dialog","action":"accept","if_present":true}`, `{"cmd":"grab","path":"/tmp/p.png","format":"png","full_page":true}`, `{"cmd":"print-pdf","path":"/tmp/p.pdf"}`, `{"cmd":"assert","kind":"url","url_contains":"example.com"}`, `{"cmd":"scroll","dy":400}`, `{"cmd":"page","action":"new","isolated_context":true}`
- DEVE validar residual com `browser-automation-cli -q --json doctor --offline --quick` — zeros em `cli_marker_dirs`, `chromium_tmp_singleton_orphans`, `live_cli_marker_processes`; `residual_disk` pass
- NUNCA trate exec como multi-passo; NUNCA divida `@eN` entre processos; NUNCA mate Chrome do usuário
