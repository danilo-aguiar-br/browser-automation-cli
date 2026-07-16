[English](COOKBOOK.md) | [Português Brasileiro](COOKBOOK.pt-BR.md)

# Cookbook — browser-automation-cli

> Practical recipes with copy-ready commands for one-shot browser work.

## Latency Note
- Chrome launch dominates cold start
- Prefer one `run` script over many separate launches when steps share state

## Default Values Reference
- Global timeout default is `0` meaning no process wall budget unless set
- Step timeout default is `0` meaning inherit global timeout
- Headless mode is default unless `--headed`
- JSON is off unless `--json` or `BROWSER_AUTOMATION_CLI_JSON`

## How To Diagnose Install Health
```bash
browser-automation-cli doctor --offline --quick --json
```

## How To Open a Page and Snapshot
```bash
browser-automation-cli --timeout 60 --json goto https://example.com
browser-automation-cli --json view
```

## How To Click and Fill in One Process
```bash
cat > /tmp/form.browser-automation.jsonl <<'JSONL'
{"cmd":"goto","url":"https://example.com"}
{"cmd":"view"}
{"cmd":"write","target":"input","value":"hello"}
{"cmd":"press","target":"button"}
JSONL
browser-automation-cli --timeout 90 --json run --script /tmp/form.browser-automation.jsonl
```

## How To Capture a Full-page Screenshot
```bash
browser-automation-cli --timeout 60 --json grab /tmp/page.png --full-page
```

## How To List Network Requests
```bash
browser-automation-cli --capture-network --timeout 60 --json run --script /tmp/nav.jsonl
browser-automation-cli --capture-network --json net list --resource-types Document,XHR
```
- Capture flags must apply to the process that performs navigation

## How To Evaluate JavaScript
```bash
browser-automation-cli --json eval 'document.title'
```

## How To Emulate a Mobile Viewport
```bash
browser-automation-cli --json emulate --device "iPhone 12"
browser-automation-cli --json resize --width 390 --height 844
```

## How To Run a Lighthouse Audit
```bash
browser-automation-cli --timeout 180 --json lighthouse https://example.com
```

## How To Inspect Heap Snapshots
```bash
browser-automation-cli --category-memory --json heap summary --path snap.heapsnapshot
```

## How To Generate Shell Completions
```bash
browser-automation-cli completions bash
```

## How To Discover Command Schemas
```bash
browser-automation-cli commands --json
browser-automation-cli schema --cmd goto --json
```
