# Mint Persistence Integration Plan

**Status**: Ready to implement (feature/mint-persistence branch)  
**Date**: November 20, 2025  
**Scope**: Wire persistence layer into IngotHashQueue, Phase3Assembler, and HTTP handlers

---

## Current Gap Analysis

### What Exists ✅
- `src/mint/internal/persist/` package (3 files, ~850 LOC)
  - `persistence.go` - Orchestrator API
  - `ingot_store.go` - Batch recovery
  - `proof_store.go` - Proof audit trail
- 47 unit tests with 78.3% coverage (all passing)
- Comprehensive test suite for crash scenarios

### What's Missing ❌
- **Zero integration** with IngotHashQueue or Phase3Assembler
- **ProofCache is volatile** (lost on restart)
- **RecoverInFlightIngots() never called** (no crash recovery)
- **SaveIngotBatch/SavePhase3Proof never called** (no persistence)
- **RemoveIngotBatch never called** (orphaned recovery files accumulate)
- **Graceful shutdown** not implemented (ingots in-flight on crash)
- **Prometheus metrics** not exposed (can't monitor persistence health)
- **Proof API lookups** not wired (verification handler can't find proofs)
- **CSV export endpoint** not wired (audit export unavailable)
- **Concurrent access safety** not tested in real service

---

## Integration Tasks (Priority Order)

### Priority 1: Core Crash Recovery & Persistence (Critical Path)

#### 1.1: Wire SaveIngotBatch in IngotHashQueue.Process()
**File**: `src/mint/internal/mint/ingot_hash_queue.go`  
**Current State**: Accumulates ingots, publishes to merkle builder  
**Change Required**: Persist batch BEFORE sending to merkle builder

**What**: Add persistence call before merkle transmission
```go
// In IngotHashQueue.Process() before publishing to Level2MerkleBuilder
if batch.Size() >= pm.TargetSize || batch.IsExpired() {
    // NEW: Persist in-flight batch (crash recovery)
    err := pm.SaveIngotBatch(batch.ID, batch.Entries)
    if err != nil {
        logger.Error("failed to persist batch", "id", batch.ID)
        // Q: Fail-safe? Continue or reject?
    }
    
    // EXISTING: Send to merkle builder
    pm.SendToMerkleBuilder(batch)
}
```

**Decision Needed**: Fail-safe or fail-fast on persistence error?
- **Fail-safe**: Log error but continue (data loss risk, but service stays up)
- **Fail-fast**: Return error, don't send to merkle (safe, but blocks on disk issue)

**Estimate**: 1 hour (locate code, add call, test locally)

---

#### 1.2: Wire RemoveIngotBatch after Phase3 publication
**File**: `src/mint/internal/mint/phase3_robotorq_unit_assembler.go`  
**Current State**: Publishes Phase3RoboTorqUnit to DistoDam, no cleanup  
**Change Required**: Delete batch from recovery store after successful NATS publish

**What**: After publishing to NATS, mark batch as recoverable
```go
// In Phase3Assembler.assemble() after NATS publish succeeds
err := natsClient.Publish("distodam.units", unitJSON)
if err != nil {
    return fmt.Errorf("publish failed: %w", err)
}

// NEW: Batch is now safely published, can delete recovery file
batchID := unit.SourceBatchID  // Where did this come from?
err = pm.RemoveIngotBatch(batchID)
if err != nil {
    logger.Warn("failed to clean up batch", "id", batchID, "error", err)
    // Don't fail: NATS publish succeeded, cleanup is optional
}
```

**Question**: Where does Phase3RoboTorqUnit store the source batchID?
- Currently: No reference to original IngotHashEntry batch
- **Need to add**: `SourceBatchID string` field to Phase3RoboTorqUnit? Or track separately?

**Estimate**: 2 hours (may require schema change to track batch origin)

---

#### 1.3: Wire SavePhase3Proof in Phase3Assembler
**File**: `src/mint/internal/mint/phase3_robotorq_unit_assembler.go`  
**Current State**: Creates unit, publishes to NATS, done  
**Change Required**: Persist proof to permanent audit trail

**What**: After successful NATS publish, save to ProofStore
```go
// In Phase3Assembler.assemble() after NATS publish succeeds
err := pm.SavePhase3Proof(unit, merkleResult)
if err != nil {
    logger.Error("failed to save proof", "unit_id", unit.UnitID, "error", err)
    // Q: Should we delete from NATS? Rollback? Or accept data loss?
}
```

**Question**: Error handling strategy?
- If persist fails after NATS publish: Phase3 is already distributed
- **Recovery**: Accept the loss? Retry async? Or fail-fast earlier?

**Estimate**: 1.5 hours (locate code, add call, handle errors)

---

#### 1.4: Call RecoverInFlightIngots at Mint startup
**File**: `src/mint/cmd/mint/main.go` or service initialization  
**Current State**: No startup recovery logic  
**Change Required**: Load orphaned batches from last crash

**What**: On service start, before entering normal processing
```go
// In main() or service.Start()
pm, err := persist.NewPersistenceManager(persistDir, logger)
if err != nil {
    logger.Fatal("failed to create persistence manager", "error", err)
}

// NEW: Recover from crash
batches, err := pm.RecoverInFlightIngots(ctx)
if err != nil {
    logger.Error("recovery failed", "error", err)
    // Q: Abort startup? Or continue without recovery?
}

if len(batches) > 0 {
    logger.Info("recovered batches from crash", "count", len(batches))
    // Re-queue batches into IngotHashQueue
    for batchID, entries := range batches {
        queue.RequeueBatch(batchID, entries)
    }
}

// Continue normal startup
service.Start(ctx)
```

**Question**: What if recovery fails partway?
- **All or nothing**: Accept recovery failure, lose batches
- **Partial recovery**: Re-queue what's readable, log failures

**Estimate**: 1 hour (add startup hook, test recovery scenario)

---

### Priority 2: Graceful Shutdown (Prevent Data Loss)

#### 2.1: Implement graceful shutdown handler
**File**: `src/mint/internal/mint/service.go` or signal handler  
**Current State**: SIGTERM probably kills service abruptly  
**Change Required**: Flush incomplete batches before exit

**What**: On shutdown signal, drain in-flight ingots to ProofStore
```go
// In signal handler or service.Shutdown()
sigChan := make(chan os.Signal, 1)
signal.Notify(sigChan, syscall.SIGTERM, syscall.SIGINT)

go func() {
    <-sigChan
    logger.Info("shutdown signal received")
    
    // NEW: Flush pending batches
    incomplete := queue.Drain()  // Get all unsent batches
    for batchID, entries := range incomplete {
        // Treat incomplete batch as recoverable
        pm.SaveIngotBatch(batchID, entries)
        logger.Info("flushed incomplete batch", "id", batchID)
    }
    
    // Gracefully close connections
    pm.Shutdown(ctx)
    service.Close()
    os.Exit(0)
}()
```

**Estimate**: 1.5 hours (add signal handling, test graceful shutdown)

---

### Priority 3: Observability & Monitoring

#### 3.1: Export persistence metrics to Prometheus
**File**: `src/mint/internal/metrics/persistence_metrics.go` (new)  
**Current State**: PersistenceMetrics tracked internally, not exposed  
**Change Required**: Expose to `/metrics` endpoint

**What**: Create Prometheus metrics
```go
// New file: metrics/persistence_metrics.go
type PersistenceMetrics struct {
    IngotsWritten prometheus.Counter
    IngotsRecovered prometheus.Counter
    ProofsWritten prometheus.Counter
    ProofsRead prometheus.Counter
    WriteErrors prometheus.Counter
    ReadErrors prometheus.Counter
    RecoveryOperations prometheus.Counter
}

// In health_handler.go, add to /metrics response
handler.Register(pm.metrics.IngotsWritten)
handler.Register(pm.metrics.ProofsWritten)
// ... etc
```

**Estimate**: 1.5 hours (define metrics, wire to handler, test export)

---

#### 3.2: Add persistence status to health endpoint
**File**: `src/mint/internal/handlers/health_handler.go`  
**Current State**: Reports cache size, signature count  
**Change Required**: Add persistence status

**What**: Include persistence stats in health JSON
```json
{
  "status": "healthy",
  "cache_size": 1500,
  "signatures_verified": 45000,
  
  "persistence": {
    "ingots_written": 8500,
    "proofs_written": 2375,
    "batches_in_recovery": 2,
    "last_recovery": "2025-11-20T10:15:32Z",
    "storage_size_mb": 125.4
  }
}
```

**Estimate**: 1 hour (modify handler, add fields, test)

---

### Priority 4: Proof Verification API Integration

#### 4.1: Wire ProofStore to verification handlers
**File**: `src/mint/internal/handlers/verification_handler.go`  
**Current State**: Looks up proofs in in-memory ProofCache only  
**Change Required**: Fallback to persistent ProofStore

**What**: Check disk if not in cache
```go
// In verification_handler.go
func (h *VerificationHandler) GetProofByUnitID(unitID string) (*models.Phase3RoboTorqUnit, error) {
    // Check in-memory cache first
    if proof := h.proofCache.Get(unitID); proof != nil {
        return proof, nil
    }
    
    // NEW: Check persistent store (older proofs)
    proof, _, err := h.pm.LookupPhase3Proof(unitID)
    if err == nil {
        return proof, nil
    }
    
    return nil, fmt.Errorf("proof not found: %s", unitID)
}
```

**Estimate**: 1.5 hours (add persistence lookup, test with old proofs)

---

#### 4.2: Implement CSV export endpoint
**File**: `src/mint/internal/handlers/audit_handler.go` (new)  
**Current State**: No audit export endpoint  
**Change Required**: POST /audit/export-proofs → CSV file

**What**: New endpoint
```go
// GET /audit/export-proofs?format=csv
func (h *AuditHandler) ExportProofs(w http.ResponseWriter, r *http.Request) {
    tempFile := filepath.Join(os.TempDir(), "proofs-export-"+time.Now().Format("20060102150405")+".csv")
    
    err := h.pm.ExportProofCSV(r.Context(), tempFile)
    if err != nil {
        http.Error(w, "export failed", http.StatusInternalServerError)
        return
    }
    
    w.Header().Set("Content-Type", "text/csv")
    w.Header().Set("Content-Disposition", "attachment; filename=proofs.csv")
    http.ServeFile(w, r, tempFile)
    
    os.Remove(tempFile)  // Cleanup after send
}
```

**Estimate**: 1 hour (create handler, wire to router, test)

---

### Priority 5: Data Integrity & Validation

#### 5.1: Validate recovered batches on startup
**File**: `src/mint/internal/persist/validation.go` (new)  
**Current State**: RecoverInFlightIngots just reads files, no validation  
**Change Required**: Check for corruption, missing fields

**What**: Validate each recovered batch
```go
func ValidateRecoveredBatch(batch []*models.IngotHashEntry) error {
    if len(batch) == 0 {
        return fmt.Errorf("empty batch")
    }
    
    for i, entry := range batch {
        if entry.BranchHash == "" {
            return fmt.Errorf("entry %d missing hash", i)
        }
        if entry.RoboStakeTotal <= 0 {
            return fmt.Errorf("entry %d invalid stake", i)
        }
    }
    
    return nil
}
```

**Estimate**: 2 hours (add validation logic, handle corrupted data, test scenarios)

---

#### 5.2: Detect inconsistencies between IngotHashQueue and ProofStore
**File**: `src/mint/internal/persist/consistency_check.go` (new)  
**Current State**: No cross-table validation  
**Change Required**: Health check for orphaned data

**What**: Periodic scan
```go
func (pm *PersistenceManager) CheckConsistency(ctx context.Context) error {
    // Get all in-flight batches
    inFlight, _ := pm.ingotStore.ReadAllBatches()
    
    // Get all proofs
    proofs, _ := pm.proofStore.ListProofs()
    
    // Warn if too many in-flight (suggests backlog or crash loop)
    if len(inFlight) > 100 {
        pm.logger.Warn("large in-flight batch count", "count", len(inFlight))
    }
    
    return nil
}
```

**Estimate**: 1.5 hours (implement check, integrate into health handler)

---

## Implementation Sequence

**Phase A (Days 1-2)**: Critical path
```
1.1 SaveIngotBatch integration
1.2 RemoveIngotBatch integration (depends on schema decision)
1.3 SavePhase3Proof integration
1.4 RecoverInFlightIngots at startup
→ Result: Crash recovery works, no data loss
```

**Phase B (Day 3)**: Graceful shutdown
```
2.1 Graceful shutdown handler
→ Result: Clean shutdown drains in-flight ingots
```

**Phase C (Days 4-5)**: Observability
```
3.1 Prometheus metrics
3.2 Health endpoint status
4.1 Proof verification API integration
4.2 CSV export endpoint
→ Result: Can monitor persistence health, access proofs via API
```

**Phase D (Days 6+)**: Data integrity (optional, do later if time)
```
5.1 Batch validation
5.2 Consistency checks
→ Result: Corruption detection, early warning
```

---

## Decision Points Requiring User Input

### Decision #1: Fail-Safe vs Fail-Fast on Persistence Error
**Location**: 1.1 (SaveIngotBatch)  
**Options**:
- **A) Fail-Safe**: Log error, continue (data loss risk, service continues)
- **B) Fail-Fast**: Return error, don't send to merkle (safe, may block service)

**USER DECISION**: **A (Fail-Safe)**
- Log loudly to Prometheus alert
- Operator can investigate
- Service stays running
- Mint should not crash over disk issues

---

### Decision #2: Error Handling on SavePhase3Proof Failure
**Location**: 1.3 (SavePhase3Proof)  
**Problem**: If SavePhase3Proof fails, is it validation failure or persistence failure?
**Options**:
- **A) Accept Loss**: Phase3 already published to NATS, can't rollback
- **B) Retry Async**: Queue for background retry
- **C) Flag for Dispute**: Store in special place, mark for manual review
- **D) Alert Only**: Log error, page operator

**USER DECISION**: **C (Flag for Dispute) + TODO for Later**

**Implementation**:
```
IF SavePhase3Proof fails (validation error):
  ├─ Store unit in DisputeStore (new table)
  ├─ Mark: reason="proof_validation_failed"
  ├─ Include error details (e.g., "merkle tree corrupt")
  ├─ Alert operator (slack/pagerduty)
  └─ Log for RCA

IF SavePhase3Proof fails (persistence error like disk full):
  ├─ Treat as fail-safe (same as Decision #1)
  ├─ Log error
  └─ Continue (unit already published, best effort)
```

**Future Work (TODO)**:
- [ ] Implement DisputeStore schema
- [ ] Implement dispute resolution API (admin endpoint to review/approve/reject)
- [ ] Add metrics: units_in_dispute, disputes_resolved, etc.
- [ ] Document RCA process for proof validation failures

---

### Decision #3: Recovery Failure Strategy
**Location**: 1.4 (RecoverInFlightIngots)  
**Options**:
- **A) All or Nothing**: Fail startup if recovery fails (safe but blocks recovery)
- **B) Partial Recovery**: Re-queue what's readable, log failures, continue (maximize data)
- **C) Skip Recovery**: If error, just start fresh (data loss)

**USER DECISION**: **B (Partial Recovery)**

**Tradeoff Analysis**:
```
Option A (All or Nothing):
  ✅ Guarantees: Either full recovery or none (no partial state)
  ❌ Startup blocked if any file corrupted
  ❌ Single bad file = lose everything
  Example: If 1 of 100 batch files corrupted, lose all 100 batches

Option B (Partial Recovery):
  ✅ Maximize recovered batches (skip corrupted, take rest)
  ✅ Service starts faster
  ✅ Partial state visible to operator
  ❌ Need strong logging to identify what was lost
  ❌ Operator confusion: "why only 99 batches?"
  Example: If 1 of 100 batch files corrupted, recover 99, lose 1

Option C (Skip Recovery):
  ✅ Fastest startup
  ✅ No ambiguity
  ❌ Lose ALL in-flight work from crash
  ❌ Unacceptable for currency system
```

**My Recommendation**: **B (Partial Recovery)** - For RoboTorq use case
- Currency system: Maximize recovered value
- File corruption is rare (disk OK, JSON parse OK)
- Operator can identify loss via metrics
- Log clearly: "recovered N batches, skipped M corrupted"

**BUT**: If you want absolute safety, go with **A (All or Nothing)**
- Safer for critical systems
- Forces operator to fix corruption explicitly
- More paranoid, less operational overhead

---

### Decision #4: Phase3RoboTorqUnit Batch Tracking
**Location**: 1.2 (RemoveIngotBatch)  
**Problem**: Phase3RoboTorqUnit doesn't know its source batchID  
**Options**:
- **A) Add Field**: `SourceBatchID string` to Phase3RoboTorqUnit
- **B) Track Separately**: Map unitID → batchID in memory
- **C) Don't Cleanup**: Leave recovery files (they age out anyway)

**USER QUESTION**: "What would this accomplish?"

**Answer: Batch Cleanup & Audit Trail**

**Current Situation**:
```
Flow without SourceBatchID:
1. IngotHashQueue receives 300 units in batch "batch-001"
2. Creates batch-001.json on disk (recovery file)
3. Sends to Level2MerkleBuilder
4. Level2MerkleBuilder creates merkle tree
5. Phase3Assembler creates Phase3RoboTorqUnit
6. Publishes to NATS
7. PROBLEM: Who deletes batch-001.json?
   - Phase3Assembler doesn't know which batch to delete
   - Recovery files accumulate on disk forever
   - Disk fills up eventually
```

**With SourceBatchID**:
```
Flow WITH SourceBatchID:
1. IngotHashQueue receives 300 units in batch "batch-001"
2. Creates batch-001.json on disk
3. Sends to Level2MerkleBuilder (passes batchID along)
4. Level2MerkleBuilder creates merkle tree (stores batchID)
5. Phase3Assembler creates Phase3RoboTorqUnit
   └─ Sets: unit.SourceBatchID = "batch-001"
6. Publishes to NATS
7. SOLUTION: RemoveIngotBatch("batch-001") called
   └─ Deletes batch-001.json from disk
```

**Benefits**:
- ✅ Disk cleanup: Recovery files deleted after successful publish
- ✅ Audit trail: Can trace which batch created which Phase3 unit
- ✅ Debugging: Query "which batch produced unit-XYZ?"
- ✅ Storage management: Know total batches vs total units

**Costs**:
- ❌ One extra string field per Phase3RoboTorqUnit (~50 bytes)
- ❌ Must thread batchID through Level2MerkleBuilder
- ❌ Must update models.Phase3RoboTorqUnit (schema change)

**Alternative C (Don't Cleanup)**:
```
Pros:
  - No code changes needed
  - Recovery files stay as audit trail
Cons:
  - Disk fills up indefinitely
  - Old recovery files never cleaned up
  - Operator must manually delete or add retention policy
```

**USER DECISION NEEDED**: 
- **Option A**: Add SourceBatchID (recommended for production) ✅ **USER CHOICE: YES, MUST ADD**
- **Option C**: Skip cleanup, rely on manual/automated retention (simpler now, ops problem later)
- **Hybrid**: Add SourceBatchID but implement cleanup later?

**Refactor Implications Downchain**:

**Phase3RoboTorqUnit Schema Change** (models.go):
```go
type Phase3RoboTorqUnit struct {
    UnitID         string                    `json:"unit_id"`
    MerkleRoot     string                    `json:"merkle_root"`
    RoboStakeTotal float64                   `json:"robo_stake_total"`
    TreeHeight     int                       `json:"tree_height"`
    MintedAt       time.Time                 `json:"minted_at"`
    
    // NEW FIELD
    SourceBatchID  string                    `json:"source_batch_id"`  // Link to recovery batch
    
    ContractIDs    []string                  `json:"contract_ids"`
    Signature      []byte                    `json:"signature"`
}
```

**Downstream Impact Map**:

```
┌─────────────────────────────────────────────────────────┐
│ Phase3RoboTorqUnit.SourceBatchID added                  │
└──────────────────────┬──────────────────────────────────┘
                       │
        ┌──────────────┼──────────────┬──────────────┐
        │              │              │              │
        ▼              ▼              ▼              ▼
   [Mint]        [DistoDam]    [Printer]     [Verification API]
    │                │             │              │
    │                │             │              │
    ├─ Phase3Assembler   │             │              │
    │  - Must populate   │             │              │
    │    SourceBatchID   │             │              │
    │                    │             │              │
    └────▶ NATS publish  │             │              │
         (unit has batch │             │              │
          ID)            │             │              │
                         │             │              │
                         ├─ Receives   │              │
                         │  Phase3     │              │
                         │  (with      │              │
                         │  batch ID)  │              │
                         │             │              │
                         ├─ Stores in  │              │
                         │  ledger     │              │
                         │  (batch ID  │              │
                         │  indexed)   │              │
                         │             │              │
                         │             ├─ Query for  │
                         │             │  forensics  │
                         │             │  (which     │
                         │             │  batch made │
                         │             │  this unit) │
                         │             │             │
                         │             │             ├─ Lookup by
                         │             │             │  batch ID
                         │             │             │  (audit)
                         │             │             │
                         └─────────────┴─────────────┘
```

**1. Phase3Assembler (Mint) - MUST CHANGE**
```go
// In phase3_robotorq_unit_assembler.go
func (pa *Phase3Assembler) assemble(
    merkleResult *Level2MerkleResult,
    batchID string,  // NEW parameter passed down
    ingotEntries []*IngotHashEntry,
) (*Phase3RoboTorqUnit, error) {
    
    unit := &Phase3RoboTorqUnit{
        UnitID: generateUnitID(),
        MerkleRoot: merkleResult.MerkleRoot,
        // ... other fields ...
        
        // NEW: Track source batch
        SourceBatchID: batchID,
    }
    
    return unit, nil
}
```

**Impact**: 
- ✅ One parameter threaded through
- ✅ One field assignment
- ❌ BREAKING CHANGE: Existing code calling assemble() needs batchID

---

**2. Level2MerkleBuilder - MUST CHANGE**
```go
// Current: Takes IngotHashEntry array
func (b *Level2MerkleBuilder) Build(entries []*IngotHashEntry) (*Level2MerkleResult, error)

// NEW: Must also receive batchID to pass downstream
func (b *Level2MerkleBuilder) Build(
    batchID string,           // NEW parameter
    entries []*IngotHashEntry,
) (*Level2MerkleResult, error)
```

**Impact**:
- ✅ Thread parameter through
- ❌ BREAKING CHANGE: All callers must pass batchID

---

**3. DistoDam (Downstream ledger) - SOFT CHANGE**
```go
// DistoDam receives Phase3RoboTorqUnit via NATS
// NEW field (SourceBatchID) is in the JSON

type RoboTorqLedgerEntry struct {
    UnitID        string
    MerkleRoot    string
    SourceBatchID string   // NEW: Now available for audit
    // ... other fields ...
}

// Can index on SourceBatchID for lookups:
// "Which units came from batch-001?"
```

**Impact**:
- ✅ No code changes (just new field in JSON)
- ✅ Optional: Add index for queries
- ✅ Can now correlate: batch → units → proofs → ledger entries

---

**4. Printer (DistoDam consumer) - SOFT CHANGE**
```go
// Printer persists Phase3RoboTorqUnit to long-term ledger
// NEW field automatically persisted

// Can now answer forensic queries:
// "Find all proofs from batch-001"
// SELECT * FROM proofs WHERE source_batch_id = 'batch-001'
```

**Impact**:
- ✅ No code changes (field auto-propagates)
- ✅ Better audit trail (batch lineage)

---

**5. Verification API (GET /verify/unit/:id) - OPTIONAL ENHANCEMENT**
```go
// Current response:
{
  "unit_id": "unit-12345",
  "merkle_root": "abc123",
  "robo_stake_total": 5.0,
  "tree_height": 12
}

// NEW response (can add):
{
  "unit_id": "unit-12345",
  "merkle_root": "abc123",
  "robo_stake_total": 5.0,
  "tree_height": 12,
  "source_batch_id": "batch-001"  // NEW: Forensic info
}
```

**Impact**:
- ✅ No code changes (field already in unit)
- ✅ Optional: Expose in API for debugging
- ✅ Operator can correlate unit to source batch

---

**6. Dispute Resolution (FUTURE) - NEW FEATURE**
```go
// If unit goes to dispute:
// Can query: "Which batch created this disputed unit?"
// Answer: unit.SourceBatchID = "batch-005"
// Then: "What were all the ingots in batch-005?"
// Answer: Read recovery file batch-005.json

DisputeStore {
  UnitID: "unit-12345",
  Reason: "merkle_tree_corruption",
  SourceBatchID: "batch-005",  // NEW: Can trace root cause
  IngotCount: 300,
  RoboStakeTotal: 5.0
}
```

**Impact**:
- ✅ Enables better RCA (root cause analysis)
- ✅ Track batch → unit → dispute lineage

---

**Summary of Refactor Cascade**:

| Component | Change Type | Effort | Impact |
|-----------|-------------|--------|--------|
| Phase3RoboTorqUnit model | Schema change | Trivial | High (breaking change) |
| Phase3Assembler | Parameter + assign | 1 hour | Must update |
| Level2MerkleBuilder | Parameter pass-through | 1 hour | Must update |
| IngotHashQueue | Parameter pass-through | 1 hour | Must update (call site) |
| DistoDam | None (auto-propagate) | 0 hours | Optional: add index |
| Printer | None (auto-propagate) | 0 hours | Already persists |
| Verification API | None (optional enhance) | 0 hours | Can expose field |
| Dispute store | None (new feature) | Future | Enables better RCA |

**Total refactor effort**: ~3 hours to thread parameter through pipeline

**Breaking changes**: Schema + calling patterns (requires coordinated update)

**BUT**: Once threaded through, enables:
- ✅ Batch cleanup (RemoveIngotBatch works)
- ✅ Forensic queries (which batch? which units?)
- ✅ Better dispute tracking (trace root cause)
- ✅ Storage lifecycle management (know age of each batch)

---

## Testing Strategy

### Unit Tests (Existing)
- ✅ Already have 47 tests covering persistence layer
- Need to verify they still pass after integration

### Integration Tests (New)
```go
// Test crash recovery flow
func TestCrashRecovery_End2End(t *testing.T) {
    // 1. Start Mint, ingest 300 units
    // 2. Simulate crash (kill service mid-batch)
    // 3. Restart Mint
    // 4. Verify batches recovered
    // 5. Verify all 300 units eventually minted
}

// Test graceful shutdown
func TestGracefulShutdown_DrainsIngots(t *testing.T) {
    // 1. Start Mint with partial batch in-flight
    // 2. Send SIGTERM
    // 3. Verify batch saved to recovery
    // 4. Restart Mint
    // 5. Verify batch recovered
}

// Test concurrent access
func TestConcurrentPersistence_ThreadSafe(t *testing.T) {
    // 1. 10 goroutines writing batches
    // 2. 5 goroutines reading proofs
    // 3. Verify no corruption or races
}
```

### E2E Tests (Existing Test Scripts)
- Modify `test-distodam-e2e.ps1` to include crash recovery verification
- Add step: kill Mint, verify recovery, confirm proofs published

---

## Effort Estimate

| Phase | Tasks | Estimate | Duration |
|-------|-------|----------|----------|
| A (Critical) | 1.1-1.4 | 5.5 hours | 1 day |
| B (Shutdown) | 2.1 | 1.5 hours | 2-3 hours |
| C (Observability) | 3.1-4.2 | 5 hours | 1 day |
| D (Validation) | 5.1-5.2 | 3.5 hours | Half day |
| **Total** | | **15.5 hours** | **2-3 days** |

**Per-task breakdown**:
- Smallest task: 3.2 (1 hour)
- Largest task: 1.2 (2 hours, depends on schema change)
- Average: 1.5-2 hours per task

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Schema change (add SourceBatchID) | Medium | Backwards-compatible, can default to empty string |
| Concurrent access issues | Medium | Existing mutex in PersistenceManager, add tests |
| Disk full during persistence | Low | Fail-safe, let service continue, alert operator |
| Corrupted recovery files | Low | Validation step (Priority 5.1) detects, skips bad data |
| Key rotation with SQLite (Phase 4) | N/A | Documented in PERSISTENCE_ENCRYPTION_DESIGN.md |

---

## Blockers & Dependencies

- **None identified** - Can start immediately on feature/mint-persistence branch
- **Optional**: Decision #4 (batch tracking) determines 1.2 complexity
- **Future**: Phase 4 (encrypted SQLite) is documented but not required for Phase 3

---

## Success Criteria

✅ **Phase A Complete**:
- [ ] RecoverInFlightIngots called on startup
- [ ] SaveIngotBatch called before merkle builder
- [ ] RemoveIngotBatch called after NATS publish
- [ ] SavePhase3Proof called after publish
- [ ] E2E test passes: crash → recover → mint succeeds

✅ **Phase B Complete**:
- [ ] SIGTERM triggers graceful shutdown
- [ ] In-flight ingots flushed to recovery
- [ ] Restart recovers flushed ingots

✅ **Phase C Complete**:
- [ ] /metrics endpoint includes persistence counters
- [ ] /health endpoint shows batch count
- [ ] GET /verify/unit/:id falls back to disk
- [ ] GET /audit/export-proofs works

✅ **Phase D Complete**:
- [ ] Recovered batches validated
- [ ] Health check detects inconsistencies

---

## Next Steps

1. **Clarify decisions** #1-4 (user input needed)
2. **Schedule implementation** (2-3 days estimated)
3. **Create subtasks** in GitHub Issues (one per section)
4. **Start Phase A** (critical path first)
5. **Continuous testing** (E2E crash scenarios at each step)

