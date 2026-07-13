# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 2.1.x   | :white_check_mark: |
| < 2.0   | :x:                |

## Reporting a Vulnerability

**Do NOT open a public GitHub issue for security vulnerabilities.**

If you discover a security vulnerability in X-MaC:

1. Email the maintainers directly (see the repo's contact info)
2. Or open a [private security advisory](https://github.com/davidnichols-ops/X-MaC/security/advisories/new)

Please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

You will receive a response within 48 hours. If the vulnerability is confirmed, a fix will be released as soon as possible and you will be credited (unless you prefer to remain anonymous).

## Security Design Principles

X-MaC is designed with safety as the top priority:

- **Trash-first cleanup** — files are moved to Trash, never `rm -rf`'d. Permanent deletion requires explicit confirmation.
- **No network calls** — the GNN model runs entirely on-device. No telemetry, no analytics, no phone-home.
- **No root required** — the CLI and GUI run as the current user. Sudo is only requested for specific maintenance tasks (DNS flush, purge) and is always optional.
- **Dry-run by default** — `xmac clean` scans but does not delete. `xmac purge` requires explicit confirmation.
- **Undo support** — cleanup transactions are recorded for undo via `xmac undo`.

## Scope

- Vulnerabilities in the Rust scan engine (`src/`)
- Vulnerabilities in the SwiftUI app (`gui/`)
- Vulnerabilities in the GNN inference pipeline (`gnn/`)
- Privilege escalation through maintenance commands
- Data exposure through scan output

**Out of scope:** vulnerabilities in third-party dependencies (report upstream).
