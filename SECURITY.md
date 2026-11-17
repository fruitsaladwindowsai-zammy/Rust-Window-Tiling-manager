# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

---

## Reporting a Vulnerability

Security issues and vulnerabilities should be reported responsibly. Please do **not** open public GitHub issues for security vulnerabilities.

### Reporting Process
1. Email your report or open a private security advisory on GitHub via **Security** > **Advisories** > **Report a vulnerability**.
2. Include:
   * Description of the vulnerability and impact.
   * Minimal reproduction steps or proof-of-concept (PoC).
   * Your Windows build version (e.g. Windows 11 23H2 / 24H2).
3. We will acknowledge receipt of your report within 48 hours and work on a coordinated fix and advisory.

---

## Privileges & Integrity Model

WinScroller 2D optionally runs with High Integrity Level (Administrator) to navigate across elevated windows via Windows UIPI. It does not perform network listening (the IPC named pipe is purely local with a local DACL).
