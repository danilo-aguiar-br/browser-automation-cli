[English](INTEGRATIONS.md) | [Português Brasileiro](INTEGRATIONS.pt-BR.md)

# Integrations — browser-automation-cli

> One process, one Chrome, one JSON envelope. Built for agent subprocesses.

## Coverage Snapshot
- Works with any agent that can spawn a subprocess and read stdout plus stderr
- Primary surfaces: Claude Code, Codex, Cursor, local shell, editor agents
- Discovery helpers: `commands --json`, `schema --cmd`, `doctor --json`
- Integration path is local subprocess only

## Flag Aliases and Version Notes
- Product names stay fixed: `view`, `press`, `write`, `grab`
- Avoid inventing aliases such as `click` or `screenshot` in agent prompts
- `0.1.0` ships the default-on DevTools parity surface plus category gates
- Experimental tools require `--experimental-vision` or `--experimental-screencast`

## Summary Table

| Surface | Integration style | Required flags | Notes |
|---------|-------------------|----------------|-------|
| Claude Code | subprocess | `--json` | multi-step via `run --script` |
| Codex | subprocess | `--json -q` | quiet stderr for cleaner transcripts |
| Cursor | shell tool | `--json` | keep timeouts explicit |
| Local shell | script | `--json` | parse with `jaq` |
| Continue / Cline | editor shell | `--json -q` | one-shot only |

## Claude Code
- Spawn one CLI process per atomic action
- Use `run --script` when `@eN` refs must survive multiple steps
```bash
browser-automation-cli doctor --offline --quick --json
browser-automation-cli --json goto https://example.com
browser-automation-cli --json view
```

## Codex
- Prefer `-q --json` so only envelopes reach the agent transcript
```bash
browser-automation-cli -q --json goto https://example.com
```

## Cursor
- Call the binary from the shell tool with explicit `--timeout`
```bash
browser-automation-cli --timeout 60 --json scrape https://example.com
```

## Local Shell
- Always capture exit codes before parsing JSON
- Run validations on your local machine before release
```bash
out=$(browser-automation-cli --json version)
echo "$out" | jaq -e '.ok == true'
```

## Continue and Cline
- Use quiet JSON mode to keep editor transcripts clean
- Do not expect session stickiness between separate process launches

## New Flags by Version
- `0.1.0`: category gates, experimental vision and screencast, capture flags, schema discovery
