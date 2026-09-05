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
- MUST execute `browser-automation-cli --json --quiet --lang en goto https://example.com` to silence non-error stderr and force the UI language to `en` or `pt-BR`
- MUST execute `browser-automation-cli --json --verbose goto https://example.com` for info tracing and `browser-automation-cli --json --debug goto https://example.com` for maximum stderr detail
- MUST execute `browser-automation-cli --json --headless goto https://example.com` to REQUIRE headless for this run, and `browser-automation-cli --json --browser-mode headed --no-xvfb goto https://example.com` to demand a window on the current Linux display
- MUST execute `browser-automation-cli --json --min-delay-ms 1500 scrape https://example.com --format text --engine http` to raise the same-origin courtesy floor for THIS invocation only
- MUST execute `browser-automation-cli --json --allow-outside-roots --artifacts-dir /var/tmp/arts grab --path /var/tmp/p.png --format png` only as declared risk acceptance
- MUST execute `browser-automation-cli --json --dump-on-failure --artifacts-dir /tmp/arts --capture-console --capture-network goto https://example.com` to write console and network evidence when the run fails
- MUST execute `browser-automation-cli --json type "hello" --focus-only --clear` to type into the focused element without resolving a target
- MUST execute `browser-automation-cli --timeout 60 --json --mitm --mitm-har /tmp/c.har --mitm-hosts example.com --mitm-ca-dir /tmp/ca mitm capture-url https://example.com` to write HAR, narrow the TLS allowlist and place the CA
- MUST execute `browser-automation-cli --timeout 60 --json --mitm --mitm-max-body-bytes 65536 --mitm-no-media-bodies --mitm-redact-secrets --mitm-ws mitm capture-url https://example.com` to cap bodies and drop media, while `--mitm-redact-secrets` and `--mitm-ws` only restate defaults and change nothing
- NEVER pass `--mitm-no-redact-secrets` unless the secret itself is what you are debugging
- MUST pass `--verbose`|`--debug` or `config set log_level`
- MUST pass `--headed` only for debugging and `--lang en|pt-BR` to switch the output language
- MUST bypass robots ONLY with both `--ignore-robots --i-accept-robots-risk`
- MUST pass `--category-memory` (heap), `--category-extensions`, `--category-third-party` (devtools3p), `--category-webmcp`
- MUST pass `--experimental-vision` (click-at), `--experimental-screencast`
- MUST pass `--mitm` plus `--mitm-har|--mitm-hosts|--mitm-ca-dir|--mitm-ws|--mitm-max-body-bytes|--mitm-no-media-bodies|--mitm-redact-secrets|--mitm-no-redact-secrets` only when required
- `--mitm-ws` restates the default: WebSocket frames are always captured under `--mitm`, so passing it changes nothing
- MUST know secret redaction in the MITM capture is ON by default, so `--mitm-redact-secrets` restates it and changes nothing
- MUST execute `browser-automation-cli --json --mitm --mitm-no-redact-secrets mitm capture-url https://example.com` to keep Authorization and Cookie values readable for THIS run
- MUST execute `browser-automation-cli --json mitm redact --secrets false` to write the PERSISTENT policy that stops masking, and `mitm redact --secrets true` to restore it; omitting `--secrets` only SHOWS the effective policy and writes nothing
- NEVER call `--mitm-no-redact-secrets` the only route to unmasking, because the persistent `mitm redact --secrets false` reaches the same result across processes
- MUST know that passing `--mitm-redact-secrets` and `--mitm-no-redact-secrets` together resolves to MASKING, because the safe reading of a contradiction about secrets is to mask
- MUST know the default is ON because the capture lands on disk and is read back by an agent, so forgetting the flag costs a missing header while the opposite default would cost a leaked session cookie
- NEVER pass `--mitm-no-redact-secrets` unless the secret itself is what you are debugging
- MUST execute `browser-automation-cli --json --no-stealth goto https://example.com` only when the anti-detection patches must be off; stealth is ON by default
- MUST execute `browser-automation-cli --json --stealth-profile auto goto https://example.com` (`chrome-linux|chrome-win|chrome-mac` only when a foreign platform is intended)
- MUST execute `browser-automation-cli --json --stealth-seed my-fleet-42 goto https://example.com` to pin one identity across one-shot processes
- MUST execute `browser-automation-cli --json --stealth-profile list version` to list valid profiles from the binary
- MUST execute `browser-automation-cli --json doctor --fingerprint --quick` to audit webdriver/platform/screen coherence
- MUST write `run --script` as a quoted heredoc with one JSON object per physical line (never `printf` with single quotes around JS)
- MUST execute `browser-automation-cli --json --proxy socks5://127.0.0.1:1080 scrape https://example.com --format text --engine http`
- MUST execute `browser-automation-cli --json --proxy http://127.0.0.1:8080 --proxy-bypass 'localhost,127.0.0.1,*.internal' goto https://example.com`
- MUST store proxy credentials with `config set proxy_username` and `config set proxy_password`; NEVER in argv, because the process table shows argv
- MUST execute `browser-automation-cli --json --input-profile human --input-seed 7 press @e1` (`--input-profile direct` for one event per action)
- MUST execute `browser-automation-cli --json --warmup goto https://example.com/deep/page`
- MUST execute `browser-automation-cli --json --warmup-url https://example.com/login goto https://example.com/deep/page`
- MUST execute `browser-automation-cli --json --headed --no-xvfb goto https://example.com` only on Linux with a headed mode
- MUST execute `browser-automation-cli --json --expect 'ok=true' --expect 'data.title~Example' scrape https://example.com --format metadata --engine http`
- MUST execute `browser-automation-cli --json --expect 'ok=true' --expect-exit-code doctor --offline --quick` to turn an unmet expectation into exit 65

## Meta
- MUST execute `browser-automation-cli --json doctor --offline --quick` (`--fix` only when repair hints required)
- MUST execute `browser-automation-cli --json commands` · `schema goto` · `schema --cmd wait` · `version` · `locale`
- MUST execute `browser-automation-cli completions bash` (zsh|fish|elvish|powershell) · `man --out /tmp/browser-automation-cli.1`
- MUST execute `browser-automation-cli --json --browser-mode headless goto https://example.com` and read the witness keys `browser_mode_requested`, `browser_mode_effective`, `browser_mode_source`, `display_backend` and `runtime_enable_used`
- MUST execute `browser-automation-cli --json --fields browser_mode_source,display_backend goto about:blank` to prove the window mode without reading the whole envelope
- MUST read `browser_mode_requested` as what argv or XDG asked for and `browser_mode_effective` as the `headless` or `headed` the launch actually did
- MUST read `browser_mode_source` as `default`, `xdg` or `flag`, and treat `default` as headless by luck rather than as a proven requirement
- MUST read `display_backend` as `headless`, `xvfb` or `host`; only `host` reaches the operator compositor
- MUST read `runtime_enable_used` as the boolean saying whether this launch enabled the CDP Runtime domain, which turns true the moment `--capture-console` is passed
- MUST know `run` strips those five keys from every step and publishes ONE copy at the top level
- MUST read `serp_endpoint` on a `search` envelope as `known` or `unknown`; `unknown` means the configured `search_base_url` does not understand the dimension parameters

## Config XDG
- MUST execute `browser-automation-cli --json config init` · `config path` · `config show` · `config list-keys` · `config get timeout` · `config set <key> <value>` after list-keys
- MUST execute `browser-automation-cli --json config unset <KEY>` to restore one key to its built-in default
- MUST know `config unset` is the inverse of `set`, while `config set <key> ""` is NOT an inverse
- MUST know `config set <key> ""` stores on a string key an empty value the normal path never produces, and on a numeric key it is a parse error
- MUST know unsetting an already absent key succeeds, so a script never has to know the prior state
- MUST discover the live key surface with `config list-keys --json` before any `config set`
- MUST consult `references/xdg-keys.md` for every key with its default and description
- MUST set binaries with `chrome_path`, `lighthouse_path`, `ffmpeg_path`
- MUST set secrets with `encryption_key` and `openrouter_api_key`
- MUST set cache with `cache_backend sqlite|memory|redis` and plain Redis in `cache_redis_url`
- MUST set behaviour with `dialog_settle_ms`, `log_level`, `lang en|pt-BR`, `timeout`, `artifacts_dir`, `http_ssrf_mode strict|allow_loopback|off`, `log_rotation daily|hourly|never`
- MUST execute `browser-automation-cli --json config set user_data_dir /path/to/profile` as the explicit decision to GIVE UP residual-zero, because the profile becomes the operator's
- MUST know `user_data_dir` ships ABSENT, and that absence is what buys the throwaway profile a one-shot leaves nothing behind
- MUST know the residual sweep judges ONLY `browser-automation-cli-chrome-*` marker dirs under the scanned roots, so an operator profile is never counted and never collected
- MUST execute `browser-automation-cli --json config unset user_data_dir` to restore the default; `config set user_data_dir ""` also clears the opt-in, because whitespace-only reads as absent for THIS key and that is the measured EXCEPTION to the empty-string rule above
- MUST know the persistent profile directory is created 0700 on Unix, because it holds cookies and tokens
- MUST execute `browser-automation-cli --json config set browser_mode headless` to write the persistent default that `--browser-mode`, `--headless` and `--headed` override for a single run
- NEVER reintroduce an inline key list in this skill; an inline list ages and truncates the live surface
- NEVER `rediss://`; NEVER log secrets; NEVER redis without `cache_redis_url`

## Navigation Wait Snapshot Interact
- MUST execute `browser-automation-cli --timeout 60 --json goto https://example.com --init-script 'window.__x=1' --handle-before-unload accept --navigation-timeout-ms 15000`
- MUST execute `browser-automation-cli --json back` · `forward` · `reload --ignore-cache`
- MUST execute `browser-automation-cli --json wait --ms 500` · `wait --selector "h1, main, #content" --wait-timeout-ms 10000 --include-snapshot` · `wait --text Example --wait-timeout-ms 5000` · `wait --state networkidle --wait-timeout-ms 15000`
- MUST execute `browser-automation-cli --json view --detailed` · `view --path /tmp/view.txt --allow-empty` only when blank intentional
- MUST execute `browser-automation-cli --json press @e1 --dblclick --include-snapshot` · `--experimental-vision --json click-at --x 10 --y 20`
- MUST execute `browser-automation-cli --json write @e2 "hello"` · `keys Enter` · `type "hello" --target @e2 --clear --submit Enter` · `hover @e1` · `drag --from @e1 --to @e2`
- MUST execute `browser-automation-cli --json fill-form --fields-json '[{"target":"@e3","value":"x"}]'` · `upload @e4 /tmp/file.txt` · `submit "#user" --timeout-ms 8000 --include-snapshot`
- MUST execute `browser-automation-cli --json exec pick --target @e1 --option Anomaly` · `exec select-option --target @e2 --option High` · `scroll --delta-y 400 --delta-x 100`
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
- MUST execute `browser-automation-cli --json cookie list --url https://example.com` · `cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'` · `cookie clear --all`
- MUST execute `browser-automation-cli --json storage export --path /tmp/auth.json --url https://example.com` · `storage import --path /tmp/auth.json --url https://example.com`
- MUST execute `browser-automation-cli --json dialog accept --if-present` · `dialog dismiss --if-present` · `dialog accept --text Ana`
- MUST execute `browser-automation-cli --json assert url example.com --contains` · `assert text "Example" --target h1` · `--capture-console --json assert console-empty` · `assert console-no-match --pattern TypeError` · `assert console --level error --max 0`
- Step - `{"cmd":"dialog","action":"accept","if_present":true}`

## Console Net
- MUST execute `browser-automation-cli --capture-console --json console clear` · `console dump --path /tmp/console.json`; `console list`, `console get`, `net list` and `net get` are steps of `run --script` and refuse at top level with exit 2
- MUST serialize `{"cmd":"console","action":"list","types":"log,warning,error"}` and `{"cmd":"console","action":"get","id":0}` in `run --script` with `--capture-console`
- MUST serialize `{"cmd":"net","action":"list","resource_types":"Document,XHR,Fetch"}` and `{"cmd":"net","action":"get","id":0,"request_path":"/tmp/req.json"}` in `run --script`
- MUST take every `resource_types` token from the CDP vocabulary, under penalty of refusal with exit 2 — Document, Stylesheet, Image, Media, Font, Script, TextTrack, XHR, Fetch, Prefetch, EventSource, WebSocket, Manifest, SignedExchange, Ping, CSPViolationReport, Preflight, FedCM, Other
- MUST read `resourceType` on every record and `dropped_oldest` on the envelope, and move the ceiling ONLY with `config set event_tracker_max_entries <N>`
- MUST capture console/network in the SAME process as the consuming command
- MUST execute `browser-automation-cli --capture-console --json console clear` to drop every console message captured in THIS process, which is the only buffer it can reach

## Scrape Family and Local Files
- MUST execute `browser-automation-cli -q --json scrape https://example.com --format markdown --select source_url,title,markdown --max-text-chars 800 --only-main-content`
- MUST execute multi-format `scrape … --format markdown,jsonld --select source_url,title,markdown` · `--redact-pii --with-content-hash --header "Accept-Language: en"` · browser `--engine browser --wait-ms 500` · `batch-scrape --urls-file /tmp/u.txt --filter http_error=false --output-mode csv --select source_url,text` · `crawl … --sort source_url --dedup-key source_url --output-mode ndjson` · `map … --search docs --sitemap-only --limit 50`
- MUST execute `browser-automation-cli -q --json scrape https://example.com --format rawHtml --engine http`
- Formats - the 15 `--format` values are text, markdown, html, rawHtml, links, metadata, screenshot, summary, product, branding, images, jsonld, json, feed, attributes (aliases md meta body shot); accepts CSV or repeated flag
- Engines - `--engine` accepts ONLY `http` (reqwest plus scraper) and `browser` (CDP); the default comes from the XDG key `scrape_default_engine`, today `http`
- MUST execute `browser-automation-cli -q --json sitemap https://www.rust-lang.org --limit 50 --select urls,count` · narrow with `--search docs --include-path /blog --exclude-path /tag --include-subdomains --ignore-query-params --sort <FIELD> --dedup-key <FIELD>`
- MUST read `urls` as an array of STRING URLs and `count` as its length; there is no per-URL object, so NEVER project `loc` or `lastmod`
- MUST execute `browser-automation-cli -q --json feed https://blog.rust-lang.org/feed.xml --select title,source_url,feed` · add `--header "Accept-Language: en"` and `--no-cache` only when required
- MUST know `sitemap` reads the declared sitemap and `feed` reads RSS or Atom, so NEITHER launches a browser
- NEVER confuse the `feed` COMMAND with the `--format feed` value of `scrape`; they are different surfaces
- MUST treat a `search` that found no organic result as a DECLARED failure with `ok` false and `error.kind` `data`, and NEVER as success carrying `count: 0`
- MUST read `data.serp_endpoint` and `data.search_base_url` on that failure envelope, because `unknown` accuses the configuration while `known` leaves only a genuinely empty web

## Payload Reduction Applies To Every Formula Above
- MUST shrink any envelope with the eight GLOBAL flags `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content`, `--max-output-bytes`
- MUST execute `browser-automation-cli --json --fields checks --filter-rows 'id=residual_disk' --limit-rows 5 doctor --offline --quick`
- MUST root every `--fields` path at `data` and NEVER write the `data.` prefix, because the prefixed form returns an empty payload with exit 0
- MUST read `agent_ops.truncated` and `agent_ops.unresolved_paths` before trusting a reduced payload
- MUST execute `browser-automation-cli --json --fields checks --sort-rows id --dedupe-by id --limit-rows 5 doctor --offline --quick` to sort, deduplicate and cap rows in one pass
- MUST execute `browser-automation-cli --json --truncate-content 200 --max-output-bytes 4096 scrape https://example.com --format markdown --engine http` to cut every string and set a hard byte ceiling
- MUST execute `browser-automation-cli --json --fields commands --max-items 3 commands`; measured, it returns three items with `agent_ops.truncated` true
- MUST know `--max-items` is an accepted ALIAS of `--limit-rows` carrying the agent contract's own spelling, so an agent arriving from a sibling CLI does not have to relearn it
- MUST know `--max-items` limits what is EMITTED while a command's own `--limit` limits what is FETCHED, and those two ceilings are genuinely different numbers
- NEVER confuse those eight with the LOCAL `--select`, `--filter`, `--sort` and `--limit` that individual commands expose
- NEVER treat `rawHtml` as an alias of `html`; `--format html` returns the `html` key and `--format rawHtml` returns the `rawHtml` key, with distinct payloads
- MUST execute `browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2 --engine browser`
- MUST execute `browser-automation-cli --timeout 120 --json crawl https://example.com --limit 20 --max-depth 2 --format text --same-host` · `map https://example.com --limit 50` · `search "example domain" --limit 10`
- MUST execute `browser-automation-cli --json parse /tmp/doc.pdf` · `parse /tmp/sheet.ods --redact-pii` · `parse /tmp/page.html --format markdown,links,metadata` (HTML takes every scrape format; pdf/docx/sheet/csv/txt take text, markdown and summary, and refuse DOM-only formats by name with exit 2)
- MUST execute `browser-automation-cli --json find-paths --glob '**/*.rs' . --type f --limit 200` · `sg-scan . --limit 100` · `sg-rewrite .` then `--apply` after dry-run · `sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data`
- MUST execute `browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/b.baseline --write-baseline --engine http`
- MUST execute `browser-automation-cli --json qr encode --text https://example.com --format png --path /tmp/qr.png` · `qr decode --path /tmp/qr.png`
- MUST know `qr encode --format` takes png, svg or terminal and defaults to png, and that omitting `--path` writes the terminal matrix to stdout instead of a file
- MUST execute `browser-automation-cli --json find-paths --glob '**/*.rs' . --type f --limit 200 --max-depth 4 --extension rs --hidden --no-ignore` to enumerate local paths with no browser launched
- MUST execute `browser-automation-cli --json image info --path /tmp/a.png --select format,width,height,sha256` · `image convert --path /tmp/a.png --format webp -o /tmp/a.webp` · `image download https://example.com/a.png -o /tmp/a.png` · `image resize --path /tmp/a.png --width 640 --keep-aspect -o /tmp/a-640.webp --format webp --quality 80`
- MUST execute `browser-automation-cli --json video info --path /tmp/in.mp4 --select format,bytes,path` (aliases → container/size_bytes) · `video convert --path /tmp/in.mp4 --format webm -o /tmp/out.webm --select path_out,auto_reencoded,bytes_out` · `video to-mp3 --path /tmp/in.mp4 -o /tmp/a.mp3` · `video trim --path /tmp/in.mp4 --start 0 --duration 0.5 -o /tmp/clip.mp4` · `video thumbnail --path /tmp/in.mp4 --at 0 -o /tmp/thumb.png` · `--timeout 120 video download https://example.com/v.mp4 -o /tmp/v.mp4 --max-bytes 52428800 --require-video`
- MUST set video caps via XDG after list-keys: `video_max_input_bytes` `video_download_max_bytes` `video_default_container` `video_default_crf` `video_default_audio_bitrate` `ffmpeg_path` `ffmpeg_timeout_secs`
- MUST execute `browser-automation-cli --json audio info --path /tmp/in.wav --select format,codec,duration,bytes,sha256` · `audio convert --path /tmp/in.wav --format mp3 -o /tmp/a.mp3` · `audio convert --path /tmp/clip.mp4 --format m4a -o /tmp/a.m4a` · `audio trim --path /tmp/a.mp3 --start 1 --duration 5 -o /tmp/cut.mp3` · `audio download https://example.com/a.mp3 -o /tmp/a.mp3` · then `upload @e1 /tmp/a.mp3`
- MUST set audio caps via XDG after list-keys: `audio_max_input_bytes` `audio_download_max_bytes` `audio_default_format` `audio_default_bitrate` `ffmpeg_path` `ffmpeg_timeout_secs`
- NEVER dump media bytes/base64 on stdout; path→path only; NEVER claim HLS/yt-dlp/pure encode as product
- Steps - `{"cmd":"scrape","url":"https://example.com","format":"text"}` · `{"cmd":"scrape","url":"https://example.com","formats":"markdown,links"}`
- MUST execute `browser-automation-cli --json scrape https://example.com --format text --engine http` (robots default); bypass ONLY with both `--ignore-robots --i-accept-robots-risk`

## Emulate Perf Lighthouse Screencast Heap
- MUST execute `browser-automation-cli --json emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G" --color-scheme dark`
- MUST execute `browser-automation-cli --json resize --width 1280 --height 720`
- MUST execute `browser-automation-cli --json perf start --reload --auto-stop --path /tmp/trace.json` · `perf stop --path /tmp/trace.json` · `perf insight --name DocumentLatency --insight-set-id <set-id>`
- MUST execute `browser-automation-cli --json perf insight --path /tmp/trace.json` to analyse a saved trace OFFLINE, with no browser launched; the path is bounded by the allowed roots, so a trace outside them is refused with `read path outside allowed roots`
- NEVER combine `--path` with `--insight-set-id`: an offline trace has no insight sets, and the pair is REFUSED with a usage error rather than silently analysing the whole file
- MUST execute `browser-automation-cli --timeout 180 --json lighthouse https://example.com --out-dir /tmp/lh --device desktop --mode navigation` then read `data.binary_source` · `--experimental-screencast --json screencast start --path /tmp/cast` · `screencast stop --path /tmp/cast.webm`
- MUST execute `browser-automation-cli --category-memory --json heap take --path /tmp/s.heapsnapshot` · `heap close --path /tmp/s.heapsnapshot` · `heap summary --path /tmp/s.heapsnapshot` · `heap compare --base /tmp/a.heapsnapshot --current /tmp/b.heapsnapshot`
- MUST execute `browser-automation-cli --category-memory --json heap details --path /tmp/s.heapsnapshot --filter-name Array` · `heap class-nodes --path /tmp/s.heapsnapshot --id 7` · `heap dup-strings --path /tmp/s.heapsnapshot`
- MUST execute `browser-automation-cli --category-memory --json heap dominators --path /tmp/s.heapsnapshot --node 42` · `heap edges --path /tmp/s.heapsnapshot --node 42` · `heap retainers --path /tmp/s.heapsnapshot --node 42` · `heap object-details --path /tmp/s.heapsnapshot --node 42` · `heap paths --path /tmp/s.heapsnapshot --node 42 --max-depth 8`
- MUST execute `browser-automation-cli --json lighthouse https://example.com --out-dir /tmp/lh` as the minimal audit, then read `data.binary_source`
- MUST execute `browser-automation-cli --json lighthouse https://example.com --lighthouse-path /usr/local/bin/lighthouse --out-dir /tmp/lh --device mobile --mode navigation` when the binary is off PATH; `--lighthouse-path` overrides both PATH and the XDG `lighthouse_path`
- MUST know `--device` takes desktop or mobile and defaults to desktop, while `--mode` takes navigation or snapshot and defaults to navigation
- MUST execute `browser-automation-cli --json perf start --path /tmp/trace.json --reload --auto-stop` to arm the trace, reload the page and stop without a second call
- MUST execute `browser-automation-cli --category-memory --json heap take --path /tmp/s.heapsnapshot --url https://example.com` to capture from the live page; `--path` is REQUIRED and `--url` opens the page first
- MUST execute `browser-automation-cli --category-memory --json heap details --path /tmp/s.heapsnapshot --filter-name Array --page-idx 0 --page-size 50` to page through class details
- MUST execute `browser-automation-cli --experimental-screencast --json screencast start --path /tmp/cast` and close the pair with `screencast stop --path /tmp/cast.webm`
- NEVER `emulate --device`; NEVER use `--node-id` (flag is `--node`)

## Extensions Third-party WebMCP MITM Workflow
- MUST execute `browser-automation-cli --category-extensions --json extension list` · `extension install /tmp/ext` · `extension reload <ext-id>` · `extension trigger <ext-id>` · `extension uninstall <ext-id>`
- MUST execute `browser-automation-cli --category-third-party --json devtools3p list --url https://example.com` · `devtools3p exec ToolName --params '{}' --url https://example.com`
- MUST execute `browser-automation-cli --category-webmcp --json webmcp list --url https://example.com` · `webmcp exec ToolName --input '{}' --url https://example.com`
- MUST execute `browser-automation-cli --json mitm init-ca` · `mitm start --seconds 30` · `mitm status`
- MUST execute `browser-automation-cli --json mitm capture-url https://example.com --har /tmp/c.har --hosts example.com`
- MUST execute `browser-automation-cli --json mitm list --limit 50` · `mitm get 0` · `mitm har --out /tmp/c.har` · `mitm export --format ndjson --out /tmp/c.ndjson` · `mitm domains|apis|graphql` · `mitm ws list --limit 50` · `mitm ws get 0`
- MUST execute `browser-automation-cli --json mitm block --host example.com --path /ads` · `mitm allow --host example.com` · `mitm redact --secrets true`
- MUST execute `browser-automation-cli --json workflow run --manifest /tmp/wf.json --journal /tmp/wf.journal` · `workflow resume --manifest /tmp/wf.json` · `workflow status --name demo`
- MUST execute `browser-automation-cli --category-extensions --json extension list`; measured without that gate the envelope is `ok` false with `error.kind` `capability-disabled` and the message `extension requires --category-extensions`
- MUST execute `browser-automation-cli --category-third-party --json devtools3p list` with no `--url` to discover on the page already open, and add `--url https://example.com` to open one first
- MUST execute `browser-automation-cli --category-webmcp --json webmcp list --url https://example.com` to enumerate the WebMCP tools the page declares
- MUST execute `browser-automation-cli --json mitm init-ca` ONCE to create the local CA under XDG data before any `mitm start` or `mitm capture-url`
- MUST execute `browser-automation-cli --json mitm redact` to SHOW the redact-secrets policy and `mitm redact --secrets true` to set it
- MUST know there is NO `mitm config` subcommand; measured, asking for one returns `ok` false with `error.kind` `usage` and `unrecognized subcommand 'config'`
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
- MUST serialize `{"cmd":"net","action":"list","resource_types":"Document,XHR,Fetch","page_size":50}` in `run --script`
- `net list` only sees traffic with `--capture-network` in the SAME process
- MUST paginate with the STEP keys `page_idx` and `page_size`, and include preserved entries with `include_preserved`; the argv flags of the same name exist but are unreachable, because the top-level form refuses first
- MUST serialize `{"cmd":"net","action":"get","id":0,"request_path":"/tmp/req.json","response_path":"/tmp/res.json"}` in `run --script`
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
- An unknown key on a `run` step is rejected (`deny_unknown_fields`): `ok` false, `error.kind` `usage`, exit 2
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
