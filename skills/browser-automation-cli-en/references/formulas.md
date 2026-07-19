# browser-automation-cli — Full Executable Formula Catalog

MANDATORY companion to `SKILL.md`. EVERY inventory command has at least one executable line.
ALWAYS copy argv as-is unless `schema <cmd> --json` forces a change.
ALWAYS pass global `--json` for machine consumers on programmatic invocations.
Binary name is exactly `browser-automation-cli` (NEVER invent alias `bac`).
FORBIDDEN product environment variables; ALWAYS use flags + XDG `config` only.
REQUIRED inventory is exactly **63** top-level command names.
ALWAYS treat `select-option` and `pick` as executable ONLY inside `run`/`exec` (NOT standalone clap).

## Meta / discovery / locale / man / residual_disk

```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli doctor --json
browser-automation-cli doctor --offline --quick --json | jaq -e '.data.residual.cli_marker_dirs == 0'
browser-automation-cli doctor --offline --quick --json | jaq -e '.data.residual.chromium_tmp_singleton_orphans == 0'
browser-automation-cli doctor --offline --quick --json | jaq -e '.data.residual.live_cli_marker_processes == 0'
browser-automation-cli doctor --offline --quick --json | jaq -e '[.data.checks[] | select(.id=="residual_disk") | .status][0] == "pass"'
browser-automation-cli doctor --offline --quick --json | jaq -e '[.data.checks[] | select(.id=="residual_disk")] | length == 1'
browser-automation-cli commands --json
browser-automation-cli schema goto --json
browser-automation-cli schema run --json
browser-automation-cli schema --cmd pick --json
browser-automation-cli schema --cmd select-option --json
browser-automation-cli schema --cmd sheet-write --json
browser-automation-cli schema --cmd sg-scan --json
browser-automation-cli schema --cmd sg-rewrite --json
browser-automation-cli schema --cmd find-paths --json
browser-automation-cli schema --cmd config --json
browser-automation-cli schema --cmd wait --json
browser-automation-cli schema --cmd assert --json
browser-automation-cli schema --cmd mitm --json
browser-automation-cli schema locale --json
browser-automation-cli schema man --json
browser-automation-cli version --json
browser-automation-cli locale --json
browser-automation-cli --lang en locale --json
browser-automation-cli --lang pt-BR locale --json
browser-automation-cli man
browser-automation-cli man --out /tmp/browser-automation-cli.1
browser-automation-cli completions bash
browser-automation-cli completions zsh
browser-automation-cli completions fish
```

## Navigation / wait

```bash
browser-automation-cli --json goto https://example.com
browser-automation-cli --timeout 60 --json goto https://example.com --init-script 'window.__ready=true' --handle-before-unload accept --navigation-timeout-ms 15000
browser-automation-cli --timeout 60 --json goto https://example.com --handle-before-unload dismiss
browser-automation-cli --json back
browser-automation-cli --json forward
browser-automation-cli --json reload --ignore-cache
# FORBIDDEN: goto --ignore-cache (reload owns --ignore-cache)
browser-automation-cli --json page info
browser-automation-cli --json page list
browser-automation-cli --json page new --url https://example.com
browser-automation-cli --json page new --isolated-context
browser-automation-cli --json page new --isolated-context my-ctx --url https://example.com
browser-automation-cli --json page select 0 --bring-to-front
browser-automation-cli --json page close --index 0
browser-automation-cli --json page tab-id
browser-automation-cli --json wait --ms 500
browser-automation-cli --json wait --text Example --text Demo --ms 1000
browser-automation-cli --json wait --selector "h1" --state load
browser-automation-cli --json wait --selector "h1, main, #content" --ms 0
browser-automation-cli --json reload --ignore-cache
# FORBIDDEN: invent goto --ignore-cache (reload owns --ignore-cache)
# wait multi-selector OR / multi-text OR / url / url_contains / navigation — full surface via run steps
# multi-selector wait success envelope MUST expose matched_selector when a selector resolved — ALWAYS inspect it
```

```bash
cat > /tmp/wait-formulas.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":300}
{"cmd":"wait","selector":"h1, main, #content","wait_timeout_ms":10000}
{"cmd":"wait","selectors":["#app",".ready"],"wait_timeout_ms":10000}
{"cmd":"wait","url":"https://example.com/","wait_timeout_ms":10000}
{"cmd":"wait","url_contains":"example.com","wait_timeout_ms":10000}
{"cmd":"wait","navigation":true,"wait_timeout_ms":10000}
{"cmd":"wait","text":["Example","Domain"],"wait_timeout_ms":5000}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/wait-formulas.jsonl
# On multi-selector wait success, inspect data.matched_selector when present
```

```bash
# page new isolated-context — flag alone uses default isolated name; pass name for named context
# run field isolated_context MUST be string name OR true
cat > /tmp/isolated-formulas.jsonl <<'JSONL'
{"cmd":"page","action":"new","isolated_context":"my-ctx","url":"https://example.com"}
{"cmd":"page","action":"new","isolated_context":true}
{"cmd":"wait","ms":300}
{"cmd":"view"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/isolated-formulas.jsonl
```

## Snapshot / input / pick / select-option

```bash
browser-automation-cli --json view
browser-automation-cli --json view --allow-empty
browser-automation-cli --json view --path /tmp/view.txt --detailed
browser-automation-cli --json press @e1 --include-snapshot
browser-automation-cli --json press @e1 --dblclick
browser-automation-cli --experimental-vision --json click-at --x 10 --y 20
browser-automation-cli --experimental-vision --json click-at --x 10 --y 20 --dblclick --include-snapshot
browser-automation-cli --json write @e2 "hello"
browser-automation-cli --json write @e2 "true" --include-snapshot
browser-automation-cli --json keys Enter
browser-automation-cli --json keys Escape --include-snapshot
browser-automation-cli --json type "hello" --target @e2 --clear --submit Enter
browser-automation-cli --json type "world" --focus-only
browser-automation-cli --json hover @e1
browser-automation-cli --json drag --from @e1 --to @e2
browser-automation-cli --json fill-form --fields-json '[{"target":"@e3","value":"x"}]'
browser-automation-cli --json upload @e4 /tmp/file.txt
# pick / select-option: inventory names; ONLY via run or exec (NOT standalone clap; custom select / badge / popover / role=option)
browser-automation-cli --json exec pick --target @e1 --option Anomalia
browser-automation-cli --json exec select-option --target @e2 --option Alta
```

```bash
cat > /tmp/pick-formulas.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"view"}
{"cmd":"pick","target":"@e1","option":"Anomalia"}
{"cmd":"select-option","target":"@e2","option":"Alta","include_snapshot":true}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/pick-formulas.jsonl
```

## Observation / print / assert / scroll / cookies / dialogs / eval

```bash
browser-automation-cli --json grab --path /tmp/page.png --full-page
# print-pdf REQUIRED: navigate first OR pass --url; blank about:blank without content is REFUSED
browser-automation-cli --json print-pdf --path /tmp/page.pdf --url https://example.com
# FORBIDDEN blank one-shot (expected refuse): browser-automation-cli --json print-pdf --path /tmp/blank.pdf
browser-automation-cli --json extract @e1
browser-automation-cli --json extract @e1 --attr href
browser-automation-cli --json extract --llm --question "Summarize the page" https://example.com
browser-automation-cli --json extract --llm --question "What is the main title?" --schema-json /tmp/extract.schema.json https://example.com
browser-automation-cli --json text @e2
browser-automation-cli --json scroll --delta-y 400
browser-automation-cli --json scroll --delta-x 100 --delta-y 200
browser-automation-cli --json attr @e1 href
browser-automation-cli --json attr @e1 value
browser-automation-cli --json assert url https://example.com --contains
browser-automation-cli --json assert text "Example"
browser-automation-cli --capture-console --json assert console --level error
browser-automation-cli --capture-console --json assert console-empty
browser-automation-cli --capture-console --json assert console-no-match --pattern TypeError
browser-automation-cli --json cookie list
browser-automation-cli --json cookie set --cookies-json '[{"name":"a","value":"b","url":"https://example.com"}]'
browser-automation-cli --json cookie clear
browser-automation-cli --json dialog accept
browser-automation-cli --json dialog accept --text "ok" --if-present
browser-automation-cli --json dialog dismiss --if-present
browser-automation-cli --json eval 'document.title'
```

## Capture (same process)

```bash
browser-automation-cli --capture-console --json console list
browser-automation-cli --capture-console --json console get 0
browser-automation-cli --capture-console --json console clear
browser-automation-cli --capture-console --json console dump --path /tmp/console.json
# empty dump MUST be valid JSON array "[]"
browser-automation-cli --capture-network --json net list
browser-automation-cli --capture-network --json net get 0
```

```bash
cat > /tmp/console-net.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"console","action":"list"}
{"cmd":"console","action":"clear"}
{"cmd":"console","action":"dump","path":"/tmp/console.json"}
{"cmd":"net","action":"list","resource_types":"Document,XHR"}
{"cmd":"assert","kind":"console_empty"}
{"cmd":"assert","kind":"console_no_match","pattern":"TypeError"}
JSONL
browser-automation-cli --capture-console --capture-network --timeout 60 --json run --script /tmp/console-net.jsonl
```

## Scrape / crawl / map / search / parse / monitor / qr / find-paths

```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http
browser-automation-cli --json scrape https://example.com --format markdown,links,metadata --engine http --only-main-content
browser-automation-cli --json scrape https://example.com --format text --format html --engine browser
browser-automation-cli --json scrape https://example.com --formats markdown --engine browser
browser-automation-cli --json scrape https://example.com --format raw-html --engine browser
browser-automation-cli --json scrape https://example.com --format links --engine browser
browser-automation-cli --json scrape https://example.com --format metadata --engine browser
browser-automation-cli --json scrape https://example.com --format screenshot --engine browser
browser-automation-cli --json scrape https://example.com --format summary --engine browser
browser-automation-cli --json scrape https://example.com --format product --engine browser
browser-automation-cli --json scrape https://example.com --format branding --engine browser
browser-automation-cli --json scrape https://example.com --format text --engine http --webhook-url https://127.0.0.1:9000/hook
printf '%s\n' https://example.com https://example.org > /tmp/urls.txt
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
browser-automation-cli --timeout 120 --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 1 --engine browser
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text
browser-automation-cli --timeout 120 --json crawl https://example.com --limit 10 --max-depth 1 --format markdown --engine browser
browser-automation-cli --json map https://example.com --limit 50 --max-depth 2
browser-automation-cli --json search "example domain" --limit 10
browser-automation-cli --json parse /tmp/page.html
browser-automation-cli --json parse /tmp/page.md
browser-automation-cli --json parse /tmp/page.txt
browser-automation-cli --json parse /tmp/doc.pdf
browser-automation-cli --json parse /tmp/doc.docx --redact-pii
browser-automation-cli --json parse /tmp/sheet.xlsx
browser-automation-cli --json parse /tmp/sheet.ods --redact-pii
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/example.baseline --write-baseline --engine http
browser-automation-cli --json monitor check --url https://example.com --baseline /tmp/example.baseline --engine http
browser-automation-cli --json qr encode --text "https://example.com" --format png --path /tmp/qr.png
browser-automation-cli --json qr decode --path /tmp/qr.png
browser-automation-cli --json find-paths --glob '**/*.rs' .
browser-automation-cli --json find-paths '\.rs$' . --extension rs --type f --limit 100
browser-automation-cli --json find-paths '\.md$' . --hidden --no-ignore --max-depth 4 --extension md --type f --limit 50
browser-automation-cli --json find-paths . --type d --max-depth 2 --limit 20
browser-automation-cli --json find-paths --glob '**/*.{rs,toml}' . --type f --limit 200
```

## Local IO — sheet-write / sg-scan / sg-rewrite (no Chrome)

```bash
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx
browser-automation-cli --json sheet-write /tmp/rows.csv -o /tmp/out.xlsx --sheet Data
browser-automation-cli --json sheet-write /tmp/rows.json -o /tmp/out.xlsx --sheet Sheet1
browser-automation-cli --json sg-scan .
browser-automation-cli --json sg-scan . --limit 100
browser-automation-cli --json sg-scan src tests --limit 500
browser-automation-cli --json sg-rewrite .
browser-automation-cli --json sg-rewrite . --apply
browser-automation-cli --json sg-rewrite src
```

## Emulate / resize / perf / lighthouse / screencast / heap / extension / third-party / webmcp

```bash
browser-automation-cli --json emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G"
browser-automation-cli --json resize --width 1280 --height 720
browser-automation-cli --json perf start
browser-automation-cli --json perf stop --path /tmp/trace.json
browser-automation-cli --json perf insight --name DocumentLatency
browser-automation-cli --json lighthouse https://example.com
browser-automation-cli --timeout 180 --json lighthouse https://example.com --lighthouse-path /usr/bin/lighthouse
browser-automation-cli --timeout 180 --json lighthouse https://example.com --out-dir /tmp/lh --device desktop --mode navigation
browser-automation-cli --timeout 180 --json lighthouse https://example.com | jaq '.data.binary_source // .binary_source // .'
browser-automation-cli --experimental-screencast --json screencast start --path /tmp/cast
browser-automation-cli --experimental-screencast --json screencast stop
browser-automation-cli --category-memory --json heap take --path /tmp/snap.heapsnapshot
browser-automation-cli --category-memory --json heap close
browser-automation-cli --category-memory --json heap summary --path /tmp/snap.heapsnapshot
browser-automation-cli --category-memory --json heap compare --base /tmp/a.heapsnapshot --current /tmp/b.heapsnapshot
browser-automation-cli --category-memory --json heap details --path /tmp/snap.heapsnapshot
browser-automation-cli --category-memory --json heap class-nodes --path /tmp/snap.heapsnapshot
browser-automation-cli --category-memory --json heap dominators --path /tmp/snap.heapsnapshot
browser-automation-cli --category-memory --json heap dup-strings --path /tmp/snap.heapsnapshot
browser-automation-cli --category-memory --json heap edges --path /tmp/snap.heapsnapshot --node-id 1
browser-automation-cli --category-memory --json heap retainers --path /tmp/snap.heapsnapshot --node-id 1
browser-automation-cli --category-memory --json heap paths --path /tmp/snap.heapsnapshot --node-id 1
browser-automation-cli --category-memory --json heap object-details --path /tmp/snap.heapsnapshot --node-id 1
# extension install|uninstall intentionally OUTSIDE run — ALWAYS top-level with --category-extensions
browser-automation-cli --category-extensions --json extension list
browser-automation-cli --category-extensions --json extension install --path /tmp/ext
browser-automation-cli --category-extensions --json extension reload --id <ext-id>
browser-automation-cli --category-extensions --json extension trigger --id <ext-id>
browser-automation-cli --category-extensions --json extension uninstall --id <ext-id>
# FORBIDDEN: extension install|uninstall inside run --script
browser-automation-cli --category-third-party --json devtools3p list
browser-automation-cli --category-third-party --json devtools3p exec SomeTool --params '{}'
browser-automation-cli --category-webmcp --json webmcp list
browser-automation-cli --category-webmcp --json webmcp exec SomeTool --input '{}'
```

## MITM (127.0.0.1 only)

```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --timeout 60 --json mitm capture-url https://example.com --seconds 30 --har /tmp/capture.har
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list --limit 50
browser-automation-cli --json mitm get 0
browser-automation-cli --json mitm har --out /tmp/capture.har
browser-automation-cli --json mitm export --out /tmp/capture.json
browser-automation-cli --json mitm domains
browser-automation-cli --json mitm apis
browser-automation-cli --json mitm graphql
browser-automation-cli --json mitm ws
browser-automation-cli --json mitm block example.com
browser-automation-cli --json mitm allow example.com
browser-automation-cli --json mitm redact
```

## Workflow / config / run / exec

```bash
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
browser-automation-cli config init --json
browser-automation-cli config path --json
browser-automation-cli config show --json
browser-automation-cli config list-keys --json
browser-automation-cli config get timeout --json
browser-automation-cli config set lang en --json
browser-automation-cli config set timeout 90 --json
browser-automation-cli config set artifacts_dir /tmp/browser-automation-cli-artifacts --json
browser-automation-cli config set ignore_robots false --json
browser-automation-cli config set namespace demo --json
browser-automation-cli config set encryption_key "replace-me" --json
browser-automation-cli config set color true --json
browser-automation-cli config set log_level info --json
browser-automation-cli config set log_to_file false --json
browser-automation-cli config set chrome_path /usr/bin/google-chrome --json
browser-automation-cli config set lighthouse_path /usr/bin/lighthouse --json
browser-automation-cli config set openrouter_api_key "replace-me" --json
browser-automation-cli config set llm_base_url "https://openrouter.ai/api/v1" --json
browser-automation-cli config set llm_model "openai/gpt-4o-mini" --json
browser-automation-cli config set cache_backend sqlite --json
browser-automation-cli config set cache_backend memory --json
browser-automation-cli config set cache_backend redis --json
browser-automation-cli config set cache_redis_url "redis://127.0.0.1:6379" --json
# FORBIDDEN: rediss:// (TLS fail-closed; plain redis:// only)
browser-automation-cli --timeout 60 --json run --script /tmp/steps.jsonl
browser-automation-cli --timeout 60 --json --json-steps run --script /tmp/steps.jsonl
browser-automation-cli --timeout 60 --json run --script /tmp/steps.array.json
browser-automation-cli --json exec goto https://example.com
browser-automation-cli --json exec wait --ms 500
browser-automation-cli --json exec pick --target @e1 --option Anomalia
```

## Clap --json usage error envelope (REQUIRED)

```bash
# When --json is already on argv, usage failures MUST emit JSON (NEVER prose-only)
set +e
out=$(browser-automation-cli --json not-a-real-cmd 2>/dev/null)
code=$?
set -e
echo "$out" | jaq -e '.ok == false'
echo "$out" | jaq -e '.error.kind == "usage"'
echo "exit=$code"
# exit MUST be 2
```

## Multi-step NDJSON templates

```bash
cat > /tmp/demo.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com","init_script":"window.__x=1","handle_before_unload":"accept","navigation_timeout_ms":15000}
{"cmd":"wait","ms":500}
{"cmd":"wait","selector":"h1, main","wait_timeout_ms":10000}
{"cmd":"wait","url_contains":"example.com"}
{"cmd":"page","action":"new","isolated_context":"my-ctx","url":"https://example.com"}
{"cmd":"view"}
{"cmd":"scroll","dy":400}
{"cmd":"assert","kind":"url","url_contains":"example.com"}
{"cmd":"print-pdf","path":"/tmp/example.pdf"}
{"cmd":"grab","path":"/tmp/example.png"}
JSONL
browser-automation-cli --timeout 60 --json run --script /tmp/demo.browser-automation.jsonl
browser-automation-cli --timeout 60 --json --json-steps run --script /tmp/demo.browser-automation.jsonl
```

```bash
cat > /tmp/form.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"fill-form","fields":[{"target":"@e3","value":"x"}]}
{"cmd":"write","target":"@e1","value":"hello"}
{"cmd":"pick","target":"@e4","option":"Anomalia"}
{"cmd":"select-option","target":"@e5","option":"Alta"}
{"cmd":"press","target":"@e2"}
{"cmd":"dialog","action":"dismiss","if_present":true}
{"cmd":"grab","path":"/tmp/form.png"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/form.browser-automation.jsonl
```

```bash
cat > /tmp/net.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"net","action":"list","resource_types":"Document,XHR"}
JSONL
browser-automation-cli --capture-network --timeout 60 --json run --script /tmp/net.browser-automation.jsonl
```

```bash
cat > /tmp/console.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":400}
{"cmd":"console","action":"list"}
{"cmd":"console","action":"dump","path":"/tmp/console.json"}
{"cmd":"assert","kind":"console_empty"}
JSONL
browser-automation-cli --capture-console --timeout 60 --json run --script /tmp/console.browser-automation.jsonl
```

## Multi-step JSON array templates (REQUIRED)

```bash
cat > /tmp/demo.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","ms":500},
  {"cmd":"wait","selector":"h1, main","wait_timeout_ms":10000},
  {"cmd":"view"},
  {"cmd":"scroll","dy":400},
  {"cmd":"assert","kind":"url","url_contains":"example.com"},
  {"cmd":"print-pdf","path":"/tmp/example-array.pdf"},
  {"cmd":"grab","path":"/tmp/example-array.png"}
]
JSON
browser-automation-cli --timeout 60 --json run --script /tmp/demo.array.json
browser-automation-cli --timeout 60 --json --json-steps run --script /tmp/demo.array.json
```

```bash
cat > /tmp/form.array.json <<'JSON'
[
  {"cmd":"goto","url":"https://example.com"},
  {"cmd":"wait","ms":500},
  {"cmd":"view"},
  {"cmd":"fill-form","fields":[{"target":"@e3","value":"x"}]},
  {"cmd":"write","target":"@e1","value":"hello"},
  {"cmd":"pick","target":"@e4","option":"Anomalia"},
  {"cmd":"press","target":"@e2"},
  {"cmd":"grab","path":"/tmp/form-array.png"}
]
JSON
browser-automation-cli --timeout 90 --json run --script /tmp/form.array.json
```

## Run fail-fast data.steps inspection

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
echo "$out" | jaq -r '.error.message // empty'
echo "$out" | jaq -r '.data.steps | map(.cmd) | @json'
echo "exit=$code"
```

## Workflow JSON template

```bash
cat > /tmp/wf.json <<'JSON'
{"name":"demo","steps":[{"id":"ping","cmd":"echo","args":{"message":"start"}}]}
JSON
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
```

## Robots dual-flag

```bash
browser-automation-cli --json scrape https://example.com --format text --engine http
browser-automation-cli --ignore-robots --i-accept-robots-risk --json scrape https://example.com --format text --engine http
```

## Logging (flags or XDG only)

```bash
browser-automation-cli --verbose --json version
browser-automation-cli --debug --json doctor --offline --quick
browser-automation-cli -q --json version
browser-automation-cli config set log_level debug --json
browser-automation-cli config set log_to_file true --json
browser-automation-cli config set log_to_file false --json
```

## Redis cache XDG setup (plain redis:// only)

```bash
browser-automation-cli config set cache_backend redis --json
browser-automation-cli config set cache_redis_url "redis://127.0.0.1:6379" --json
browser-automation-cli config get cache_backend --json
browser-automation-cli config get cache_redis_url --json
browser-automation-cli config list-keys --json
browser-automation-cli doctor --offline --quick --json
# FORBIDDEN: config set cache_redis_url "rediss://..."  (TLS fail-closed)
# ALWAYS fall back with: config set cache_backend sqlite
```

## Lighthouse binary_source inspection

```bash
browser-automation-cli --timeout 180 --json lighthouse https://example.com \
  | jaq '.data.binary_source // .binary_source // .'
browser-automation-cli config set lighthouse_path /usr/bin/lighthouse --json
browser-automation-cli --timeout 180 --json lighthouse https://example.com --lighthouse-path /usr/bin/lighthouse \
  | jaq -e '.ok == true'
# Resolve order REQUIRED: flag → XDG lighthouse_path → PATH
# Envelope binary_source MUST be real|mock
```

## Residual-zero disk verification (REQUIRED after browser work)

```bash
# ALWAYS one-shot residual-zero. AFTER browser DIE when idle, residual_disk MUST show zeros.
browser-automation-cli -q --timeout 60 --json goto https://example.com
out=$(browser-automation-cli -q doctor --offline --quick --json)
echo "$out" | jaq -e '.ok == true'
echo "$out" | jaq -e '.data.residual.cli_marker_dirs == 0'
echo "$out" | jaq -e '.data.residual.chromium_tmp_singleton_orphans == 0'
echo "$out" | jaq -e '.data.residual.live_cli_marker_processes == 0'
echo "$out" | jaq -e '(.data.residual.scavenge_safe_candidates | type) == "number"'
echo "$out" | jaq -e '[.data.checks[] | select(.id=="residual_disk") | .status][0] == "pass"'
# BORN scavenges stale Singleton-only Chromium tmp dirs (age ≥60s, owned, no live holder)
# FINALIZE kills CLI Chrome markers and dual-scavenges owned Chromium tmp + stale Singleton GC
# NEVER kill host user Chrome / Flatpak Chrome
browser-automation-cli --timeout 60 --json print-pdf --path /tmp/residual.pdf --url https://example.com
browser-automation-cli doctor --offline --quick --json | jaq -e '.data.residual.cli_marker_dirs == 0 and .data.residual.chromium_tmp_singleton_orphans == 0'
```

## Inventory checklist (63 names)

doctor commands schema version locale goto view press click-at write keys type wait hover drag fill-form select-option pick upload back forward reload eval grab print-pdf monitor run exec extract text scroll cookie attr assert console net page dialog scrape batch-scrape crawl map search parse qr find-paths sg-scan sg-rewrite sheet-write mitm workflow config emulate resize perf lighthouse screencast heap extension devtools3p webmcp completions man

ALWAYS confirm exactly 63 names above. NEVER invent alias `bac`. ALWAYS load at least one executable line per name from this catalog.
ALWAYS execute `pick` / `select-option` only inside `run` / `exec`.
