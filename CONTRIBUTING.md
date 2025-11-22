# Contributing to RoboTorq

Thanks for jumping in. This is the short, human version. If anything below feels heavy, ignore the optional bits and open a PR — we can guide you.

## Super Simple Flow (little contributions)
1. Create a branch: `git checkout -b feature/<thing-you-fixed>`
2. Make the change.
3. Add/adjust a test (if it’s code, not docs).
4. Run `docker compose up -d` (if you need services) or just `go test` / `cargo test`.
5. Open a PR describing what changed and why.

That’s it. Fancy process only for big architectural shifts.

## Core Principles (Plain)
- Keep proofs and data honest.
- Small changes > giant rewrites.
- Log and measure new behavior.
- Don’t ship secrets.

## Getting Started (Details)
1. Branch or fork first — never push straight to `main`.
2. Peek at an architecture doc if you touch a service (`src/<service>/ARCHITECTURE.md`).
3. Scan `port mapping/PORT_MAPPINGS.md` before adding ports.
4. Use `docker compose up -d` if the code needs other services running.

## Branching Model
- Base branch: `main` (stable) & `release/*` (hardening / staging).
- Create feature branches: `feature/<concise-kebab-description>`.
- Avoid rebasing public branches; use merges or squash at PR completion.

## Commit Messages
Use whatever is readable and verbose enough to explain the key issues.

Handy patterns:
```
feat(refinery): single unit queue
fix(mint): wrong merkle leaf order
docs(vault): clarify decay formula
```
If unsure, just write a clear sentence.

## Pull Request Basics
Put these in the description:
- What you changed & why.
- Any test added or updated.
- If it breaks something existing: say what and how to migrate.

Optional extras (use only if relevant): metrics added, performance notes, security concerns.

## Testing (Practical)
- Add at least one test for new logic.
- If concurrency is involved, run `go test -race` or equivalent.
- Skip exhaustive coverage perfection for tiny fixes — we’ll nudge if needed.
- Use Grafana to view the full pipeline in action with e2e tests. If this is broken, do not push.

## Performance & Concurrency (Quick Rules)
- No tight loops that just sleep/spin.
- Don’t grow slices/maps forever.
- Add a metric if you create a new throughput/latency critical path.

## Security (Essentials)
- No secrets in code.
- Validate external input (length/format/basic sanity).
- Use existing crypto libs; don’t invent new primitives.
- Report real vulns privately (see `SECURITY.md`).

## Documentation
If you changed data flow or proofs: update that service’s `ARCHITECTURE.md`. README only gets lighter summaries.

## Code Style (Skim)
Use clear names, return errors up, structured logs. See `CODE_STYLE.md` if you want more detail — not required for small contributions.

## Large / Risky Changes (Design Proposal)
If you want to:
- Add/remove a service
- Change proof chain structure or merkle strategy
- Alter economic formulas (demurrage / UBD)

Then: open a GitHub issue first and add a `design` label.

Put in that issue:
1. Problem (one paragraph)
2. Proposed approach (bullets or quick diagram)
3. Impact (services, data, migration)
4. Alternatives (optional)
5. Open questions

Wait for maintainer feedback before heavy coding. Simple fixes skip this.

## License & Patent
By contributing you agree to Apache 2.0 + the Patent Pledge (see `LICENSE`). Only submit code you can legally donate.

## Communication
- Issues: bugs & feature ideas.
- Design proposals: issue with `design` label.
- Security: follow `SECURITY.md` for private reporting.

## Quick Reference Mini Checklist
Before opening a PR for code:
```
- Branch created
- Basic test added or updated
- No secrets
- Concurrency safe (if applicable)
- Architecture doc updated (only if you changed data/proofs)
```

Thanks for helping build physics‑anchored value.
