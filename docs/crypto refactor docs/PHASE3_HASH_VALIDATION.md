# Phase 3: Mint Hash Validation & Merkle Aggregation

**Branch**: `feature/proof-chain-phase3`  
**Parent**: `feature/digger-refactor` (after Phase 2 merge)  
**Date**: November 16, 2025  
**Status**: 📋 **PLANNED** - Ready to Start

---

## 🎯 Mission

Upgrade Mint service to consume **hash-only ingots** from Refinery, validate merkle trees, build **second-level merkle aggregation** (1000 ingot hashes → 1 RT merkle root), and publish minimal RoboTorqUnit data to DistoDam.

**Key Insight**: Mint is a **merkle aggregator**, not a data warehouse. It validates refinery proofs and builds the final cryptographic commitment.

---

## 📊 Architecture: Two-Level Merkle Tree

### Current State (Phase 2 Complete)
- ✅ Refinery builds merkle trees from 3600 JTU hashes → `branch_hash` (Level 1)
- ✅ Refinery publishes `Phase2Ingot` with `branch_hash` to NATS `mint.ingots`
- ❌ Mint still expects old `TokenTorqIngot` with full `Units[]` array
- ❌ Mint builds simple SHA256 concatenation (not merkle tree)

### Target State (Phase 3)
- ✅ Mint receives `Phase2Ingot` from NATS (hash-only, no units)
- ✅ Mint validates ingot structure (branch_hash exists, hash_count = 3600)
- ✅ Mint accumulates 1000 ingot `branch_hash` values
- ✅ Mint builds **Level 2 merkle tree** from 1000 branch hashes
- ✅ Mint creates `Phase3RoboTorqUnit` with `merkle_root` (32 bytes)
- ✅ Mint publishes minimal RT to DistoDam (no ingot array, just hash)
- ✅ Full ingot data stored in **ledger** for verification

---

## 🔄 Data Flow (Proof Chain)

```
Digger                      Refinery                    Mint                    DistoDam
  |                            |                          |                          |
  | 3600 JTU hashes           |                          |                          |
  |-------------------------->|                          |                          |
  |    (ore.batch)            |                          |                          |
  |                           |                          |                          |
  |                      (build merkle tree)             |                          |
  |                      Level 1: 3600 hashes            |                          |
  |                           ↓                          |                          |
  |                      branch_hash (32B)               |                          |
  |                           |                          |                          |
  |                           |-- Phase2Ingot ---------->|                          |
  |                           |   (mint.ingots)          |                          |
  |                           |                          |                          |
  |                           |                     (validate ingot)                |
  |                           |                     (queue branch_hash)             |
  |                           |                     (wait for 1000)                 |
  |                           |                          |                          |
  |                           |                     (build merkle tree)             |
  |                           |                     Level 2: 1000 hashes            |
  |                           |                          ↓                          |
  |                           |                     merkle_root (32B)               |
  |                           |                          |                          |
  |                           |                          |-- Phase3RoboTorqUnit -->|
  |                           |                          |   (distodam.units)       |
  |                           |                          |                          |
  |                           |                     (store ledger)                  |
  |                           |                     - All 1000 ingot hashes         |
  |                           |                     - Merkle tree structure         |
  |                           |                     - Verification proofs           |
```

**Compression Ratio**:
- **Input**: 1000 ingots × (64-byte branch_hash + metadata) = ~100 KB
- **Output**: 1 RT × (32-byte merkle_root + metadata) = ~500 bytes
- **Ledger**: 1000 hashes + merkle tree = ~50 KB (for verification)
- **Network Broadcast**: 500 bytes (200× reduction from storing full ingots)

---

## 📋 Implementation Plan

### Milestone 1: Phase2Ingot NATS Consumer ⏳ **TODO**
**Goal**: Mint receives and validates hash-only ingots

**Tasks**:
- [ ] Update NATS subscriber to parse `Phase2Ingot` JSON
- [ ] Validate ingot structure:
  - `branch_hash` exists (64-char hex string)
  - `hash_count == 3600` (exact ingot size)
  - `merkle_height == 12` (binary tree depth for 3600 hashes)
  - `contract_ids` and `digger_ids` arrays non-empty
- [ ] Add Prometheus metrics:
  - `mint_phase2_ingots_received_total` - Counter
  - `mint_ingot_validation_errors_total{reason="..."}` - Counter
  - `mint_ingot_hash_count_histogram` - Histogram (should peak at 3600)
- [ ] Log ingot receipt with structured fields
- [ ] Create E2E test: Refinery → NATS → Mint (validate ingot received)

**Files to Create**:
- `internal/mint/phase2_ingot_receiver.go` - New NATS subscriber
- `internal/mint/phase2_ingot_receiver_test.go` - Unit tests
- `internal/models/phase2_ingot.go` - Data model (or import from refinery)
- `test-phase3-milestone1.py` - Python E2E test

**Data Model** (shared with Refinery):
```go
// internal/models/phase2_ingot.go
package models

import "time"

type Phase2Ingot struct {
    ID           string    `json:"id"`                // ingot-20251116-001
    BranchHash   string    `json:"branch_hash"`       // Merkle root (64-char hex)
    HashCount    int       `json:"hash_count"`        // 3600
    MerkleHeight int       `json:"merkle_height"`     // 12 (⌈log₂(3600)⌉)
    ContractIDs  []string  `json:"contract_ids"`      // Unique contracts in ingot
    DiggerIDs    []string  `json:"digger_ids"`        // Unique diggers
    RefineryID   string    `json:"refinery_id"`       // Which refinery created this
    Timestamp    time.Time `json:"timestamp"`         // UTC
}

// Validation
func (pi *Phase2Ingot) Validate() error {
    if len(pi.BranchHash) != 64 {
        return fmt.Errorf("invalid branch_hash length: %d (expected 64)", len(pi.BranchHash))
    }
    if pi.HashCount != 3600 {
        return fmt.Errorf("invalid hash_count: %d (expected 3600)", pi.HashCount)
    }
    if pi.MerkleHeight != 12 {
        return fmt.Errorf("invalid merkle_height: %d (expected 12)", pi.MerkleHeight)
    }
    if len(pi.ContractIDs) == 0 {
        return errors.New("contract_ids cannot be empty")
    }
    if len(pi.DiggerIDs) == 0 {
        return errors.New("digger_ids cannot be empty")
    }
    // Validate branch_hash is valid hex
    if _, err := hex.DecodeString(pi.BranchHash); err != nil {
        return fmt.Errorf("branch_hash is not valid hex: %w", err)
    }
    return nil
}
```

**NATS Subscriber**:
```go
// internal/mint/phase2_ingot_receiver.go
package mint

import (
    "encoding/json"
    "log/slog"
    "github.com/nats-io/nats.go"
    "mint/internal/models"
)

type Phase2IngotReceiver struct {
    natsConn     *nats.Conn
    subscription *nats.Subscription
    queue        *IngotHashQueue  // New queue component (Milestone 2)
    logger       *slog.Logger
    metrics      *Phase2IngotMetrics
}

func NewPhase2IngotReceiver(nc *nats.Conn, queue *IngotHashQueue, logger *slog.Logger, metrics *Phase2IngotMetrics) (*Phase2IngotReceiver, error) {
    return &Phase2IngotReceiver{
        natsConn: nc,
        queue:    queue,
        logger:   logger,
        metrics:  metrics,
    }, nil
}

func (pir *Phase2IngotReceiver) Start() error {
    sub, err := pir.natsConn.Subscribe("mint.ingots", pir.handleIngot)
    if err != nil {
        return fmt.Errorf("failed to subscribe to mint.ingots: %w", err)
    }
    pir.subscription = sub
    pir.logger.Info("Phase2IngotReceiver started", "subject", "mint.ingots")
    return nil
}

func (pir *Phase2IngotReceiver) handleIngot(msg *nats.Msg) {
    var ingot models.Phase2Ingot
    if err := json.Unmarshal(msg.Data, &ingot); err != nil {
        pir.logger.Error("failed to unmarshal ingot", "error", err)
        pir.metrics.ValidationErrors.WithLabelValues("unmarshal").Inc()
        return
    }

    // Validate ingot
    if err := ingot.Validate(); err != nil {
        pir.logger.Warn("invalid ingot received", "error", err, "ingot_id", ingot.ID)
        pir.metrics.ValidationErrors.WithLabelValues("validation").Inc()
        return
    }

    // Add to queue
    if err := pir.queue.AddIngotHash(ingot.BranchHash, &ingot); err != nil {
        pir.logger.Error("failed to queue ingot hash", "error", err, "ingot_id", ingot.ID)
        pir.metrics.QueueErrors.Inc()
        return
    }

    pir.metrics.IngotsReceived.Inc()
    pir.logger.Info("Phase 2 ingot received",
        "ingot_id", ingot.ID,
        "branch_hash", ingot.BranchHash[:16]+"...",
        "hash_count", ingot.HashCount,
        "contracts", len(ingot.ContractIDs),
        "diggers", len(ingot.DiggerIDs))
}

func (pir *Phase2IngotReceiver) Stop() {
    if pir.subscription != nil {
        pir.subscription.Unsubscribe()
        pir.logger.Info("Phase2IngotReceiver stopped")
    }
}
```

**Metrics**:
```go
// internal/mint/metrics.go
type Phase2IngotMetrics struct {
    IngotsReceived    prometheus.Counter
    ValidationErrors  *prometheus.CounterVec  // Labels: reason={unmarshal,validation}
    QueueErrors       prometheus.Counter
    IngotHashCountHistogram prometheus.Histogram
}

func NewPhase2IngotMetrics() *Phase2IngotMetrics {
    return &Phase2IngotMetrics{
        IngotsReceived: prometheus.NewCounter(prometheus.CounterOpts{
            Name: "mint_phase2_ingots_received_total",
            Help: "Total Phase 2 ingots received from NATS",
        }),
        ValidationErrors: prometheus.NewCounterVec(prometheus.CounterOpts{
            Name: "mint_ingot_validation_errors_total",
            Help: "Ingot validation errors by reason",
        }, []string{"reason"}),
        QueueErrors: prometheus.NewCounter(prometheus.CounterOpts{
            Name: "mint_ingot_queue_errors_total",
            Help: "Errors queuing ingot hashes",
        }),
        IngotHashCountHistogram: prometheus.NewHistogram(prometheus.HistogramOpts{
            Name:    "mint_ingot_hash_count",
            Help:    "Distribution of hash counts in received ingots",
            Buckets: []float64{100, 500, 1000, 2000, 3600, 5000, 10000},
        }),
    }
}
```

**E2E Test** (Python):
```python
# test-phase3-milestone1.py
"""
Phase 3 Milestone 1: Verify Mint receives Phase2Ingots from NATS
"""
import json
import subprocess
import time
from nats.aio.client import Client as NATS

async def test_phase2_ingot_receipt():
    # Connect to NATS
    nc = NATS()
    await nc.connect("nats://localhost:4222")
    
    # Publish test ingot
    test_ingot = {
        "id": "ingot-milestone1-test",
        "branch_hash": "a" * 64,  # Valid 64-char hex
        "hash_count": 3600,
        "merkle_height": 12,
        "contract_ids": ["test-contract-001"],
        "digger_ids": ["test-digger-001"],
        "refinery_id": "test-refinery",
        "timestamp": "2025-11-16T12:00:00Z"
    }
    
    await nc.publish("mint.ingots", json.dumps(test_ingot).encode())
    await nc.flush()
    
    # Wait for Mint to process
    time.sleep(2)
    
    # Check Mint logs for receipt
    result = subprocess.run(
        ["docker", "logs", "robotorq-network-mint-1", "--since", "5s"],
        capture_output=True, text=True
    )
    
    assert "Phase 2 ingot received" in result.stdout
    assert "ingot-milestone1-test" in result.stdout
    assert "hash_count: 3600" in result.stdout
    
    print("✅ Milestone 1: Mint successfully received Phase2Ingot")
    
    await nc.close()

if __name__ == "__main__":
    import asyncio
    asyncio.run(test_phase2_ingot_receipt())
```

**Success Criteria**:
- ✅ Mint subscribes to `mint.ingots` on startup
- ✅ Receives `Phase2Ingot` JSON from NATS
- ✅ Validates `branch_hash`, `hash_count`, `merkle_height`
- ✅ Rejects invalid ingots (logs warning, increments error metric)
- ✅ Logs valid ingot receipt with structured fields
- ✅ E2E test passes: Python → NATS → Mint

---

### Milestone 2: Ingot Hash Queue ⏳ **TODO**
**Goal**: Store ingot hashes (not full ingots) in queue

**Tasks**:
- [ ] Create `IngotHashQueue` component (similar to Refinery's QueueManager)
- [ ] Implement `AddIngotHash(hash string, metadata *Phase2Ingot)`
- [ ] Implement `GetIngotHashes(count int) []IngotHashEntry` (blocking)
- [ ] Store hash + minimal metadata (contract_ids, digger_ids, timestamp)
- [ ] Add queue depth metrics
- [ ] Wire Phase2IngotReceiver to call AddIngotHash()
- [ ] Update main.go to create and wire IngotHashQueue

**Files**:
- `internal/mint/ingot_hash_queue.go` - New queue component
- `internal/mint/ingot_hash_queue_test.go` - Unit tests
- `cmd/mint/main.go` - Wire IngotHashQueue

**Data Structure**:
```go
// internal/mint/ingot_hash_queue.go
package mint

import (
    "context"
    "sync"
    "time"
)

type IngotHashEntry struct {
    BranchHash   string              // 64-char hex (32-byte SHA256)
    ContractIDs  []string            // Contracts in this ingot
    DiggerIDs    []string            // Diggers in this ingot
    RefineryID   string              // Which refinery created
    Timestamp    time.Time           // When received by Mint
    Index        int64               // Sequential order
}

type IngotHashQueue struct {
    entries  []IngotHashEntry
    capacity int
    mu       sync.Mutex
    notEmpty *sync.Cond
    index    int64  // Auto-incrementing index
}

func NewIngotHashQueue(capacity int) *IngotHashQueue {
    q := &IngotHashQueue{
        entries:  make([]IngotHashEntry, 0, capacity),
        capacity: capacity,
        index:    0,
    }
    q.notEmpty = sync.NewCond(&q.mu)
    return q
}

func (ihq *IngotHashQueue) AddIngotHash(hash string, ingot *models.Phase2Ingot) error {
    ihq.mu.Lock()
    defer ihq.mu.Unlock()
    
    if len(ihq.entries) >= ihq.capacity {
        return fmt.Errorf("queue full (capacity: %d)", ihq.capacity)
    }
    
    entry := IngotHashEntry{
        BranchHash:  hash,
        ContractIDs: ingot.ContractIDs,
        DiggerIDs:   ingot.DiggerIDs,
        RefineryID:  ingot.RefineryID,
        Timestamp:   time.Now().UTC(),
        Index:       ihq.index,
    }
    
    ihq.entries = append(ihq.entries, entry)
    ihq.index++
    ihq.notEmpty.Signal()
    return nil
}

func (ihq *IngotHashQueue) GetIngotHashes(ctx context.Context, count int) ([]IngotHashEntry, error) {
    ihq.mu.Lock()
    defer ihq.mu.Unlock()
    
    // Block until enough hashes available
    for len(ihq.entries) < count {
        select {
        case <-ctx.Done():
            return nil, ctx.Err()
        default:
            ihq.notEmpty.Wait()
        }
    }
    
    // Extract first N entries
    result := make([]IngotHashEntry, count)
    copy(result, ihq.entries[:count])
    
    // Remove from queue
    ihq.entries = ihq.entries[count:]
    
    return result, nil
}

func (ihq *IngotHashQueue) Len() int {
    ihq.mu.Lock()
    defer ihq.mu.Unlock()
    return len(ihq.entries)
}
```

**Configuration**:
```bash
INGOT_HASH_QUEUE_SIZE=5000  # Capacity (5× batch size for buffer)
```

**Tests**:
```go
// internal/mint/ingot_hash_queue_test.go
func TestIngotHashQueue_AddAndGet(t *testing.T) {
    q := NewIngotHashQueue(10)
    
    // Add ingot hash
    ingot := &models.Phase2Ingot{
        ID:          "test-1",
        BranchHash:  "abcd1234...",
        HashCount:   3600,
        ContractIDs: []string{"contract-1"},
        DiggerIDs:   []string{"digger-1"},
    }
    
    err := q.AddIngotHash(ingot.BranchHash, ingot)
    assert.NoError(t, err)
    assert.Equal(t, 1, q.Len())
    
    // Get hash (should not block - we have 1 entry)
    ctx := context.Background()
    entries, err := q.GetIngotHashes(ctx, 1)
    assert.NoError(t, err)
    assert.Equal(t, "abcd1234...", entries[0].BranchHash)
    assert.Equal(t, 0, q.Len())
}

func TestIngotHashQueue_BlockingGet(t *testing.T) {
    q := NewIngotHashQueue(100)
    
    retrieved := false
    go func() {
        ctx := context.Background()
        entries, _ := q.GetIngotHashes(ctx, 5)  // Blocks until 5 available
        assert.Equal(t, 5, len(entries))
        retrieved = true
    }()
    
    time.Sleep(100 * time.Millisecond)
    assert.False(t, retrieved, "Should still be blocked")
    
    // Add 5 ingot hashes
    for i := 0; i < 5; i++ {
        ingot := &models.Phase2Ingot{
            BranchHash:  fmt.Sprintf("hash-%d", i),
            ContractIDs: []string{"contract"},
            DiggerIDs:   []string{"digger"},
        }
        q.AddIngotHash(ingot.BranchHash, ingot)
    }
    
    time.Sleep(100 * time.Millisecond)
    assert.True(t, retrieved, "Should have unblocked")
}
```

**Success Criteria**:
- ✅ Queue stores ingot hashes (not full ingots)
- ✅ `AddIngotHash()` non-blocking with backpressure
- ✅ `GetIngotHashes(1000)` blocks until 1000 hashes available
- ✅ Thread-safe (concurrent adds/gets)
- ✅ Context cancellation support
- ✅ Queue depth metrics accurate

---

### Milestone 3: Level 2 Merkle Tree Builder ⏳ **TODO**
**Goal**: Build merkle tree from 1000 ingot hashes

**Tasks**:
- [ ] Implement `BuildLevel2MerkleTree(ingotHashes []string) (*MerkleTree, error)`
- [ ] Use same binary tree algorithm as Refinery (pair-wise SHA256)
- [ ] Handle odd counts (duplicate last hash)
- [ ] Compute merkle root (32-byte hex string)
- [ ] Add comprehensive unit tests (determinism, 1000 hashes, odd counts)
- [ ] Benchmark performance (<20ms target for 1000 hashes)

**Files**:
- `internal/mint/level2_merkle.go` - Merkle tree builder
- `internal/mint/level2_merkle_test.go` - Unit tests

**Implementation** (reuse Refinery's algorithm):
```go
// internal/mint/level2_merkle.go
package mint

import (
    "crypto/sha256"
    "encoding/hex"
    "fmt"
)

type MerkleTree struct {
    Root   string   // Merkle root hash (64-char hex)
    Height int      // Tree depth
    Leaves []string // Original hashes (for verification)
}

func BuildLevel2MerkleTree(ingotHashes []string) (*MerkleTree, error) {
    if len(ingotHashes) == 0 {
        return nil, fmt.Errorf("cannot build merkle tree from empty hash list")
    }
    
    // Start with leaf hashes
    currentLevel := make([]string, len(ingotHashes))
    copy(currentLevel, ingotHashes)
    
    height := 1
    
    // Build tree level by level
    for len(currentLevel) > 1 {
        nextLevel := make([]string, 0, (len(currentLevel)+1)/2)
        
        for i := 0; i < len(currentLevel); i += 2 {
            var left, right string
            left = currentLevel[i]
            
            // Handle odd count: duplicate last hash
            if i+1 < len(currentLevel) {
                right = currentLevel[i+1]
            } else {
                right = left
            }
            
            // Hash(left + right)
            combined := left + right
            hash := sha256.Sum256([]byte(combined))
            parentHash := hex.EncodeToString(hash[:])
            
            nextLevel = append(nextLevel, parentHash)
        }
        
        currentLevel = nextLevel
        height++
    }
    
    return &MerkleTree{
        Root:   currentLevel[0],
        Height: height,
        Leaves: ingotHashes,
    }, nil
}
```

**Tests**:
```go
// internal/mint/level2_merkle_test.go
func TestBuildLevel2MerkleTree_1000Hashes(t *testing.T) {
    // Generate 1000 test hashes
    hashes := make([]string, 1000)
    for i := 0; i < 1000; i++ {
        hash := sha256.Sum256([]byte(fmt.Sprintf("ingot-%d", i)))
        hashes[i] = hex.EncodeToString(hash[:])
    }
    
    tree, err := BuildLevel2MerkleTree(hashes)
    assert.NoError(t, err)
    
    // Verify root hash
    assert.NotEmpty(t, tree.Root)
    assert.Equal(t, 64, len(tree.Root))  // 32 bytes = 64 hex chars
    
    // Verify height: ⌈log₂(1000)⌉ = 10
    assert.Equal(t, 10, tree.Height)
}

func TestBuildLevel2MerkleTree_Determinism(t *testing.T) {
    hashes := generateTestHashes(1000)
    
    tree1, _ := BuildLevel2MerkleTree(hashes)
    tree2, _ := BuildLevel2MerkleTree(hashes)
    
    // Same input → same merkle root
    assert.Equal(t, tree1.Root, tree2.Root)
}

func TestBuildLevel2MerkleTree_OrderSensitivity(t *testing.T) {
    hashes := generateTestHashes(100)
    
    tree1, _ := BuildLevel2MerkleTree(hashes)
    
    // Reverse order
    reversed := make([]string, len(hashes))
    for i, h := range hashes {
        reversed[len(hashes)-1-i] = h
    }
    tree2, _ := BuildLevel2MerkleTree(reversed)
    
    // Different order → different merkle root
    assert.NotEqual(t, tree1.Root, tree2.Root)
}

func BenchmarkBuildLevel2MerkleTree_1000(b *testing.B) {
    hashes := generateTestHashes(1000)
    
    b.ResetTimer()
    for i := 0; i < b.N; i++ {
        BuildLevel2MerkleTree(hashes)
    }
}
// Expected: <20ms per operation
```

**Success Criteria**:
- ✅ Builds merkle tree from 1000 ingot hashes
- ✅ Merkle root is deterministic (same input → same output)
- ✅ Handles odd counts (999, 1001 hashes)
- ✅ Height = 10 for 1000 hashes (⌈log₂(1000)⌉)
- ✅ Performance: <20ms for 1000 hashes
- ✅ Order-sensitive (different order → different root)

---

### Milestone 4: Phase3RoboTorqUnit Assembly ⏳ **TODO**
**Goal**: Create minimal RoboTorqUnit with merkle root

**Tasks**:
- [ ] Create `Phase3RoboTorqAssembler` component
- [ ] Loop: GetIngotHashes(1000) → BuildLevel2MerkleTree() → Publish RT
- [ ] Create `Phase3RoboTorqUnit` data model (hash-only, no ingots array)
- [ ] Store ledger entry (1000 ingot hashes + merkle tree)
- [ ] Publish to NATS `distodam.units`
- [ ] Wire assembler into main.go
- [ ] Add assembly time metrics

**Files**:
- `internal/mint/phase3_rt_assembler.go` - RT assembly logic
- `internal/models/phase3_robotorq.go` - Data model
- `internal/mint/ledger.go` - Ledger storage (in-memory for Phase 3)
- `cmd/mint/main.go` - Wire assembler

**Phase3RoboTorqUnit Model**:
```go
// internal/models/phase3_robotorq.go
package models

import "time"

type Phase3RoboTorqUnit struct {
    UnitID       string    `json:"unit_id"`        // RT-20251116-001
    MerkleRoot   string    `json:"merkle_root"`    // Level 2 merkle root (64-char hex)
    IngotCount   int       `json:"ingot_count"`    // 1000
    MerkleHeight int       `json:"merkle_height"`  // 10 (⌈log₂(1000)⌉)
    MintedAt     time.Time `json:"minted_at"`      // UTC timestamp
    
    // Aggregated metadata
    TotalHashCount   int      `json:"total_hash_count"`   // 1000 × 3600 = 3,600,000 JTU hashes
    UniqueContracts  []string `json:"unique_contracts"`   // All unique contract IDs
    UniqueDiggers    []string `json:"unique_diggers"`     // All unique digger IDs
    UniqueRefineries []string `json:"unique_refineries"`  // All unique refinery IDs
    
    // NO Ingots[] array! Stored in ledger separately
    // NO TotalJoules! Not tracked in hash-only flow
    // NO TotalRoboStake! Not tracked in hash-only flow
}

func (rt *Phase3RoboTorqUnit) Validate() error {
    if len(rt.MerkleRoot) != 64 {
        return fmt.Errorf("invalid merkle_root length: %d", len(rt.MerkleRoot))
    }
    if rt.IngotCount != 1000 {
        return fmt.Errorf("invalid ingot_count: %d (expected 1000)", rt.IngotCount)
    }
    if rt.MerkleHeight != 10 {
        return fmt.Errorf("invalid merkle_height: %d (expected 10)", rt.MerkleHeight)
    }
    if rt.TotalHashCount != 3_600_000 {
        return fmt.Errorf("invalid total_hash_count: %d (expected 3,600,000)", rt.TotalHashCount)
    }
    return nil
}
```

**Assembler**:
```go
// internal/mint/phase3_rt_assembler.go
package mint

import (
    "context"
    "fmt"
    "log/slog"
    "time"
    "mint/internal/models"
)

type Phase3RTAssembler struct {
    queue      *IngotHashQueue
    ledger     *Ledger
    publisher  *DistoDamPublisher  // Milestone 5
    logger     *slog.Logger
    metrics    *Phase3RTMetrics
}

func NewPhase3RTAssembler(queue *IngotHashQueue, ledger *Ledger, publisher *DistoDamPublisher, logger *slog.Logger, metrics *Phase3RTMetrics) *Phase3RTAssembler {
    return &Phase3RTAssembler{
        queue:     queue,
        ledger:    ledger,
        publisher: publisher,
        logger:    logger,
        metrics:   metrics,
    }
}

func (asm *Phase3RTAssembler) Start(ctx context.Context) {
    asm.logger.Info("Phase3RTAssembler started", "batch_size", 1000)
    
    for {
        select {
        case <-ctx.Done():
            asm.logger.Info("Phase3RTAssembler shutting down")
            return
        default:
            // Get 1000 ingot hashes (blocks until available)
            entries, err := asm.queue.GetIngotHashes(ctx, 1000)
            if err != nil {
                if err == context.Canceled {
                    return
                }
                asm.logger.Error("failed to get ingot hashes", "error", err)
                continue
            }
            
            // Build Level 2 merkle tree
            start := time.Now()
            rt, err := asm.assembleRT(entries)
            if err != nil {
                asm.logger.Error("failed to assemble RT", "error", err)
                asm.metrics.AssemblyErrors.Inc()
                continue
            }
            
            duration := time.Since(start)
            asm.metrics.AssemblyDuration.Observe(duration.Seconds())
            
            // Store in ledger
            if err := asm.ledger.StoreRT(rt, entries); err != nil {
                asm.logger.Error("failed to store RT in ledger", "error", err)
                continue
            }
            
            // Publish to DistoDam
            if err := asm.publisher.PublishRT(rt); err != nil {
                asm.logger.Error("failed to publish RT", "error", err)
                asm.metrics.PublishErrors.Inc()
                continue
            }
            
            asm.metrics.RTsAssembled.Inc()
            asm.logger.Info("Phase 3 RT assembled",
                "unit_id", rt.UnitID,
                "merkle_root", rt.MerkleRoot[:16]+"...",
                "ingot_count", rt.IngotCount,
                "total_hash_count", rt.TotalHashCount,
                "contracts", len(rt.UniqueContracts),
                "diggers", len(rt.UniqueDiggers),
                "assembly_time_ms", duration.Milliseconds())
        }
    }
}

func (asm *Phase3RTAssembler) assembleRT(entries []IngotHashEntry) (*models.Phase3RoboTorqUnit, error) {
    // Extract branch hashes
    branchHashes := make([]string, len(entries))
    for i, entry := range entries {
        branchHashes[i] = entry.BranchHash
    }
    
    // Build Level 2 merkle tree
    tree, err := BuildLevel2MerkleTree(branchHashes)
    if err != nil {
        return nil, fmt.Errorf("failed to build merkle tree: %w", err)
    }
    
    // Aggregate metadata
    contractSet := make(map[string]bool)
    diggerSet := make(map[string]bool)
    refinerySet := make(map[string]bool)
    
    for _, entry := range entries {
        for _, cid := range entry.ContractIDs {
            contractSet[cid] = true
        }
        for _, did := range entry.DiggerIDs {
            diggerSet[did] = true
        }
        refinerySet[entry.RefineryID] = true
    }
    
    // Convert sets to slices
    contracts := make([]string, 0, len(contractSet))
    for c := range contractSet {
        contracts = append(contracts, c)
    }
    diggers := make([]string, 0, len(diggerSet))
    for d := range diggerSet {
        diggers = append(diggers, d)
    }
    refineries := make([]string, 0, len(refinerySet))
    for r := range refinerySet {
        refineries = append(refineries, r)
    }
    
    // Create RT
    rt := &models.Phase3RoboTorqUnit{
        UnitID:           generateRTID(),
        MerkleRoot:       tree.Root,
        IngotCount:       len(entries),
        MerkleHeight:     tree.Height,
        MintedAt:         time.Now().UTC(),
        TotalHashCount:   len(entries) * 3600,  // 1000 ingots × 3600 hashes each
        UniqueContracts:  contracts,
        UniqueDiggers:    diggers,
        UniqueRefineries: refineries,
    }
    
    return rt, nil
}

func generateRTID() string {
    return fmt.Sprintf("RT-%s-%03d", 
        time.Now().UTC().Format("20060102"), 
        time.Now().UnixNano()%1000)
}
```

**Ledger Storage** (in-memory for Phase 3, PostgreSQL in Phase 4):
```go
// internal/mint/ledger.go
package mint

import (
    "fmt"
    "sync"
    "mint/internal/models"
)

type LedgerEntry struct {
    RT           *models.Phase3RoboTorqUnit
    IngotHashes  []string              // 1000 branch hashes
    MerkleTree   *MerkleTree           // Full tree structure
    StoredAt     time.Time
}

type Ledger struct {
    entries map[string]*LedgerEntry  // unit_id → entry
    mu      sync.RWMutex
}

func NewLedger() *Ledger {
    return &Ledger{
        entries: make(map[string]*LedgerEntry),
    }
}

func (l *Ledger) StoreRT(rt *models.Phase3RoboTorqUnit, ingotEntries []IngotHashEntry) error {
    l.mu.Lock()
    defer l.mu.Unlock()
    
    // Extract hashes
    hashes := make([]string, len(ingotEntries))
    for i, e := range ingotEntries {
        hashes[i] = e.BranchHash
    }
    
    // Rebuild merkle tree (for verification)
    tree, err := BuildLevel2MerkleTree(hashes)
    if err != nil {
        return fmt.Errorf("failed to rebuild merkle tree: %w", err)
    }
    
    // Verify merkle root matches RT
    if tree.Root != rt.MerkleRoot {
        return fmt.Errorf("merkle root mismatch: expected %s, got %s", rt.MerkleRoot, tree.Root)
    }
    
    entry := &LedgerEntry{
        RT:          rt,
        IngotHashes: hashes,
        MerkleTree:  tree,
        StoredAt:    time.Now().UTC(),
    }
    
    l.entries[rt.UnitID] = entry
    return nil
}

func (l *Ledger) GetRT(unitID string) (*LedgerEntry, error) {
    l.mu.RLock()
    defer l.mu.RUnlock()
    
    entry, exists := l.entries[unitID]
    if !exists {
        return nil, fmt.Errorf("RT not found: %s", unitID)
    }
    return entry, nil
}

func (l *Ledger) Count() int {
    l.mu.RLock()
    defer l.mu.RUnlock()
    return len(l.entries)
}
```

**Success Criteria**:
- ✅ Assembler loops: GetIngotHashes(1000) → BuildMerkleTree() → StoreRT()
- ✅ Phase3RoboTorqUnit created with merkle_root (64-char hex)
- ✅ Ledger stores 1000 ingot hashes + merkle tree structure
- ✅ Assembly time <50ms (20ms merkle + 30ms overhead)
- ✅ Unique contracts/diggers/refineries aggregated correctly

---

### Milestone 5: DistoDam NATS Publisher ⏳ **TODO**
**Goal**: Publish minimal RT to DistoDam via NATS

**Tasks**:
- [ ] Create `DistoDamPublisher` component
- [ ] Publish Phase3RoboTorqUnit to NATS `distodam.units`
- [ ] Add retry logic (3 attempts, exponential backoff)
- [ ] Add publish success/failure metrics
- [ ] Create E2E test: Refinery → Mint → DistoDam (via NATS logs)

**Files**:
- `internal/mint/distodam_publisher.go` - NATS publisher
- `internal/mint/distodam_publisher_test.go` - Unit tests
- `test-phase3-milestone5-e2e.py` - Python E2E test

**Implementation**:
```go
// internal/mint/distodam_publisher.go
package mint

import (
    "encoding/json"
    "fmt"
    "log/slog"
    "time"
    "github.com/nats-io/nats.go"
    "mint/internal/models"
)

type DistoDamPublisher struct {
    natsConn *nats.Conn
    logger   *slog.Logger
    metrics  *PublisherMetrics
}

func NewDistoDamPublisher(nc *nats.Conn, logger *slog.Logger, metrics *PublisherMetrics) *DistoDamPublisher {
    return &DistoDamPublisher{
        natsConn: nc,
        logger:   logger,
        metrics:  metrics,
    }
}

func (dp *DistoDamPublisher) PublishRT(rt *models.Phase3RoboTorqUnit) error {
    data, err := json.Marshal(rt)
    if err != nil {
        return fmt.Errorf("failed to marshal RT: %w", err)
    }
    
    // Retry logic: 3 attempts with exponential backoff
    var lastErr error
    for attempt := 0; attempt < 3; attempt++ {
        if err := dp.natsConn.Publish("distodam.units", data); err == nil {
            dp.metrics.RTsPublished.Inc()
            dp.logger.Info("RT published to DistoDam",
                "unit_id", rt.UnitID,
                "merkle_root", rt.MerkleRoot[:16]+"...",
                "size_bytes", len(data))
            return nil
        } else {
            lastErr = err
            backoff := time.Duration(1<<uint(attempt)) * time.Second
            dp.logger.Warn("failed to publish RT, retrying",
                "attempt", attempt+1,
                "backoff_seconds", backoff.Seconds(),
                "error", err)
            time.Sleep(backoff)
        }
    }
    
    dp.metrics.PublishErrors.Inc()
    return fmt.Errorf("failed to publish RT after 3 attempts: %w", lastErr)
}
```

**E2E Test** (Python):
```python
# test-phase3-milestone5-e2e.py
"""
Phase 3 Milestone 5: Full pipeline E2E test
Refinery → Mint → DistoDam (via NATS)
"""
import asyncio
import json
import subprocess
import time
from nats.aio.client import Client as NATS

async def test_full_pipeline():
    print("🚀 Phase 3 Milestone 5: Full Pipeline E2E Test")
    
    # Connect to NATS
    nc = NATS()
    await nc.connect("nats://localhost:4222")
    
    # Subscribe to distodam.units (verify Mint publishes RT)
    rt_received = []
    
    async def rt_handler(msg):
        rt = json.loads(msg.data.decode())
        rt_received.append(rt)
        print(f"✅ DistoDam received RT: {rt['unit_id']}")
    
    await nc.subscribe("distodam.units", cb=rt_handler)
    
    # Publish 1000 test ingots to mint.ingots
    print("📤 Publishing 1000 Phase2Ingots to Mint...")
    for i in range(1000):
        ingot = {
            "id": f"ingot-milestone5-{i:04d}",
            "branch_hash": f"{i:064x}",  # Unique 64-char hex
            "hash_count": 3600,
            "merkle_height": 12,
            "contract_ids": [f"contract-{i % 10}"],  # 10 unique contracts
            "digger_ids": [f"digger-{i % 5}"],       # 5 unique diggers
            "refinery_id": "test-refinery-001",
            "timestamp": "2025-11-16T12:00:00Z"
        }
        await nc.publish("mint.ingots", json.dumps(ingot).encode())
        
        if (i + 1) % 100 == 0:
            print(f"  Published {i + 1}/1000 ingots...")
    
    await nc.flush()
    print("✅ All 1000 ingots published")
    
    # Wait for Mint to process
    print("⏳ Waiting 30 seconds for Mint to assemble RT...")
    await asyncio.sleep(30)
    
    # Verify RT received
    assert len(rt_received) >= 1, "Expected at least 1 RT published"
    
    rt = rt_received[0]
    print(f"\n✅ RT Successfully Assembled and Published!")
    print(f"  Unit ID: {rt['unit_id']}")
    print(f"  Merkle Root: {rt['merkle_root'][:16]}...")
    print(f"  Ingot Count: {rt['ingot_count']}")
    print(f"  Total Hash Count: {rt['total_hash_count']:,}")
    print(f"  Unique Contracts: {len(rt['unique_contracts'])}")
    print(f"  Unique Diggers: {len(rt['unique_diggers'])}")
    
    # Validate RT structure
    assert rt['ingot_count'] == 1000, f"Expected 1000 ingots, got {rt['ingot_count']}"
    assert rt['merkle_height'] == 10, f"Expected height 10, got {rt['merkle_height']}"
    assert rt['total_hash_count'] == 3_600_000, f"Expected 3.6M hashes, got {rt['total_hash_count']}"
    assert len(rt['merkle_root']) == 64, "Merkle root should be 64-char hex"
    
    print("\n🎉 PHASE 3 MILESTONE 5 COMPLETE - Full Pipeline Working!")
    
    await nc.close()

if __name__ == "__main__":
    asyncio.run(test_full_pipeline())
```

**Success Criteria**:
- ✅ RT published to NATS `distodam.units`
- ✅ Retry logic works on NATS failures
- ✅ Published RT is minimal (~500 bytes, not ~100KB)
- ✅ E2E test: 1000 ingots → 1 RT → DistoDam subscription receives it
- ✅ Metrics show successful publish

---

## 🎯 Phase 3 Success Criteria

**Target State**:
- ✅ Mint receives `Phase2Ingot` from NATS (hash-only, no units)
- ✅ Validates ingot structure (3600 hashes, merkle root)
- ✅ Accumulates 1000 ingot hashes in queue
- ✅ Builds Level 2 merkle tree (1000 branch hashes → merkle root)
- ✅ Creates `Phase3RoboTorqUnit` with merkle root (no ingots array)
- ✅ Stores full ledger entry (1000 hashes + merkle tree)
- ✅ Publishes minimal RT to DistoDam (~500 bytes vs ~100KB)
- ✅ E2E test: Refinery → Mint → DistoDam flow working

**Performance Targets**:
- Assembly time: <50ms per RT (20ms merkle + 30ms overhead)
- Ledger storage: ~50KB per RT (1000 hashes + merkle tree)
- Network broadcast: ~500 bytes per RT (merkle root + metadata)
- Throughput: 1 RT per minute (1000 ingots/min input rate)

**Test Validation**:
- Unit tests: 95%+ coverage
- Integration tests: Full pipeline validated
- E2E test: 1000 Phase2Ingots → 1 Phase3RT → DistoDam receives

---

## 📊 Data Size Comparison

### OLD Flow (Phase 1 - Deprecated)
```
1 RT = 1000 ingots × 100KB/ingot = 100 MB
Ledger storage: 100 MB per RT
Network broadcast: 100 MB per RT
```

### NEW Flow (Phase 3 - Hash-Only)
```
1 RT = 1000 ingot hashes × 64 bytes = 64 KB (hashes only)
       + merkle tree overhead = ~50 KB total
Ledger storage: 50 KB per RT (200× reduction!)
Network broadcast: 500 bytes per RT (200,000× reduction!)
```

**Bandwidth Savings**: **99.5% reduction** 🎉

---

## 🧪 Testing Strategy

### Unit Tests
- **IngotHashQueue**: Add/Get operations, blocking, concurrency
- **Level2Merkle**: 1000 hashes, determinism, odd counts, performance
- **Phase3RTAssembler**: Assembly logic, metadata aggregation
- **Ledger**: Store/retrieve, merkle verification
- **DistoDamPublisher**: Publish, retry logic, error handling

### Integration Tests
- Full pipeline: Phase2Ingot → Queue → Merkle → Ledger → Publish
- NATS failover scenarios
- Queue overflow handling

### E2E Tests
- **Milestone 1**: NATS → Mint (ingot receipt)
- **Milestone 5**: Refinery → Mint → DistoDam (full pipeline)

### Performance Benchmarks
```bash
go test -bench=. -benchmem ./internal/mint

# Expected results:
# BenchmarkLevel2Merkle_1000-8    50000   20000 ns/op   32768 B/op   10 allocs/op
# BenchmarkRTAssembly-8           10000   50000 ns/op   65536 B/op   50 allocs/op
```

---

## 🔧 Configuration

### Environment Variables
```bash
# NATS Connection
NATS_URL=nats://nats:4222

# Queue Settings
INGOT_HASH_QUEUE_SIZE=5000      # Capacity (5× batch size)

# Batch Settings
RT_BATCH_SIZE=1000               # Ingots per RT (fixed)

# Logging
LOG_LEVEL=info                   # debug|info|warn|error

# Metrics
METRICS_PORT=8081                # Prometheus /metrics endpoint
```

### Docker Compose
```yaml
services:
  mint:
    build: ./src/mint
    ports:
      - "8080:8080"
      - "8081:8081"  # Metrics
    environment:
      - NATS_URL=nats://nats:4222
      - INGOT_HASH_QUEUE_SIZE=5000
      - RT_BATCH_SIZE=1000
      - LOG_LEVEL=info
    depends_on:
      - nats
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://localhost:8080/health"]
      interval: 30s
```

---

## 📈 Prometheus Metrics

### Phase 3 Metrics
```
# Ingot Receipt
mint_phase2_ingots_received_total              Counter
mint_ingot_validation_errors_total{reason}     CounterVec
mint_ingot_queue_errors_total                  Counter
mint_ingot_hash_count                          Histogram

# RT Assembly
mint_phase3_rts_assembled_total                Counter
mint_rt_assembly_duration_seconds              Histogram
mint_rt_assembly_errors_total                  Counter

# Publishing
mint_rts_published_total                       Counter
mint_publish_errors_total                      Counter

# Queue
mint_ingot_hash_queue_depth                    Gauge
mint_ingot_hash_queue_utilization_percent      Gauge

# Ledger
mint_ledger_entries_total                      Gauge
mint_ledger_storage_bytes                      Gauge
```

---

## 🔮 Future Phases

### Phase 4: Cryptographic Signatures
- Sign RTs with Mint's SPHINCS+ key (archival security)
- DistoDam verifies signature before accepting
- Implement slashing for invalid signatures

### Phase 5: PostgreSQL Ledger
- Replace in-memory ledger with PostgreSQL
- Store merkle trees in JSONB columns
- Implement proof retrieval API

### Phase 6: Verification API
- `GET /verify/{unit_id}` - Retrieve RT proof
- `GET /verify/ingot/{branch_hash}` - Check ingot inclusion
- `GET /verify/jtu/{jtu_hash}` - Full chain verification (JTU → Ingot → RT)

### Phase 7: Horizontal Scaling
- Multiple Mint replicas with Raft consensus
- Leader election for RT assembly
- Failover support

---

## 📚 Key Files

```
src/mint/
├── cmd/mint/main.go                          # Entry point (wire Phase 3 components)
├── internal/
│   ├── models/
│   │   ├── phase2_ingot.go                   # Input data model
│   │   └── phase3_robotorq.go                # Output data model
│   └── mint/
│       ├── phase2_ingot_receiver.go          # NATS subscriber (Milestone 1)
│       ├── ingot_hash_queue.go               # Hash queue (Milestone 2)
│       ├── level2_merkle.go                  # Merkle tree builder (Milestone 3)
│       ├── phase3_rt_assembler.go            # RT assembly (Milestone 4)
│       ├── distodam_publisher.go             # NATS publisher (Milestone 5)
│       ├── ledger.go                         # In-memory ledger
│       └── *_test.go                         # Unit tests (95%+ coverage)
├── test-phase3-milestone1.py                 # E2E test (ingot receipt)
├── test-phase3-milestone5-e2e.py             # E2E test (full pipeline)
└── PHASE3_HASH_VALIDATION.md                 # This file
```

---

## 🎓 Key Takeaways

1. **Mint is a merkle aggregator**, not a data warehouse
2. **Two-level merkle tree**: 3600 JTU hashes → 1000 ingot hashes → 1 RT root
3. **Hash-only flow**: No full ingot data transferred (99.5% bandwidth savings)
4. **Ledger stores proofs**: Full merkle trees + hashes for verification
5. **Network broadcasts minimal data**: Only merkle root + metadata (~500 bytes)
6. **Deterministic merkle trees**: Same input → same root (verifiable)

---

## 🚀 Getting Started

### Step 1: Create Branch
```bash
git checkout feature/digger-refactor
git pull origin feature/digger-refactor
git checkout -b feature/proof-chain-phase3
```

### Step 2: Implement Milestone 1
```bash
# Create Phase2Ingot receiver
code src/mint/internal/mint/phase2_ingot_receiver.go

# Add unit tests
code src/mint/internal/mint/phase2_ingot_receiver_test.go

# Wire into main.go
code src/mint/cmd/mint/main.go

# Run tests
cd src/mint
go test ./internal/mint/phase2_ingot_receiver_test.go -v

# Create E2E test
code test-phase3-milestone1.py
python test-phase3-milestone1.py
```

### Step 3: Commit & Push
```bash
git add src/mint/internal/mint/phase2_ingot_receiver.go
git add src/mint/internal/mint/phase2_ingot_receiver_test.go
git add src/mint/cmd/mint/main.go
git add test-phase3-milestone1.py
git commit -m "feat(mint): Add Phase2Ingot NATS receiver (Phase 3 Milestone 1)

- Subscribe to mint.ingots subject
- Validate Phase2Ingot structure (branch_hash, hash_count, merkle_height)
- Log ingot receipt with structured fields
- Add Prometheus metrics for validation errors
- E2E test: Refinery → NATS → Mint

Tests:
- phase2_ingot_receiver_test.go: 100% coverage
- test-phase3-milestone1.py: E2E validation

Refs: PHASE3_HASH_VALIDATION.md Milestone 1"

git push origin feature/proof-chain-phase3
```

### Step 4: Repeat for Milestones 2-5
Follow the same pattern: implement → test → commit → push

---

**Next Step**: Start Milestone 1 - Phase2Ingot NATS receiver

*"Data flows down, proofs flow up, only hashes persist."* 🔐⚡💰
