---
name: browser-automation-cli
description: This skill MUST be used when a task requires opening, reading, operating, collecting from or diagnosing web pages, and it MUST activate proactively even when the user never names browser-automation-cli. It covers navigating, clicking, typing and filling forms, accessibility snapshots, screenshots and PDF, scrape, crawl, site maps, sitemap and feed, LLM extraction, parsing PDF, DOCX and spreadsheets, console, network, HAR and MITM, REST and GraphQL API discovery, device emulation, Lighthouse, performance traces, heap, extensions, recording and replaying interactions, QR, XLSX spreadsheets, local image, video and audio, an invisible headed window for Cloudflare Turnstile and residual-zero audits. It takes URLs, selectors, local files and multi-step scripts, and delivers a JSON envelope with a branchable exit code, artifacts on disk, a reduced payload and proof of the window mode. It ALWAYS configures through flags and XDG config and NEVER through environment variables.
---
# browser-automation-cli


## Mission and Activation
### REQUIRED
- MUST activate this skill for every task that opens, reads, operates, collects from or measures a web page, even when the user never names the CLI
- MUST activate this skill for local media, document parsing, QR and spreadsheets too, which run without Chrome
- MUST ALWAYS invoke the binary `browser-automation-cli` by its full name
- MUST pass `--json` on EVERY programmatic call and parse ONLY stdout with `jaq`
- MUST read the exit code BEFORE stdout and require `ok` true BEFORE reading `data`
- MUST pass an explicit `--timeout` on every call that opens a browser
### FORBIDDEN
- NEVER use an alias or a shortened binary name
- NEVER parse stderr as JSON
- NEVER mask an exit code with `|| true`
- NEVER invent a flag, value, key or subcommand the binary lacks
- NEVER use this skill for official library documentation, Rust crates, SSH or databases


## Live Surface Discovery
### REQUIRED
- MUST discover the surface from the binary and NEVER from memory
- MUST run `commands` for the inventory and `schema <cmd>` for one command contract
- MUST run `<cmd> --help` to see the local and global flags of that command
- MUST run `doctor --offline --quick` when the host looks wrong, because it does NOT launch Chrome
- MUST confirm every step key against `schema <cmd>` BEFORE serializing a new step
### Ready Formulas
- RUN `browser-automation-cli --json commands --detail`
- RUN `browser-automation-cli --json schema --cmd scrape`
- RUN `browser-automation-cli scrape --help`
- RUN `browser-automation-cli --json version` and `browser-automation-cli --json locale`
- RUN `browser-automation-cli --json doctor --offline --quick --fix`
- RUN `browser-automation-cli completions bash` and `browser-automation-cli man --out /tmp/b.1`


## One-Shot Lifecycle
### REQUIRED
- MUST treat every process as BORN, EXECUTE, FINALIZE and DIE, with Chrome born and killed inside it
- MUST know there is NO daemon, NO persistent session and NO state between processes
- MUST treat an `@eN` ref as valid ONLY inside the process that produced it
- MUST put all multi-step work into one single `run --script`
- MUST treat `exec` as one inline step, with the same surface as `run` steps
- MUST know the Chrome launched by the CLI opens NO DevTools TCP port and speaks CDP over a pipe through a single-client loopback bridge
- MUST carry authenticated state between processes ONLY with `storage export` and `storage import`
- MUST carry traffic captures between processes ONLY with `mitm capture-url` and `--capture-path`
- MUST use the system Chrome or point to the binary with `config set chrome_path`
### FORBIDDEN
- NEVER reuse `@eN` in a second process
- NEVER attach an external tool, debugger or second CDP client to that Chrome
- NEVER expect a console or network capture to survive DIE


## Window Mode and Private Display
### REQUIRED
- MUST know `browser_mode` is `auto`, `headed` or `headless`
- MUST know `auto` resolves to headed inside a private Xvfb ONLY on Linux with `Xvfb` on PATH and without `--no-xvfb`
- MUST know `auto` resolves to headless everywhere else, and that `--no-xvfb` with `auto` stays headless
- MUST treat `--headed` and `--headless` as shorthands of `--browser-mode`, and the flag ALWAYS beats `config set browser_mode`
- MUST know `--headed` on Linux with Xvfb draws the window inside the private Xvfb, off the operator screen, even on a Wayland desktop
- MUST know that isolation comes from the `--ozone-platform=x11` pin, applied ONLY when the private Xvfb started
- MUST pass `--headed --no-xvfb` as the ONLY route to put the window on the current display on purpose
- MUST use headed as REQUIRED against a Cloudflare Turnstile challenge, because headless does NOT emit the token
- MUST know concurrent headed runs receive distinct private displays, so headed parallelism is safe
- MUST know the private Xvfb requires an authentication cookie and is torn down at DIE with no action from you
- MUST read `display_backend`, which is `headless`, `xvfb` or `host` and states the display ACTUALLY used after launch
- MUST know `display_backend` reports only the intent before any launch, and that ONLY `host` paints on the operator screen
- MUST prove the mode with `browser_mode_requested`, `browser_mode_effective` and `browser_mode_source`, which is `default`, `xdg` or `flag`
- MUST read that `run` witness ONCE at the top of the envelope
- MUST read `browser_mode_auto_resolves` in the `doctor` check `virtual_display` to learn what `auto` does on this host
- MUST read `launch_args` from `doctor --fingerprint` to see the real argv handed to Chrome, which is `null` before any launch
### FORBIDDEN
- NEVER infer the window mode from the flag you passed
- NEVER treat `browser_mode_source` equal to `default` as a proven requirement
- NEVER parse the `message` text of the `virtual_display` check
- NEVER run `config set chrome_legacy_oxide_launch true`, because that path reopens an unauthenticated DevTools port, never starts Xvfb and draws a headed window on the operator display
### Ready Formulas
- RUN `browser-automation-cli --json --fields checks --filter-rows 'id=virtual_display' doctor --offline --quick`
- RUN `browser-automation-cli --timeout 90 --json --headed scrape https://example.com --engine browser --format markdown` for Turnstile with no window on the desktop
- RUN `browser-automation-cli --timeout 120 --json --headed --no-xvfb goto https://example.com` ONLY when the operator demands to see the window
- RUN `browser-automation-cli --json config set browser_mode headless` to write the persistent host default
- RUN `browser-automation-cli --timeout 60 --json doctor --fingerprint` and read `launch_args`


## JSON Envelope and Exit Codes
### REQUIRED
- MUST expect success as `schema_version`, `ok` true and `data`
- MUST expect failure as `ok` false and `error` carrying `kind`, `message` and `exit_code`
- MUST read partial `data.steps` when a `run` fails
- MUST read `runtime_enable_used`, the boolean stating whether the CDP Runtime domain was enabled in this run
- MUST read `serp_endpoint` on the `search` envelope, and treat `unknown` as an endpoint that does NOT guarantee limit, region or time window
- MUST treat a `search` with no organic result as a failure with `error.kind` equal to `data`, reading `serp_endpoint` and `search_base_url` inside `data`
- MUST read `data.binary_source` from `lighthouse` and NEVER treat `mock` as a real audit
- MUST read `data.dialog_settled` after `dialog accept` or `dialog dismiss`, and NEVER insert an artificial wait when it is true
- MUST retry ONLY a transient launch or network failure
### Exit Codes
- MUST treat `0` as success and `2` as usage, fixing the argv BEFORE retrying
- MUST treat `6` as blocked and `64` as capability-disabled, including a path outside the allowed roots
- MUST treat `65` as data, including an unmet `--expect` under `--expect-exit-code`
- MUST treat `66` as no-input, `69` as unavailable and `70` as software, browser or protocol
- MUST treat `74` as io, `75` as precondition and `78` as config
- MUST treat `124` as timeout, including when a failed Chrome launch is torn down under `--timeout`
- MUST treat `130` as cancelled, including by a termination signal, and `141` as broken-pipe


## Payload Reduction
### REQUIRED
- MUST shrink the payload with the binary flags, and NEVER by piping stdout through `jaq`
- MUST pass `--fields` with ONE single CSV of paths relative to `data`, such as `residual` and NEVER `data.residual`
- MUST pass `--filter-rows` with `key=value`, `key!=value` or `key~substring`, repeatable and combined with AND
- MUST pass `--limit-rows`, `--sort-rows` and `--dedupe-by` on list payloads
- MUST pass `--count-only` to receive only `count`, narrowing first with `--fields` when `data` holds more than one list
- MUST pass `--truncate-content` to cut every string and `--max-output-bytes` as a hard byte ceiling
- MUST read `agent_ops.truncated` as the ONLY cut signal and `agent_ops.unresolved_paths` as the paths that did not resolve
- MUST treat a filter with no match as an empty list with `ok` true, knowing a missing field NEVER matches
- MUST treat `--select`, `--filter`, `--limit` and `--sort` as LOCAL flags written AFTER the subcommand
### FORBIDDEN
- NEVER repeat `--fields`, because the repetition exits 2
- NEVER assume `agent_ops` exists just because you passed a reduction flag
### Ready Formulas
- RUN `browser-automation-cli --json --fields checks --count-only doctor --offline --quick`
- RUN `browser-automation-cli --json --limit-rows 5 --fields commands commands`
- RUN `browser-automation-cli --json --fields keys --filter-rows 'key~proxy' config list-keys`
- RUN `browser-automation-cli --json --truncate-content 2000 --max-output-bytes 60000 scrape https://example.com --format markdown`


## Global Flags
### Output and Time
- MUST pass `--json` for the envelope and `--json-steps` for one NDJSON object per `run` step
- MUST pass `-q` or `--quiet` to silence human logs on stderr
- MUST pass `-v` or `--verbose` for info tracing and `--debug` for maximum detail
- MUST pass `--plain` for stderr without ANSI colors
- MUST pass `--lang en` or `--lang pt-BR` to force the message language
- MUST pass `--correlation-id <ID>` to echo a join key on envelopes and steps
- MUST pass `--timeout <SECS>` as the whole-process ceiling and `--step-timeout <SECS>` as the ceiling of each `run` step
- MUST pass `--max-concurrency <N>` to bound batch, crawl and CDP fan-out
- MUST pass `--artifacts-dir <DIR>` for screenshots, PDFs and evidence
- MUST pass `--allow-outside-roots` ONLY as explicit risk acceptance, and ALWAYS prefer `config set allowed_roots`
- MUST use `--fields`, `--filter-rows`, `--limit-rows`, `--sort-rows`, `--dedupe-by`, `--count-only`, `--truncate-content` and `--max-output-bytes` as the section `Payload Reduction` states
### Browser and Identity
- MUST pass `--browser-mode`, `--headed`, `--headless` and `--no-xvfb` as the section `Window Mode and Private Display` states
- MUST pass `--capture-console` in the SAME process as `console` and `--capture-network` in the SAME process as `net`
- MUST pass `--dump-on-failure` with `--artifacts-dir` and a capture flag to write evidence on failure
- MUST keep stealth ON and pass `--no-stealth` ONLY to turn the patches off for this run
- MUST pass `--stealth-profile auto`, which follows the host, and use `chrome-linux`, `chrome-win`, `chrome-mac` or `list` ONLY when it matches the host or to list profiles
- MUST pass `--stealth-seed <SEED>` to pin the same identity across the processes of a crawl
- MUST know a seed also stores the launched Chrome major in `state_dir/stealth/host-major-<hash>.txt`, while without a seed or under `--no-stealth` nothing about it touches the disk
- MUST read `user_agent_major_source` in every `scrape` envelope, where `projected` means an overridden User-Agent, `host_binary` a major read from this host's Chrome and `host_unprobed` the crate table with nothing probed
- MUST treat `user_agent_major_source` equal to `null` as stealth off, or on `--engine browser` as no launch yet
- MUST know the patch NEVER emulates `navigator.userAgentData`, which exists only in a secure context, and that headed on the host profile keeps Chrome's native object and User-Agent
- MUST know headless or a foreign profile sends a complete `userAgentMetadata` in the override, so JavaScript, `getHighEntropyValues` and every `sec-ch-ua-*` header tell the same version
- MUST read `planned_version_source` from `doctor --fingerprint` as `null` with an override, `chrome_binary` with no override and a probed binary, and `crate_table` when the probe failed
- MUST read the mismatch `ua_data_brands_vs_user_agent` as a major disagreement between `ua_data_brands` and the User-Agent
- MUST pass `--input-profile human` for human pacing or `direct` for one event per action
- MUST pass `--input-seed <SEED>` to reproduce a `human` run exactly
- MUST pass `--warmup` to visit the origin root before the target, or `--warmup-url <URL>` to warm another URL
### Network Proxy and Robots
- MUST pass `--proxy <URL>` with `http`, `https` or `socks5` as the egress proxy of Chrome and the HTTP engine
- MUST pass `--proxy-bypass <HOSTS>` in Chrome bypass-list syntax
- MUST store proxy credentials ONLY with `config set proxy_username` and `config set proxy_password`, and NEVER in argv
- MUST pass `--min-delay-ms <MS>` to raise the courtesy floor, knowing the MAXIMUM of the flag, `scrape_min_delay_ms` and `Crawl-delay` wins
- MUST pass `--ignore-robots` and `--i-accept-robots-risk` TOGETHER, as the section `Residual-Zero and Robots` states
### Category Gates
- MUST pass `--category-memory` for `heap` and `--category-extensions` for `extension`
- MUST pass `--category-third-party` for `devtools3p` and `--category-webmcp` for `webmcp`
- MUST pass `--experimental-vision` for `click-at` and `--experimental-screencast` for `screencast`
- NEVER enable a gate without the family that requires it
### MITM
- MUST pass `--mitm` to route the Chrome of this process through a local MITM proxy
- MUST pass `--mitm-har <FILE>` to write HAR at FINALIZE and `--mitm-hosts <HOSTS>` to decrypt only those hosts
- MUST pass `--mitm-ca-dir <DIR>` ONLY to move the CA directory
- MUST pass `--mitm-max-body-bytes <BYTES>` to bound retained bodies and `--mitm-no-media-bodies` to drop media
- MUST know `--mitm-ws` and `--mitm-redact-secrets` only restate the default and change NOTHING
- MUST pass `--mitm-no-redact-secrets` to keep `Authorization` and `Cookie` readable, knowing that asking for both resolves by MASKING
### Assertion
- MUST pass `--expect <EXPR>` with `key=value`, `key!=value` or `key~substring`, repeatable and ANDed
- MUST read `agent_ops.expectation_unmet` to see every unmet expectation
- MUST pass `--expect-exit-code` to exit 65, because without it the exit stays 0
- RUN `browser-automation-cli --json --fields offline --expect 'offline=true' --expect-exit-code doctor --offline --quick`


## XDG Configuration
### REQUIRED
- MUST configure ONLY through flags and `config init`, `config path`, `config show`, `config get`, `config set`, `config unset` and `config list-keys`
- MUST know the flag ALWAYS beats the stored XDG key
- MUST use `config unset <key>` to restore the built-in default
- MUST store secrets such as `openrouter_api_key` and `encryption_key` ONLY through `config set`
- MUST store binaries with `chrome_path`, `lighthouse_path` and `ffmpeg_path`
- MUST use `cache_backend` equal to `redis` ONLY with `cache_redis_url` in `redis://`
- MUST move the ceiling of the `net` and `console` buffers ONLY with `config set event_tracker_max_entries`
- MUST tune the HTTP/2 fingerprint of the http engine ONLY through the `http2_*` keys, which have NO flag
- MUST read `references/xdg-keys.md` for the default and description of every key
### FORBIDDEN
- FORBIDDEN to use environment variables, `.env` or `export` as product configuration
- NEVER write a secret, cookie or token to a log
### Ready Formulas
- RUN `browser-automation-cli --json config init` and `browser-automation-cli --json config path`
- RUN `browser-automation-cli --json config show` and `browser-automation-cli --json config get timeout`
- RUN `browser-automation-cli --json config set dialog_settle_ms 2000`
- RUN `browser-automation-cli --json config unset dialog_settle_ms`


## Command Inventory
### REQUIRED
- MUST recognize all 71 - doctor, commands, schema, version, locale, goto, view, press, click-at, write, keys, type, wait, hover, drag, submit, fill-form, select-option, pick, upload, back, forward, reload, eval, grab, print-pdf, monitor, run, exec, record, extract, text, scroll, cookie, storage, attr, assert, console, net, page, dialog, scrape, batch-scrape, crawl, map, sitemap, feed, search, parse, qr, image, video, audio, find-paths, sg-scan, sg-rewrite, sheet-write, mitm, workflow, config, emulate, resize, perf, lighthouse, screencast, heap, extension, devtools3p, webmcp, completions, man
- MUST know `pick` and `select-option` exit 2 as top-level subcommands and MUST go through `exec` or `run`
- MUST know `console list`, `console get`, `net list` and `net get` exit 2 at top level and exist ONLY as `run` steps
- MUST know `console clear` and `console dump` work at top level
### Local Families Without Chrome
- MUST use `image`, `video` and `audio` for local media
- MUST use `parse` to extract text from HTML, Markdown, TXT, PDF, DOCX, XLSX and ODS
- MUST use `qr`, `sheet-write`, `find-paths`, `sg-scan` and `sg-rewrite` as local tools
- MUST use `workflow run`, `workflow resume` and `workflow status` for a journaled DAG
- MUST use `sitemap`, `feed`, `map` and `search` through the HTTP engine
- MUST know `sg-rewrite` is a dry run by default and writes only with `--apply`
- RUN `browser-automation-cli --json find-paths --glob '**/*.rs' --limit 200 .`
- RUN `browser-automation-cli --json sg-scan . --limit 100` and `browser-automation-cli --json sg-rewrite . --apply` only after reviewing the dry run
- RUN `browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data --force`
- RUN `browser-automation-cli --json workflow resume --manifest /tmp/wf.json --journal /tmp/wf.journal`


## Navigation and Interaction
### REQUIRED
- MUST pass `goto <URL>` with `--navigation-timeout-ms` and `--handle-before-unload accept` or `dismiss`
- MUST pass `reload --ignore-cache` for a hard reload, and NEVER that flag on `goto`
- MUST pass `view --detailed` for the full accessibility tree, and NEVER `--verbose`
- MUST pass `view --allow-empty` ONLY when a blank snapshot is intentional
- MUST pass `type <TEXT>` with `--target` or with `--focus-only`
- MUST pass `fill-form --fields-json` and `cookie set --cookies-json`, and NEVER a payload through `--json`
- MUST pass `submit <TARGET>` to submit the form and wait for navigation or a request
- MUST pass `grab --path` and `print-pdf --path`, and NEVER a positional path
- MUST pass `print-pdf --url` in one-shot, because a blank page is refused
- MUST pass `--include-snapshot` on the action to receive fresh refs in the same process
- MUST read `matched_selector` after a `wait` with several selectors
- MUST know native `select-option` and `pick` report `via` equal to `native_select`
- MUST know `storage export` writes with mode 0600
- MUST drive `emulate` through flags and NEVER invent `--device`
### Ready Formulas
- RUN `browser-automation-cli --timeout 60 --json goto https://example.com --init-script 'window.__ready=1' --handle-before-unload accept --navigation-timeout-ms 15000`
- RUN `browser-automation-cli --timeout 60 --json exec select-option --target '#priority' --option High`
- RUN `browser-automation-cli --timeout 60 --json grab --path /tmp/p.webp --format webp --quality 80 --full-page`
- RUN `browser-automation-cli --timeout 60 --json print-pdf --path /tmp/p.pdf --url https://example.com`
- RUN `browser-automation-cli --timeout 60 --json --experimental-vision click-at --x 10 --y 20 --include-snapshot`
- RUN `browser-automation-cli --timeout 60 --json storage export --path /tmp/auth.json --url https://example.com`
- RUN `browser-automation-cli --timeout 60 --json storage import --path /tmp/auth.json --url https://example.com`
- RUN `browser-automation-cli --timeout 60 --json cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'`
- RUN `browser-automation-cli --timeout 60 --json emulate --viewport '390x844x3,mobile,touch' --network-conditions 'Slow 3G'`


## Multi-step Scripts
### REQUIRED
- MUST use `run --script <file>` with NDJSON of one step per line or a JSON array, every line carrying the `cmd` key
- MUST know `--timeout` covers the whole script and `--step-timeout` covers each step
- MUST use `run --script -` to read NDJSON from stdin into one live session, with one BORN, one DIE and per-line validation
- MUST know an unknown key in a `run` step is REFUSED with exit 2 BEFORE the browser launches
- MUST serialize `console` and `net` as steps with `action` equal to `list` or `get`
- MUST take a fresh `view` after every `eval` step, because it emits `refs_invalidated` true and kills the refs
- MUST replay a `record` output with `run --script` over the recorded file
### FORBIDDEN
- NEVER use `run --script <(...)`, because the file jail refuses process substitution
- NEVER split `@eN` steps across processes
- NEVER put `mitm`, `storage`, `config`, `workflow`, `crawl`, `map`, `batch-scrape`, `search`, `parse`, `qr`, `find-paths`, `sg-scan`, `sg-rewrite`, `sheet-write`, `monitor`, `extension install` or `extension uninstall` inside `run`
### Ready Steps
- RUN `browser-automation-cli --timeout 90 --json --json-steps --capture-network --capture-console run --script /tmp/steps.jsonl`
- RUN `printf '%s\n' '{"cmd":"goto","url":"https://example.com"}' '{"cmd":"view"}' | browser-automation-cli --timeout 60 --json run --script -`
- RUN `browser-automation-cli --timeout 60 --json record --url https://example.com --path /tmp/rec.jsonl --seconds 30 --max-events 200`
- RUN the step `{"cmd":"wait","selectors":["h1","main"],"wait_timeout_ms":10000}`
- RUN the step `{"cmd":"view","verbose":true}`
- RUN the step `{"cmd":"write","target":"@e1","value":"hello"}`
- RUN the step `{"cmd":"submit","target":"#user","timeout_ms":8000}`
- RUN the step `{"cmd":"pick","target":"@e2","option":"High"}`
- RUN the step `{"cmd":"dialog","action":"accept","if_present":true}`
- RUN the step `{"cmd":"net","action":"list","resource_types":"Document,XHR,Fetch","page_size":50}`
- RUN the step `{"cmd":"console","action":"list","types":"error,warning","include_preserved":true}`


## Scraping and Collection
### REQUIRED
- MUST start with `--engine http`, which does NOT launch a browser
- MUST switch to `--engine browser` ONLY when the page depends on JavaScript or Turnstile
- MUST request formats with `--format` as CSV or by repeating the flag, checking the 15 values in `scrape --help`
- MUST know `attributes`, `html` and `rawHtml` are distinct keys in `data`
- MUST read `unsupported_format` and `content_kind` BEFORE the key of the requested format, and read the body from `text` when `unsupported_format` is set
- MUST pass `--only-main-content` to trim the page before parsing
- MUST use `batch-scrape` for a closed list, `crawl` to follow links and `map` to only enumerate URLs
- MUST use `monitor check` with a baseline to detect page changes
- MUST use `extract --llm` or `scrape --format json` ONLY with `openrouter_api_key` stored
### FORBIDDEN
- NEVER treat `rawHtml` as an alias of `html`
- NEVER conclude an empty body from a missing key
- NEVER use `--engine browser` by habit, because it costs a whole Chrome
- NEVER call `crawl` when `map` already answers
### Ready Formulas
- RUN `browser-automation-cli --json scrape https://example.com --format markdown,links,metadata --engine http --only-main-content`
- RUN `browser-automation-cli --json scrape https://example.com --format attributes --attribute-selector a --attribute-name href`
- RUN `browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 4 --output-mode ndjson`
- RUN `browser-automation-cli --timeout 300 --json crawl https://example.com --limit 20 --max-depth 2 --exclude-path /login`
- RUN `browser-automation-cli --timeout 60 --json map https://example.com --limit 200 --use-sitemap true`
- RUN `browser-automation-cli --timeout 60 --json sitemap https://example.com --limit 100`
- RUN `browser-automation-cli --timeout 60 --json feed https://example.com/feed.xml`
- RUN `browser-automation-cli --timeout 60 --json search 'example domain' --limit 10 --country br`
- RUN `browser-automation-cli --json parse /tmp/doc.pdf --redact-pii`
- RUN `browser-automation-cli --timeout 120 --json extract https://example.com --llm --question 'What is the title' --schema-json /tmp/s.json`
- RUN `browser-automation-cli --timeout 60 --json monitor check --url https://example.com --baseline /tmp/b.txt --write-baseline --diff-mode json`
- RUN `browser-automation-cli --json qr encode --text https://example.com --format png --path /tmp/qr.png` and `browser-automation-cli --json qr decode --path /tmp/qr.png`


## Network Console and MITM
### REQUIRED
- MUST use `net` for traffic of the process ITSELF and `mitm` for a file capture read back by another process
- MUST pass `resource_types` as ONE CSV list matched exactly against the CDP vocabulary, such as `Document`, `XHR`, `Fetch`, `WebSocket` and `Other`
- MUST expect exit 2 for an unknown type, BEFORE any launch
- MUST read `dropped_oldest` on `net` and `console` and rebuild the total as `total` plus `dropped_oldest`
- MUST pass `include_preserved` on `list` and on `get` so one index addresses the SAME record
- MUST read `data.capture_path` after `mitm capture-url` and feed that file back with `--capture-path`
- MUST use `--capture-path` on `mitm status`, `mitm list`, `mitm get`, `mitm har`, `mitm export`, `mitm domains`, `mitm apis`, `mitm graphql`, `mitm ws list` and `mitm ws get`
- MUST narrow decryption with `--hosts` and the record with `--capture-hosts`, because Chrome produces background traffic
- MUST treat zero endpoints from `mitm apis` on a static page as an honest answer
- MUST use `mitm redact --secrets false` ONLY to write the persistent unmasked policy
### Ready Formulas
- RUN `browser-automation-cli --json mitm init-ca`
- RUN `browser-automation-cli --timeout 60 --json mitm capture-url https://example.com --seconds 30 --har /tmp/c.har --hosts example.com --capture-hosts example.com`
- RUN `browser-automation-cli --json mitm domains --capture-path /tmp/capture.json`
- RUN `browser-automation-cli --json mitm export --format ndjson --out /tmp/c.ndjson --capture-path /tmp/capture.json`
- RUN `browser-automation-cli --json mitm block --host example.com --path /ads`
- RUN `browser-automation-cli --timeout 60 --json --mitm --mitm-har /tmp/run.har --mitm-no-media-bodies goto https://example.com`
- RUN `browser-automation-cli --timeout 60 --json --capture-console assert console-no-match --pattern TypeError`


## APIs Through the Page
### REQUIRED
- MUST remember `eval` executes in the origin context of the PAGE
- MUST navigate to the target origin BEFORE the `fetch`, otherwise it returns `Failed to fetch`
- MUST chain `goto` and `eval` in one single `run --script` with `typed` true to read `data.value` and `data.value_type`
- MUST wrap every `fetch` in try/catch and return the error message
- MUST treat a null value with exit 0 from a rejected promise as a silent failure
- MUST know a promise resolves automatically, with no await key
- MUST use `mitm apis` and `mitm graphql` to discover REST and GraphQL endpoints BEFORE calling them
- MUST pass `eval --file-path` for a large result and `--service-worker-id` to run inside a service worker
### Ready Formulas
- RUN `printf '%s\n' '{"cmd":"goto","url":"https://example.com"}' '{"cmd":"eval","expression":"(async()=>{try{const r=await fetch(\"/api\");return r.status}catch(e){return String(e)}})()","typed":true}' | browser-automation-cli --timeout 90 --json run --script -`
- RUN `browser-automation-cli --json mitm apis --kind rest --capture-path /tmp/capture.json`
- RUN `browser-automation-cli --json mitm graphql --limit 20 --capture-path /tmp/capture.json`


## Local Media
### REQUIRED
- MUST use `image info`, `image convert`, `image resize`, `image download` and `image exif`
- MUST use `video info`, `video download`, `video convert`, `video to-mp3`, `video trim`, `video thumbnail` and `video manifest`
- MUST use `audio info`, `audio download`, `audio convert` and `audio trim`
- MUST point to ffmpeg with `config set ffmpeg_path` when it is not on PATH
- MUST project media with `--select` and process batches with `--paths-file`
- MUST treat local webp as lossless, with `quality_applied` false
- MUST treat `--keep-exif` as intent, with `keep_exif_honored` false
- MUST read image text as an agent, because the CLI has NO OCR
- NEVER request pixel base64, raw frames or PCM on stdout, except an intentional `grab --include-base64`
- NEVER run ffmpeg by hand when `video convert` or `audio convert` does the job
- NEVER use AVIF or HEIC as output format
### Ready Formulas
- RUN `browser-automation-cli --json image convert --path /tmp/a.png --format jpeg --quality 85 -o /tmp/a.jpg`
- RUN `browser-automation-cli --json image resize --path /tmp/a.png --width 800 --keep-aspect -o /tmp/b.png`
- RUN `browser-automation-cli --json image info --path /tmp/a.png --select format,width,height,sha256`
- RUN `browser-automation-cli --json image exif --path /tmp/a.jpg --select tags`
- RUN `browser-automation-cli --json video info --path /tmp/v.mp4 --select container,duration_secs,streams`
- RUN `browser-automation-cli --timeout 300 --json video convert --path /tmp/v.mov --format mp4 --video-codec h264 -o /tmp/v.mp4`
- RUN `browser-automation-cli --timeout 120 --json video trim --path /tmp/v.mp4 --start 10 --duration 5 -o /tmp/c.mp4`
- RUN `browser-automation-cli --json video manifest --path /tmp/m.m3u8 --base-url https://example.com/m.m3u8`
- RUN `browser-automation-cli --timeout 120 --json audio convert --path /tmp/a.wav --format opus --bitrate 96k -o /tmp/a.opus`


## Diagnostics Perf and Extensions
### REQUIRED
- MUST point to Lighthouse with `--lighthouse-path` or `config set lighthouse_path`
- MUST use `perf start --reload --auto-stop --path` for a load trace in one process, and `perf insight --path` to analyze offline
- MUST pass `heap take --url`, because without a URL the target is `about:blank`
- MUST analyze the snapshot with `heap summary`, `heap details`, `heap class-nodes`, `heap compare`, `heap dominators`, `heap dup-strings`, `heap edges`, `heap retainers`, `heap paths`, `heap object-details` and `heap close`
- MUST use `screencast start` with a directory and `screencast stop` with a `.webm` or `.mp4` file
- MUST use `extension install`, `extension list`, `extension reload`, `extension trigger` and `extension uninstall`
- MUST pass `--url` on `devtools3p list`, `devtools3p exec`, `webmcp list` and `webmcp exec`, because a one-shot has NO open page
- MUST use `page info`, `page list`, `page new`, `page select`, `page close` and `page tab-id` for the tabs of the process itself
### Ready Formulas
- RUN `browser-automation-cli --timeout 180 --json lighthouse https://example.com --out-dir /tmp/lh --device mobile`
- RUN `browser-automation-cli --timeout 90 --json perf start --reload --auto-stop --path /tmp/trace.json`
- RUN `browser-automation-cli --json perf insight --path /tmp/trace.json --name LCPBreakdown`
- RUN `browser-automation-cli --timeout 90 --json --category-memory heap take --path /tmp/s.heapsnapshot --url https://example.com`
- RUN `browser-automation-cli --json --category-memory heap retainers --path /tmp/s.heapsnapshot --node 42 --page-size 20`
- RUN `browser-automation-cli --timeout 60 --json --experimental-screencast screencast stop --path /tmp/cast.webm`
- RUN `browser-automation-cli --timeout 60 --json --category-extensions extension install /tmp/ext`
- RUN `browser-automation-cli --timeout 60 --json --category-webmcp webmcp list --url https://example.com`


## Residual-Zero and Robots
### REQUIRED
- MUST treat residual-zero as part of the success of every browser one-shot
- MUST validate with `doctor --offline --quick`, reading `data.residual` and the `residual_disk` check
- MUST require `residual_disk` different from `fail`, with zero `orphan_marker_dirs` and zero `ghost_marker_processes`
- MUST treat `sibling_live_processes` above zero as healthy concurrency with `warn`
- MUST know `config set user_data_dir` gives up residual-zero, and `config unset user_data_dir` restores it
- MUST respect robots by default and bypass it ONLY with BOTH `--ignore-robots` and `--i-accept-robots-risk`
- RUN `browser-automation-cli --json --fields residual,checks --filter-rows 'id=residual_disk' doctor --offline --quick`
### FORBIDDEN
- NEVER declare residual-zero without reading `data.residual`
- NEVER delete generic host temp files nor kill the user Chrome
- NEVER bypass robots with a single flag
