# Oracle Removal - Replace with BidNet + DistoDam Settlement

**Date**: November 18, 2025  
**Branch**: feature/vault  
**Status**: ✅ COMPLETE

---

## Rationale

**Problem**: Centralized Oracle service = single point of failure  
**Solution**: TorqVaults execute drips, DistoDam ledger provides cryptographic proof  
**Result**: No new service needed, leverage existing BidNet + DistoDam

---

## Progress

- **Total Tasks**: 8
- **Completed**: 0
- **In Progress**: 0
- **Remaining**: 8

---

## Task List

### 📋 To Do

- [ ] **Task 1**: Update Executive Summary - Remove Oracle, add DistoDam settlement
- [ ] **Task 2**: Update Section 2.2 (Transaction Lifecycle) - TorqVault executes drips
- [ ] **Task 3**: Replace Section 5 (Oracle Settlement) → "5. DistoDam Settlement Layer"
- [ ] **Task 4**: Update Section 7 (BidNet) - Add collateral/slashing mechanism
- [ ] **Task 5**: Update Section 9 (NATS Schemas) - Remove oracle topics, add distodam topics
- [ ] **Task 6**: Update Section 10 (Cryptography) - DistoDam signs settlements with SPHINCS+
- [ ] **Task 7**: Update Phase Roadmap - Remove Oracle, add DistoDam integration
- [ ] **Task 8**: Update Conclusion - Emphasize decentralization via DistoDam

---

## Detailed Changes

### Task 1: Executive Summary
**Change**: Remove "Oracle settlement - 10-second ticks"  
**Add**: "DistoDam settlement - Covenant-enforced drips with cryptographic proof"

### Task 2: Transaction Lifecycle (Section 2.2)
**Old Flow**:
```
Winner fronts liquidity → Oracle ticks → drip RT → final settlement
```

**New Flow**:
```
Winner fronts liquidity → Winner publishes drip covenant to DistoDam
→ DistoDam enforces drips (merkle commitments every 10s)
→ If winner fails → slash collateral, activate fallback TorqVault
```

### Task 3: Section 5 Complete Rewrite
**Old**: "Oracle Settlement" (leader election, tick cycles, etc.)  
**New**: "DistoDam Settlement Layer"
- Covenant structure (drip schedule)
- Merkle commitment verification
- Slashing mechanism
- Fallback activation

### Task 4: BidNet Enhancements (Section 7)
**Add**:
- Collateral requirement (10% of flow amount)
- Slashing conditions (missed drip, wrong amount, late)
- Reputation system (successful settlements increase trust score)

### Task 5: NATS Message Schemas (Section 9)
**Remove**:
- `wallet.ledger.update` (was Oracle → Wallets)
- Oracle-related topics

**Add**:
- `distodam.covenant.submit` (TorqVault → DistoDam)
- `distodam.drip.execute` (DistoDam → All)
- `distodam.slash.alert` (DistoDam → BidNet if failure)

### Task 6: Cryptography (Section 10)
**Update**: DistoDam signs all drip executions with SPHINCS+ (archival proofs)  
**Add**: Covenant verification (TorqVault signature + DistoDam counter-signature)

### Task 7: Phase Roadmap (Section 12)
**Phase 1**: Remove "mock Oracle" → Add "DistoDam covenant integration"  
**Phase 2**: Remove "Oracle with 10-second ticks" → Add "DistoDam automated drips"  
**Phase 4**: Remove "Multi-Oracle consensus" → Add "DistoDam slashing + reputation"

### Task 8: Conclusion
**Emphasize**: 
- Decentralized settlement (no Oracle service)
- DistoDam as trust anchor
- TorqVaults compete on execution reliability

---

## Architecture Comparison

### BEFORE (Oracle-Based)
```
User → BidNet → TorqVault (fronts RT)
       ↓
    Oracle Service (centralized!)
       ↓ ticks every 10s
    Sender → TorqVault (drip recovery)
```

**Problems**:
- Oracle = single point of failure
- Oracle needs leader election, BFT, redundancy
- New service to build + maintain

### AFTER (DistoDam-Based)
```
User → BidNet → TorqVault (fronts RT + posts collateral)
       ↓
TorqVault publishes covenant to DistoDam
       ↓
DistoDam enforces drips (merkle commitments)
       ↓
Sender → DistoDam → TorqVault (drip recovery)
       ↓ (if TorqVault fails)
DistoDam slashes collateral → Fallback TorqVault activated
```

**Benefits**:
- ✅ No Oracle service needed
- ✅ DistoDam already exists (ledger of record)
- ✅ Economic incentives (collateral/slashing)
- ✅ Decentralized (any TorqVault can bid)

---

## Next Action

Starting Task 1: Update Executive Summary...
