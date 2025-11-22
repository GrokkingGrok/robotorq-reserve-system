# Security Policy

RoboTorq is a physics‑anchored monetary system; integrity and safety are non‑negotiable. This document explains how to report vulnerabilities and our handling process.

## Supported Branches
| Branch | Status | Accepting Security Fixes |
|--------|--------|---------------------------|
| `main` | Stable | Yes |
| `release/*` | Staging | Yes (prioritized) |
| `feature/*` | Development | Case‑by‑case |

Unmaintained or archived branches are not patched.

## Reporting a Vulnerability
Please do NOT open a public GitHub issue for sensitive security findings.

Preferred channels:
1. Direct Message: `@ProjectAsimov on X`
2. GitHub Security Advisory ("Report a vulnerability" in repository Security tab)

Include:
- A clear description of the issue
- Affected components (service names and paths)
- Steps to reproduce (minimal PoC)
- Impact assessment (confidentiality, integrity, availability)
- Suggested remediation (if known)
- Your preferred contact method

We will acknowledge receipt and provide an estimated triage timeline.

## Handling Process
1. Triage: validate reproducibility & severity
2. Classification: assign CVSS vector (internal)
3. Mitigation: develop & test patch in private branch
4. Coordination: request additional details if needed
5. Disclosure: publish advisory + patch

## Disclosure Policy
- Critical issues (remote code execution, proof chain manipulation, key exposure) are fast‑tracked.
- We prefer coordinated disclosure: you agree not to publish details until patch release.
- Public advisories include credit if you request acknowledgement.

## Cryptography Notes
- All cryptographic primitives must use approved libraries (see `crypto refactor docs/`).
- Do not implement custom hashing or signature schemes.
- File changes affecting proof chain logic require thorough review + test vectors.

## Secure Development Guidelines
- No secrets (API keys, private certs) committed—use environment variables.
- Sanitize/validate external inputs (length, type, range, encoding).
- Avoid dynamic code execution from user input.
- Enforce timeouts and resource limits for workload execution (digger contracts).
- Prefer constant‑time comparisons for sensitive tokens/hashes when feasible.

## Logging & Privacy
- Avoid logging private keys, raw secrets, or full auth tokens.
- Redact or hash identifiers if storing for metrics.

## Dependencies
- Keep Docker base images minimal and updated.
- Use `go mod tidy`, `cargo update`, and dependency scanning (GitHub Dependabot) regularly.

## Vulnerability Classes of Interest
- Proof chain falsification / replay
- Merkle root collision attacks
- Demurrage manipulation (bypass decay or double extraction)
- Unauthorized minting / ingot forging
- Privilege escalation (service → service lateral movement)
- Data exfiltration through message bus

## Bug Bounty (Planned)
A formal bounty program is planned post v1.0. Until then, high‑quality responsibly disclosed reports may receive public recognition.

## Emergency Contact
If you believe active exploitation is underway, mark subject: `URGENT: ACTIVE EXPLOIT`.

## Out of Scope (Non‑Security)
- Markdown typos
- Stylistic code style issues
- Feature requests

## After Disclosure
We will:
- Publish patch & advisory summary
- Attribute reporter (if desired)
- Update relevant docs and tests

Thank you for helping secure physics‑anchored economics.
