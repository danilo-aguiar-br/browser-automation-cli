# Privacy Policy — browser-automation-cli

## Summary

This CLI is **local-first** and **agent-first**. It does **not** implement remote telemetry, analytics, crash-reporting phones-home, or third-party tracking endpoints.

## Data processed

| Kind | Where it stays |
|------|----------------|
| Browser sessions | Local Chrome process + temporary profile under the OS temp dir; reaped on FINALIZE |
| Residual temp dirs | Local only: CLI marker dirs (`browser-automation-cli-chrome-`) and stale Singleton-only Chromium dirs under `/tmp` scavenged by BORN/FINALIZE; doctor `residual_disk` inspects them locally — **never uploaded** |
| Config | XDG config dir only via `config` (`path`, `init`, `show`, `set`, `get`, `list-keys`) |
| UI locale preference | Set with `--lang pt-BR` or `config set lang pt-BR` (XDG). Resolved once at boot for human `suggestion` strings only; machine JSON stays English. **Never uploaded.** |
| Optional logs | Local file under XDG state when `log_to_file` is enabled |
| Cache | Local SQLite / optional Redis URL you configure via `config set` |
| LLM keys | XDG only (`openrouter_api_key`, `llm_base_url`, `llm_model`) — never hardcoded |

## What we never do

- No automatic upload of browsing data, screenshots, HAR, or heap snapshots
- No phone-home for version checks
- No advertising identifiers
- No mixing of secrets into stdout JSON envelopes beyond what you pass as arguments

## Operator responsibility

- You control which URLs and pages the tool opens
- You control whether MITM / network capture is enabled
- You are responsible for compliance when automating third-party sites (robots, ToS, personal data)

## Contact

See `SECURITY.md` for vulnerability reporting.
