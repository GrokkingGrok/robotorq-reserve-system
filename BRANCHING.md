# 🧭 Branching & Version Policy

## Overview
This repository follows a **simple, safe branching model** designed for parallel development and easy rollback.  
The goal is to protect known-good baselines (`v0`), isolate experimental work (`feature/*`), and promote stable versions to `main`.

```text
main ───────────────▶ production / long-term line
│
└── v0 ─────────────▶ stable baseline (frozen)
│
├── feature/trust-refactor
├── feature/digger-outflow
└── feature/distodam-funding
```

---

## Branch Types

| Branch | Purpose | Who Updates It |
|:--|:--|:--|
| **`main`** | Production / stable release line | Maintainers only |
| **`v0`** | Locked-in baseline of version 0 | Maintainers |
| **`feature/*`** | Active development branches for new features, refactors, or experiments | Anyone |
| **`hotfix/*`** | Quick fixes cut directly from `main` | Maintainers (when needed) |

---

## Workflow

### 1️⃣  Starting a new feature
Create your branch from the current baseline (`v0`):

```bash
git checkout v0
git pull origin v0
git checkout -b feature/<short-description>
```

Examples:

```bash
git checkout -b feature/trust-refactor
git checkout -b feature/digger-outflow
```

### 2️⃣  Working on your branch

Commit early and often:

```bash
git add .
git commit -m "Refactor Trust service to use outflow queue"
git push -u origin feature/trust-refactor
```

### 3️⃣  Merging back

Open a pull request **into the same baseline** (`v0` for now).
After testing and review, merge to `v0`.

When the next version is stable, `v0` can be merged into `main`:

```bash
git checkout main
git merge v0
git push origin main
```

---

## Tagging Releases

Tag known-good commits so you can always rebuild them:

```bash
git checkout v0
git tag -a v0.0.0 -m "Stable baseline before Trust refactor"
git push origin v0.0.0
```

Later versions:

```bash
git tag -a v1.0.0 -m "Stable release with Trust refactor"
git push origin v1.0.0
```

---

## Branch Lifetime & Cleanup

* Keep feature branches short-lived.
* Once merged and no longer needed, delete them:

  ```bash
  git branch -d feature/trust-refactor
  git push origin --delete feature/trust-refactor
  ```
* Never force-push to `main` or `v0`.

---

## Optional Conventions

| Prefix      | Meaning                         |
| ----------- | ------------------------------- |
| `feature/*` | New feature or refactor         |
| `fix/*`     | Bug fix not urgent              |
| `hotfix/*`  | Critical production fix         |
| `docs/*`    | Documentation updates           |
| `infra/*`   | CI/CD or infrastructure changes |

---
