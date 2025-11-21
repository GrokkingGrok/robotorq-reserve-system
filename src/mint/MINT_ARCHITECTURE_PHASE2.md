# Mint Service - Phase 2/3 Architecture Documentation

**Version**: 3.0 (Phase 2/3 Clean Architecture)  
**Last Updated**: November 15, 2025  
**Status**: ✅ Active Development

---

## 🎯 Mission

The **Mint** is the proof aggregation point in the RoboTorq currency creation pipeline. It receives **Phase2Ingots** (hash-only ingots from Refinery), builds merkle trees from batches of 1000 ingot hashes, assembles **Phase3RoboTorqUnits** with complete merkle proof chains, and publishes them to DistoDam for treasury management.

**Core Responsibility**: **1000 Phase2Ingots** (hashes only) → **1 Phase3RoboTorqUnit** with merkle root + proof chain

**Key Difference from Phase 1**: 
- No HTTP ingot server (Refinery now publishes directly to NATS)
- No dual-queue aggregation (single hash queue, deterministic batching)
- Hash-only architecture (3600-hash Phase2Ingots, not full unit data)
- Merkle tree proofs (enables verification without storing raw data)

---

## 📊 Data Flow

```
Refinery (NATS: mint.phase2.ingots)
    ↓ [Phase2Ingot: {ingot_id, hashes[3600], merkle_branch}]
    ↓
Phase2IngotReceiver (NATS subscriber)
    ↓
IngotHashQueue (stores 2000 ingot hashes, batch @ 1000)
    ↓
Level2MerkleBuilder (builds merkle tree from 1000 hashes)
    ↓ [Level2MerkleResult: {root_hash, proof_chain, timestamp}]
    ↓
Phase3RoboTorqUnitAssembler (creates Phase3RoboTorqUnit)
    ↓ [Phase3RoboTorqUnit: {unit_id, merkle_root, proof_chain, signature}]
    ↓
Phase3DistoDamPublisher (NATS: distodam.units)
    ↓
DistoDam (Treasury)
```

**Throughput**: ~300 hashes/sec sustained, ~1 Phase3RoboTorqUnit every 3-4 seconds

**Proof Chain**: Complete cryptographic verification from original token → joule → unit → ingot → merkle → RT

---

## 🏗️ Component Architecture

### 1. **Phase2IngotReceiver** - NATS Subscriber

**File**: `internal/mint/phase2_ingot_receiver.go`

**Responsibilities**:
- Subscribe to NATS topic `mint.phase2.ingots`
- Extract 3600 hashes from Phase2Ingot
- Push each hash to IngotHashQueue
- Track metrics for Prometheus

**Architecture**:
```
Phase2Ingot {
  ingot_id: "ingot-20251115-120000.123456"
  hashes: [                    # 3600 SHA256 hashes (each 32 bytes)
    "0521434e13fa9bddc71d777d689635f73b1d91df68b74abcbd97af58a051ff10",
    "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b",
    ...
  ]
  merkle_branch: {             # Proof that these hashes belong to original tokens
    level_0_hashes: [...],     # Hash of each JouleTorqUnit
    level_1_hash: "...",       # Refinery's merkle branch
    timestamp: 1731654000      
  }
}
```

**Key Methods**:
```go
type Phase2IngotReceiver struct {
    hashQueue   *IngotHashQueue
    metrics     *Phase2IngotReceiverMetrics
    logger      *slog.Logger
}

// handleNATSIngot processes Phase2Ingots from NATS
func (r *Phase2IngotReceiver) handleNATSIngot(msg *nats.Msg) {
    var ingot models.Phase2Ingot
    json.Unmarshal(msg.Data, &ingot)
    
    // Extract 3600 hashes from ingot
    for i, hash := range ingot.Hashes {
        hashItem := &HashItem{
            Hash:      hash,
            IngotID:   ingot.ID,
            Index:     i,
            Timestamp: ingot.MerkleBranch.Timestamp,
        }
        
        r.hashQueue.Add(hashItem)
        r.metrics.HashesReceivedTotal.Inc()
    }
    
    r.metrics.IngotsReceivedTotal.Inc()
    logger.Info("Phase2Ingot processed",
        "ingot_id", ingot.ID,
        "hashes", len(ingot.Hashes))
}

// Start begins NATS subscription
func (r *Phase2IngotReceiver) Start() error {
    r.natsConn.Subscribe("mint.phase2.ingots", r.handleNATSIngot)
    r.logger.Info("Phase2IngotReceiver started", "topic", "mint.phase2.ingots")
    return nil
}

// Stop closes NATS subscription
func (r *Phase2IngotReceiver) Stop() error {
    r.sub.Unsubscribe()
    return nil
}
```

**Metrics**:
```
phase2_ingot_receiver_ingots_received_total    # Counter: total Phase2Ingots received
phase2_ingot_receiver_hashes_received_total    # Counter: total individual hashes extracted
phase2_ingot_receiver_ingot_processing_latency # Histogram: time to extract 3600 hashes
phase2_ingot_receiver_errors_total             # Counter: JSON decode errors, etc
```

---

### 2. **IngotHashQueue** - Fixed-Size Hash Buffer

**File**: `internal/mint/ingot_hash_queue.go`

**Responsibilities**:
- Store up to 2000 individual hashes
- Signal Level2MerkleBuilder when 1000 hashes accumulated
- Thread-safe with sync.Cond for efficient waiting
- Metrics for queue utilization

**Architecture**:
```go
type IngotHashQueue struct {
    hashes        []*HashItem         # Ring buffer: 2000 hashes max
    capacity      int                 # 2000
    batchSize     int                 # 1000: trigger merkle build
    mu            sync.Mutex
    notFull       *sync.Cond           # Wait: queue full → can't add
    notEmpty      *sync.Cond           # Wait: queue empty → can't read
    readyForBatch *sync.Cond           # Signal: 1000 hashes ready
    head, tail    int                  # Ring buffer pointers
    size          int                  # Current count
    metrics       *IngotHashQueueMetrics
}

// Add appends hash to queue (blocking if full)
func (q *IngotHashQueue) Add(hash *HashItem) error {
    q.mu.Lock()
    defer q.mu.Unlock()
    
    for q.size >= q.capacity {
        q.notFull.Wait()  # Block until space available
    }
    
    q.hashes[q.tail] = hash
    q.tail = (q.tail + 1) % q.capacity
    q.size++
    
    // Signal when batch ready
    if q.size%q.batchSize == 0 {
        q.readyForBatch.Broadcast()
    }
    
    return nil
}

// GetBatch retrieves up to batchSize hashes (blocking if < 1000 available)
func (q *IngotHashQueue) GetBatch(ctx context.Context) ([]*HashItem, error) {
    q.mu.Lock()
    defer q.mu.Unlock()
    
    for q.size < q.batchSize {
        select {
        case <-ctx.Done():
            return nil, ctx.Err()
        default:
            q.readyForBatch.Wait()
        }
    }
    
    batch := make([]*HashItem, q.batchSize)
    for i := 0; i < q.batchSize; i++ {
        batch[i] = q.hashes[q.head]
        q.head = (q.head + 1) % q.capacity
        q.size--
    }
    
    q.notFull.Broadcast()  # Signal if producers waiting
    return batch, nil
}
```

**Configuration**:
- `QUEUE_CAPACITY=2000`: Max hashes in memory (66 KB: 2000 × 32 bytes)
- `BATCH_SIZE=1000`: Hashes per merkle tree (33 KB per batch)

**Metrics**:
```
ingot_hash_queue_hashes_total       # Counter: total hashes added
ingot_hash_queue_depth              # Gauge: current queue size
ingot_hash_queue_batches_ready      # Counter: batches ready for merkle build
ingot_hash_queue_add_latency        # Histogram: time to add hash
```

---

### 3. **Level2MerkleBuilder** - Merkle Tree Construction

**File**: `internal/mint/level2_merkle_builder.go`

**Responsibilities**:
- Build merkle tree from 1000 ingot hashes
- Compute merkle root hash
- Store complete proof chain (all internal nodes)
- Output Level2MerkleResult with signatures

**Architecture**:
```
Input: 1000 Phase2Ingot hashes
         ↓
       Level 0 (1000 leaf nodes) = ingot hashes [0..999]
         ↓
       Level 1 (500 nodes) = hash pairs: H(hash[0] || hash[1]), H(hash[2] || hash[3]), ...
         ↓
       Level 2 (250 nodes) = hash of level 1 pairs: H(L1[0] || L1[1]), ...
         ↓
       Level 3 (125 nodes)
         ↓
       ... (continue binary tree)
         ↓
       Level 10 (1 node) = MERKLE_ROOT
       
Proof Chain = [all 1875 internal nodes] stored in proof_archive
```

**Key Code**:
```go
type Level2MerkleBuilder struct {
    hashQueue    *IngotHashQueue
    logger       *slog.Logger
    metrics      *Level2MerkleMetrics
    proofArchive map[string][]byte  # merkleRoot → proof chain
}

// BuildMerkleTree constructs tree from 1000 hashes
func (b *Level2MerkleBuilder) BuildMerkleTree(ctx context.Context, hashes []string) (*Level2MerkleResult, error) {
    if len(hashes) != 1000 {
        return nil, fmt.Errorf("expected 1000 hashes, got %d", len(hashes))
    }
    
    // Create leaf level (copy input hashes)
    leaves := make([]string, 1000)
    copy(leaves, hashes)
    
    // Build tree levels
    proofChain := []string{} // All internal nodes
    currentLevel := leaves
    
    for len(currentLevel) > 1 {
        nextLevel := []string{}
        
        for i := 0; i < len(currentLevel); i += 2 {
            if i+1 < len(currentLevel) {
                // Hash pair
                combined := currentLevel[i] + currentLevel[i+1]
                nodeHash := sha256.Sum256([]byte(combined))
                nodeHashStr := hex.EncodeToString(nodeHash[:])
                
                nextLevel = append(nextLevel, nodeHashStr)
                proofChain = append(proofChain, nodeHashStr)
            } else {
                // Odd node, promote to next level
                nextLevel = append(nextLevel, currentLevel[i])
            }
        }
        
        currentLevel = nextLevel
    }
    
    merkleRoot := currentLevel[0]
    b.proofArchive[merkleRoot] = encodeProofChain(proofChain)
    
    return &Level2MerkleResult{
        MerkleRoot:   merkleRoot,
        ProofChain:   proofChain,
        LeafCount:    1000,
        Timestamp:    time.Now(),
    }, nil
}

// GetProofPath returns merkle path from leaf to root
func (b *Level2MerkleBuilder) GetProofPath(merkleRoot, leafHash string, leafIndex int) ([]string, error) {
    proofChain := b.proofArchive[merkleRoot]
    // ... reconstruct path from leaf_index through binary tree
    return path, nil
}
```

**Performance**:
- **Build Time**: ~2-5ms per merkle tree (1000 → 1 hash)
- **Proof Chain Size**: 1875 hashes × 32 bytes = 60 KB per merkle result
- **Throughput**: ~200-500 merkle trees/sec (batches every 3-5 seconds)

**Metrics**:
```
level2_merkle_builder_trees_built_total    # Counter
level2_merkle_builder_tree_build_latency   # Histogram: ~3-5ms
level2_merkle_builder_proofs_stored_total  # Counter: proof chains in archive
```

---

### 4. **Phase3RoboTorqUnitAssembler** - Unit Creation

**File**: `internal/mint/phase3_robotorq_unit_assembler.go`

**Responsibilities**:
- Read Level2MerkleResults from Level2MerkleBuilder
- Create Phase3RoboTorqUnit (includes merkle root, proof chain, signatures)
- Sign with SPHINCS+ (post-quantum cryptography)
- Push to output channel for publishing

**Architecture**:
```go
type Phase3RoboTorqUnitAssembler struct {
    logger          *slog.Logger
    metrics         *Phase3AssemblerMetrics
    merkleBuilder   *Level2MerkleBuilder
    unitChannel     chan *models.Phase3RoboTorqUnit
    privateKey      []byte  // SPHINCS+ private key
    publicKey       string  // SPHINCS+ public key (hex)
    proofCache      *ProofCache
    signatureArchive *SignatureArchive
}

// Start reads merkle results and creates RoboTorqUnits
func (a *Phase3RoboTorqUnitAssembler) Start(ctx context.Context) {
    for {
        select {
        case <-ctx.Done():
            a.logger.Info("Phase3RoboTorqUnitAssembler shutting down")
            return
        default:
            // Get merkle result from builder
            result := a.merkleBuilder.GetLatestResult(ctx)
            
            // Create Phase3RoboTorqUnit
            unit := &models.Phase3RoboTorqUnit{
                UnitID:        fmt.Sprintf("rt-%d-%s", time.Now().Unix(), result.MerkleRoot[:8]),
                MerkleRoot:    result.MerkleRoot,
                ProofChain:    result.ProofChain,
                LeafCount:     result.LeafCount,
                Timestamp:     result.Timestamp,
                AssemblyTime:  time.Now(),
                Signature:     a.signWithSPHINCSPlus(result.MerkleRoot),
                PublicKey:     a.publicKey,
            }
            
            a.metrics.UnitsAssembledTotal.Inc()
            a.unitChannel <- unit
        }
    }
}

// signWithSPHINCSPlus creates post-quantum signature
func (a *Phase3RoboTorqUnitAssembler) signWithSPHINCSPlus(message string) string {
    sig := oqs.Sign(a.privateKey, []byte(message))
    return hex.EncodeToString(sig)
}
```

**Data Structure** (Phase3RoboTorqUnit):
```go
type Phase3RoboTorqUnit struct {
    UnitID         string
    MerkleRoot     string        // 32-byte hash in hex
    ProofChain     []string      // ~1875 hashes (60 KB)
    LeafCount      int           // Always 1000
    Timestamp      time.Time     // Original Phase2Ingot timestamp
    AssemblyTime   time.Time     // When assembled
    Signature      string        // SPHINCS+ signature in hex
    PublicKey      string        // SPHINCS+ public key in hex
}
```

**Metrics**:
```
phase3_robotorq_unit_assembler_units_created_total      # Counter
phase3_robotorq_unit_assembler_unit_assembly_latency    # Histogram
phase3_robotorq_unit_assembler_signature_latency        # Histogram: ~10-20ms for SPHINCS+
```

---

### 5. **Phase3DistoDamPublisher** - NATS Publisher

**File**: `internal/mint/phase3_distodam_publisher.go`

**Responsibilities**:
- Read Phase3RoboTorqUnits from assembler channel
- Publish to NATS topic `distodam.units`
- Handle NATS errors and retries
- Track metrics

**Architecture**:
```go
type Phase3DistoDamPublisher struct {
    natsConn    *nats.Conn
    unitChannel <-chan *models.Phase3RoboTorqUnit
    logger      *slog.Logger
    metrics     *Phase3DistoDamPublisherMetrics
}

// Start publishes units to DistoDam
func (p *Phase3DistoDamPublisher) Start(ctx context.Context) {
    for {
        select {
        case <-ctx.Done():
            p.logger.Info("Phase3DistoDamPublisher shutting down, draining queue...")
            p.drainRemainingUnits(ctx)
            return
        case unit := <-p.unitChannel:
            if err := p.publishUnit(ctx, unit); err != nil {
                p.logger.Error("Failed to publish unit", "error", err, "unit_id", unit.UnitID)
                p.metrics.PublishErrorsTotal.Inc()
            } else {
                p.metrics.UnitsPublishedTotal.Inc()
            }
        }
    }
}

// publishUnit sends unit to DistoDam
func (p *Phase3DistoDamPublisher) publishUnit(ctx context.Context, unit *models.Phase3RoboTorqUnit) error {
    data, _ := json.Marshal(unit)
    return p.natsConn.Publish("distodam.units", data)
}

// drainRemainingUnits ensures all queued units are published before shutdown
func (p *Phase3DistoDamPublisher) drainRemainingUnits(ctx context.Context) {
    for {
        select {
        case unit := <-p.unitChannel:
            p.publishUnit(ctx, unit)
        case <-ctx.Done():
            return
        default:
            return
        }
    }
}
```

**NATS Configuration**:
- **Topic**: `distodam.units`
- **Message Format**: JSON Phase3RoboTorqUnit
- **Durability**: DistoDam maintains durable subscription

**Metrics**:
```
phase3_distodam_publisher_units_published_total         # Counter
phase3_distodam_publisher_publish_latency               # Histogram: 1-5ms per publish
phase3_distodam_publisher_publish_errors_total          # Counter: NATS errors
phase3_distodam_publisher_queue_depth                   # Gauge: pending units
```

---

### 6. **VerificationHandler** - HTTP API (Phase 5)

**File**: `internal/mint/verification_handler.go`

**Responsibilities**:
- Provide HTTP API to query merkle proofs
- Verify SPHINCS+ signatures
- Return complete proof chains
- Support Merkle path calculations

**API Endpoints**:
```
GET /health
Response: {"status": "healthy", "units_verified": 1523}

POST /verify-proof
Request: {
  "merkle_root": "0521434e13...",
  "leaf_hash": "1a2b3c4d5e...",
  "leaf_index": 42,
  "proof_chain": ["...", "...", ...]
}
Response: {
  "valid": true,
  "merkle_path": ["h1", "h2", "h3"],
  "verification_time_ms": 2.5
}

POST /verify-signature
Request: {
  "unit_id": "rt-1731654000-0521434e",
  "signature": "...",
  "public_key": "..."
}
Response: {
  "valid": true,
  "verification_time_ms": 15.3
}

GET /proof/{merkle_root}
Response: {
  "merkle_root": "0521434e13...",
  "proof_chain": ["...", "..."],
  "leaf_count": 1000,
  "timestamp": "2025-11-15T12:00:00Z"
}
```

---

## ⚙️ Configuration

**Environment Variables** (from `internal/config/config.go`):
```bash
MINT_HTTP_PORT=8080                  # Verification API port (Phase 5 only)
MINT_NATS_URL=nats://localhost:4222  # NATS connection
MINT_BATCH_SIZE=1000                 # Hashes per merkle tree
MINT_QUEUE_CAPACITY=2000             # Max hashes in queue
MINT_LOG_LEVEL=info                  # Log verbosity
```

---

## 🔄 Lifecycle

### Startup Sequence:
1. Load configuration from environment
2. Connect to NATS
3. Create IngotHashQueue (empty)
4. Create Level2MerkleBuilder (ready to build)
5. Create Phase3RoboTorqUnitAssembler (channel capacity: 10)
6. Create Phase3DistoDamPublisher (channel ready)
7. Create VerificationHandler (HTTP server)
8. Start Phase2IngotReceiver (begin NATS subscription)
9. Start Phase3RoboTorqUnitAssembler (polling merkle builder)
10. Start Phase3DistoDamPublisher (polling unit channel)
11. Start VerificationHandler (HTTP on :8081)

### Graceful Shutdown (on SIGINT/SIGTERM):
1. Stop Phase2IngotReceiver (close NATS subscription)
2. Stop VerificationHandler (close HTTP server)
3. Wait for Phase3RoboTorqUnitAssembler to drain
4. Wait for Phase3DistoDamPublisher to drain
5. Close NATS connection
6. Exit

### Emergency Shutdown:
1. Stop Phase2IngotReceiver (immediate)
2. Stop VerificationHandler (immediate)
3. Force-close NATS connection
4. Log remaining units in queue (for recovery)

---

## 📊 Performance Targets

| Metric | Target | Current |
|--------|--------|---------|
| Hash ingestion | 300/sec | TBD |
| Merkle build latency (1000→1) | <5ms | TBD |
| Unit assembly latency | <20ms | TBD |
| SPHINCS+ signature latency | 10-15ms | TBD |
| End-to-end unit latency | <50ms | TBD |
| Units/hour | ~900 (1/3.6s) | TBD |
| Memory footprint | <200MB | TBD |
| NATS publish success rate | >99.9% | TBD |

---

## 🧪 Testing Strategy

### Unit Tests:
- `phase2_ingot_receiver_test.go`: NATS subscription, hash extraction
- `ingot_hash_queue_test.go`: Ring buffer, batch signaling
- `level2_merkle_builder_test.go`: Tree construction, proof chains
- `phase3_robotorq_unit_assembler_test.go`: Unit creation, signatures
- `phase3_distodam_publisher_test.go`: NATS publishing

### Integration Tests:
- `integration_test.go`: Full Phase 2→3 pipeline
  - Send Phase2Ingot via NATS
  - Verify IngotHashQueue accumulates 3600 hashes
  - Verify Level2MerkleBuilder creates merkle tree
  - Verify Phase3RoboTorqUnit published to DistoDam

### E2E Tests:
- `test_digger_e2e.ps1`: Digger → Refinery → Mint → DistoDam
  - Execute contract (creates ore)
  - Verify Phase2Ingot received by Mint
  - Verify Phase3RoboTorqUnit published to DistoDam

---

## 🔍 Observability

### Prometheus Metrics (`:9090/metrics`):
- `phase2_ingot_receiver_*`: Ingot receipt rate, throughput
- `ingot_hash_queue_*`: Queue depth, batch ready signals
- `level2_merkle_builder_*`: Tree build rate, proof chain size
- `phase3_robotorq_unit_assembler_*`: Unit creation rate, signature latency
- `phase3_distodam_publisher_*`: Publish success rate, latency

### Structured Logging (JSON):
```json
{"msg": "Phase2Ingot processed", "ingot_id": "...", "hashes": 3600}
{"msg": "merkle tree built", "root_hash": "...", "latency_ms": 3.5}
{"msg": "Phase3RoboTorqUnit created", "unit_id": "...", "signature_latency_ms": 12.3}
{"msg": "unit published to DistoDam", "unit_id": "..."}
```

### Health Check:
```
GET /health
Response: {"status": "healthy", "queue_depth": 1234, "units_published_total": 5678}
```

---

## 📝 Phase 1 → Phase 2/3 Migration Summary

**REMOVED** (Phase 1 Components):
- ❌ IngotReceiver (HTTP server)
- ❌ IngotBuffer (100k-capacity ring buffer)
- ❌ BatchAggregator (time/count-based batching)
- ❌ BatchHasher / SimpleBatchHasher (Phase 1 hash function)
- ❌ MintEngine (old pipeline orchestrator)

**ADDED** (Phase 2/3 Components):
- ✅ Phase2IngotReceiver (NATS subscriber for hash-only ingots)
- ✅ IngotHashQueue (deterministic batching of 1000 hashes)
- ✅ Level2MerkleBuilder (complete merkle tree construction)
- ✅ Phase3RoboTorqUnitAssembler (RT unit creation with signatures)
- ✅ Phase3DistoDamPublisher (NATS publisher to DistoDam)
- ✅ VerificationHandler (HTTP API for proof verification)

**Benefits**:
1. **No HTTP Coupling**: Refinery publishes directly to NATS (cleaner architecture)
2. **Hash-Only Design**: Reduces data transmission (32 bytes vs 3600+ bytes per hash)
3. **Deterministic Batching**: 1000 hashes = 1 batch, no time-based flushing
4. **Complete Proofs**: Merkle chains enable verification without raw data retention
5. **Post-Quantum Ready**: SPHINCS+ signatures for future quantum resilience

---

## 🔗 Related Documentation

- [`REFINERY_ARCHITECTURE.md`](../refinery/REFINERY_ARCHITECTURE.md): Phase2Ingot format
- [`PROOF_CHAIN_ARCHITECTURE.md`](../../PROOF_CHAIN_ARCHITECTURE.md): Complete proof chain design
- [`README.md` Appendix O](../../README.md): RoboTorq data structures
- [`VAULT_IMPLEMENTATION_PLAN.md`](../../VAULT_IMPLEMENTATION_PLAN.md): Overall Phase 2/3 plan
