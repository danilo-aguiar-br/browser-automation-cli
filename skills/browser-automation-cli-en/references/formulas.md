# browser-automation-cli — Exhaustive Argv Formulas

## Use Rules
- MUST copy formulas literally; swap ONLY placeholders; full binary `browser-automation-cli` ALWAYS
- MUST pass `--json`; parse stdout with `jaq`; check exit before trust; require `.ok == true` before `.data`
- MUST discover via `commands --json`, `config list-keys --json`, `schema <cmd> --json`
- NEVER invent aliases, product env vars, or missing flags

## Agent-First Contract
- After real `dialog accept|dismiss` MUST read `.data.dialog_settled`; when true DO NOT artificial-wait before next page observation
- Multi-tab dialogs keyed by `session_id`; tab switch under open dialog is best-effort domain enable
- MUST set settle via `config set dialog_settle_ms`; run wait uses `wait_timeout_ms`; run scrape uses `format`|`formats` (text MUST NOT dump huge html)
- Native `select-option`/`pick` dispatch input then change, report `via: native_select`
- `submit` submits form/field-owner and waits nav/request; storage requires `--path` (`--url` when origin must load same process); export 0600; OUT of run
- grab png|jpeg|webp only — NEVER avif; lighthouse `binary_source` real|mock — NEVER treat mock as LHR validation
- mitm/storage/extension install|uninstall outside run; `exec` single-step only

## Global Flags
- MUST execute `browser-automation-cli --json --json-steps --timeout 90 --step-timeout 20 --capture-console --capture-network run --script /tmp/steps.jsonl`
- MUST execute `browser-automation-cli --json -q --plain --max-concurrency 4 --artifacts-dir /tmp/arts --correlation-id req-42 goto https://example.com`
- MUST pass `--verbose`|`--debug` or `config set log_level`; robots bypass ONLY both `--ignore-robots --i-accept-robots-risk`
- MUST pass `--category-memory` (heap), `--category-extensions`, `--category-third-party` (devtools3p), `--category-webmcp`
- MUST pass `--experimental-vision` (click-at), `--experimental-screencast`
- MUST pass `--mitm` plus `--mitm-har|--mitm-hosts|--mitm-ca-dir|--mitm-ws|--mitm-max-body-bytes|--mitm-no-media-bodies|--mitm-redact-secrets` only when required

## Meta
- MUST execute `browser-automation-cli --json doctor --offline --quick` (`--fix` only when repair hints required)
- MUST execute `browser-automation-cli --json commands` · `schema goto` · `schema --cmd wait` · `version` · `locale`
- MUST execute `browser-automation-cli completions bash` (zsh|fish|elvish|powershell) · `man --out /tmp/browser-automation-cli.1`

## Config XDG
- MUST execute `browser-automation-cli --json config init|path|show|list-keys|get` · `config get timeout` · `config set <key> <value>` after list-keys
- MUST set encryption_key openrouter_api_key chrome_path lighthouse_path ffmpeg_path
- MUST set cache_backend sqlite|memory|redis; Redis only plain redis:// via cache_redis_url (NEVER rediss://)
- MUST set dialog_settle_ms log_level lang en|pt-BR timeout artifacts_dir http_ssrf_mode strict|allow_loopback|off log_rotation daily|hourly|never
- Full keys (list-keys live) - allowed_roots,artifacts_dir,browser_close_wait_secs,browser_scrape_max_body_bytes,cache_backend,cache_max_resp_bulk_bytes,cache_max_resp_line_bytes,cache_redis_url,cdp_connection_probe_timeout_secs,cdp_discovery_max_body_bytes,cdp_discovery_timeout_secs,cdp_event_broadcast_capacity,cdp_event_drain_poll_ms,cdp_network_idle_settle_ms,cdp_target_event_wait_ms,chrome_path,chrome_search_paths,color,default_jpeg_quality,default_viewport_height,default_viewport_width,dialog_settle_ms,encryption_key,eval_drain_slice_ms,event_pump_slice_ms,event_tracker_max_entries,extension_attach_poll_ms,ffmpeg_path,ffmpeg_timeout_secs,file_parse_cache_ttl_secs,heap_dominator_max_states,heap_final_iters,heap_inner_iters,heap_max_class_nodes,heap_max_edges,heap_max_path_depth,heap_max_paths,heap_max_retainers,heap_outer_iters,heap_snapshot_max_bytes,http_connect_timeout_secs,http_pool_max_idle_per_host,http_redirect_max,http_ssrf_mode,http_timeout_secs,ignore_robots,interact_settle_ms,lang,lighthouse_path,lighthouse_timeout_secs,lightpanda_cdp_connect_timeout_secs,lightpanda_discovery_timeout_ms,lightpanda_max_log_lines,lightpanda_poll_interval_ms,lightpanda_ready_slice_ms,lightpanda_session_timeout_secs,lightpanda_startup_timeout_secs,lightpanda_target_init_timeout_secs,llm_base_url,llm_http_timeout_secs,llm_model,log_level,log_rotation,log_to_file,max_cli_json_payload_bytes,max_json_file_bytes,max_log_files,max_ndjson_line_bytes,max_sg_file_bytes,mitm_ca_cache_size,mitm_capture_wait_max_ms,mitm_capture_wait_min_ms,mitm_chrome_settle_ms,mitm_list_limit_max,mitm_proxy_seconds_max,mitm_ws_frames_cap,mitm_ws_preview_chars,namespace,nav_micro_settle_ms,openrouter_api_key,perf_autostop_settle_ms,perf_trace_inner_iters,perf_trace_inner_slice_ms,perf_trace_outer_iters,perf_trace_outer_slice_ms,platform_child_wait_secs,redis_allow_remote,redis_connect_timeout_secs,redis_io_timeout_secs,retry_base_delay_ms,retry_budget_secs,retry_cdp_base_delay_ms,retry_cdp_budget_secs,retry_cdp_max_attempts,retry_cdp_max_delay_secs,retry_default_max_attempts,retry_http_base_delay_ms,retry_http_budget_secs,retry_http_max_attempts,retry_http_max_delay_secs,retry_llm_base_delay_ms,retry_llm_budget_secs,retry_llm_max_attempts,retry_llm_max_delay_secs,retry_max_delay_secs,robots_loopback_exempt,robots_max_body_bytes,robots_probe_timeout_secs,scrape_crawl_limit_max,scrape_crawl_max_depth,scrape_http_cache_ttl_secs,scrape_max_body_bytes,scrape_max_parse_bytes,scrape_search_limit_max,screencast_ffmpeg_framerate,screencast_jpeg_quality,screencast_start_pump_iters,screencast_stop_pump_iters,search_base_url,shutdown_deadline_secs,shutdown_poll_ms,state_collect_deadline_secs,state_event_recv_secs,state_load_settle_ms,support_settle_ms,timeout,webhook_max_attempts,webhook_post_timeout_secs,webhook_retry_base_delay_ms

## Navigation Wait Snapshot Interact
- MUST execute `browser-automation-cli --timeout 60 --json goto https://example.com --init-script 'window.__x=1' --handle-before-unload accept --navigation-timeout-ms 15000` · `back` · `forward` · `reload --ignore-cache`
- MUST execute `browser-automation-cli --json wait --ms 500` · `wait --selector "h1, main, #content" --wait-timeout-ms 10000 --include-snapshot` · `wait --text Example --wait-timeout-ms 5000` · `wait --state networkidle --wait-timeout-ms 15000`
- MUST execute `browser-automation-cli --json view --detailed` · `view --path /tmp/view.txt --allow-empty` only when blank intentional
- MUST execute `browser-automation-cli --json press @e1 --dblclick --include-snapshot` · `--experimental-vision --json click-at --x 10 --y 20` · `write @e2 "hello"` · `keys Enter` · `type "hello" --target @e2 --clear --submit Enter` · `hover @e1` · `drag --from @e1 --to @e2`
- MUST execute `browser-automation-cli --json fill-form --fields-json '[{"target":"@e3","value":"x"}]'` · `upload @e4 /tmp/file.txt` · `scroll --delta-y 400 --delta-x 100` · `exec pick --target @e1 --option Anomaly` · `exec select-option --target @e2 --option High` · `submit "#user" --timeout-ms 8000 --include-snapshot`
- Steps - `{"cmd":"pick","target":"@e1","option":"Anomaly"}` · `{"cmd":"select-option","target":"@e2","option":"High"}` · `{"cmd":"submit","target":"#user","timeout_ms":8000}` · `{"cmd":"wait","selector":"h1","wait_timeout_ms":10000}`

## Content Eval Artifacts
- MUST execute `browser-automation-cli --json extract @e1` · `extract @e1 --attr href` · `--timeout 120 --json extract --llm --question "What is the title?" --schema-json /tmp/s.json https://example.com`
- MUST execute `browser-automation-cli --json text @e2` · `attr @e1 href` · `eval 'document.title' --file-path /tmp/eval.json` · `eval '(el)=>el.textContent' --args '["@e1"]'` · `eval 'confirm("go?")' --dialog-action accept`
- MUST execute `browser-automation-cli --category-extensions --json eval 'chrome.runtime.id' --service-worker-id <sw-id>`
- MUST execute `browser-automation-cli --json grab --path /tmp/p.png --format png --full-page` · `grab --path /tmp/p.webp --format webp --quality 90 --element @e1`
- MUST execute `browser-automation-cli --timeout 60 --json print-pdf --path /tmp/p.pdf --url https://example.com`
- MUST execute `browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/b.baseline --write-baseline --engine http` · `qr encode --text https://example.com --format png --path /tmp/qr.png` · `qr decode --path /tmp/qr.png`
- Steps - `{"cmd":"grab","path":"/tmp/p.png","format":"png"}` · `{"cmd":"print-pdf","path":"/tmp/p.pdf","url":"https://example.com"}`

## Tabs Cookies Storage Dialogs Asserts Console Net
- MUST execute `browser-automation-cli --json page list|info|tab-id` · `page new --isolated-context session-a --url https://example.com` · `page select 0 --bring-to-front` · `page close --index 1`
- MUST execute `browser-automation-cli --json cookie list --url https://example.com` · `cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'` · `cookie clear`
- MUST execute `browser-automation-cli --json storage export --path /tmp/auth.json --url https://example.com` · `storage import --path /tmp/auth.json --url https://example.com`
- MUST execute `browser-automation-cli --json dialog accept --if-present` · `dialog dismiss --if-present` · `dialog accept --text Ana`
- MUST execute `browser-automation-cli --json assert url example.com --contains` · `assert text "Example" --target h1` · `--capture-console --json assert console-empty` · `assert console-no-match --pattern TypeError` · `assert console --level error --max 0`
- MUST execute `browser-automation-cli --capture-console --json console list --types log,warning,error` · `console get 0` · `console clear` · `console dump --path /tmp/console.json`
- MUST execute `browser-automation-cli --capture-network --json net list --resource-types Document,XHR,Fetch` · `net get 0 --request-path /tmp/req.json --response-path /tmp/res.json`
- Step - `{"cmd":"dialog","action":"accept","if_present":true}`

## Scrape Family and Local Files
- MUST execute `browser-automation-cli --json scrape https://example.com --format text --engine http` · `scrape https://example.com --format markdown,links,metadata --engine http --only-main-content` · `scrape https://example.com --format summary --format product --format branding --engine browser`
- Formats - text markdown html raw-html links metadata screenshot summary product branding (aliases md meta body shot)
- MUST execute `browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2 --engine browser` · `crawl https://example.com --limit 20 --max-depth 2 --format text --same-host` · `map https://example.com --limit 50` · `search "example domain" --limit 10`
- MUST execute `browser-automation-cli --json parse /tmp/doc.pdf` · `parse /tmp/sheet.ods --redact-pii` (HTML MD text PDF DOCX XLSX ODS) · `find-paths --glob '**/*.rs' . --type f --limit 200` · `sg-scan . --limit 100` · `sg-rewrite .` then `--apply` after dry-run · `sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data`
- Steps - `{"cmd":"scrape","url":"https://example.com","format":"text"}` · `{"cmd":"scrape","url":"https://example.com","formats":"markdown,links"}`

## Emulate Perf Lighthouse Screencast Heap
- MUST execute `browser-automation-cli --json emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G" --color-scheme dark` · `resize --width 1280 --height 720`
- MUST execute `browser-automation-cli --json perf start --reload --auto-stop --path /tmp/trace.json` · `perf stop --path /tmp/trace.json` · `perf insight --name DocumentLatency --insight-set-id <set-id>`
- MUST execute `browser-automation-cli --timeout 180 --json lighthouse https://example.com --out-dir /tmp/lh --device desktop --mode navigation` then read `data.binary_source` · `--experimental-screencast --json screencast start --path /tmp/cast` · `screencast stop --path /tmp/cast.webm`
- MUST execute `browser-automation-cli --category-memory --json heap take --path /tmp/s.heapsnapshot` · `heap close|summary --path /tmp/s.heapsnapshot` · `heap compare --base /tmp/a.heapsnapshot --current /tmp/b.heapsnapshot`
- MUST execute `browser-automation-cli --category-memory --json heap details --path /tmp/s.heapsnapshot --filter-name Array` · `heap class-nodes --path /tmp/s.heapsnapshot --id 7` · `heap dup-strings --path /tmp/s.heapsnapshot`
- MUST execute `browser-automation-cli --category-memory --json heap dominators|edges|retainers|object-details --path /tmp/s.heapsnapshot --node 42` · `heap paths --path /tmp/s.heapsnapshot --node 42 --max-depth 8`
- NEVER use `--node-id` (flag is `--node`)

## Extensions Third-party WebMCP MITM
- MUST execute `browser-automation-cli --category-extensions --json extension list` · `extension install /tmp/ext` · `extension reload|trigger|uninstall <ext-id>`
- MUST execute `browser-automation-cli --category-third-party --json devtools3p list --url https://example.com` · `devtools3p exec ToolName --params '{}' --url https://example.com` · `--category-webmcp --json webmcp list --url https://example.com` · `webmcp exec ToolName --input '{}' --url https://example.com`
- MUST execute `browser-automation-cli --json mitm init-ca` · `mitm start --seconds 30` · `mitm status` · `mitm capture-url https://example.com --har /tmp/c.har --hosts example.com`
- MUST execute `browser-automation-cli --json mitm list --limit 50` · `mitm get 0` · `mitm har --out /tmp/c.har` · `mitm export --format ndjson --out /tmp/c.ndjson` · `mitm domains|apis|graphql` · `mitm ws list --limit 50` · `mitm ws get 0`
- MUST execute `browser-automation-cli --json mitm block --host example.com --path /ads` · `mitm allow --host example.com` · `mitm redact --secrets true`
- NEVER expose MITM outside 127.0.0.1; NEVER put mitm inside run

## Workflow Exec Run Robots Residual
- MUST execute `browser-automation-cli --json workflow run --manifest /tmp/wf.json --journal /tmp/wf.journal` · `workflow resume --manifest /tmp/wf.json` · `workflow status --name demo`
- MUST execute `browser-automation-cli --json exec goto https://example.com` · `exec wait --selector h1 --wait-timeout-ms 2000` · `exec pick --target @e1 --option Anomaly` · `exec select-option --target @e2 --option High` · `exec submit --target "#user" --timeout-ms 8000`
- MUST execute `browser-automation-cli --timeout 90 --json --json-steps --capture-console --capture-network run --script /tmp/steps.jsonl`
- Run fields - goto.url wait.wait_timeout_ms view.verbose|detailed|allow_empty write/type target+value keys.key drag from+to fill-form.fields pick/select-option target+option submit.target scroll dy/dx grab/print-pdf.path scrape url+format|formats assert.kind page/cookie/console/net/dialog/perf/heap/extension.action isolated_context
- Keep OUT of run - meta config mitm storage workflow crawl map batch-scrape search parse qr find-paths sg-scan sg-rewrite sheet-write monitor extension install/uninstall nested run/exec
- MUST execute `browser-automation-cli --json scrape https://example.com --format text --engine http` (robots default); bypass ONLY with both `--ignore-robots --i-accept-robots-risk`
- MUST execute `browser-automation-cli -q --json doctor --offline --quick` and require zero residual.cli_marker_dirs residual.chromium_tmp_singleton_orphans residual.live_cli_marker_processes and residual_disk pass
- NEVER mass-delete host temps; NEVER kill user/Flatpak Chrome
