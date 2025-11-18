# Year 2100 Security Review - Branching Strategy

**Date**: November 18, 2025  
**Base Branch**: `year2100` (parent: `v0`)  
**Purpose**: Systematic branching strategy for 25-task security review to prevent integration chaos

---

## 🌳 Branch Hierarchy

```
v0 (baseline, stable)
 │
 └── year2100 (security review base, PROTECTED)
      ├── year2100/task-01-crypto-versioning
      ├── year2100/task-02-distodam-keys
      ├── year2100/task-03-governance
      ├── year2100/task-04-vault-monitoring
      ├── year2100/task-05-multi-verifier
      ├── year2100/task-06-privacy
      ├── year2100/task-07-event-sourcing ⚠️ CRITICAL
      ├── year2100/task-08-ux
      ├── year2100/task-09-wrapper-detection
      ├── year2100/task-10-nats-resilience ⚠️ CRITICAL
      ├── year2100/task-11-backing-monitor
      ├── year2100/task-12-succession
      ├── year2100/task-13-protocol-spec
      ├── year2100/task-14-verifier-oath ⚠️ CRITICAL
      ├── year2100/task-15-oracle-clarification
      ├── year2100/task-16-cross-reference
      ├── year2100/task-17-action-plan
      ├── year2100/task-18-arch-updates
      ├── year2100/task-19-final-review
      ├── year2100/task-20-service-plans (creates Plans/ directories)
      ├── year2100/task-21-simulation
      ├── year2100/task-22-security-ceremony
      ├── year2100/task-23-printer-review
      ├── year2100/task-24-trust-review
      └── year2100/task-25-service-implementation-plans
           ├── year2100/task-25/vault
           ├── year2100/task-25/wallet
           ├── year2100/task-25/mint
           ├── year2100/task-25/refinery
           ├── year2100/task-25/distodam
           ├── year2100/task-25/bidnet
           ├── year2100/task-25/digger
           ├── year2100/task-25/trust
           ├── year2100/task-25/printer
           └── year2100/task-25/simulation
```

---

## 🎯 Workflow Rules

### 1. Base Branch Protection
**`year2100` branch is PROTECTED**:
- Never commit directly to `year2100`
- All work happens on feature branches
- Only merge via pull requests (optional) or verified clean merges
- Keep `year2100` deployable at all times

### 2. Feature Branch Naming
**Pattern**: `year2100/task-{NN}-{short-description}`

**Examples**:
```bash
year2100/task-07-event-sourcing
year2100/task-10-nats-resilience
year2100/task-14-verifier-oath
year2100/task-25/vault
```

**Why**: Clear task association, easy filtering (`git branch | grep year2100/`), namespace isolation

### 3. Branch Lifecycle

#### Create Branch
```powershell
# Ensure you're on year2100
git checkout year2100
git pull origin year2100  # Sync with remote

# Create task branch
git checkout -b year2100/task-07-event-sourcing
```

#### Work on Task
```powershell
# Make changes
# ...

# Commit frequently (atomic commits)
git add .
git commit -m "feat(vault): Add JetStream dual-write for vault_events

- Configure JetStream stream: vault_events (10-year retention)
- Implement dual-write: PostgreSQL + JetStream
- Add disaster recovery procedure docs

Addresses: Task 7 (Event Sourcing Loss), Threat #7"

# Push to remote (backup + collaboration)
git push origin year2100/task-07-event-sourcing
```

#### Complete Task
```powershell
# Ensure branch is up to date
git checkout year2100
git pull origin year2100

git checkout year2100/task-07-event-sourcing
git rebase year2100  # Resolve conflicts if any

# Merge back to year2100
git checkout year2100
git merge --no-ff year2100/task-07-event-sourcing -m "merge: Task 7 (Event Sourcing) complete

Implemented:
- NATS JetStream dual-write for vault_events
- 10-year retention, immutable logs
- Disaster recovery procedures
- Acceptance tests (recovery, failover, replay)

Status: Task 7 COMPLETE (Catastrophic severity mitigated)
Closes: YEAR_2100_REVIEW_TODO_FIXED.md Task 7"

# Push merged year2100
git push origin year2100

# Delete feature branch (optional, keep for audit trail)
git branch -d year2100/task-07-event-sourcing
git push origin --delete year2100/task-07-event-sourcing
```

---

## 🔀 Merge Strategy

### Sequential Tasks (Dependencies)
If Task B depends on Task A, merge A first:

```powershell
# Complete Task 7 (Event Sourcing)
git checkout year2100
git merge --no-ff year2100/task-07-event-sourcing
git push origin year2100

# Now start Task 4 (Vault Monitoring) which depends on Task 7
git checkout -b year2100/task-04-vault-monitoring
# Task 4 now has Task 7's changes
```

### Parallel Tasks (No Dependencies)
Can work simultaneously on different branches:

```powershell
# Developer 1: Task 1 (Crypto Versioning)
git checkout -b year2100/task-01-crypto-versioning

# Developer 2: Task 2 (DistoDam Keys) - independent
git checkout year2100
git checkout -b year2100/task-02-distodam-keys

# Merge independently when done
# First to merge: no conflicts
# Second to merge: may need rebase
```

### Conflict Resolution
```powershell
# If conflict during merge
git checkout year2100
git merge year2100/task-14-verifier-oath
# CONFLICT in src/mint/MINT_ARCHITECTURE.md

# Resolve conflict in editor
# Then:
git add src/mint/MINT_ARCHITECTURE.md
git commit -m "merge: Resolve Task 14 conflict in MINT_ARCHITECTURE.md"
git push origin year2100
```

---

## 📊 Task Grouping Strategies

### Strategy 1: Critical Path First (Recommended)
Complete CRITICAL tasks before mainnet:

```bash
# Week 1: Catastrophic Threats
year2100/task-07-event-sourcing      # 3 days
year2100/task-10-nats-resilience     # 3 days

# Week 2: Critical Threats
year2100/task-01-crypto-versioning   # 2 days
year2100/task-14-verifier-oath       # 2 days
year2100/task-11-backing-monitor     # 2 days

# Week 3: Meta-Tasks
year2100/task-16-cross-reference     # 1 day
year2100/task-17-action-plan         # 1 day
year2100/task-18-arch-updates        # 3 days

# Week 4: Service Plans
year2100/task-25-service-implementation-plans  # 5-7 days (parallelizable)
```

**Merge Order**: 7 → 10 → 1 → 14 → 11 → 16 → 17 → 18 → 25

### Strategy 2: Parallel Workstreams
Assign independent tasks to different team members:

**Workstream A (Infrastructure)**:
- Task 7 (Event Sourcing)
- Task 10 (NATS Resilience)
- Task 4 (Vault Monitoring)

**Workstream B (Crypto & Identity)**:
- Task 1 (Crypto Versioning)
- Task 14 (Verifier Oath)
- Task 5 (Multi-Verifier)

**Workstream C (Economics)**:
- Task 9 (Wrapper Detection)
- Task 11 (Backing Monitor)
- Task 21 (Simulations)

**Workstream D (Documentation)**:
- Task 13 (Protocol Spec)
- Task 18 (Arch Updates)
- Task 25 (Service Plans)

**Merge Coordination**: Daily sync to merge completed tasks, resolve conflicts early

### Strategy 3: Task 25 Sub-Branches
Since Task 25 creates 10 service plans, use sub-branches:

```bash
# Create Task 25 base branch
git checkout year2100
git checkout -b year2100/task-25-service-plans

# Create sub-branches for each service
git checkout -b year2100/task-25/vault
git checkout year2100/task-25-service-plans
git checkout -b year2100/task-25/wallet
git checkout year2100/task-25-service-plans
git checkout -b year2100/task-25/mint
# ... (10 total)

# Merge sub-branches back to task-25-service-plans
git checkout year2100/task-25-service-plans
git merge --no-ff year2100/task-25/vault
git merge --no-ff year2100/task-25/wallet
# ...

# Finally merge task-25-service-plans to year2100
git checkout year2100
git merge --no-ff year2100/task-25-service-plans
```

---

## 🚨 Anti-Patterns to Avoid

### ❌ DON'T: Commit directly to `year2100`
```bash
# BAD
git checkout year2100
echo "Quick fix" >> YEAR_2100_REVIEW_TODO_FIXED.md
git commit -m "fix typo"  # Bypasses review, no task isolation
```

**Why Bad**: No audit trail, can't revert easily, breaks task isolation

**✅ DO**:
```bash
git checkout -b year2100/fix-typo
echo "Fixed" >> YEAR_2100_REVIEW_TODO_FIXED.md
git commit -m "docs: Fix typo in TODO tracker"
git checkout year2100
git merge --no-ff year2100/fix-typo
```

### ❌ DON'T: Long-lived branches
```bash
# BAD: Working on Task 7 for 2 weeks without merging
git checkout year2100/task-07-event-sourcing
# ... 2 weeks of changes ...
# Now year2100 has diverged significantly → merge hell
```

**✅ DO**: Merge frequently (daily if possible), rebase often:
```bash
# Daily sync
git checkout year2100/task-07-event-sourcing
git fetch origin year2100
git rebase origin/year2100  # Keep branch up to date
```

### ❌ DON'T: Work on multiple tasks in one branch
```bash
# BAD
git checkout -b year2100/task-07-and-10-and-14
# Mixed commits for 3 tasks → can't merge selectively
```

**✅ DO**: One task per branch, finish before starting next

### ❌ DON'T: Delete branches before merging
```bash
# BAD
git branch -D year2100/task-07-event-sourcing  # Oops, lost work!
```

**✅ DO**: Only delete AFTER successful merge to `year2100`

---

## 🔍 Branch Management Commands

### View All Task Branches
```powershell
# List all year2100 branches
git branch | grep year2100

# Or with remote branches
git branch -a | grep year2100
```

### Check Branch Status
```powershell
# See which branches are merged
git checkout year2100
git branch --merged | grep year2100      # Merged (safe to delete)
git branch --no-merged | grep year2100   # Not merged (still in progress)
```

### Cleanup Merged Branches
```powershell
# Delete local branches that are merged
git branch --merged year2100 | grep year2100/ | ForEach-Object { git branch -d $_.Trim() }

# Delete remote branches
git push origin --delete year2100/task-07-event-sourcing
```

### Sync with Remote
```powershell
# Fetch all branches
git fetch origin

# Prune deleted remote branches
git remote prune origin

# Pull latest year2100
git checkout year2100
git pull origin year2100
```

---

## 📋 Pre-Merge Checklist

Before merging any task branch to `year2100`:

- [ ] **All tests pass**: `go test ./...` (or equivalent)
- [ ] **No merge conflicts**: `git merge --no-commit --no-ff year2100/task-XX` → check for conflicts
- [ ] **Documentation updated**: Task marked COMPLETE in `YEAR_2100_REVIEW_TODO_FIXED.md`
- [ ] **Commit messages clean**: Follow conventional commits format
- [ ] **Code reviewed** (if team workflow): PR approved or pair-reviewed
- [ ] **Architecture docs updated**: If task modifies service architecture
- [ ] **Dependencies satisfied**: If task has prerequisites, they're merged first
- [ ] **Acceptance tests written**: If task has acceptance criteria

---

## 🎯 Rollback Strategy

If a task merge breaks `year2100`:

### Option 1: Revert Merge Commit
```powershell
# Find merge commit
git log --oneline --merges | Select-Object -First 5

# Revert merge (creates new commit)
git revert -m 1 <merge-commit-hash>
git push origin year2100
```

### Option 2: Reset to Previous State (Dangerous)
```powershell
# Only if year2100 not pushed yet
git reset --hard HEAD~1  # Go back 1 commit

# If already pushed (requires force-push, coordinate with team!)
git reset --hard <good-commit-hash>
git push origin year2100 --force-with-lease
```

### Option 3: Fix Forward
```powershell
# Create hotfix branch
git checkout -b year2100/hotfix-task-07
# Fix the issue
git commit -m "fix(vault): Correct JetStream configuration error"
git checkout year2100
git merge --no-ff year2100/hotfix-task-07
git push origin year2100
```

---

## 🏁 Final Integration to `v0`

After ALL 25 tasks complete:

```powershell
# Ensure year2100 is clean
git checkout year2100
git status  # Should be clean

# Run full test suite
go test ./...
pwsh test-digger-e2e.ps1  # E2E validation

# Merge to v0
git checkout v0
git pull origin v0
git merge --no-ff year2100 -m "merge: Year 2100 Security Review Complete

Completed all 25 tasks:
- Threat reviews (1-14): All threats analyzed, gaps documented
- Meta-tasks (15-25): Action plans, arch updates, service plans

CRITICAL tasks implemented:
- Task 7: Event sourcing (JetStream dual-write)
- Task 10: NATS multi-region cluster
- Task 14: Verifier oath + slashing
- Task 11: BackingMonitor service

Status: Ready for mainnet security audit

Deliverables:
- YEAR_2100_REVIEW_TODO_FIXED.md (25 tasks)
- TASK_25_SERVICE_PLANS.md (10 service plans)
- Plans/*/YEAR_2100_PLAN.md (10 files)
- Updated architecture docs (7 services)

Duration: ~4-5 weeks
Contributors: [List contributors]"

git push origin v0

# Tag the release
git tag -a security-review-v1.0 -m "Year 2100 Security Review Complete"
git push origin security-review-v1.0

# Archive year2100 branch (optional)
git branch -m year2100 year2100-archived
git push origin year2100-archived
git push origin --delete year2100
```

---

## 📈 Progress Tracking

### Daily Stand-up Template
```markdown
**What was completed yesterday?**
- Merged: year2100/task-07-event-sourcing
- In Progress: year2100/task-10-nats-resilience (80% complete)

**What's planned for today?**
- Complete Task 10 NATS resilience
- Start Task 14 verifier oath

**Any blockers?**
- Task 4 blocked by Task 7 (now unblocked)
- Need clarification on Task 11 burn mechanism (governance constraint)

**Branches status**:
- Merged to year2100: 7 tasks
- In progress: 2 tasks (10, 14)
- Not started: 16 tasks
```

### Weekly Review Checklist
- [ ] All merged branches deleted (keep `year2100` clean)
- [ ] `YEAR_2100_REVIEW_TODO_FIXED.md` updated with completed tasks
- [ ] Remote branches synced (`git fetch --prune`)
- [ ] Conflicts resolved proactively
- [ ] Critical path tasks on schedule
- [ ] Team coordination (if parallel work)

---

## 🛡️ Best Practices Summary

1. **Always branch from `year2100`**: Never from feature branches
2. **One task per branch**: Clear scope, easy rollback
3. **Merge frequently**: Daily if possible, avoid long-lived branches
4. **Rebase before merge**: Keep history clean, resolve conflicts early
5. **Use `--no-ff` merges**: Preserve task branch history in graph
6. **Write descriptive merge commits**: Explain what task accomplished
7. **Delete merged branches**: Keep branch list manageable
8. **Push feature branches**: Backup work, enable collaboration
9. **Follow dependencies**: Check `Task Interdependencies Map` in TODO
10. **Test before merge**: Run full test suite, no broken commits on `year2100`

---

## 🚀 Quick Reference

### Start New Task
```powershell
git checkout year2100
git pull origin year2100
git checkout -b year2100/task-XX-description
```

### Daily Sync
```powershell
git checkout year2100/task-XX-description
git fetch origin year2100
git rebase origin/year2100
```

### Complete Task
```powershell
git checkout year2100
git pull origin year2100
git merge --no-ff year2100/task-XX-description -m "merge: Task XX complete"
git push origin year2100
git branch -d year2100/task-XX-description
```

### Emergency Rollback
```powershell
git checkout year2100
git revert -m 1 <bad-merge-commit>
git push origin year2100
```

---

*"Branch with purpose. Merge with confidence. Ship with pride."* 🌳✨
