# RoboTorq Network - AI Agent Instructions

*Last Updated: November 15, 2025*

---

## 🎯 Project Mission

**RoboTorq** is a physics-based monetary system where robotic labor creates measurable value. This is **NOT** a cryptocurrency—it's a NATS-based distributed system with cryptographic proofs that turns energy + computation + time into currency.

**Core Principle**: `1 RoboTorq = 1 kWh × 3600 tokens/sec × 1 hour` of verified robotic work.

The network transforms: `Energy (Joules) → JouleTorq → TokenTorq → RoboTorq → Physical Currency`

---

## 📚 Essential Reading

Before coding, understand the economics:
- **README.md** (5,745 lines): Complete economic model, formulas, and philosophy
- **BRANCHING.md**: Git workflow (`v0` baseline, `feature/*` branches)
- Service-specific `ARCHITECTURE.md` files (Mint, Refinery, Trust, etc.)

**Key Concept**: The white paper IS the spec. When in doubt, reference `README.md` appendices for data structures, formulas, and workflows.

---

## 🏗️ Architecture Overview

### Service Boundaries

```
┌─────────────────────────────────────────────────────────────────┐
│                    ROBOTORQ NETWORK                              │
│                                                                  │
│  Digger (executor)                                              │
│    ↓ JouleTorqOre (proofs of work)                              │
│  Refinery (assembler)                                           │
│    ↓ TokenTorqIngot (3600J batches)                             │
│  Mint (batcher)                                                 │
│    ↓ RoboTorq (finalized currency)                              │
│  DistoDam (distributor)                                         │
│    ↓ Universal Basic Dividend (UBD streams)                     │
│  Trust (contract evaluator)                                     │
│    ↔ BidNet (contract gateway - future)                         │
│  Wallet (user interface)                                        │
│    ↔ Printer (physical RT creator)                              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Communication Backbone: NATS (NOT Blockchain!)

**Critical**: This is a **message-passing system**, not blockchain/crypto.
- **No chain**: Just NATS pub/sub for real-time messaging
- **No mining**: Value is minted based on verified work
- **No gas fees**: Transaction fees fund network operations (demurrage)

**NATS Topics Pattern**:
```
mint.ingots         → Refinery → Mint
mint.batches        → Mint → DistoDam
contracts.pending   → Trust → BidNet → DistoDam
contracts.funded    → DistoDam → Trust → Executor
wallet.transfers    → Wallet ↔ Wallet
printer.burn        → Wallet → Printer (off-grid)
```

**NATS Client Pattern** (see `src/natsx/`):
```go
import "b2b/natsx"

nc, err := natsx.New(os.Getenv("NATS_URL"))
defer nc.Close()

// Publish
nc.PublishJSON("topic", data)

// Subscribe
nc.Subscribe("topic", func(msg *nats.Msg) { /* handle */ })
```

---

## 🔧 Development Patterns

### Project Structure (Go Services)

Standard layout for all Go services:
```
service/
├── cmd/
│   └── service/
│       └── main.go           # Entry point, wire components
├── internal/
│   ├── config/               # Environment config, validation
│   ├── metrics/              # Prometheus metrics
│   └── {service}/            # Core business logic
│       ├── component.go
│       └── component_test.go
├── Dockerfile                # Multi-stage Alpine build
├── go.mod, go.sum
├── {SERVICE}_ARCHITECTURE.md # Component design, data flows
└── TESTING_PLAN.md           # Test strategy, coverage goals
```

### Configuration Pattern

**Environment-first** (12-factor app):
```go
// internal/config/config.go
type Config struct {
    HTTPPort         string        `default:"8080"`
    NatsURL          string        `default:"nats://nats:4222"`
    BatchSize        int           `default:"1000"`
    FlushInterval    time.Duration `default:"60s"`
    BufferCapacity   int           `default:"100000"`
    LogLevel         string        `default:"info"`
}

func Load() (*Config, error) {
    // Load from env, apply defaults, validate
}

func (c *Config) Validate() error {
    // Business rule checks
}
```

**Docker Compose Override**:
```yaml
services:
  mint:
    environment:
      - BATCH_SIZE=500       # Override for testing
      - LOG_LEVEL=debug
```

### Logging Standards

**Structured JSON logs** (zap or slog):
```go
logger.Info("batch_sent",
    "batch_id", batch.ID,
    "ingot_count", len(batch.Ingots),
    "total_joules", batch.TotalJoules,
    "hash", batch.Hash)
```

**Never** use `fmt.Println()` in services. Logs are parsed by Prometheus/Grafana.

### Metrics Pattern (Prometheus)

Every service exposes `/metrics`:
```go
// internal/metrics/metrics.go
type Metrics struct {
    IngotsReceivedTotal  prometheus.Counter
    BatchesCreatedTotal  prometheus.Counter
    ProcessingLatency    prometheus.Histogram
    BufferUtilization    prometheus.Gauge
}

// Instrument business logic
m.IngotsReceivedTotal.Inc()
m.ProcessingLatency.Observe(latency.Seconds())
```

**Standard Metrics**:
- `{service}_requests_total` - Counter
- `{service}_errors_total` - Counter  
- `{service}_processing_latency_seconds` - Histogram (buckets: 0.001, 0.01, 0.1, 1.0)
- `{service}_queue_depth` - Gauge

See `prometheus.yml` for scrape config.

### Error Handling Philosophy

**Fail-fast validation, graceful degradation, explicit retries**:
```go
// Validate inputs immediately
if ingot.Joules != 3600 {
    return fmt.Errorf("invalid joules: expected 3600, got %d", ingot.Joules)
}

// Retry with exponential backoff (NATS publishing)
for attempt := 0; attempt <= retries; attempt++ {
    if err := nc.Publish(topic, data); err == nil {
        return nil
    }
    time.Sleep(backoff * time.Duration(1<<uint(attempt)))
}

// Graceful shutdown: drain queues before exiting
ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt)
defer cancel()
```

---

## 🧪 Testing Requirements

### Coverage Targets

- **Unit tests**: 95%+ coverage (see `TESTING_PLAN.md` per service)
- **Integration tests**: E2E flows with embedded NATS server
- **PowerShell E2E**: `test-e2e-flow.ps1` - full pipeline test

### Test Patterns

**Unit Test Example** (Mint's `IngotBuffer`):
```go
func TestIngotBuffer_PushPop(t *testing.T) {
    buffer := NewIngotBuffer(10)
    
    ingot := &TokenTorqIngot{ID: "test-1"}
    assert.NoError(t, buffer.Push(ingot))
    
    popped, err := buffer.Pop(context.Background())
    assert.NoError(t, err)
    assert.Equal(t, "test-1", popped.ID)
}
```

**Integration Test Pattern**:
```go
func TestMint_E2E(t *testing.T) {
    // Start embedded NATS
    natsServer := natstest.RunServer(&natstest.DefaultTestOptions)
    defer natsServer.Shutdown()
    
    // Initialize components
    mint := NewMintService(config)
    mint.Start(ctx)
    defer mint.Shutdown()
    
    // Publish test data
    publishIngot(nc, ingot)
    
    // Verify output
    select {
    case batch := <-batchChannel:
        assert.Equal(t, 1000, len(batch.Ingots))
    case <-time.After(65 * time.Second):
        t.Fatal("timeout")
    }
}
```

**PowerShell E2E** (see `test-e2e-flow.ps1`):
```powershell
# Verify services running
Invoke-WebRequest "http://localhost:8080/health" -UseBasicParsing

# Send test data
$ore = @{ joules=900; tokens=60 } | ConvertTo-Json
Invoke-RestMethod "http://localhost:8081/receive-ore" -Method Post -Body $ore

# Check logs for success
docker logs mint --since 10s | Select-String "batch sent"
```

### Running Tests

```bash
# Unit tests
cd src/mint
go test ./... -v -cover

# Integration tests
go test ./... -run Integration -v

# Benchmarks
go test -bench=. -benchmem ./internal/mint

# E2E (requires Docker Compose up)
pwsh test-e2e-flow.ps1
```

---

## 🚀 Build & Deployment

### Docker Build Pattern

**Multi-stage Alpine** (see `src/mint/Dockerfile`):
```dockerfile
FROM golang:1.24-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -o service ./cmd/service

FROM alpine:3.20
RUN apk --no-cache add ca-certificates
COPY --from=builder /app/service .
HEALTHCHECK --interval=30s CMD wget -qO- http://localhost:8080/health || exit 1
CMD ["./service"]
```

### Docker Compose Workflow

```bash
# Start all services
docker-compose up -d

# View logs
docker logs -f robotorq-network-mint-1

# Restart single service
docker-compose restart mint

# Check health
docker-compose ps
```

**Health Check Pattern**:
```go
// All services expose GET /health
http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
    status := map[string]interface{}{
        "status": "healthy",
        "buffer_depth": buffer.Len(),
        "uptime_seconds": time.Since(startTime).Seconds(),
    }
    json.NewEncoder(w).Encode(status)
})
```

### Prometheus/Grafana Monitoring

**Access**:
- Prometheus: `http://localhost:9090`
- Grafana: `http://localhost:3030` (admin/admin)

**Query Examples**:
```promql
# Mint throughput
rate(mint_batches_created_total[1m])

# Refinery ingot assembly rate
rate(refinery_ingots_assembled_total[5m])

# DistoDam reservoir level
distodam_reservoir_rt
```

**Pre-built Dashboard**: `Grafana/torq-observability-dashboard.json`

---

## 📐 Data Structures (Critical!)

### JouleTorqOre (Digger → Refinery)

```go
type JouleTorqOre struct {
    DiggerID        string    `json:"digger_id"`       // Executor ID
    ContractID      string    `json:"contract_id"`     // BRLA reference
    TokensGenerated int       `json:"tokens_generated"` // AI tokens processed
    Joules          int       `json:"joules"`          // Energy expended (usually 900J)
    MilestoneIndex  int       `json:"milestone_index"` // Progress tracker
    Timestamp       int64     `json:"timestamp"`       // Unix seconds
    RoboStakeAmount float64   `json:"robo_stake_amount"` // RT value
    ProofOfWork     *string   `json:"proof_of_work"`   // Cryptographic proof
    Signature       *string   `json:"signature"`       // Digital signature
}
```

**Validation**: See `README.md` Appendix O for complete rules.

### TokenTorqIngot (Refinery → Mint)

```go
type TokenTorqIngot struct {
    ID              string             `json:"id"`              // UUID
    ContractID      string             `json:"contract_id"`     
    AccumulatedJoules int              `json:"accumulated_joules"` // Must = 3600
    TotalTokens     int                `json:"total_tokens"`    
    RoboStakeAmount float64            `json:"robo_stake_amount"`
    Ores            []JouleTorqOre     `json:"ores"`            // Source ores (4 @ 900J)
    CreatedAt       time.Time          `json:"created_at"`      
    Hash            string             `json:"hash"`            // SHA256(ores)
}
```

**Invariant**: `AccumulatedJoules == 3600` (enforced by Mint validation).

### RoboTorq Batch (Mint → DistoDam)

```go
type RoboTorqBatch struct {
    BatchID         string             `json:"batch_id"`        // UUID
    Ingots          []TokenTorqIngot   `json:"ingots"`          // 1000 ingots
    TotalJoules     int                `json:"total_joules"`    // Sum(ingots)
    TotalRoboStake  float64            `json:"total_robo_stake"`
    Hash            string             `json:"hash"`            // SHA256(ingot hashes)
    Timestamp       time.Time          `json:"timestamp"`       
}
```

### Contract (Trust ↔ BidNet ↔ DistoDam)

```go
type Contract struct {
    ID            string     `json:"id"`              // UUID
    Type          string     `json:"type"`            // "production" | "currency_exchange"
    OpportunityID string     `json:"opportunity_id"`  
    Builder       string     `json:"builder"`         // Enterprise ID
    DiggerURL     string     `json:"digger_url"`      // Execution endpoint
    RoboStake     float64    `json:"robo_stake"`      // RT required
    ROI           float64    `json:"roi"`             // Expected return %
    Torq          int        `json:"torq"`            // Total torque
    Status        string     `json:"status"`          // "pending" → "approved" → "funded" → "executed"
    CreatedAt     time.Time  `json:"created_at"`      
    FundedAt      *time.Time `json:"funded_at,omitempty"`
    ExecutedAt    *time.Time `json:"executed_at,omitempty"`
    CompletedAt   *time.Time `json:"completed_at,omitempty"`
}
```

**Lifecycle**: `pending` (Trust) → `approved` (BidNet) → `funded` (DistoDam) → `executed` (Digger).

---

## 🎨 Component-Specific Patterns

### Mint Service

**Core Responsibility**: Batch 1000 ingots or flush after 60s, hash, publish.

**Key Files**:
- `cmd/mint/main.go` - Wire components
- `internal/mint/ingot_receiver.go` - HTTP + NATS dual input
- `internal/mint/batch_aggregator.go` - Time/size thresholds
- `internal/mint/mint_engine.go` - Hash generation, NATS publish

**Gotchas**:
- Ingots MUST have 3600J (reject anything else)
- Batch flush on **size OR time** (whichever first)
- Graceful shutdown: drain buffer before exit

### Refinery Service

**Core Responsibility**: Accumulate ore (Tokens, Joules, RoboTorq, data) → 1 ingot (3600J).

**State Management**:
```go
// In-memory accumulator (single contract at a time)
accumulatedJoules := 0
ores := []JouleTorqOre{}

// When ore arrives
accumulatedJoules += ore.Joules
ores = append(ores, ore)

if accumulatedJoules >= 3600 {
    ingot := assembleIngot(ores)
    publishToMint(ingot)
    reset()
}
```

**Batch Publishing**: Every 60s, send ingots[] to Mint via NATS.

### Trust Service

**Core Responsibility**: Evaluate opportunities, create contracts, monitor execution.

**Pipeline**:
```
Opportunity (HTTP POST) 
  → Appraiser (evaluate, create contract)
  → NATS publish "contracts.pending"
  → BidNet (future: contract gateway)
```

**Key Components**:
- `opportunity_handler.go` - REST API
- `appraiser.go` - Contract creation logic
- `fundsync.go` - Listen for "contracts.funded", notify executor

### DistoDam Service (Future Refactor)

**Current State**: Placeholder.

**Future Design** (see `src/distodam/REFACTOR_TODO.md`):
- Subscribe to `contracts.approved` (from BidNet)
- Allocate RT from reservoir
- Publish `contracts.funded` (to Trust)
- Manage UBD streams (distribution)

### Wallet & Printer (Mobile + Embedded)

**Wallet**: User-facing mobile app (future React Native).

**Printer**: Raspberry Pi + 3D printer controller.
- Burns digital RT → prints physical bills with NFC tags
- See `src/printer/IMPLEMENTATION_TODO.md` for hardware specs

---

## 🔒 Security & Cryptography

### Signature Pattern (Dilithium5 - Post-Quantum)

**IMPORTANT**: RoboTorq uses **Dilithium5** (NIST FIPS 204), NOT Ed25519!
- **Why**: Quantum-resistant (protects against Shor's algorithm)
- **Security**: NIST Level 5 (AES-256 equivalent)
- **Signature size**: ~4595 bytes (vs Ed25519's 64 bytes)
- **Public key size**: 2592 bytes

```rust
// Digger (Rust) - see src/digger-app/digger/src-tauri/src/crypto.rs
use pqcrypto_dilithium::dilithium5;
use pqcrypto_traits::sign::*;

// Generate keypair
let (pk, sk) = dilithium5::keypair();

// Sign
let message = b"data";
let signature = dilithium5::detached_sign(message, &sk);

// Verify
let valid = dilithium5::verify_detached_signature(&signature, message, &pk).is_ok();
```

```go
// Refinery/Mint (Go) - TODO: Implement with cloudflare/circl
import "github.com/cloudflare/circl/sign/dilithium/mode5"

// Verify signature
var pubKey mode5.PublicKey
copy(pubKey[:], pubKeyBytes)
valid := mode5.Verify(&pubKey, message, signature)
```

**Current State**: Dilithium is **stubbed out** in Digger's `crypto.rs`
- Signing: Returns empty `vec![]`
- Verification: Always returns `true`
- **This is INSECURE** - only for development/testing!

**Future Implementation**: See TODO checklist in `src/digger-app/digger/src-tauri/src/crypto.rs`

### Hash Pattern (SHA256)

```go
import "crypto/sha256"

// Deterministic batch hashing
data := fmt.Sprintf("%s|%s|%d", id, timestamp, joules)
hash := sha256.Sum256([]byte(data))
hashHex := hex.EncodeToString(hash[:])
```

**Used In**:
- Ingot hashes (Refinery)
- Batch hashes (Mint)
- Proof-of-work verification

### Anti-Counterfeiting (Physical RT)

**NFC Tag Data**:
```json
{
  "serial_number": "RT-10-2025-ABC123XYZ",
  "denomination": 10.0,
  "minted_at": "2025-11-15T14:30:10Z",
  "minted_by": "printer-abc123",
  "signature": "0x9abc...",          // Printer's Ed25519 sig
  "printer_public_key": "0xdef0..."  // For verification
}
```

**Redemption Check**:
1. Read NFC tag
2. Verify signature with printer's public key
3. Query network: "Has serial been redeemed?"
4. If valid + unredeemed → credit wallet, mark serial redeemed

---

## 🚨 Common Pitfalls & Solutions

### "Buffer Full" Errors

**Symptom**: Mint returns `429 Too Many Requests`.

**Cause**: IngotBuffer at capacity (100k ingots).

**Fix**:
```bash
# Increase buffer size
docker-compose down
# Edit docker-compose.yml
BUFFER_CAPACITY=200000
docker-compose up -d
```

**Or**: Reduce batch flush interval (process faster):
```yaml
FLUSH_INTERVAL=30s  # Was 60s
```

### NATS Connection Drops

**Symptom**: Services log "NATS publish failed".

**Cause**: NATS server restart, network partition.

**Fix**: Retry logic with exponential backoff (already in `natsx` client):
```go
// natsx/client.go handles this
func (c *Client) PublishJSON(topic string, data interface{}) error {
    // Auto-retry 3x with backoff
}
```

**Check Health**:
```bash
curl http://localhost:8222/healthz
docker logs robotorq-network-nats-1
```

### Ingot "Not 3600 Joules" Rejection

**Symptom**: Mint rejects ingots with validation error.

**Cause**: Refinery assembled partial ingot (< 4 ores).

**Debug**:
```bash
# Check Refinery state
curl http://localhost:8081/health | jq '.ingot_assembly'

# Expected output
{
  "accumulated_joules": 2700,  # Waiting for 1 more ore
  "progress_to_next_ingot_percent": 75
}
```

**Solution**: Send more ores OR wait for timeout flush.

### Docker Compose Port Conflicts

**Symptom**: `bind: address already in use`.

**Fix**: Change host port (not container port):
```yaml
ports:
  - "8084:8080"  # Host:Container (container always 8080)
```

---

## 🔮 Future Work (Don't Implement Yet!)

### Phase 2: BidNet (Contract Gateway)

**Status**: Documented in `src/bidnet/IMPLEMENTATION_TODO.md` but NOT implemented.

**When to Build**: After DistoDam refactor complete.

**Purpose**: Pluggable contract evaluation (ROI scoring, gaming resistance).

### Phase 2: Currency Exchange Contracts

**Status**: Spec exists in BidNet docs.

**Purpose**: RT ↔ USDC/fiat on-ramps.

**Wait For**: Production DistoDam + Trust stabilization.

### Phase 3: Vault Services

**Files**: `src/vault/{STASHVAULT,TORQEDVAULT}_IMPLEMENTATION.md`

**Purpose**: Savings accounts, interest mechanisms, lending.

**Blockers**: Requires mature DistoDam UBD streams.

---

## 💡 Quick Reference

### Environment Variables Cheat Sheet

```bash
# All services
NATS_URL=nats://nats:4222
LOG_LEVEL=debug|info|warn|error
HTTP_PORT=8080

# Mint-specific
BATCH_SIZE=1000
FLUSH_INTERVAL=60s
BUFFER_CAPACITY=100000

# Refinery-specific
REFINERY_INGOT_BATCH_INTERVAL=60
REFINERY_JOULE_QUEUE_SIZE=1000

# Trust-specific
TRUST_ROI_THRESHOLD=10.0
```

### Docker Commands

```bash
# Build single service
cd src/mint && docker build -t mint:latest .

# View service logs
docker logs -f robotorq-network-mint-1

# Restart all
docker-compose restart

# Clean rebuild
docker-compose down && docker-compose up --build -d

# Shell into container
docker exec -it robotorq-network-mint-1 sh
```

### NATS CLI (Debug)

```bash
# Inside NATS container
docker exec -it robotorq-network-nats-1 sh

# List subjects
nats stream ls

# Subscribe to topic
nats sub "mint.batches"

# Publish test message
nats pub "mint.ingots" '{"id":"test"}'
```

---

## 📞 Getting Help

1. **Check Architecture Docs**: Each service has `{SERVICE}_ARCHITECTURE.md`
2. **Read Test Plans**: `TESTING_PLAN.md` shows expected behaviors
3. **Review README.md**: Especially Appendices O, P, N for data structures
4. **Check IMPLEMENTATION_TODO.md**: Shows current phase and blockers
5. **Examine E2E Scripts**: `test-e2e-flow.ps1` demonstrates correct flows

**Critical Files for New Contributors**:
- `README.md` (lines 1-2000): Economics & formulas
- `BRANCHING.md`: Git workflow
- `docker-compose.yaml`: Service configuration
- `src/mint/MINT_ARCHITECTURE.md`: Best example of complete service

---

## ✅ Pre-Commit Checklist

Before pushing code:

- [ ] Unit tests pass: `go test ./... -v`
- [ ] Coverage ≥95%: `go test -cover ./...`
- [ ] Integration test passes
- [ ] E2E script succeeds: `pwsh test-e2e-flow.ps1`
- [ ] No `fmt.Println()` debugging
- [ ] Metrics instrumented
- [ ] Structured logging used
- [ ] Health endpoint works: `curl localhost:8080/health`
- [ ] Docker build succeeds: `docker build .`
- [ ] Updated relevant `ARCHITECTURE.md` if changing design
- [ ] Added tests for new functionality

---

## 🎓 Learning Path for New AI Agents

1. **Day 1**: Read README.md Sections 1-5 (What is Torq? → TTP)
2. **Day 2**: Read Mint Architecture, run `test-e2e-flow.ps1`
3. **Day 3**: Study Refinery Architecture, trace JouleTorq → TokenTorq flow
4. **Day 4**: Explore Trust service, understand contract lifecycle
5. **Day 5**: Review all IMPLEMENTATION_TODO.md files (future roadmap)

**Hands-On Exercise**: Modify Mint's `BATCH_SIZE` to 500, rebuild, verify metrics change in Prometheus.

---

**Remember**: This is a **monetary network based on physics**, not hype. Every line of code represents real energy, real computation, and real value. Code accordingly.

*"Watts > Wall Street"* 🤖⚡💰
