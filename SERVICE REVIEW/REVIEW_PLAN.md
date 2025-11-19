# Service Review Plan

**Date**: November 19, 2025  
**Branch**: year2100  
**Purpose**: Identify what blocks Vault, Testnet, and Mainnet - service by service

---

## 🎯 The Simple Process

For each service (in order), create 2 files:

### 1. `Plans/{Service}/{SERVICE}_SERVICE_REVIEW.md`
- **Current State**: What exists now?
- **Gaps Found**: What's missing or broken?
- **Prioritization**: PRE-VAULT | PRE-TESTNET | PRE-MAINNET

### 2. `Plans/{Service}/{SERVICE}_SERVICE_PLAN.md`
- **Actionable TODOs**: Specific tasks to fix gaps
- **Organized by priority tier**
- **Estimated time for each task**

---

## 📋 Service Review Order

### Phase 1: Infrastructure & Data Pipeline (Services 1-6)

**1. Transactions (NATS)**
- [ ] Review complete
- [ ] Plan created
- **Why first**: Everything depends on message bus

**2. Simulation**  
- [ ] Review complete
- [ ] Plan created
- **Why second**: Validates all other services

**3. Digger**
- [ ] Review complete
- [ ] Plan created
- **Why third**: Source of all work (produces ore)

**4. Refinery**
- [ ] Review complete
- [ ] Plan created
- **Why fourth**: Processes ore into units/ingots

**5. Mint**
- [ ] Review complete
- [ ] Plan created
- **Why fifth**: Verifies ingots, creates RT

**6. DistoDam**
- [ ] Review complete
- [ ] Plan created
- **Why sixth**: Settlement layer (transactions need this)

---

### Phase 2: Markets & Contracts (Services 7-8)

**7. BidNet**
- [ ] Review complete
- [ ] Plan created
- **Why seventh**: Market mechanisms (depends on DistoDam)

**8. Trust**
- [ ] Review complete
- [ ] Plan created
- **Why eighth**: Contract orchestration (uses BidNet)

---

### Phase 3: User Layer (Services 9-11)

**9. Wallet**
- [ ] Review complete
- [ ] Plan created
- **Why ninth**: User transactions (needs all above)

**10. Vault**
- [ ] Review complete
- [ ] Plan created
- **Why tenth**: Long-term storage (needs everything)

**11. Printer**
- [ ] Review complete
- [ ] Plan created
- **Why eleventh**: Physical RT bills (people need printed RT to open wallets, system can't propagate without it)

---

## 🚨 Priority Tiers Explained

### **PRE-VAULT** (Blocks Vault development)
- Must complete before you can build/test Vault
- Example: Event sourcing in DistoDam (Vault needs to store transactions)

### **PRE-TESTNET** (Blocks local/regional deployment)
- Must complete before real users can test
- Example: Basic monitoring, error handling, integration tests

### **PRE-MAINNET** (Blocks global public launch)
- Must complete before production launch
- Example: Multi-region NATS, security audits, multiple verifiers

---

## 📝 Review Template

Each `{SERVICE}_SERVICE_REVIEW.md` should answer:

1. **What does this service do?** (1-2 sentences)
2. **What's the current state?** (Built? Planned? Missing?)
3. **What blocks Vault?** (PRE-VAULT gaps)
4. **What blocks Testnet?** (PRE-TESTNET gaps)
5. **What blocks Mainnet?** (PRE-MAINNET gaps)

Keep it SHORT - each review should be <500 words.

---

## ✅ Plan Template

Each `{SERVICE}_SERVICE_PLAN.md` should list:

### PRE-VAULT Tasks (if any)
- [ ] Task 1: Description (Estimated: X hours)
- [ ] Task 2: Description (Estimated: X hours)

### PRE-TESTNET Tasks
- [ ] Task 1: Description (Estimated: X hours)
- [ ] Task 2: Description (Estimated: X hours)

### PRE-MAINNET Tasks
- [ ] Task 1: Description (Estimated: X hours)
- [ ] Task 2: Description (Estimated: X hours)

**Total Estimate**: X days

---

## 🎯 Execution Strategy

### Step 1: Review Phase (1-2 weeks)
- Go through services 1-11 in order
- Create review + plan for each
- Don't implement yet, just document

### Step 2: Consolidate (1 day)
- Merge all PRE-VAULT tasks into one list
- Merge all PRE-TESTNET tasks into one list
- Merge all PRE-MAINNET tasks into one list

### Step 3: Execute (Iterative)
- Complete all PRE-VAULT tasks → Build Vault
- Complete all PRE-TESTNET tasks → Launch testnet
- Complete all PRE-MAINNET tasks → Launch mainnet

---

## 📊 Progress Tracking

**Services Reviewed**: 0 / 11  
**Services Planned**: 0 / 11

**PRE-VAULT Tasks Identified**: 0  
**PRE-TESTNET Tasks Identified**: 0  
**PRE-MAINNET Tasks Identified**: 0

**Current Focus**: Starting service reviews

---

## 🔑 Key Questions for Each Service

1. **Does it exist?** (Yes/No/Partial)
2. **Does Vault depend on it?** (Yes/No)
3. **What's broken or missing?** (List)
4. **When must it be fixed?** (PRE-VAULT | PRE-TESTNET | PRE-MAINNET)

---

## 📌 Success Criteria

**Review Phase Complete When**:
- ✅ All 11 services reviewed
- ✅ All 11 plans created
- ✅ PRE-VAULT tasks clearly identified
- ✅ Can start building Vault

**Execution Phase Complete When**:
- ✅ All PRE-VAULT tasks done → Vault works
- ✅ All PRE-TESTNET tasks done → Local users testing
- ✅ All PRE-MAINNET tasks done → Global launch ready

---

*"Keep it simple. Review → Plan → Execute."* 🎯
