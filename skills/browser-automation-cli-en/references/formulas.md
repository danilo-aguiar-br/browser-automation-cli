# browser-automation-cli Ready Formulas


## Formula Contract
- MUST copy each formula literally and swap ONLY the example value
- MUST ALWAYS invoke the full binary `browser-automation-cli` with `--json`
- MUST check the exit code, require `ok` equal to true and only then read `data`
- MUST check each flag in `browser-automation-cli <cmd> --help` and each step key in `schema <cmd>` before adapting
- MUST pass an explicit `--timeout` on every formula that opens a browser
- MUST run `pick` and `select-option` ONLY through `exec` or a `run` step, because as a top-level subcommand they exit 2
- MUST run `console list`, `console get`, `net list` and `net get` ONLY as a `run` step, because at top level they exit 2
- MUST know an unknown key on a `run` or `exec` step is REFUSED with exit 2 and `error.kind` equal to `usage` before the browser launches
- MUST keep every step with an `@eN` ref inside ONE single `run --script`, because the ref dies with the process
- MUST bypass robots ONLY with BOTH `--ignore-robots` and `--i-accept-robots-risk`
- NEVER run `config set chrome_legacy_oxide_launch true`, because that path reopens an unauthenticated DevTools port and draws the window on the operator display
- NEVER embed `mitm`, `storage`, `config`, `workflow`, `extension install` or `extension uninstall` inside `run`


## Meta and Discovery
- RUN `browser-automation-cli --json doctor --offline --quick` and READ `data.residual` and the `residual_disk` check
- RUN `browser-automation-cli --json --fields checks --filter-rows 'id=virtual_display' doctor --offline --quick` and READ `browser_mode_auto_resolves`, NEVER the check message
- RUN `browser-automation-cli --json --timeout 60 doctor --fingerprint` and READ `launch_args`, which carries the real argv handed to Chrome and is `null` before any launch
- RUN `browser-automation-cli --json --timeout 60 doctor --fix` and APPLY the repair hint attached to each failing check
- RUN `browser-automation-cli --json commands --detail` and READ the description, category and surfaces of each command
- RUN `browser-automation-cli --json schema goto` and READ the keys the command accepts
- RUN `browser-automation-cli --json schema --cmd run` as the equivalent flag form
- RUN `browser-automation-cli --json version` and READ the binary version
- RUN `browser-automation-cli --json locale` and READ the resolved language
- RUN `browser-automation-cli --json completions bash` and SWAP `bash` for `zsh`, `fish`, `elvish` or `powershell` to match the shell
- RUN `browser-automation-cli --json man --out /tmp/browser-automation-cli.1` and READ the written roff page


## XDG Configuration
- RUN `browser-automation-cli --json config path` and READ the resolved XDG paths, NEVER invent a path
- RUN `browser-automation-cli --json config init` to create the XDG layout and the default `config.toml`
- RUN `browser-automation-cli --json config show` and READ the effective values
- RUN `browser-automation-cli --json config list-keys` and READ every accepted key with its default before any `config set`
- RUN `browser-automation-cli --json config get timeout` and READ the value of ONE key
- RUN `browser-automation-cli --json config get` and READ the whole set when no key is passed
- RUN `browser-automation-cli --json config set dialog_settle_ms 2000` and CHECK the key in `config list-keys` before writing
- RUN `browser-automation-cli --json config set proxy_username operator` and STORE proxy credentials ONLY through `proxy_username` and `proxy_password`, NEVER in argv
- RUN `browser-automation-cli --json config set browser_mode headless` to write the persistent default that `--browser-mode`, `--headless` and `--headed` override for one run
- RUN `browser-automation-cli --json config set user_data_dir /tmp/persistent-profile` ONLY as the explicit decision to give up residual-zero
- RUN `browser-automation-cli --json config unset user_data_dir` to restore the absent default that preserves residual-zero
- RUN `browser-automation-cli --json config unset dialog_settle_ms` to return the key to its built-in default, NEVER `config set` with an empty string
- RUN `browser-automation-cli --json --fields keys --filter-rows 'key=chrome_legacy_oxide_launch' config list-keys` and REQUIRE `default` equal to `false`, while NEVER writing `true` to that key


## Global Flags
- RUN `browser-automation-cli --json --json-steps --timeout 90 --step-timeout 20 run --script /tmp/steps.jsonl` and READ one NDJSON object per step on stdout
- RUN `browser-automation-cli --json --quiet --plain --lang pt-BR --correlation-id request-42 version` and READ `correlation_id` echoed on the envelope
- RUN `browser-automation-cli --json -q --lang en version` and KNOW bare `pt` is refused and machine JSON stays in English
- RUN `browser-automation-cli --json --verbose doctor --offline --quick` for info-level stderr
- RUN `browser-automation-cli --json --debug doctor --offline --quick` for maximum tracing detail on stderr
- RUN `browser-automation-cli --json --timeout 120 --max-concurrency 4 batch-scrape --urls-file /tmp/urls.txt --format text` to cap I/O parallelism
- RUN `browser-automation-cli --json --timeout 60 --browser-mode auto goto https://example.com` and READ `browser_mode_requested`, `browser_mode_effective`, `browser_mode_source` and `display_backend`
- RUN `browser-automation-cli --json --timeout 60 --headless goto https://example.com` to REQUIRE headless for this run above any stored `browser_mode`
- RUN `browser-automation-cli --json --timeout 60 --headed goto https://example.com` and KNOW that on Linux with Xvfb the window is drawn inside the private Xvfb, off the operator screen, and `display_backend` reads `xvfb`
- RUN `browser-automation-cli --json --timeout 60 --headed --no-xvfb goto https://example.com` as the ONLY route to see the window on the current display and READ `display_backend` equal to `host`
- RUN `browser-automation-cli --json --timeout 60 --no-stealth goto https://example.com` to turn off the anti-detection patches for this run
- RUN `browser-automation-cli --json --timeout 60 --stealth-profile auto --stealth-seed fleet-42 goto https://example.com` to pin the same identity across processes
- RUN `browser-automation-cli --json --stealth-profile list version` to list the `auto`, `chrome-linux`, `chrome-win` and `chrome-mac` profiles
- RUN `browser-automation-cli --json --timeout 60 --warmup goto https://example.com/deep/page` to visit the origin root before the target
- RUN `browser-automation-cli --json --timeout 60 --warmup-url https://example.com/login goto https://example.com/deep/page` to warm another URL instead of the root
- RUN `browser-automation-cli --json --timeout 60 --input-profile human --input-seed 7 run --script /tmp/steps.jsonl` to reproduce the same jitter and SWAP to `--input-profile direct` for one event per action
- RUN `browser-automation-cli --json --timeout 60 --artifacts-dir /tmp/artifacts --dump-on-failure --capture-console --capture-network run --script /tmp/steps.jsonl` to write console and network evidence on failure
- RUN `browser-automation-cli --json --timeout 60 --min-delay-ms 1500 scrape https://example.com --format text` and KNOW the effective wait is the MAXIMUM of the flag, `scrape_min_delay_ms` and `Crawl-delay`
- RUN `browser-automation-cli --json --timeout 60 --proxy socks5://127.0.0.1:1080 --proxy-bypass 'localhost,127.0.0.1' scrape https://example.com --format text` and NEVER put a user or password in the proxy URL
- RUN `browser-automation-cli --json --allow-outside-roots parse /var/tmp/report.pdf` ONLY as explicit risk acceptance to read outside the allowed roots
- RUN `browser-automation-cli --json --timeout 60 --ignore-robots --i-accept-robots-risk scrape https://example.com --format text` with BOTH flags, because one alone does NOT bypass robots
- RUN `browser-automation-cli --json --timeout 120 --category-memory heap take --path /tmp/s.heapsnapshot --url https://example.com` to unlock the `heap` family
- RUN `browser-automation-cli --json --timeout 60 --category-extensions extension list` to unlock the `extension` family
- RUN `browser-automation-cli --json --timeout 60 --category-third-party devtools3p list --url https://example.com` to unlock the `devtools3p` family
- RUN `browser-automation-cli --json --timeout 60 --category-webmcp webmcp list --url https://example.com` to unlock the `webmcp` family
- RUN `browser-automation-cli --json --timeout 60 --experimental-vision click-at --x 10 --y 20` to unlock `click-at`
- RUN `browser-automation-cli --json --timeout 60 --experimental-screencast screencast start --path /tmp/cast` to unlock `screencast`
- RUN `browser-automation-cli --json --timeout 60 --mitm --mitm-har /tmp/c.har --mitm-hosts example.com --mitm-ca-dir /tmp/ca goto https://example.com` to write HAR, narrow decryption and place the CA
- RUN `browser-automation-cli --json --timeout 60 --mitm --mitm-max-body-bytes 65536 --mitm-no-media-bodies --mitm-ws --mitm-redact-secrets goto https://example.com` and KNOW `--mitm-ws` and `--mitm-redact-secrets` only restate the default
- RUN `browser-automation-cli --json --timeout 60 --mitm --mitm-no-redact-secrets goto https://example.com` ONLY when the secret itself is what you debug
- RUN `browser-automation-cli --json --fields checks --filter-rows 'status!=pass' --sort-rows id --dedupe-by id --limit-rows 5 doctor --offline --quick` and READ `agent_ops.truncated`
- RUN `browser-automation-cli --json --fields checks --count-only doctor --offline --quick` and READ the count instead of the rows
- RUN `browser-automation-cli --json --timeout 60 --truncate-content 200 --max-output-bytes 4096 scrape https://example.com --format markdown` and READ `agent_ops.truncated` and `agent_ops.unresolved_paths`
- RUN `browser-automation-cli --json --fields checks --filter-rows 'id=residual_disk' --expect 'status=pass' --expect-exit-code doctor --offline --quick` to exit 65 when the expectation fails


## Navigation
- RUN `browser-automation-cli --json --timeout 60 goto https://example.com --init-script 'window.__ready=1' --handle-before-unload accept --navigation-timeout-ms 15000` and READ `browser_mode_effective`
- RUN `browser-automation-cli --json --timeout 60 back` to go back in the process history
- RUN `browser-automation-cli --json --timeout 60 forward` to go forward in the process history
- RUN `browser-automation-cli --json --timeout 60 reload --ignore-cache --init-script 'window.__ready=1' --handle-before-unload dismiss` and NEVER pass `--ignore-cache` to `goto`
- RUN `browser-automation-cli --json --timeout 60 wait --ms 500 --text Example --selector 'h1, main' --min-count 1 --state load --wait-timeout-ms 10000 --include-snapshot` and READ `matched_selector`
- RUN `browser-automation-cli --json --timeout 60 wait --network-idle 500 --dom-stable 300 --wait-timeout-ms 15000` to wait for an idle network and a stable DOM


## Interaction
- RUN `browser-automation-cli --json --timeout 60 press @e1 --dblclick --include-snapshot` and READ the attached snapshot
- RUN `browser-automation-cli --json --timeout 60 --experimental-vision click-at --x 120 --y 340 --dblclick --include-snapshot` to click by CSS coordinate
- RUN `browser-automation-cli --json --timeout 60 write @e2 'hello' --include-snapshot` to fill an input, select, checkbox or radio
- RUN `browser-automation-cli --json keys Enter --timeout 60 --include-snapshot` to press one key
- RUN `browser-automation-cli --json --timeout 60 type 'hello world' --target @e2 --clear --submit Enter --include-snapshot` to type into a target
- RUN `browser-automation-cli --json --timeout 60 type 'hello world' --focus-only` to type into the ALREADY focused element without resolving a target
- RUN `browser-automation-cli --json --timeout 60 hover @e1 --include-snapshot` to hover the pointer over the target
- RUN `browser-automation-cli --json --timeout 60 drag --from @e1 --to @e2 --anchor before --synthetic-payload '{"items":[{"mimeType":"text/plain","data":"x"}],"dragOperationsMask":1}' --include-snapshot` and KNOW the synthetic payload bypasses the page `dragstart`
- RUN `browser-automation-cli --json --timeout 60 drag --from @e1 --to-x 400 --to-y 220` to drop at an absolute coordinate
- RUN `browser-automation-cli --json --timeout 60 submit '#login' --timeout-ms 8000 --include-snapshot` to submit the form and wait for navigation or a request
- RUN `browser-automation-cli --json --timeout 60 fill-form --fields-json '[{"target":"@e3","value":"x"}]' --include-snapshot` and NEVER pass the payload through `--json`
- RUN `browser-automation-cli --json --timeout 60 upload @e4 /tmp/file.txt --include-snapshot` to attach a file to a file input
- RUN `browser-automation-cli --json --timeout 60 scroll --target @e5 --delta-x 100 --delta-y 400 --include-snapshot` to scroll by delta
- RUN `browser-automation-cli --json --timeout 60 scroll --to-x 0 --to-y 2000` to scroll the window to an absolute offset
- RUN `browser-automation-cli --json --timeout 60 exec pick --target @e1 --option Anomaly` and READ `via`
- RUN `browser-automation-cli --json --timeout 60 exec select-option --target @e2 --option High` and READ `via` equal to `native_select` on a native select
- RUN `browser-automation-cli --json --timeout 90 run --script /tmp/choice.jsonl` with the lines `{"cmd":"goto","url":"https://example.com"}` and `{"cmd":"view"}` and `{"cmd":"pick","target":"@e1","option":"Anomaly"}` and `{"cmd":"select-option","target":"@e2","option":"High"}`


## Reading and Artifacts
- RUN `browser-automation-cli --json --timeout 60 view --detailed --path /tmp/tree.txt` and NEVER use `view --verbose`
- RUN `browser-automation-cli --json --timeout 60 view --allow-empty` ONLY when the blank snapshot is intentional
- RUN `browser-automation-cli --json --timeout 60 text @e1` and READ the visible text of the target
- RUN `browser-automation-cli --json --timeout 60 attr @e1 href` and READ the attribute value
- RUN `browser-automation-cli --json --timeout 60 extract @e1 --attr href` to read an attribute instead of text
- RUN `browser-automation-cli --json --timeout 120 extract --url https://example.com --llm --question 'What is the title?' --schema-json /tmp/schema.json` with `openrouter_api_key` stored in XDG
- RUN `browser-automation-cli --json --timeout 60 eval '(el)=>el.textContent' --args '["@e1"]' --dialog-action accept --file-path /tmp/eval.json --typed` and READ `data.value` and `data.value_type`
- RUN `browser-automation-cli --json --timeout 60 --category-extensions eval 'chrome.runtime.id' --service-worker-id sw-1` to evaluate inside the extension service worker
- RUN `browser-automation-cli --json --timeout 60 grab --path /tmp/page.png --format png --full-page` and NEVER use a positional path
- RUN `browser-automation-cli --json --timeout 60 grab --path /tmp/element.webp --format webp --quality 80 --element @e1 --include-base64` ONLY when the agent needs base64 in the envelope
- RUN `browser-automation-cli --json --timeout 60 print-pdf --path /tmp/page.pdf --url https://example.com` with `--url` ALWAYS in one-shot
- RUN `browser-automation-cli --json --timeout 60 assert url example.com --contains` to assert a URL substring
- RUN `browser-automation-cli --json --timeout 60 assert text Example --target h1` to assert text inside the target


## Tabs Cookies Storage and Dialogs
- RUN `browser-automation-cli --json --timeout 60 page` and READ the URL and title of the current page
- RUN `browser-automation-cli --json --timeout 60 page info` as the explicit form of the same report
- RUN `browser-automation-cli --json --timeout 60 page list` to list the tabs of this process
- RUN `browser-automation-cli --json --timeout 60 page new --url https://example.com --background --isolated-context session-a` to open a tab in an isolated context without focus
- RUN `browser-automation-cli --json --timeout 60 page select 0 --bring-to-front` to select the tab and raise its window
- RUN `browser-automation-cli --json --timeout 60 page select --page-id 1 --no-bring-to-front` to select the tab without raising its window
- RUN `browser-automation-cli --json --timeout 60 page close --index 1` to close the tab by index
- RUN `browser-automation-cli --json --timeout 60 page close --page-id 1` as the `pageId` form
- RUN `browser-automation-cli --json --timeout 60 page tab-id` and READ the stable id of the active tab
- RUN `browser-automation-cli --json --timeout 60 cookie list --url https://example.com` to list cookies scoped to the URL
- RUN `browser-automation-cli --json --timeout 60 cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'` and NEVER pass the payload through `--json`
- RUN `browser-automation-cli --json --timeout 60 cookie clear --all` to clear the whole jar of this process
- RUN `browser-automation-cli --json --timeout 60 storage export --path /tmp/auth.json --url https://example.com` and KNOW the file is created with mode 0600
- RUN `browser-automation-cli --json --timeout 60 storage import --path /tmp/auth.json --url https://example.com` to restore the origin cookies and storage
- RUN `browser-automation-cli --json --timeout 60 dialog accept --text Ana --if-present` and READ `data.dialog_settled`, with no artificial wait when it is true
- RUN `browser-automation-cli --json --timeout 60 dialog dismiss --if-present` to dismiss the dialog without failing when none is open


## Console and Network
- RUN `browser-automation-cli --json --timeout 90 --capture-console run --script /tmp/console.jsonl` with the lines `{"cmd":"goto","url":"https://example.com"}` and `{"cmd":"console","action":"list","page_idx":0,"page_size":50,"types":"log,warning,error","include_preserved":true,"service_worker_id":"sw-1"}` and READ `dropped_oldest`
- RUN `browser-automation-cli --json --timeout 90 --capture-console run --script /tmp/console.jsonl` with the lines `{"cmd":"goto","url":"https://example.com"}` and `{"cmd":"console","action":"get","id":0,"include_preserved":true}` to read ONE message by the same index as the list
- RUN `browser-automation-cli --json --timeout 60 --capture-console console clear` to drop the messages captured in this process
- RUN `browser-automation-cli --json --timeout 60 --capture-console console dump --path /tmp/console.json` to write the captured messages to a file
- RUN `browser-automation-cli --json --timeout 90 --capture-network run --script /tmp/network.jsonl` with the lines `{"cmd":"goto","url":"https://example.com"}` and `{"cmd":"net","action":"list","page_idx":0,"page_size":50,"resource_types":"Document,XHR,Fetch","include_preserved":true}` and READ `resourceType` on every record
- RUN `browser-automation-cli --json --timeout 90 --capture-network run --script /tmp/network.jsonl` with the lines `{"cmd":"goto","url":"https://example.com"}` and `{"cmd":"net","action":"get","id":"0","request_path":"/tmp/req.bin","response_path":"/tmp/res.bin","include_preserved":true}` to write the request bodies
- RUN `browser-automation-cli --json --timeout 60 --capture-console assert console --level error --max 0` in the SAME process as the capture
- RUN `browser-automation-cli --json --timeout 60 --capture-console assert console-empty` to require zero messages of any level
- RUN `browser-automation-cli --json --timeout 60 --capture-console assert console-no-match --pattern 'TypeError|ReferenceError'` to require that no message matches the regex


## Scrape and Collection
- RUN `browser-automation-cli --json --timeout 60 scrape https://example.com --format markdown,links,metadata,images,jsonld --engine http --only-main-content --select source_url,title,markdown --max-text-chars 800` and READ `unsupported_format` before the requested key
- RUN `browser-automation-cli --json --timeout 60 scrape https://example.com --format html,rawHtml,text --include-selector main --exclude-selector nav --redact-pii --with-content-hash --header 'Accept-Language: en' --no-cache true` and READ `html` and `rawHtml` as DISTINCT keys
- RUN `browser-automation-cli --json --timeout 120 scrape https://example.com --format screenshot,summary,product,branding --engine browser --wait-ms 500 --action '{"cmd":"press","target":"#load-more"}'` to act on the page before collecting
- RUN `browser-automation-cli --json --timeout 120 scrape https://example.com --format json --schema-json /tmp/schema.json --question 'What is the price?'` with `openrouter_api_key` stored in XDG
- RUN `browser-automation-cli --json --timeout 60 scrape https://example.com --format attributes --attribute-selector a --attribute-name href --webhook-url http://127.0.0.1:8787/hook` and PAIR each selector with one name in the same order
- RUN `browser-automation-cli --json --timeout 60 scrape https://example.com/feed.xml --format feed` and KNOW `--format` accepts the 15 values text, markdown, html, rawHtml, links, metadata, screenshot, summary, product, branding, images, jsonld, json, feed and attributes
- RUN `browser-automation-cli --json --timeout 120 batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2 --engine http --only-main-content --select source_url,text --max-text-chars 500 --filter http_error=false --output-mode csv --sort source_url --dedup-key source_url` for a closed list of URLs
- RUN `browser-automation-cli --json --timeout 180 batch-scrape --urls-file /tmp/urls.txt --format markdown --engine browser --output-mode ndjson --dedup-similar true --include-selector article --exclude-selector footer --redact-pii --with-content-hash --webhook-url http://127.0.0.1:8787/hook` for pages that depend on JavaScript
- RUN `browser-automation-cli --json --timeout 180 crawl https://example.com --limit 20 --max-depth 2 --format text --same-host --engine http --select source_url,text --max-text-chars 500 --filter http_error=false --output-mode ndjson --sort source_url --dedup-key source_url --only-main-content --include-selector main --exclude-selector nav --redact-pii --with-content-hash` to discover pages from the seed
- RUN `browser-automation-cli --json --timeout 180 crawl https://example.com --no-same-host --include-path /docs --exclude-path /tag --include-regex 'guide' --exclude-regex 'draft' --use-sitemap true --ignore-query-params --follow-rel-next true --dedup-similar true --output-mode llms-txt --webhook-url http://127.0.0.1:8787/hook` to build a site summary
- RUN `browser-automation-cli --json --timeout 60 crawl https://example.com --sitemap-only --dry-run` and READ the effective plan without fetching anything
- RUN `browser-automation-cli --json --timeout 120 map https://example.com --limit 50 --max-depth 2 --select urls,count --include-path /docs --exclude-path /tag --use-sitemap true --search guide --sort url --dedup-key url --include-subdomains --ignore-query-params` and READ `urls` and `count`
- RUN `browser-automation-cli --json --timeout 60 map https://example.com --sitemap-only` to list ONLY sitemap URLs
- RUN `browser-automation-cli --json --timeout 60 sitemap https://www.rust-lang.org --limit 50 --select urls,count --include-path /learn --exclude-path /tag --search guide --sort url --dedup-key url --include-subdomains --ignore-query-params` and READ `urls` as a list of strings
- RUN `browser-automation-cli --json --timeout 60 feed https://blog.rust-lang.org/feed.xml --select title,source_url,feed --header 'Accept-Language: en' --no-cache true` to read RSS, Atom or JSON Feed with no browser
- RUN `browser-automation-cli --json --timeout 60 search 'example domain' --limit 10 --select results --sort url --dedup-key url --include-domains example.com --country br --search-lang pt --time-filter w` and READ `serp_endpoint`
- RUN `browser-automation-cli --json --timeout 60 search 'example domain' --exclude-domains pinterest.com` and TREAT `ok` false with `error.kind` equal to `data` as a search with no organic result, reading `data.serp_endpoint` and `data.search_base_url`
- RUN `browser-automation-cli --json parse /tmp/document.pdf --redact-pii --format text,markdown,summary` to extract text from pdf, docx, xlsx or ods with no browser
- RUN `browser-automation-cli --json parse /tmp/page.html --format markdown,links,metadata` and KNOW HTML accepts every scrape format


## Local Tools
- RUN `browser-automation-cli --json find-paths '\.rs$' /tmp/project --extension rs --hidden --no-ignore --max-depth 4 --type f --limit 200` to enumerate local paths with no browser
- RUN `browser-automation-cli --json find-paths --glob '**/*.toml' /tmp/project --type f` to filter by glob
- RUN `browser-automation-cli --json sg-scan /tmp/project --limit 100` and READ the structural findings
- RUN `browser-automation-cli --json sg-rewrite /tmp/project` for the dry-run report before writing
- RUN `browser-automation-cli --json sg-rewrite /tmp/project --apply` ONLY after reviewing the dry-run


## Image
- RUN `browser-automation-cli --json image info --path /tmp/a.jpg --include-gps --select format,width,height,sha256,exif` and READ the dimensions and sha256
- RUN `browser-automation-cli --json image info --paths-file /tmp/images.txt --select path,format,width,height` to inspect a batch
- RUN `browser-automation-cli --json image info --stdin --select format,bytes` with the image bytes on stdin
- RUN `browser-automation-cli --json image convert --path /tmp/a.png --format jpeg --quality 85 -o /tmp/a.jpg --strip-exif` and NEVER request avif or heic
- RUN `browser-automation-cli --json image convert --stdin --format webp --out /tmp/a.webp --keep-exif` and READ `keep_exif_honored` and `quality_applied`
- RUN `browser-automation-cli --json image convert --paths-file /tmp/images.txt --format png` to convert a batch
- RUN `browser-automation-cli --json image resize --path /tmp/a.png --width 640 --keep-aspect -o /tmp/a-640.jpg --format jpeg --quality 80` to resize pixels
- RUN `browser-automation-cli --json image resize --stdin --width 320 --height 240 --out /tmp/b.png` with the bytes on stdin
- RUN `browser-automation-cli --json image resize --paths-file /tmp/images.txt --width 1024 --keep-aspect` to resize a batch
- RUN `browser-automation-cli --json --timeout 60 image download https://example.com/a.png -o /tmp/a.png --max-bytes 10485760 --require-image` to download with magic verification
- RUN `browser-automation-cli --json --timeout 60 image download https://example.com/data.bin --out /tmp/data.bin --allow-non-image` ONLY for intentional raw bytes
- RUN `browser-automation-cli --json image exif --path /tmp/a.jpg --include-gps --select path,count,exif` and READ the EXIF tags
- RUN `browser-automation-cli --json image exif --stdin --select tags` with the bytes on stdin
- RUN `browser-automation-cli --json image exif --paths-file /tmp/images.txt --select path,tag_count` for a batch


## Video
- RUN `browser-automation-cli --json video info --path /tmp/in.mp4 --select container,duration_secs,streams,sha256` and READ the streams with no media dump
- RUN `browser-automation-cli --json video info --stdin --select container,duration` with the video on stdin
- RUN `browser-automation-cli --json video info --paths-file /tmp/videos.txt --select path,container` for a batch
- RUN `browser-automation-cli --json --timeout 120 video download https://example.com/v.mp4 -o /tmp/v.mp4 --max-bytes 52428800 --require-video --select path,bytes` to download direct media
- RUN `browser-automation-cli --json --timeout 120 video download https://example.com/v.bin --out /tmp/v.bin --allow-non-video` ONLY for an intentional non-video body
- RUN `browser-automation-cli --json --timeout 300 video convert --path /tmp/in.mov --format mp4 -o /tmp/out.mp4 --video-codec h264 --audio-codec aac --crf 23 --no-faststart --strip-metadata --select path_out,auto_reencoded,video_codec` and READ `auto_reencoded`
- RUN `browser-automation-cli --json --timeout 300 video convert --stdin --format webm --out /tmp/out.webm --drop-audio` with the video on stdin
- RUN `browser-automation-cli --json --timeout 600 video convert --paths-file /tmp/videos.txt --format mkv` to convert a batch
- RUN `browser-automation-cli --json --timeout 300 video to-mp3 --path /tmp/in.mp4 -o /tmp/a.mp3 --bitrate 192k --audio-stream 0 --select path_out` to extract the audio
- RUN `browser-automation-cli --json --timeout 300 video to-mp3 --stdin --out /tmp/b.mp3` with the video on stdin
- RUN `browser-automation-cli --json --timeout 600 video to-mp3 --paths-file /tmp/videos.txt` for a batch
- RUN `browser-automation-cli --json --timeout 300 video trim --path /tmp/in.mp4 --start 5 --duration 10 -o /tmp/cut.mp4 --format mp4 --video-codec copy --audio-codec copy --select path_out,duration` to cut by duration
- RUN `browser-automation-cli --json --timeout 300 video trim --stdin --start 5 --to 15 --out /tmp/cut.webm` to cut up to an instant
- RUN `browser-automation-cli --json --timeout 600 video trim --paths-file /tmp/videos.txt --start 0 --duration 30` for a batch
- RUN `browser-automation-cli --json --timeout 120 video thumbnail --path /tmp/in.mp4 --at 3 -o /tmp/frame.png --select path_out` to extract one frame
- RUN `browser-automation-cli --json --timeout 120 video thumbnail --stdin --out /tmp/frame.jpg` with the video on stdin
- RUN `browser-automation-cli --json --timeout 300 video thumbnail --paths-file /tmp/videos.txt --at 1` for a batch
- RUN `browser-automation-cli --json video manifest --path /tmp/playlist.m3u8 --base-url https://example.com/hls/playlist.m3u8 --select kind,variant_count,representations` to summarise HLS or DASH without downloading media
- RUN `browser-automation-cli --json video manifest --stdin --select kind` with the manifest on stdin
- RUN `browser-automation-cli --json video manifest --paths-file /tmp/manifests.txt` for a batch


## Audio
- RUN `browser-automation-cli --json audio info --path /tmp/in.wav --select format,codec,duration,bytes,sha256` and READ the codec and duration
- RUN `browser-automation-cli --json audio info --stdin --select codec` with the audio on stdin
- RUN `browser-automation-cli --json audio info --paths-file /tmp/audios.txt --select path,codec` for a batch
- RUN `browser-automation-cli --json --timeout 120 audio download https://example.com/a.mp3 -o /tmp/a.mp3 --max-bytes 20971520 --require-audio --select path,bytes` to download direct media
- RUN `browser-automation-cli --json --timeout 120 audio download https://example.com/a.bin --out /tmp/a.bin --allow-non-audio` ONLY for an intentional non-audio body
- RUN `browser-automation-cli --json --timeout 300 audio convert --path /tmp/in.wav --format mp3 -o /tmp/a.mp3 --codec mp3 --bitrate 192k --sample-rate 44100 --channels 2 --audio-stream 0 --strip-metadata --select path_out,lossy_transcode,suggestion` and READ `lossy_transcode`
- RUN `browser-automation-cli --json --timeout 300 audio convert --stdin --format flac --out /tmp/a.flac` with the audio on stdin
- RUN `browser-automation-cli --json --timeout 600 audio convert --paths-file /tmp/audios.txt --format opus` to convert a batch
- RUN `browser-automation-cli --json --timeout 300 audio trim --path /tmp/a.mp3 --start 1 --duration 5 -o /tmp/cut.mp3 --format mp3 --codec mp3 --bitrate 128k --select path_out,duration` to cut by duration
- RUN `browser-automation-cli --json --timeout 300 audio trim --stdin --start 1 --to 6 --out /tmp/cut.ogg` to cut up to an instant
- RUN `browser-automation-cli --json --timeout 600 audio trim --paths-file /tmp/audios.txt --start 0 --duration 30` for a batch


## Emulation and Perf
- RUN `browser-automation-cli --json --timeout 60 emulate --user-agent 'Mozilla/5.0' --locale en-US --timezone America/New_York --latitude=40.71 --longitude=-74.00 --media print --network-conditions 'Slow 3G' --cpu-throttling-rate 4 --color-scheme dark --extra-headers '{"X-Test":"1"}' --viewport '390x844x3,mobile,touch' --screen 390x844` and NEVER use `--device`
- RUN `browser-automation-cli --json --timeout 60 emulate --offline` to force the page offline
- RUN `browser-automation-cli --json --timeout 60 resize --width 1280 --height 720 --scale 2 --mobile --screen 1280x720` to resize the viewport
- RUN `browser-automation-cli --json --timeout 90 perf start --path /tmp/trace.json --reload --auto-stop` to record the load trace
- RUN `browser-automation-cli --json --timeout 90 perf stop --path /tmp/trace.json` and READ the available insight sets
- RUN `browser-automation-cli --json perf insight --path /tmp/trace.json --name LCPBreakdown` to analyse a saved trace offline and NEVER combine `--path` with `--insight-set-id`
- RUN `browser-automation-cli --json --timeout 90 perf insight --insight-set-id set-1 --insight-name DocumentLatency` to analyse the set of the live session
- RUN `browser-automation-cli --json --timeout 180 lighthouse https://example.com --out-dir /tmp/lh --device mobile --mode navigation --lighthouse-path /usr/local/bin/lighthouse` and READ `data.binary_source`, NEVER treating `mock` as validation
- RUN `browser-automation-cli --json --timeout 60 --experimental-screencast screencast start --path /tmp/cast` with `--path` as the frames DIRECTORY
- RUN `browser-automation-cli --json --timeout 60 --experimental-screencast screencast stop --path /tmp/cast.webm` with `--path` as the video FILE


## Heap
- RUN `browser-automation-cli --json --timeout 120 --category-memory heap take --path /tmp/s.heapsnapshot --url https://example.com` with `--url` ALWAYS, because without it the one-shot snapshots `about:blank`
- RUN `browser-automation-cli --json --category-memory heap close --path /tmp/s.heapsnapshot` to release the open handle
- RUN `browser-automation-cli --json --category-memory heap compare --base /tmp/a.heapsnapshot --current /tmp/b.heapsnapshot --class-index 3` and READ the growth on the `--current` side
- RUN `browser-automation-cli --json --category-memory heap summary --path /tmp/s.heapsnapshot` and READ the per-class totals
- RUN `browser-automation-cli --json --category-memory heap details --path /tmp/s.heapsnapshot --filter-name Array --page-idx 0 --page-size 50` to page through class details
- RUN `browser-automation-cli --json --category-memory heap class-nodes --path /tmp/s.heapsnapshot --id 7 --filter-name Array --page-idx 0 --page-size 50` to list the nodes of one class
- RUN `browser-automation-cli --json --category-memory heap dominators --path /tmp/s.heapsnapshot --node 42` and NEVER use `--node-id`
- RUN `browser-automation-cli --json --category-memory heap dup-strings --path /tmp/s.heapsnapshot --page-idx 0 --page-size 50` to list duplicated strings
- RUN `browser-automation-cli --json --category-memory heap edges --path /tmp/s.heapsnapshot --node 42 --page-idx 0 --page-size 50` to list outgoing edges
- RUN `browser-automation-cli --json --category-memory heap retainers --path /tmp/s.heapsnapshot --node 42 --page-idx 0 --page-size 50` to list what retains the node
- RUN `browser-automation-cli --json --category-memory heap paths --path /tmp/s.heapsnapshot --node 42 --max-depth 8 --max-nodes 10000 --max-siblings 20` to enumerate paths back to a GC root
- RUN `browser-automation-cli --json --category-memory heap object-details --path /tmp/s.heapsnapshot --node 42` and READ size, distance and retained size


## Extensions and Third Parties
- RUN `browser-automation-cli --json --timeout 60 --category-extensions extension list` to list the extensions loaded in THIS process
- RUN `browser-automation-cli --json --timeout 60 --category-extensions extension install /tmp/extension` to launch Chrome with the unpacked extension loaded
- RUN `browser-automation-cli --json --timeout 60 --category-extensions extension reload abcdefghijklmnop --path /tmp/extension` to reload the extension by id
- RUN `browser-automation-cli --json --timeout 60 --category-extensions extension trigger abcdefghijklmnop --path /tmp/extension` to trigger the action through the service worker
- RUN `browser-automation-cli --json --timeout 60 --category-extensions extension uninstall abcdefghijklmnop` to uninstall the extension by id
- RUN `browser-automation-cli --json --timeout 60 --category-third-party devtools3p list --url https://example.com` with `--url`, because without it discovery runs on a blank page
- RUN `browser-automation-cli --json --timeout 60 --category-third-party devtools3p exec ToolName --params '{}' --url https://example.com` to execute one tool by name
- RUN `browser-automation-cli --json --timeout 60 --category-webmcp webmcp list --url https://example.com` to list the tools the page declares
- RUN `browser-automation-cli --json --timeout 60 --category-webmcp webmcp exec ToolName --input '{}' --url https://example.com` to execute one tool by name


## MITM
- RUN `browser-automation-cli --json mitm init-ca` ONCE to create the local CA under XDG
- RUN `browser-automation-cli --json --timeout 60 mitm capture-url https://example.com --seconds 20 --har /tmp/c.har --hosts example.com --capture-hosts example.com` and READ `data.capture_path`
- RUN `browser-automation-cli --json --timeout 45 mitm start --seconds 30` to start the proxy on 127.0.0.1 with an ephemeral port
- RUN `browser-automation-cli --json mitm status --capture-path /tmp/capture.json` and READ the CA paths and the bind policy
- RUN `browser-automation-cli --json mitm list --host example.com --limit 50 --capture-path /tmp/capture.json` to list the captured exchanges
- RUN `browser-automation-cli --json mitm get 0 --capture-path /tmp/capture.json` to read ONE exchange by id
- RUN `browser-automation-cli --json mitm har --out /tmp/c.har --capture-path /tmp/capture.json` to export HAR 1.2
- RUN `browser-automation-cli --json mitm export --format ndjson --out /tmp/c.ndjson --capture-path /tmp/capture.json` to export the capture as `json` or `ndjson`
- RUN `browser-automation-cli --json mitm domains --capture-path /tmp/capture.json` and FILTER browser background hosts before concluding
- RUN `browser-automation-cli --json mitm apis --kind rest --capture-path /tmp/capture.json` and TREAT zero endpoints on a static page as an honest answer
- RUN `browser-automation-cli --json mitm graphql --limit 100 --capture-path /tmp/capture.json` to list GraphQL operations
- RUN `browser-automation-cli --json mitm ws list --limit 100 --capture-path /tmp/capture.json` to list WebSocket frames
- RUN `browser-automation-cli --json mitm ws get 0 --capture-path /tmp/capture.json` to read ONE frame by id
- RUN `browser-automation-cli --json mitm block --host example.com --path /ads` to block a host and path prefix
- RUN `browser-automation-cli --json mitm allow --host example.com` to add the host to the TLS interception allowlist
- RUN `browser-automation-cli --json mitm redact` WITHOUT `--secrets` to SHOW the effective policy without writing anything
- RUN `browser-automation-cli --json mitm redact --secrets false` to stop masking persistently and SWAP to `--secrets true` to restore it


## Workflow Run Exec and Record
- RUN `browser-automation-cli --json --timeout 300 workflow run --manifest /tmp/wf.json --journal /tmp/wf.journal` to validate the DAG and execute the steps
- RUN `browser-automation-cli --json --timeout 300 workflow resume --manifest /tmp/wf.json --journal /tmp/wf.journal` to resume from the journal
- RUN `browser-automation-cli --json workflow status --journal /tmp/wf.journal --name demo` and READ the status of each step
- RUN `browser-automation-cli --json --json-steps --timeout 90 run --script /tmp/steps.jsonl` with the lines `{"cmd":"goto","url":"https://example.com","handle_before_unload":"accept","navigation_timeout_ms":15000}` and `{"cmd":"wait","selector":"h1, main","wait_timeout_ms":10000}` and `{"cmd":"view","verbose":true}` and READ `data.steps`
- RUN `browser-automation-cli --json --timeout 90 run --script -` and SEND one NDJSON line per step on stdin, NEVER `<(...)`, which the file jail refuses
- RUN `browser-automation-cli --json --timeout 60 exec goto https://example.com` as a SINGLE step, NEVER as multi-step
- RUN `browser-automation-cli --json --timeout 90 record --url https://example.com --path /tmp/recording.ndjson --seconds 30 --max-events 200` and KNOW the first ceiling reached wins
- RUN `browser-automation-cli --json --json-steps --timeout 90 run --script /tmp/recording.ndjson` to replay the recording


## Monitor QR and Spreadsheet
- RUN `browser-automation-cli --json --timeout 60 monitor check --url https://example.com --baseline /tmp/baseline.txt --write-baseline --engine http --diff-mode json` to compare against the baseline and report WHAT changed
- RUN `browser-automation-cli --json --timeout 120 monitor check --url https://example.com --baseline /tmp/baseline.txt --engine browser --diff-mode git` for pages that depend on JavaScript
- RUN `browser-automation-cli --json qr encode --text https://example.com --format png --path /tmp/qr.png` and SWAP to `svg` or `terminal`, omitting `--path` for the matrix on stdout
- RUN `browser-automation-cli --json qr decode --path /tmp/qr.png` and READ the decoded payload
- RUN `browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/output.xlsx --sheet Data --force` to write XLSX from CSV or JSON
