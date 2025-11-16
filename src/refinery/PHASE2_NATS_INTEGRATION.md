# Phase 2: Refinery NATS Integration - Hash-Only Flow

**Branch**: `digger-refactor-phase2`  
**Parent**: `feature/digger-refactor`  
**Date**: November 16, 2025  
**Status**: 🚧 **IN PROGRESS**

---

## 🎯 Mission

Integrate Refinery with NATS to consume hash batches from Digger and build TokenTorqIngots using **merkle tree aggregation** - WITHOUT transferring full JTU data.

**Key Insight**: Refinery is a **hash aggregator**, not a data warehouse.

---

## 📊 Architecture: Hash-Only Flow

### Current State (Phase 1 Complete)
- ✅ Digger executes contracts → generates JTUs → stores in SQLite
- ✅ Digger sends **hashes only** to NATS (`ore.batch` subject)
- ✅ Refinery has HTTP endpoint `/receive-ore` (accepts full ore data)
- ❌ Refinery NOT consuming NATS messages yet

### Target State (Phase 2)
- ✅ Refinery subscribes to NATS `ore.batch` subject
- ✅ Receives **hash batches** (32 bytes each, not ~1500 bytes/unit)
- ✅ Accumulates 3600 hashes in queue
- ✅ Builds **merkle tree** from hashes
- ✅ Creates `TokenTorqIngot` with `branch_hash` (merkle root)
- ✅ Publishes ingots to Mint via NATS
- ✅ Full JTUs remain on Digger (for audit/verification)

---

## 🔄 Data Flow

```
Digger                          NATS                    Refinery
  |                              |                          |
  | Execute contract             |                          |
  | Generate 10,000 JTUs         |                          |
  | Store in SQLite              |                          |
  |                              |                          |
  |-- Publish HASHES ONLY ------>|                          |
  |    (ore.batch)               |-- Deliver batch -------->|
  |    [hash1, hash2, ...]       |                          |
  |                              |                (receive hashes)
  |                              |                (queue hashes)
  |                              |                (wait for 3600)
  |                              |                          |
  |                              |                (build merkle tree)
  |                              |                (compute branch_hash)
  |                              |                          |
  |                              |<-- Publish ingot --------|
  |                              |    (mint.ingots)         |
  |                              |                          |
  |                              |---------- ingot -------->| Mint
  |                              |                          |
  |                              |                (validates merkle root)
```

---

## 📋 Implementation Plan

### Milestone 1: NATS Subscriber (CURRENT)
**Goal**: Refinery listens to NATS and logs received hash batches

**Tasks**:
- [ ] Add NATS client to Refinery
- [ ] Create `NATSSubscriber` component
- [ ] Subscribe to `ore.batch` subject
- [ ] Parse `HashBatchMessage` JSON
- [ ] Log received batches (count, contract_id, digger_id)

**Files**:
- `internal/refinery/nats_subscriber.go` (NEW)
- `cmd/refinery/main.go` (UPDATE - add NATS connection)
- `go.mod` (UPDATE - add `github.com/nats-io/nats.go`)

**Success Criteria**:
- ✅ Refinery connects to NATS on startup
- ✅ Logs show: "received hash batch: contract_id=X, hash_count=Y"
- ✅ No errors on hash batch delivery

---

### Milestone 2: Hash Queue Manager
**Goal**: Store hashes in queue (not full units)

**Tasks**:
- [ ] Update `QueueManager` to store `HashEntry` structs
- [ ] Implement `AddHash(hash, contract_id, digger_id)`
- [ ] Implement `GetHashes(count int) []HashEntry`
- [ ] Add blocking behavior (wait until 3600 hashes available)
- [ ] Remove old unit-based logic

**Files**:
- `internal/refinery/queue_manager.go` (UPDATE)
- `internal/refinery/queue_manager_test.go` (UPDATE)

**Data Structure**:
```go
type HashEntry struct {
    Hash       string    // 32-byte hex string
    ContractID string
    DiggerID   string
    Timestamp  time.Time
}
```

**Success Criteria**:
- ✅ Can add 10,000 hashes to queue
- ✅ GetHashes(3600) blocks until 3600 available
- ✅ FIFO ordering maintained
- ✅ Thread-safe (concurrent AddHash calls)

---

### Milestone 3: Merkle Tree Builder
**Goal**: Compute merkle root from 3600 hashes

**Tasks**:
- [ ] Implement `calculateMerkleRoot(hashes []string) string`
- [ ] Build binary tree (pair-wise hashing)
- [ ] Handle odd-numbered levels (duplicate last hash)
- [ ] Use SHA256 for intermediate hashes
- [ ] Return root hash (32-byte hex string)

**Files**:
- `internal/refinery/merkle.go` (NEW)
- `internal/refinery/merkle_test.go` (NEW)

**Algorithm**:
```
Level 0: [h1, h2, h3, h4, ..., h3600]      // 3600 hashes
Level 1: [H(h1+h2), H(h3+h4), ...]         // 1800 hashes
Level 2: [H(H1+H2), H(H3+H4), ...]         // 900 hashes
...
Level 11: [ROOT_HASH]                       // 1 hash (merkle root)
```

**Success Criteria**:
- ✅ Deterministic (same hashes → same root)
- ✅ Handles 3600 hashes (exact ingot size)
- ✅ Test with known hash set (verify root)
- ✅ Performance: <10ms for 3600 hashes

---

### Milestone 4: Ingot Assembly (Hash-Based)
**Goal**: Create TokenTorqIngot with merkle proof (no unit data)

**Tasks**:
- [ ] Update `IngotAssembler` to use `GetHashes(3600)`
- [ ] Call `calculateMerkleRoot()` on hash batch
- [ ] Create `TokenTorqIngot` with `branch_hash`
- [ ] Remove `Units` field from ingot (hash-only!)
- [ ] Add `ContractIDs`, `DiggerIDs` metadata

**Files**:
- `internal/refinery/ingot_assembler.go` (UPDATE)
- `internal/models/token_torq.go` (UPDATE)

**Updated Ingot Model**:
```go
type TokenTorqIngot struct {
    ID          string    `json:"id"`
    BranchHash  string    `json:"branch_hash"`   // Merkle root
    HashCount   int       `json:"hash_count"`    // 3600
    ContractIDs []string  `json:"contract_ids"`  // Unique contracts
    DiggerIDs   []string  `json:"digger_ids"`    // Unique diggers
    Timestamp   time.Time `json:"timestamp"`
    
    // NO Units field - Refinery doesn't store full JTUs!
}
```

**Success Criteria**:
- ✅ Ingot assembled from 3600 hashes
- ✅ `branch_hash` is valid merkle root
- ✅ `hash_count` = 3600
- ✅ No full JTU data in ingot

---

### Milestone 5: E2E Integration Test
**Goal**: Full pipeline from Digger to Refinery to Mint

**Tasks**:
- [ ] Start NATS, Digger, Refinery
- [ ] Execute contract on Digger (10,000 JTUs)
- [ ] Verify hashes published to NATS
- [ ] Verify Refinery receives hashes
- [ ] Verify ingot assembled (3 ingots from 10k hashes)
- [ ] Verify ingots have correct merkle roots

**Files**:
- `test-phase2-e2e.ps1` (NEW)

**Test Script**:
```powershell
# Start services
docker-compose up -d nats
cd src/refinery; go run cmd/refinery/main.go &
cd src/digger; cargo run --release &

# Execute contract
curl -X POST http://localhost:9000/execute `
  -H "Content-Type: application/json" `
  -d '{"contract_id":"phase2-test","duration_seconds":5}'

# Wait for processing
Start-Sleep -Seconds 30

# Check Refinery logs
docker logs robotorq-network-refinery-1 | Select-String "ingot assembled"

# Verify ingot count (10k hashes ÷ 3600 = 2.77 → 2 full ingots)
# Expected: 2 ingots with 3600 hashes each
```

**Success Criteria**:
- ✅ 10,000 hashes transmitted via NATS
- ✅ Refinery receives all hashes
- ✅ 2 full ingots assembled (7200 hashes used)
- ✅ Each ingot has valid `branch_hash`
- ✅ Remaining 2800 hashes queued for next ingot

---

## 🎯 Phase 2 Success Criteria

**Complete When**:
- ✅ Refinery subscribes to NATS `ore.batch`
- ✅ Hash batches flow into queue
- ✅ Merkle tree built from 3600 hashes
- ✅ `TokenTorqIngot` contains `branch_hash` (merkle root)
- ✅ **NO full JTU data transferred** (hash-only flow)
- ✅ E2E test: Digger → NATS → Refinery → ingot
- ✅ Bandwidth: ~32 bytes/hash (not ~1500 bytes/unit)
- ✅ Performance: 300 hashes/sec throughput

---

## 📈 Bandwidth Savings

**Old Flow (HTTP /receive-ore)**:
- 1 JTU = ~1500 bytes (full data + signature)
- 3600 units = 5.4 MB per ingot
- 10,000 units = 15 MB transfer

**New Flow (NATS hash-only)**:
- 1 hash = 32 bytes (SHA256 hex string)
- 3600 hashes = 115 KB per ingot
- 10,000 hashes = 320 KB transfer

**Savings**: **98% bandwidth reduction** 🎉

---

## 🔮 Future Phases

- **Phase 3**: Mint batch validation (verify ingot merkle trees)
- **Phase 4**: Robot identity + Falcon-1024 signatures
- **Phase 5**: DistoDam distribution (ingots → wallets)
- **Phase 6**: TOON binary encoding (60% additional savings)

---

## 📝 Notes

**Why hash-only?**
1. **Efficiency**: 32 bytes vs 1500 bytes per unit
2. **Security**: Hashes are commitments, full data on-demand only
3. **Scalability**: Merkle trees enable verification without full data
4. **Audit trail**: Full JTUs stay on Digger for forensics

**Merkle Tree Properties**:
- Deterministic (same inputs → same root)
- Compact (log₂(N) proof size)
- Verifiable (can prove inclusion without revealing all data)
- Quantum-resistant (SHA256 hash-based)

**Design Decision**: Refinery is a **hash aggregator**, not a data warehouse. The atomic proof-of-work (JTUs) lives on the Digger where it was created. Refinery only needs to prove "these 3600 hashes exist" via the merkle root.

---

*"Watts > Wall Street"* 🤖⚡💰
