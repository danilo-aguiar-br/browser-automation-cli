# browser-automation-cli — Catálogo Completo de Fórmulas Executáveis

Companheiro OBRIGATÓRIO de `SKILL.md`. CADA comando de topo tem ao menos uma linha executável.
DEVE copiar argv as-is salvo quando `schema --cmd <name> --json` forçar mudança.
DEVE passar `--json` global em invocações programáticas.
Nome do binário é exatamente `browser-automation-cli` (NUNCA alias `bac`).

## Meta / discovery

```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli commands --json
browser-automation-cli schema --cmd goto --json
browser-automation-cli version --json
browser-automation-cli completions bash
```

## Navigation

```bash
browser-automation-cli --json goto https://example.com
browser-automation-cli --timeout 60 --json goto https://example.com --init-script 'window.__ready=true' --handle-before-unload --navigation-timeout-ms 15000
browser-automation-cli --json back
browser-automation-cli --json forward
browser-automation-cli --json reload --ignore-cache
browser-automation-cli --json page info
browser-automation-cli --json page list
browser-automation-cli --json page new --url https://example.com
browser-automation-cli --json page select 0 --bring-to-front
browser-automation-cli --json page close --index 0
browser-automation-cli --json wait --ms 500
browser-automation-cli --json wait --text Example --text Demo --ms 1000
browser-automation-cli --json wait --selector "h1" --state load
```

## Snapshot / input

```bash
browser-automation-cli --json view
browser-automation-cli --json press @e1 --include-snapshot
browser-automation-cli --experimental-vision --json click-at --x 10 --y 20
browser-automation-cli --json write @e2 "hello"
browser-automation-cli --json keys Enter
browser-automation-cli --json type "hello" --target @e2 --clear --submit Enter
browser-automation-cli --json type "world" --focus-only
browser-automation-cli --json hover @e1
browser-automation-cli --json drag --from @e1 --to @e2
browser-automation-cli --json fill-form --json '[{"target":"@e3","value":"x"}]'
browser-automation-cli --json upload @e4 /tmp/file.txt
```

## Observation / print / assert / scroll / cookies / dialogs / eval

```bash
browser-automation-cli --json grab --path /tmp/page.png --full-page
browser-automation-cli --json print-pdf --path /tmp/page.pdf --url https://example.com
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
browser-automation-cli --json cookie list
browser-automation-cli --json cookie set --json '[{"name":"a","value":"b","url":"https://example.com"}]'
browser-automation-cli --json cookie clear
browser-automation-cli --json dialog accept
browser-automation-cli --json dialog dismiss
browser-automation-cli --json eval 'document.title'
```

## Capture (same process)

```bash
browser-automation-cli --capture-console --json console list
browser-automation-cli --capture-console --json console get 0
browser-automation-cli --capture-console --json console clear
browser-automation-cli --capture-console --json console dump --path /tmp/console.json
browser-automation-cli --capture-network --json net list
browser-automation-cli --capture-network --json net get 0
```

## Scrape / crawl / map / search / parse / monitor / qr / find-paths

```bash
browser-automation-cli --json scrape https://example.com --format markdown --engine http
browser-automation-cli --json scrape https://example.com --format markdown --engine http --only-main-content
browser-automation-cli --json scrape https://example.com --format markdown --engine browser
browser-automation-cli --json scrape https://example.com --format text --engine http
browser-automation-cli --json scrape https://example.com --format html --engine browser
browser-automation-cli --json scrape https://example.com --format raw-html --engine browser
browser-automation-cli --json scrape https://example.com --format links --engine browser
browser-automation-cli --json scrape https://example.com --format metadata --engine browser
browser-automation-cli --json scrape https://example.com --format screenshot --engine browser
browser-automation-cli --json scrape https://example.com --format summary --engine browser
browser-automation-cli --json scrape https://example.com --format product --engine browser
browser-automation-cli --json scrape https://example.com --format branding --engine browser
browser-automation-cli --json scrape https://example.com --format text --engine http --webhook-url https://127.0.0.1:9000/hook
browser-automation-cli --json batch-scrape --urls-file /tmp/urls.txt --format text --concurrency 2
browser-automation-cli --json crawl https://example.com --limit 20 --max-depth 2 --format text
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
browser-automation-cli --json find-paths '\.rs$' . --extension rs --type f --limit 100
browser-automation-cli --json find-paths '\.md$' . --hidden --no-ignore --max-depth 4 --extension md --type f --limit 50
browser-automation-cli --json find-paths . --type d --max-depth 2 --limit 20
```

## Emulate / resize / perf / lighthouse / screencast / heap / extension / third-party / webmcp

```bash
browser-automation-cli --json emulate --user-agent "Mozilla/5.0" --viewport "390x844x3,mobile,touch" --network-conditions "Slow 3G"
browser-automation-cli --json resize --width 1280 --height 720
browser-automation-cli --json perf start
browser-automation-cli --json perf stop --path /tmp/trace.json
browser-automation-cli --json perf insight --name DocumentLatency
browser-automation-cli --json lighthouse https://example.com
browser-automation-cli --experimental-screencast --json screencast start --path /tmp/cast
browser-automation-cli --experimental-screencast --json screencast stop
browser-automation-cli --category-memory --json heap take --path /tmp/snap.heapsnapshot
browser-automation-cli --category-memory --json heap summary --path /tmp/snap.heapsnapshot
browser-automation-cli --category-memory --json heap compare --base /tmp/a.heapsnapshot --current /tmp/b.heapsnapshot
browser-automation-cli --category-extensions --json extension list
browser-automation-cli --category-extensions --json extension install --path /tmp/ext
browser-automation-cli --category-third-party --json devtools3p list
browser-automation-cli --category-third-party --json devtools3p exec SomeTool --params '{}'
browser-automation-cli --category-webmcp --json webmcp list
browser-automation-cli --category-webmcp --json webmcp exec SomeTool --input '{}'
```

## MITM (127.0.0.1 only)

```bash
browser-automation-cli --json mitm init-ca
browser-automation-cli --json mitm start --seconds 30
browser-automation-cli --json mitm status
browser-automation-cli --json mitm list --limit 50
browser-automation-cli --json mitm get 0
browser-automation-cli --json mitm har --out /tmp/capture.har
browser-automation-cli --json mitm export --out /tmp/capture.json
browser-automation-cli --json mitm domains
browser-automation-cli --json mitm apis
```

## Workflow / config / run / exec

```bash
browser-automation-cli --json workflow run --manifest /tmp/wf.json
browser-automation-cli --json workflow resume --manifest /tmp/wf.json
browser-automation-cli --json workflow status --name demo
browser-automation-cli config init --json
browser-automation-cli config path --json
browser-automation-cli config show --json
browser-automation-cli config get timeout --json
browser-automation-cli config set lang en --json
browser-automation-cli config set timeout 90 --json
browser-automation-cli config set artifacts_dir /tmp/browser-automation-cli-artifacts --json
browser-automation-cli config set ignore_robots false --json
browser-automation-cli config set namespace demo --json
browser-automation-cli config set encryption_key "replace-me" --json
browser-automation-cli config set color true --json
browser-automation-cli config set log_level info --json
browser-automation-cli config set chrome_path /usr/bin/google-chrome --json
browser-automation-cli config set lighthouse_path /usr/bin/lighthouse --json
browser-automation-cli config set openrouter_api_key "replace-me" --json
browser-automation-cli config set llm_base_url "https://openrouter.ai/api/v1" --json
browser-automation-cli config set llm_model "openai/gpt-4o-mini" --json
browser-automation-cli --timeout 60 --json run --script /tmp/steps.jsonl
browser-automation-cli --json exec goto https://example.com
```

## Multi-step NDJSON templates

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
```

```bash
cat > /tmp/form.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"wait","ms":500}
{"cmd":"view"}
{"cmd":"fill-form","fields":[{"target":"@e3","value":"x"}]}
{"cmd":"write","target":"@e1","value":"hello"}
{"cmd":"press","target":"@e2"}
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
JSONL
browser-automation-cli --capture-console --timeout 60 --json run --script /tmp/console.browser-automation.jsonl
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
```

## Inventário checklist (56 nomes)

doctor commands schema version goto view press click-at write keys type wait hover drag fill-form upload back forward reload eval grab print-pdf monitor run exec extract text scroll cookie attr assert console net page dialog scrape batch-scrape crawl map search parse qr find-paths mitm workflow config emulate resize perf lighthouse screencast heap extension devtools3p webmcp completions
