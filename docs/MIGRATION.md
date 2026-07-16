[English](MIGRATION.md) | [Português Brasileiro](MIGRATION.pt-BR.md)

# Migration — browser-automation-cli

> Move to the one-shot process model without guessing the command map.

## What Changes
- `0.1.0` is the first public product line
- Canonical command names are `view`, `press`, `write`, and `grab`
- Multi-step automation must use `run --script` in one process
- Category and experimental surfaces are opt-in

## Step-by-Step Migration
- Install the binary from path or git
- Replace session-daemon calls with one-shot subprocess invocations
- Rewrite multi-step agent plans into NDJSON scripts for `run`
- Switch output consumers to `--json` envelopes
- Map old tool names through `commands --json` and the DevTools tool map

## JSON Schema Changes
- Before: free-form prose or ad-hoc JSON without `schema_version`
- After success:
```json
{"schema_version":1,"ok":true,"data":{}}
```
- After error with `--json`:
```json
{"schema_version":1,"ok":false,"error":{"message":"..."}}
```
- Live per-command input fragments come from `schema --cmd`

## Compatibility Notes
- No previous stable crates.io line exists for this repository
- Branding and history cleanup recreated a clean public root commit
- `publish = false` until the first intentional crates.io release

## Rollback
- Pin to the previous local commit or installed binary path
- Keep scripts compatible with the success envelope fields `ok` and `schema_version`

## See Also
- [CHANGELOG.md](../CHANGELOG.md)
- [docs/AGENTS.md](AGENTS.md)
- [docs/schemas/README.md](schemas/README.md)
