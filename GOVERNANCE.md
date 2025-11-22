# Project Governance

RoboTorq aims for transparent, lightweight governance suited to an emerging open reserve system.

## Roles
| Role | Responsibilities |
|------|------------------|
| Maintainer | Review & merge PRs, triage issues, enforce Code of Conduct, coordinate releases |
| Contributor | Submit PRs/issues, improve docs/tests, follow guidelines |
| Security Reviewer | Assist in vulnerability triage & crypto changes audit |
| Architect (ad hoc) | Draft RFCs for major cross‑service changes |

## Decision Process
- **Minor changes** (bug fixes, small features): decided via PR review (≥1 maintainer approval + passing CI).
- **Moderate changes** (new service, protocol adjustment): open Feature Request with design section → discussion → PR after consensus.
- **Major changes / RFCs** (proof chain redesign, cryptographic shifts): design issue labeled `rfc` + sequence diagrams + migration plan. Require ≥2 maintainer approvals.

## Release Management
- Branch `main`: stable snapshots.
- Branch `release/*`: staging & hardening; may receive backports.
- Tags: semantic versioning (e.g. `v0.1.0`), pre‑1.0 may introduce breaking changes with clear notes.

## Conflict Resolution
1. Seek technical consensus in PR / issue discussion.
2. Escalate to maintainers if stalemate.
3. Maintainers decide based on project principles (verifiability, safety, clarity, performance).

## Adding Maintainers
Criteria:
- Sustained quality contributions (code, tests, docs)
- Demonstrated architectural understanding (proof chain, demurrage, reserve logic)
- Positive community conduct
Nomination via issue labeled `maintainer-nomination`; decision by existing maintainers.

## Removing Maintainers
Grounds: inactivity (6+ months), repeated guideline violations, CoC breaches. Decision by ≥2 active maintainers; public rationale documented.

## Security & Crypto Changes
- Require explicit review by a Security Reviewer.
- Must include test vectors and backward compatibility plan (if applicable).

## Transparency
- No private decision making except for security triage prior to disclosure.
- All other strategic changes tracked in issues / discussions.

## Amendments
Propose governance changes via PR modifying this file with clear rationale and impact statement.

## Patent & License Alignment
All governance decisions must preserve Apache 2.0 + Patent Pledge intent (open, non‑restrictive availability).

---
Focus: enabling reliable physics‑anchored value—not accumulating gatekeeping power.
