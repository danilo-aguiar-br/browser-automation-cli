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
- MUST execute `browser-automation-cli --json config init` · `config path` · `config show` · `config list-keys` · `config get timeout` · `config set <key> <value>` after list-keys
- MUST discover the live key surface with `config list-keys --json` before any `config set`
- MUST consult `references/xdg-keys.md` for all 176 keys with default and description
- MUST set binaries with `chrome_path`, `lighthouse_path`, `ffmpeg_path`
- MUST set secrets with `encryption_key` and `openrouter_api_key`
- MUST set cache with `cache_backend sqlite|memory|redis` and plain Redis in `cache_redis_url`
- MUST set behaviour with `dialog_settle_ms`, `log_level`, `lang en|pt-BR`, `timeout`, `artifacts_dir`, `http_ssrf_mode strict|allow_loopback|off`, `log_rotation daily|hourly|never`
- NEVER reintroduce an inline key list in this skill; an inline list ages and truncates the live surface
- NEVER `rediss://`; NEVER log secrets; NEVER redis without `cache_redis_url`

## Navigation Wait Snapshot Interact
- MUST execute `browser-automation-cli --timeout 60 --json goto https://example.com --init-script 'window.__x=1' --handle-before-unload accept --navigation-timeout-ms 15000` · `back` · `forward` · `reload --ignore-cache`
- MUST execute `browser-automation-cli --json wait --ms 500` · `wait --selector "h1, main, #content" --wait-timeout-ms 10000 --include-snapshot` · `wait --text Example --wait-timeout-ms 5000` · `wait --state networkidle --wait-timeout-ms 15000`
- MUST execute `browser-automation-cli --json view --detailed` · `view --path /tmp/view.txt --allow-empty` only when blank intentional
- MUST execute `browser-automation-cli --json press @e1 --dblclick --include-snapshot` · `--experimental-vision --json click-at --x 10 --y 20` · `write @e2 "hello"` · `keys Enter` · `type "hello" --target @e2 --clear --submit Enter` · `hover @e1` · `drag --from @e1 --to @e2`
- MUST execute `browser-automation-cli --json fill-form --fields-json '[{"target":"@e3","value":"x"}]'` · `upload @e4 /tmp/file.txt` · `scroll --delta-y 400 --delta-x 100` · `exec pick --target @e1 --option Anomaly` · `exec select-option --target @e2 --option High` · `submit "#user" --timeout-ms 8000 --include-snapshot`
- Steps - `{"cmd":"pick","target":"@e1","option":"Anomaly"}` · `{"cmd":"select-option","target":"@e2","option":"High"}` · `{"cmd":"submit","target":"#user","timeout_ms":8000}` · `{"cmd":"wait","selector":"h1","wait_timeout_ms":10000}`
- NEVER `--ignore-cache` on goto; NEVER `view --verbose`; NEVER `fill-form --json` as payload

## Content Eval Artifacts
- MUST execute `browser-automation-cli --json extract @e1` · `extract @e1 --attr href` · `--timeout 120 --json extract --llm --question "What is the title?" --schema-json /tmp/s.json https://example.com`
- MUST execute `browser-automation-cli --json text @e2` · `attr @e1 href` · `eval 'document.title' --file-path /tmp/eval.json` · `eval '(el)=>el.textContent' --args '["@e1"]'` · `eval 'confirm("go?")' --dialog-action accept`
- MUST execute `browser-automation-cli --category-extensions --json eval 'chrome.runtime.id' --service-worker-id <sw-id>`
- MUST execute `browser-automation-cli --json grab --path /tmp/p.png --format png --full-page` · `grab --path /tmp/p.webp --format webp --quality 90 --element @e1`
- MUST execute `browser-automation-cli --timeout 60 --json print-pdf --path /tmp/p.pdf --url https://example.com`
- Steps - `{"cmd":"grab","path":"/tmp/p.png","format":"png"}` · `{"cmd":"print-pdf","path":"/tmp/p.pdf","url":"https://example.com"}`
- NEVER grab avif; NEVER omit `--path` on grab/print-pdf; NEVER omit `--url` on one-shot print-pdf

## Tabs Cookies Storage Dialogs Asserts
- MUST execute `browser-automation-cli --json page list` · `page info` · `page tab-id` · `page new --isolated-context session-a --url https://example.com` · `page select 0 --bring-to-front` · `page close --index 1`
- MUST execute `browser-automation-cli --json cookie list --url https://example.com` · `cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'` · `cookie clear`
- MUST execute `browser-automation-cli --json storage export --path /tmp/auth.json --url https://example.com` · `storage import --path /tmp/auth.json --url https://example.com`
- MUST execute `browser-automation-cli --json dialog accept --if-present` · `dialog dismiss --if-present` · `dialog accept --text Ana`
- MUST execute `browser-automation-cli --json assert url example.com --contains` · `assert text "Example" --target h1` · `--capture-console --json assert console-empty` · `assert console-no-match --pattern TypeError` · `assert console --level error --max 0`
- Step - `{"cmd":"dialog","action":"accept","if_present":true}`

## Console Net
- MUST execute `browser-automation-cli --capture-console --json console list --types log,warning,error` · `console get 0` · `console clear` · `console dump --path /tmp/console.json`
- MUST execute `browser-automation-cli --capture-network --json net list --resource-types Document,XHR,Fetch` · `net get 0 --request-path /tmp/req.json --response-path /tmp/res.json`
- MUST capture console/network in the SAME process as the consuming command

## Scrape Family and Local Files
- MUST execute `browser-automation-cli -q --json scrape https://example.com --format markdown --select source_url,title,markdown --max-text-chars 800 --only-main-content` · multi-format `scrape … --format markdown,jsonld --select source_url,title,markdown` · `--redact-pii --with-content-hash --header "Accept-Language: en"` · browser `--engine browser --wait-ms 500` · `batch-scrape --urls-file /tmp/u.txt --filter http_error=false --output-mode csv --select source_url,text` · `crawl … --sort source_url --dedup-key source_url --output-mode ndjson` · `map … --search docs --sitemap-only --limit 50`
- MUST execute `browser-automation-cli -q --json scrape https://example.com --format rawHtml --engine http`
- Formats - the 14 `--format` values are text, markdown, html, rawHtml, links, metadata, screenshot, summary, product, branding, images, jsonld, json, feed (aliases md meta body shot); accepts CSV or repeated flag
- Engines - `--engine` accepts ONLY `http` (reqwest plus scraper) and `browser` (CDP); the default comes from the XDG key `scrape_default_engine`, today `http`
- NEVER treat `rawHtml` as an alias of `html`; `--format html` returns the `html` key and `--format rawHtml` returns the `rawHtml` key, with distinct payloads
- MUST execute `browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2 --engine browser` · `crawl https://example.com --limit 20 --max-depth 2 --format text --same-host` · `map https://example.com --limit 50` · `search "example domain" --limit 10`
- MUST execute `browser-automation-cli --json parse /tmp/doc.pdf` · `parse /tmp/sheet.ods --redact-pii` (HTML MD text PDF DOCX XLSX ODS) · `find-paths --glob '**/*.rs' . --type f --limit 200` · `sg-scan . --limit 100` · `sg-rewrite .` then `--apply` after dry-run · `sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data`
- MUST execute `browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/b.baseline --write-baseline --engine http` · `qr encode --text https://example.com --format png --path /tmp/qr.png` · `qr decode --path /tmp/qr.png`
- MUST execute `browser-automation-cli --json image info --path /tmp/a.png --select format,width,height,sha256` · `image convert --path /tmp/a.png --format webp -o /tmp/a.webp` · `image download https://example.com/a.png -o /tmp/a.png` · `image resize --path /tmp/a.png --width 640 --keep-aspect -o /tmp/a-640.webp --format webp --quality 80`
- MUST execute `browser-automation-cli --json video info --path /tmp/in.mp4 --select format,bytes,path` (aliases → container/size_bytes) · `video convert --path /tmp/in.mp4 --format webm -o /tmp/out.webm --select path_out,auto_reencoded,bytes_out` · `video to-mp3 --path /tmp/in.mp4 -o /tmp/a.mp3` · `video trim --path /tmp/in.mp4 --start 0 --duration 0.5 -o /tmp/clip.mp4` · `video thumbnail --path /tmp/in.mp4 --at 0 -o /tmp/thumb.png` · `--timeout 120 video download https://example.com/v.mp4 -o /tmp/v.mp4 --max-bytes 52428800 --require-video`
- MUST set video caps via XDG after list-keys: `video_max_input_bytes` `video_download_max_bytes` `video_default_container` `video_default_crf` `video_default_audio_bitrate` `ffmpeg_path` `ffmpeg_timeout_secs`
- MUST execute `browser-automation-cli --json audio info --path /tmp/in.wav --select format,codec,duration,bytes,sha256` · `audio convert --path /tmp/in.wav --format mp3 -o /tmp/a.mp3` · `audio convert --path /tmp/clip.mp4 --format m4a -o /tmp/a.m4a` · `audio trim --path /tmp/a.mp3 --start 1 --duration 5 -o /tmp/cut.mp3` · `audio download https://example.com/a.mp3 -o /tmp/a.mp3` · then `upload @e1 /tmp/a.mp3`
- MUST set audio caps via XDG after list-keys: `audio_max_input_bytes` `audio_download_max_bytes` `audio_default_format` `audio_default_bitrate` `ffmpeg_path` `ffmpeg_timeout_secs`
- NEVER dump media bytes/base64 on stdout; path→path only; NEVER claim HLS/yt-dlp/pure encode as product
- Steps - `{"cmd":"scrape","url":"https://example.com","format":"text"}` · `{"cmd":"scrape","url":"https://example.com","formats":"markdown,links"}`
- MUST execute `browser-automation-cli --json scrape https://example.com --format text --engine http` (robots default); bypass ONLY with both `--ignore-robots --i-accept-robots-risk`

## Emulate Perf Lighthouse Screencast Heap
- MUST execute `browser-automation-cli --json emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G" --color-scheme dark` · `resize --width 1280 --height 720`
- MUST execute `browser-automation-cli --json perf start --reload --auto-stop --path /tmp/trace.json` · `perf stop --path /tmp/trace.json` · `perf insight --name DocumentLatency --insight-set-id <set-id>`
- MUST execute `browser-automation-cli --timeout 180 --json lighthouse https://example.com --out-dir /tmp/lh --device desktop --mode navigation` then read `data.binary_source` · `--experimental-screencast --json screencast start --path /tmp/cast` · `screencast stop --path /tmp/cast.webm`
- MUST execute `browser-automation-cli --category-memory --json heap take --path /tmp/s.heapsnapshot` · `heap close --path /tmp/s.heapsnapshot` · `heap summary --path /tmp/s.heapsnapshot` · `heap compare --base /tmp/a.heapsnapshot --current /tmp/b.heapsnapshot`
- MUST execute `browser-automation-cli --category-memory --json heap details --path /tmp/s.heapsnapshot --filter-name Array` · `heap class-nodes --path /tmp/s.heapsnapshot --id 7` · `heap dup-strings --path /tmp/s.heapsnapshot`
- MUST execute `browser-automation-cli --category-memory --json heap dominators --path /tmp/s.heapsnapshot --node 42` · `heap edges --path /tmp/s.heapsnapshot --node 42` · `heap retainers --path /tmp/s.heapsnapshot --node 42` · `heap object-details --path /tmp/s.heapsnapshot --node 42` · `heap paths --path /tmp/s.heapsnapshot --node 42 --max-depth 8`
- NEVER `emulate --device`; NEVER use `--node-id` (flag is `--node`)

## Extensions Third-party WebMCP MITM Workflow
- MUST execute `browser-automation-cli --category-extensions --json extension list` · `extension install /tmp/ext` · `extension reload <ext-id>` · `extension trigger <ext-id>` · `extension uninstall <ext-id>`
- MUST execute `browser-automation-cli --category-third-party --json devtools3p list --url https://example.com` · `devtools3p exec ToolName --params '{}' --url https://example.com` · `--category-webmcp --json webmcp list --url https://example.com` · `webmcp exec ToolName --input '{}' --url https://example.com`
- MUST execute `browser-automation-cli --json mitm init-ca` · `mitm start --seconds 30` · `mitm status` · `mitm capture-url https://example.com --har /tmp/c.har --hosts example.com`
- MUST execute `browser-automation-cli --json mitm list --limit 50` · `mitm get 0` · `mitm har --out /tmp/c.har` · `mitm export --format ndjson --out /tmp/c.ndjson` · `mitm domains|apis|graphql` · `mitm ws list --limit 50` · `mitm ws get 0`
- MUST execute `browser-automation-cli --json mitm block --host example.com --path /ads` · `mitm allow --host example.com` · `mitm redact --secrets true`
- MUST execute `browser-automation-cli --json workflow run --manifest /tmp/wf.json --journal /tmp/wf.journal` · `workflow resume --manifest /tmp/wf.json` · `workflow status --name demo`
- NEVER expose MITM outside 127.0.0.1; NEVER put mitm/extension install|uninstall inside run


## Network and API Formulas
- MUST record a capture with `browser-automation-cli --timeout 60 --json mitm capture-url https://example.com --har /tmp/c.har --seconds 10 --hosts example.com`
- MUST read the recorded path in `data.capture_path` of the `mitm capture-url` envelope
- MUST use `--hosts` as the allowlist that narrows TLS interception
- MUST re-read the capture with `browser-automation-cli --json mitm domains --capture-path /tmp/capture.json`
- MUST re-read with `mitm apis --capture-path /tmp/capture.json` · `mitm graphql --capture-path /tmp/capture.json --limit 100`
- MUST re-read with `mitm list --capture-path /tmp/capture.json --limit 50` · `mitm get 0 --capture-path /tmp/capture.json` · `mitm ws list --capture-path /tmp/capture.json`
- `--capture-path` reads a capture recorded by ANOTHER invocation and is the ONLY bridge across processes
- Measured on example.com: `capture_count` 37 and 9 distinct hosts
- Measured: the capture includes Chrome background noise such as accounts.google.com and play.google.com
- MUST filter that noise by host before concluding any analysis
- Measured: `mitm apis --capture-path` returned zero endpoints on a static page
- Zero endpoints is an honest answer and is NEVER a capture failure
- MUST execute `browser-automation-cli --capture-network --json net list --resource-types Document,XHR,Fetch --page-size 50`
- `net list` only sees traffic with `--capture-network` in the SAME process
- MUST paginate with `--page-idx` and `--page-size`; include preserved entries with `--include-preserved`
- MUST execute `browser-automation-cli --capture-network --json net get 0 --request-path /tmp/req.json --response-path /tmp/res.json`
- MUST serialize `{"cmd":"net","action":"list","resource_types":"Document"}` inside `run` with `--capture-network`
- MUST navigate BEFORE calling an API through `eval` because `eval` runs in the PAGE origin context
- Measured A/B: without a prior `goto`, `fetch` returns the string `Failed to fetch`
- Measured A/B: with a prior `goto` to the same origin, it returns `ok:200`
- MUST execute `browser-automation-cli --json eval 'fetch("/api").then(async r=>({ok:r.status})).catch(e=>({err:String(e)}))' --typed`
- `--typed` returns `data.value` and `data.value_type` instead of the legacy `data.result`
- Measured: `eval '({a:1,b:"x"})' --typed` returns `value_type` equal to object
- A promise is resolved automatically and an await key NEVER exists
- Measured: a rejected promise without try/catch returns a null value with exit 0
- That failure is SILENT, so you MUST wrap every API call in try/catch or `.catch`
- MUST use the `typed` key on the `eval` step inside `run`
- An `eval` step emits `refs_invalidated` true, so `@eN` refs die after the eval
- MUST re-capture refs with `view` after any `eval` inside `run`
- Measured trap: an unknown key on a `run` step is silently accepted with ok true and exit 0
- MUST check every key name with `schema <cmd> --json` before serializing the step


## Exec Run Residual
- MUST execute `browser-automation-cli --json exec goto https://example.com` · `exec wait --selector h1 --wait-timeout-ms 2000` · `exec pick --target @e1 --option Anomaly` · `exec select-option --target @e2 --option High` · `exec submit --target "#user" --timeout-ms 8000` · `exec scrape --url https://example.com --format text`
- MUST execute `browser-automation-cli --timeout 90 --json --json-steps --capture-console --capture-network run --script /tmp/steps.jsonl`
- MUST execute `browser-automation-cli --json record --url https://example.com --path /tmp/rec.ndjson --seconds 30 --max-events 200`
- MUST treat `record` as the page-interaction recorder emitting replayable NDJSON; `--url` and `--path` are REQUIRED; `--seconds` defaults to 30 and `--max-events` defaults to 200; the first cap reached wins
- MUST close the record → replay cycle with `browser-automation-cli --timeout 90 --json --json-steps run --script /tmp/rec.ndjson`
- Run fields - goto.url wait.wait_timeout_ms view.verbose|detailed|allow_empty write/type target+value keys.key drag from+to fill-form.fields pick/select-option target+option submit.target scroll dy/dx grab/print-pdf.path scrape url+format|formats assert.kind page/cookie/console/net/dialog/perf/heap/extension.action isolated_context
- Keep OUT of run - meta config mitm storage workflow crawl map batch-scrape search parse qr image video find-paths sg-scan sg-rewrite sheet-write monitor extension install/uninstall nested run/exec (path-light media is top-level only)
- MUST execute `browser-automation-cli -q --json doctor --offline --quick` and require residual_disk not fail; zero residual.orphan_marker_dirs residual.ghost_marker_processes; after DIE alone also zero residual.cli_marker_dirs residual.chromium_tmp_singleton_orphans (residual_disk pass). NEVER require zero residual.live_cli_marker_processes; sibling_live_processes>0 is healthy concurrency
- NEVER treat exec as multi-step; NEVER split `@eN` across processes
- NEVER mass-delete host temps; NEVER kill user/Flatpak Chrome
