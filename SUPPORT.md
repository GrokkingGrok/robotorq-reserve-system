# Support

Need help? This guide explains how to get assistance effectively.

## Self‑Service First
1. Read `README.md` for run / architecture basics.
2. Consult deep dives: proof chain (`docs/PROOF_CHAIN_ARCHITECTURE.md`), economics (`docs/ECONOMICS_OVERVIEW.md`).
3. Check existing issues: search before creating duplicates.
4. Inspect logs: `docker compose logs -f <service>`.
5. View dashboards: Grafana (http://localhost:3000) for pipeline health.

## Channels
| Type | Use Case | Channel |
|------|----------|---------|
| Bugs | Runtime defects | GitHub Issue (Bug Report template) |
| Features | Enhancements / new capabilities | GitHub Issue (Feature Request) |
| Security | Vulnerability / exploit concerns | Private per `SECURITY.md` |
| Architecture | Larger design changes | Feature Request + design label |
| Documentation | Missing / unclear docs | Issue labeled `docs` |

## Response Targets (Non‑binding)
| Severity | Initial Acknowledgement | Follow‑up | Resolution Goal |
|----------|-------------------------|----------|----------------|
| Critical security | 72h | Daily | ASAP hotfix |
| High | 5 days | Weekly | Sprint |
| Moderate | 7 days | Bi‑weekly | 2 Sprints |
| Low | 14 days | Monthly | Backlog |

## Quality of Reports
High‑quality submissions include:
- Minimal reproduction steps
- Affected commit / branch
- Relevant logs (redacted)
- Expected vs actual behavior
- Impact rationale

## Not Supported
- Fork‑specific customizations
- Proprietary integrations without reproducible open setup
- End‑user wallet balances (demo only pre‑v1)

## Commercial / Enterprise
No formal support program yet. Roadmap: post v1 stable.

## Escalation
If an acknowledged issue stalls without update beyond the follow‑up window, comment with `@maintainers escalation request` referencing prior dates.

## Code of Conduct
All interactions must follow `CODE_OF_CONDUCT.md`.

## License & Patent
Usage and contributions bound by `LICENSE` and Patent Pledge.

Thank you for building verifiable physics‑anchored value.
