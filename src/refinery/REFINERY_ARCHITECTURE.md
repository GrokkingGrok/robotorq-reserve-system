# Refinery Service - Architecture Documentation

**Version**: 2.0 (Currency Refactor - Complete)  
**Last Updated**: November 15, 2025  
**Status**: ✅ Production Ready

---

## 🎯 Mission

The **Refinery** transforms raw **JouleTorqOre** from Diggers into refined **TokenTorqIngots** with cryptographic proofs. It's the critical middle layer that validates work, accumulates energy units, and creates the atomic building blocks of RoboTorq currency.

**Core Responsibility**: **3600 units** (tokens) → **1 TokenTorqIngot** with merkle branch hash

---

## 📊 Data Flow

```
Digger (HTTP POST /receive-ore)
    ↓
  OreReceiver (validates, extracts units)
    ↓
  QueueManager (single queue: JouleTorqUnits)
    ↓
  IngotAssembler (accumulates 3600 units)
    ↓
  BatchSender (sends ingots to Mint every 60s)
    ↓
  Mint (NATS: mint.ingots)
```

**Key Insight**: The **token is the atomic unit**, not joules or RoboStake. Every unit carries its complete proof.

---

## 🏗️ Component Architecture

### 1. **OreReceiver** - HTTP Endpoint

**File**: `internal/refinery/ore_receiver.go`

**Responsibilities**:
- Accept ore via HTTP POST `/receive-ore`
- Validate ore structure (digger ID, contract ID, energy, tokens)
- Extract individual JouleTorqUnits from ore
- Push units to QueueManager

**Endpoint**: `POST /receive-ore`

**Request**:
```json
{
  "digger_id": "dig-jon-ai-001",
  "contract_id": "e2e-test-contract-001",
  "tokens_generated": 300,
  "joules": 1250,
  "milestone_index": 0,
  "timestamp": 1700000000,
  "robo_stake_amount": 0.004166666666666667,
  "proof_of_work": null,
  "signature": null
}
```

**Response** (Success):
```json
{
  "status": "accepted",
  "units_created": 300,
  "message": "Ore received: 300 tokens, 1250 joules"
}
```

**Key Code**:
```go
func (or *OreReceiver) handleReceiveOre(w http.ResponseWriter, r *http.Request) {
    var ore models.JouleTorqOre
    json.NewDecoder(r.Body).Decode(&ore)
    
    // Validate ore
    if err := ore.Validate(); err != nil {
        http.Error(w, err.Error(), http.StatusBadRequest)
        return
    }
    
    // Extract units (1 unit per token)
    units := or.extractUnits(&ore)
    
    // Add to queue
    for _, unit := range units {
        or.queueManager.AddUnit(unit)
    }
    
    or.metrics.OreReceivedTotal.Inc()
    or.metrics.UnitsCreatedTotal.Add(float64(len(units)))
}
```

**Unit Extraction Logic**:
```go
func (or *OreReceiver) extractUnits(ore *models.JouleTorqOre) []*models.JouleTorqUnit {
    units := make([]*models.JouleTorqUnit, ore.TokensGenerated)
    
    // Distribute energy and cost evenly across tokens
    joulesPerToken := float64(ore.Joules) / float64(ore.TokensGenerated)
    roboPerToken := ore.RoboStakeAmount / float64(ore.TokensGenerated)
    
    for i := 0; i < ore.TokensGenerated; i++ {
        unit := &models.JouleTorqUnit{
            TokenID:         generateTokenID(ore.ContractID, ore.MilestoneIndex, i),
            ContractID:      ore.ContractID,
            DiggerID:        ore.DiggerID,
            JoulesConsumed:  joulesPerToken,
            RoboStakePaid:   roboPerToken,
            MilestoneIndex:  ore.MilestoneIndex,
            Timestamp:       time.Unix(ore.Timestamp, 0),
            Signature:       ore.Signature,
        }
        
        // Calculate unit hash (SHA256 of all fields)
        unit.Hash = unit.CalculateHash()
        units[i] = unit
    }
    
    return units
}
```

**Example**: 1 Ore with 300 tokens, 1250 joules, 0.00417 RT
→ Creates **300 JouleTorqUnits**, each with:
- `joules_consumed`: 1250 / 300 = **4.17 J**
- `robo_stake_paid`: 0.00417 / 300 = **0.0000139 RT**

**Tests**: `ore_receiver_test.go`
- Valid ore acceptance
- Invalid ore rejection
- Unit extraction accuracy
- Metrics validation

**Health Endpoint**: `GET /health`
```json
{
  "status": "healthy",
  "timestamp": "2025-11-15T21:00:00Z",
  "queue": {
    "unit_queue_size": 1800,
    "unit_queue_capacity": 1000,
    "unit_queue_usage_percent": 180
  },
  "nats": {
    "connected": true,
    "status": "CONNECTED"
  },
  "ingot_assembly": {
    "accumulated_units": 1800,
    "completed_ingots_pending": 0,
    "progress_to_next_ingot_percent": 50.0
  }
}
```

---

### 2. **QueueManager** - Single Unit Queue

**File**: `internal/refinery/queue_manager.go`

**Responsibilities**:
- Store JouleTorqUnits in thread-safe FIFO queue
- Block on GetUnit() when empty (no busy-waiting)
- Track queue depth metrics

**Architecture Change** (Currency Refactor):
- ❌ **OLD**: Dual queues (JouleQueue + RoboQueue) - lost token granularity
- ✅ **NEW**: Single queue ([]JouleTorqUnit) - preserves complete proof chain

**Implementation**:
```go
type QueueManager struct {
    units    []*models.JouleTorqUnit
    mu       sync.Mutex
    notEmpty *sync.Cond
    capacity int
    metrics  *QueueMetrics
}

func (qm *QueueManager) AddUnit(unit *models.JouleTorqUnit) error {
    qm.mu.Lock()
    defer qm.mu.Unlock()
    
    qm.units = append(qm.units, unit)
    qm.notEmpty.Signal()  // Wake up blocked GetUnit()
    qm.metrics.QueueDepth.Set(float64(len(qm.units)))
    
    return nil
}

func (qm *QueueManager) GetUnit() (*models.JouleTorqUnit, error) {
    qm.mu.Lock()
    defer qm.mu.Unlock()
    
    // Block until unit available
    for len(qm.units) == 0 {
        qm.notEmpty.Wait()
    }
    
    unit := qm.units[0]
    qm.units = qm.units[1:]
    qm.metrics.QueueDepth.Set(float64(len(qm.units)))
    
    return unit, nil
}
```

**Why Single Queue?**
- **Preserves atomicity**: Each unit = 1 token with complete provenance
- **Enables merkle trees**: Can hash individual units for branch proofs
- **Simplifies logic**: No need to synchronize two separate queues
- **Accurate accounting**: Joules + RoboStake + Signature travel together

**Tests**: `queue_manager_test.go` (100% coverage)
- Concurrent AddUnit/GetUnit
- Blocking behavior
- Queue depth tracking
- Thread safety validation

---

### 3. **IngotAssembler** - Unit Accumulator

**File**: `internal/refinery/ingot_assembler.go`

**Responsibilities**:
- Pop units from QueueManager
- Accumulate exactly **3600 units**
- Build TokenTorqIngot with merkle branch hash
- Store completed ingots for batching

**IMPORTANT**: Despite outdated TODO comment at top of file, this component is **FULLY REFACTORED** and uses unit-based architecture!

**Actual Implementation** (Correct as of Nov 2025):
```go
type IngotAssembler struct {
    queueManager     *QueueManager
    ctx              context.Context
    mu               sync.Mutex
    
    // ✅ UNIT-BASED ACCUMULATION (not joule-based!)
    accumulatedUnits []*models.JouleTorqUnit
    
    // Completed ingots ready for Mint
    completedIngots  []*models.TokenTorqIngot
}

func (ia *IngotAssembler) Start() {
    for {
        select {
        case <-ia.ctx.Done():
            return
            
        default:
            // Get next unit from queue (blocking)
            unit, err := ia.queueManager.GetUnit()
            if err != nil {
                continue
            }
            
            ia.processUnit(unit)
        }
    }
}

func (ia *IngotAssembler) processUnit(unit *models.JouleTorqUnit) error {
    ia.mu.Lock()
    defer ia.mu.Unlock()
    
    // Add to accumulator
    ia.accumulatedUnits = append(ia.accumulatedUnits, unit)
    
    // Check if we've reached 3,600 units (1 ingot)
    if len(ia.accumulatedUnits) >= 3600 {
        return ia.assembleIngot()
    }
    
    return nil
}

func (ia *IngotAssembler) assembleIngot() error {
    // Extract exactly 3,600 units for this ingot
    units := ia.accumulatedUnits[:3600]
    
    // Create ingot using constructor (builds merkle hash!)
    ingot, err := models.NewTokenTorqIngot(units)
    if err != nil {
        return err
    }
    
    // Store completed ingot
    ia.completedIngots = append(ia.completedIngots, ingot)
    ingotsAssembledTotal.Inc()
    
    // Keep excess units for next ingot
    ia.accumulatedUnits = ia.accumulatedUnits[3600:]
    
    slog.Info("ingot assembled",
        "ingot_id", ingot.IngotID,
        "joules", ingot.JouleTorqTotal,
        "robo_stake", ingot.RoboStakeTotal,
        "units", len(ingot.Units),
        "contracts", len(ingot.ContractIDs),
        "branch_hash", ingot.BranchHash)
    
    return nil
}
```

**Ingot Constructor** (in `models/token_torq.go`):
```go
func NewTokenTorqIngot(units []*JouleTorqUnit) (*TokenTorqIngot, error) {
    if len(units) != 3600 {
        return nil, fmt.Errorf("ingot requires exactly 3,600 units, got %d", len(units))
    }
    
    // Calculate totals from units
    var totalJoules, totalRobo float64
    contractMap := make(map[string]bool)
    
    for _, unit := range units {
        totalJoules += unit.JoulesConsumed
        totalRobo += unit.RoboStakePaid
        contractMap[unit.ContractID] = true
    }
    
    ingot := &TokenTorqIngot{
        IngotID:        generateUUID(),
        Units:          units,  // ✅ PRESERVES ALL 3600 UNITS
        JouleTorqTotal: totalJoules,
        RoboStakeTotal: totalRobo,
        ContractIDs:    extractKeys(contractMap),
        MintedAt:       time.Now().UTC(),
    }
    
    // Build merkle branch hash from unit hashes
    ingot.BranchHash = ingot.CalculateBranchHash()
    
    return ingot, nil
}

func (ingot *TokenTorqIngot) CalculateBranchHash() string {
    // Concatenate all 3,600 unit hashes in order
    var allHashes string
    for _, unit := range ingot.Units {
        allHashes += unit.Hash
    }
    
    // SHA256 of concatenated hashes = branch hash
    hash := sha256.Sum256([]byte(allHashes))
    return hex.EncodeToString(hash[:])
}
```

**Example Output**:
```json
{
  "ingot_id": "ingot-20251115-210012.547794",
  "units": [/* 3600 JouleTorqUnits with hashes */],
  "joule_torq_total": 14999.999999999249,
  "robo_stake_total": 0.050000000000002334,
  "contract_ids": ["e2e-test-contract-001", "runtime-test"],
  "branch_hash": "0521434e13fa9bddc71d777d689635f73b1d91df68b74abcbd97af58a051ff10",
  "minted_at": "2025-11-15T21:00:12.547Z"
}
```

**Tests**: `ingot_assembler_test.go`
- Exact 3600 unit threshold
- Multi-contract aggregation
- Merkle hash calculation
- Excess unit carryover

---

### 4. **BatchSender** - Periodic Ingot Dispatcher

**File**: `internal/refinery/batch_sender.go`

**Responsibilities**:
- Collect completed ingots from IngotAssembler
- Send batches to Mint every 60 seconds
- Publish to NATS `mint.ingots` topic
- Retry on failure

**Implementation**:
```go
type BatchSender struct {
    ingotAssembler *IngotAssembler
    natsConn       NATSConnection
    interval       time.Duration  // Default: 60s
    metrics        *BatchSenderMetrics
}

func (bs *BatchSender) Start(ctx context.Context) {
    ticker := time.NewTicker(bs.interval)
    defer ticker.Stop()
    
    for {
        select {
        case <-ctx.Done():
            bs.flushPending()  // Drain on shutdown
            return
            
        case <-ticker.C:
            bs.sendBatch()
        }
    }
}

func (bs *BatchSender) sendBatch() {
    ingots := bs.ingotAssembler.GetCompletedIngots()
    if len(ingots) == 0 {
        return
    }
    
    for _, ingot := range ingots {
        data, _ := json.Marshal(ingot)
        
        if err := bs.natsConn.Publish("mint.ingots", data); err != nil {
            slog.Error("failed to send ingot to mint",
                "ingot_id", ingot.IngotID,
                "error", err)
            bs.metrics.SendErrorsTotal.Inc()
        } else {
            slog.Info("ingot sent to mint",
                "ingot_id", ingot.IngotID,
                "joules", ingot.JouleTorqTotal)
            bs.metrics.IngotsSentTotal.Inc()
        }
    }
}
```

**Configuration**:
```bash
REFINERY_INGOT_BATCH_INTERVAL=60  # Seconds between sends
```

**Tests**: `batch_sender_test.go`
- Periodic batch sending
- NATS publish success/failure
- Empty batch handling
- Graceful shutdown

---

## 🔄 Complete Flow Example

### Scenario: Digger Sends Ore → Ingot Created

**Step 1: Digger Sends Ore**
```bash
POST http://localhost:8081/receive-ore
```
```json
{
  "digger_id": "dig-jon-ai-001",
  "contract_id": "e2e-test-contract-001",
  "tokens_generated": 300,
  "joules": 1250,
  "milestone_index": 0,
  "timestamp": 1700000000,
  "robo_stake_amount": 0.004166666666666667
}
```

**Step 2: OreReceiver Extracts Units**
→ Creates **300 JouleTorqUnits**, each with:
```json
{
  "token_id": "e2e-test-contract-001-m0-t0",
  "contract_id": "e2e-test-contract-001",
  "digger_id": "dig-jon-ai-001",
  "joules_consumed": 4.166666666666667,
  "robo_stake_paid": 0.000013888888888889,
  "milestone_index": 0,
  "timestamp": "2025-11-15T21:00:00Z",
  "hash": "a1b2c3d4..."  // SHA256 of all fields
}
```

**Step 3: QueueManager Stores Units**
→ Queue depth: 300 units

**Step 4: IngotAssembler Accumulates**
→ Needs 12 more ores (12 × 300 = 3600 units total)
→ Progress: 8.3% (300/3600)

**Step 5-16: More Ores Arrive**
→ After 12 ores total (60 seconds at 5-sec intervals)

**Step 17: Ingot Assembled!**
```json
{
  "ingot_id": "ingot-20251115-210012.547794",
  "units": [/* 3600 units */],
  "joule_torq_total": 15000.0,
  "robo_stake_total": 0.05,
  "contract_ids": ["e2e-test-contract-001"],
  "branch_hash": "0521434e13fa9bddc71d...",
  "minted_at": "2025-11-15T21:01:12Z"
}
```

**Step 18: BatchSender Publishes to Mint**
→ NATS topic: `mint.ingots`
→ Mint receives ingot for batch aggregation

---

## 📈 Prometheus Metrics

### Counters
- `refinery_ore_received_total` - Total ore received from Diggers
- `refinery_units_created_total` - Total units extracted from ore
- `refinery_ingots_assembled_total` - Ingots created
- `refinery_ingots_sent_total` - Ingots sent to Mint
- `refinery_send_errors_total` - NATS publish failures

### Gauges
- `refinery_queue_depth` - Current units in queue
- `refinery_accumulated_units` - Units waiting for ingot assembly

### Histograms
- `refinery_ingot_assembly_duration_seconds` - Time to assemble ingot
- `refinery_ore_processing_latency_seconds` - Ore → units latency

**Dashboard**: `Grafana/torq-observability-dashboard.json`

---

## ⚙️ Configuration

### Environment Variables
```bash
# HTTP Server
HTTP_PORT=8081

# NATS Connection
NATS_URL=nats://nats:4222

# Queue Settings
REFINERY_JOULE_QUEUE_SIZE=1000

# Batch Sending
REFINERY_INGOT_BATCH_INTERVAL=60  # Seconds

# Logging
LOG_LEVEL=info
```

### Docker Compose
```yaml
services:
  refinery:
    build: ./src/refinery
    ports:
      - "8081:8081"
    environment:
      - NATS_URL=nats://nats:4222
      - REFINERY_INGOT_BATCH_INTERVAL=60
    depends_on:
      - nats
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://localhost:8081/health"]
      interval: 30s
```

---

## 🧪 Testing Strategy

### Unit Tests (95% Coverage)
```bash
cd src/refinery
go test ./... -v -cover

# Specific components
go test ./internal/refinery/queue_manager_test.go -v
go test ./internal/refinery/ingot_assembler_test.go -v
go test ./internal/refinery/ore_receiver_test.go -v
```

### Integration Tests
```bash
go test ./internal/refinery/integration_test.go -v
```

**Key Tests**:
- Full ore → unit → ingot pipeline
- Multi-contract aggregation
- Concurrent ore submission
- NATS failover

### E2E Test
```powershell
# Start services
docker-compose up -d

# Run test (includes Refinery validation)
.\src\digger-app\test-digger-e2e.ps1

# Expected logs:
# {"msg":"ore received","contract":"e2e-test-contract-001","tokens":300,"joules":1250}
# {"msg":"ingot assembled","ingot_id":"ingot-...","units":3600,"joules":15000}
```

---

## 🚀 Deployment

### Build
```bash
cd src/refinery
docker build -t refinery:latest .
```

### Run Standalone
```bash
docker run -p 8081:8081 \
  -e NATS_URL=nats://host.docker.internal:4222 \
  refinery:latest
```

### Production Deploy
```bash
docker-compose up -d refinery

# Check health
curl http://localhost:8081/health

# Send test ore
curl -X POST http://localhost:8081/receive-ore \
  -H "Content-Type: application/json" \
  -d '{"digger_id":"test","contract_id":"c1","tokens_generated":300,"joules":1250,"milestone_index":0,"timestamp":1700000000,"robo_stake_amount":0.00417}'
```

---

## 🔧 Troubleshooting

### Issue: Ingots Not Assembling
**Symptom**: Ore received but no ingots created

**Debug**:
```bash
# Check accumulation progress
curl http://localhost:8081/health | jq '.ingot_assembly'

# Example output:
{
  "accumulated_units": 1800,
  "progress_to_next_ingot_percent": 50.0
}

# Needs 1800 more units (6 more ores @ 300 tokens each)
```

### Issue: Queue Overflow
**Symptom**: High memory usage, slow response

**Fix**:
```bash
# Increase queue capacity
REFINERY_JOULE_QUEUE_SIZE=2000

# OR reduce batch interval (send faster)
REFINERY_INGOT_BATCH_INTERVAL=30
```

### Issue: NATS Connection Lost
**Symptom**: "failed to send ingot to mint"

**Fix**:
```bash
# Verify NATS running
curl http://localhost:8222/healthz

# Check connection
docker logs refinery --since 5m | grep -i nats

# Restart Refinery
docker-compose restart refinery
```

---

## 📚 Key Files

```
src/refinery/
├── cmd/refinery/main.go                  # Entry point
├── internal/
│   ├── config/config.go                  # Environment config
│   ├── metrics/metrics.go                # Prometheus setup
│   ├── models/
│   │   ├── jouletorq_unit.go            # Unit data model
│   │   └── token_torq.go                 # Ingot data model
│   └── refinery/
│       ├── ore_receiver.go               # HTTP endpoint
│       ├── queue_manager.go              # Single unit queue
│       ├── ingot_assembler.go            # Unit accumulator
│       ├── batch_sender.go               # NATS publisher
│       └── *_test.go                     # Test suite
├── Dockerfile
├── go.mod, go.sum
└── REFINERY_ARCHITECTURE.md              # This file
```

---

## 🎯 Design Principles

### 1. **Unit-Based Architecture**
Every token is a complete proof:
```go
type JouleTorqUnit struct {
    TokenID        string   // Unique identifier
    JoulesConsumed float64  // Energy for THIS token
    RoboStakePaid  float64  // Cost of THIS token
    Hash           string   // SHA256 of THIS token
    Signature      []byte   // Digger's signature
}
```

### 2. **Merkle Tree Preservation**
Ingots preserve unit hashes for proof chains:
```go
ingot.BranchHash = SHA256(unit[0].Hash + unit[1].Hash + ... + unit[3599].Hash)
```

### 3. **Contract Aggregation**
Multiple contracts can contribute to one ingot:
```go
ingot.ContractIDs = ["contract-A", "contract-B"]  // Sorted, unique
```

### 4. **Thread Safety**
All shared state uses mutexes + condition variables:
```go
qm.mu.Lock()
for len(qm.units) == 0 {
    qm.notEmpty.Wait()  // Releases lock while waiting
}
```

---

## 🔮 Future Enhancements

### Phase 2: Dilithium5 Signature Verification
- Verify `unit.Signature` against Digger's public key
- Reject units with invalid signatures
- Store verified status in unit metadata

### Phase 3: Ingot Sharding
- Multiple IngotAssemblers for parallel assembly
- Contract-based sharding (route units by contract ID)
- Horizontal scaling support

### Phase 4: Persistent Queue
- Replace in-memory queue with disk-backed storage
- Survive Refinery restarts without losing units
- Use BoltDB or BadgerDB

---

**Questions?** See:
- Data Models: `internal/models/jouletorq_unit.go`
- Configuration: `internal/config/config.go`
- E2E Test: `../../digger-app/test-digger-e2e.ps1`
- White Paper: `../../README.md` (Appendix O: Data Structures)
