[English](SECURITY.md) | [Português Brasileiro](SECURITY.pt-BR.md)

# Security Policy

## Supported Versions
- `0.1.x` is the current supported line

| Version | Supported |
|---------|-----------|
| 0.1.x   | yes       |

## Reporting a Vulnerability
- Do not open a public GitHub issue for security-sensitive problems
- Email the maintainer at daniloaguiarbr@proton.me
- Include a clear description and attack scenario
- Include steps to reproduce and the affected version
- Include expected versus actual behaviour
- Include known mitigations when available

## Response SLA
- Critical (CVSS 9.0-10.0): acknowledge within 24 hours
- High (CVSS 7.0-8.9): acknowledge within 48 hours
- Medium (CVSS 4.0-6.9): acknowledge within 72 hours
- Low (CVSS 0.1-3.9): acknowledge within 5 business days

## Fix SLA
- Critical: target fix or mitigation within 7 days after confirmation
- High: target fix within 14 days after confirmation
- Medium: target fix within 30 days after confirmation
- Low: target fix in the next scheduled release window

## Disclosure Policy
- Coordinate disclosure timing with the reporter
- Prefer private fixes before public advisory text
- Credit reporters who want recognition after the fix ships

## Security Update Policy
- Security fixes ship in patch releases when possible
- CHANGELOG entries mark security fixes under Fixed
- Users should upgrade to the latest supported patch promptly

## Hall of Fame
- No public security reports have been credited yet
- Legitimate reporters may be listed here after coordinated disclosure

## Best Practices for Users
- Keep Chrome or Chromium updated on the host
- Never pass secrets on argv when stdin alternatives exist
- Treat `--ignore-robots` as an explicit high-risk choice
- Keep `BROWSER_AUTOMATION_CLI_ENCRYPTION_KEY` out of shell history and logs
- Prefer `--json` pipelines that discard stderr secrets from durable logs
- Do not point the CLI at untrusted pages without isolation expectations
