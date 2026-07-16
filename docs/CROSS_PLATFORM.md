[English](CROSS_PLATFORM.md) | [Português Brasileiro](CROSS_PLATFORM.pt-BR.md)

# Cross Platform — browser-automation-cli

> Stop rewriting browser automation for every host OS.

## The Pain You Already Know
- Browser tooling often assumes one OS path layout
- CI agents fail when Chrome discovery is host-specific and undocumented
- Shell quoting and path separators break fragile wrappers

## Support Matrix

| Platform | Status | Notes |
|----------|--------|-------|
| Linux x86_64 | primary | Chromium and Google Chrome common paths |
| Linux aarch64 | supported | requires local Chrome or Chromium |
| macOS x86_64 | supported | system Chrome discovery |
| macOS aarch64 | supported | system Chrome discovery |
| Windows x86_64 | supported | Windows-specific process helpers |

## Linux Notes
- Common binaries include `chromium-browser`, `chromium`, and `google-chrome`
- Run `doctor` after package install to confirm discovery
- Headless is default for agent CI

## macOS Notes
- Install Google Chrome from the official channel
- Prefer full binary path only when PATH discovery fails

## Windows Notes
- Use PowerShell or cmd with explicit quoting around URLs
- Prefer `--json` to avoid locale-dependent prose parsing

## Containers
- Install Chrome or Chromium in the image before runtime tests
- Provide enough shared memory for Chrome when the runtime requires it
- Keep one-shot process cleanup expectations under orchestration restarts

## Shell Support
- bash, zsh, fish, and PowerShell can spawn the binary
- Completions are generated through `completions <shell>`

## File Paths and XDG
- Artifacts follow `--artifacts-dir` when provided
- Cache and state locations stay under user-local directories
- Encrypted state requires `BROWSER_AUTOMATION_CLI_ENCRYPTION_KEY`

## Performance by Target
- Linux CI is the primary optimization target
- Cold start remains Chrome-bound on every OS

## Agents Validated per Platform
- Linux: Claude Code, Codex, shell CI, GitHub Actions
- macOS: local shell agents and editor integrations
- Windows: shell and editor integrations with explicit quoting
