# DistoDam Refactor TODO

**Status**: 📋 Planning Phase  
**Priority**: Phase 2 (After BidNet MVP)  
**Target**: Modular architecture aligned with Mint service patterns

---

## 🎯 **Prerequisites**

Before starting DistoDam refactor, ensure:

- ✅ **BidNet MVP Complete** - Contract evaluation service operational
- ✅ **Topics Established**:
  - `contracts.approved` (BidNet → DistoDam)
  - `contracts.rejected` (BidNet → Trust)
  - `mint.batches` (Mint → DistoDam)
  - `ubd.requests` (Wallets → DistoDam) - *pending UBD service*
- ✅ **MintEvent Structure Finalized** - No field changes after Mint refactor
- ✅ **Contract Structure Finalized** - BidNet defines approved contract schema

---

## 📐 **Architecture Goals**

### **Core Principles**
1. ✅ **Modular Component Design** - Follow Mint service pattern
2. ✅ **Atomic Reservoir Operations** - CompareAndSwap, never lose data
3. ✅ **All-or-Nothing Funding** - No partial funding (simplifies logic)
4. ✅ **NATS Integration** - Dual input, multiple publishers
5. ✅ **Comprehensive Testing** - 95%+ coverage, embedded NATS tests
6. ✅ **Full Observability** - Prometheus metrics, structured logging

### **Non-Goals (Deferred)**
- ❌ Multi-dam rebalancing (wait for horizontal scaling)
- ❌ BRLa labor routing (deprecated - removed)
- ❌ Water mark triggers (stub until distributed deployment)
- ❌ Partial funding (all-or-nothing only)

---

## 📥 **Input Topics & Data Models**

### **1. Mint Batches** (Inflows)
**Topic**: `mint.batches`  
**Publisher**: Mint service  
**Purpose**: Add RoboTorq to reservoir

**MintEvent Structure** (finalized in Mint refactor):
```go
type MintEvent struct {
    BatchHash       string    `json:"batch_hash"`        // SHA256 proof
    TotalRoboTorq   float64   `json:"total_robo"`        // Sum from all ingots ← USE THIS
    IngotsProcessed int       `json:"ingots_count"`      // Batch size
    SaleValueUSD    float64   `json:"sale_value"`        // Accounting only
    BatchID         string    `json:"batch_id"`          // UUID
    Timestamp       time.Time `json:"timestamp"`         // UTC
}
```

**Action**: Extract `TotalRoboTorq`, convert to micro-RT, add to reservoir

---

### **2. Approved Contracts** (Funding Requests)
**Topic**: `contracts.approved`  
**Publisher**: BidNet service  
**Purpose**: Fund contracts that passed BidNet evaluation

**Contract Structure** (TBD - will be finalized by BidNet):
```go
type Contract struct {
    // Core fields (from Trust)
    ID            string    `json:"id"`
    OpportunityID string    `json:"opportunity_id"`
    Builder       string    `json:"builder"`
    DiggerURL     string    `json:"digger_url"`
    RoboStake     float64   `json:"robo_stake"`        // Amount to fund ← USE THIS
    ROI           float64   `json:"roi"`
    Torq          int       `json:"torq"`
    
    // BidNet additions (expect these after BidNet evaluation)
    Status        string    `json:"status"`            // "approved"
    ApprovedAt    time.Time `json:"approved_at"`
    ApprovedBy    string    `json:"approved_by"`       // "bidnet-001"
    Evaluation    *BidEvaluation `json:"evaluation"`    // BidNet scores
    
    // Timestamps
    CreatedAt     time.Time  `json:"created_at"`
    FundedAt      *time.Time `json:"funded_at,omitempty"`  // Set by DistoDam
}

type BidEvaluation struct {
    ROIScore      float64 `json:"roi_score"`
    RiskScore     float64 `json:"risk_score"`
    GiskardScore  float64 `json:"giskard_score,omitempty"`   // Future
    CalvinScore   float64 `json:"calvin_score,omitempty"`    // Future
    DaneelScore   float64 `json:"daneel_score,omitempty"`    // Future
}
```

**Action**: 
1. Validate `Status == "approved"`
2. Check reservoir balance >= `RoboStake` (in micro-RT)
3. Atomically deduct from reservoir
4. Set `FundedAt = now()`
5. Publish to `contracts.funded`

---

### **3. UBD Funding Requests** (Future)
**Topic**: `ubd.requests`  
**Publisher**: Wallet service (TBD)  
**Purpose**: Fund Universal Basic Distribution recipients

**UBDFundingRequest Structure** (DRAFT - needs Wallet service input):
```go
type UBDFundingRequest struct {
    RequestID   string    `json:"request_id"`    // UUID
    WalletID    string    `json:"wallet_id"`     // Recipient wallet
    Amount      float64   `json:"amount"`        // RT to fund
    Priority    int       `json:"priority"`      // 0-10 (higher = urgent)
    Reason      string    `json:"reason"`        // "monthly_ubi", "emergency", etc.
    RequestedAt time.Time `json:"requested_at"`  // UTC timestamp
}
```

**Action**:
1. Validate `Amount > 0`
2. Check reservoir balance >= `Amount`
3. Atomically deduct from reservoir
4. Publish to `ubd.funded`

**Note**: UBD service doesn't exist yet - this is placeholder for future integration

---

## 📤 **Output Topics & Events**

### **1. Funded Contracts**
**Topic**: `contracts.funded`  
**Subscriber**: Trust service  
**Purpose**: Notify Trust that contract has funding, ready for execution

**Published Message**: Modified `Contract` with `FundedAt` set
```go
{
    "id": "contract-123",
    "robo_stake": 0.05,
    "status": "funded",              // Changed by DistoDam
    "funded_at": "2025-11-14T12:05:00Z",  // Set by DistoDam
    "funded_by": "distodam-001",     // Added by DistoDam
    // ... rest of contract fields
}
```

---

### **2. Funded UBD Requests**
**Topic**: `ubd.funded`  
**Subscriber**: Wallet service (TBD)  
**Purpose**: Notify wallet that UBD payment is funded

**Published Message** (DRAFT):
```go
type UBDFundedEvent struct {
    RequestID   string    `json:"request_id"`
    WalletID    string    `json:"wallet_id"`
    Amount      float64   `json:"amount"`
    FundedAt    time.Time `json:"funded_at"`
    FundedBy    string    `json:"funded_by"`    // "distodam-001"
    TxHash      string    `json:"tx_hash"`      // Future: blockchain tx
}
```

---

## 🏗️ **Component Architecture**

Follow Mint service modular design:

### **1. ReservoirManager**
**Responsibility**: Atomic reservoir balance operations

**Interface**:
```go
type ReservoirManager interface {
    // Add inflow to reservoir (from Mint)
    AddInflow(amountRT float64) error
    
    // Attempt to deduct for funding (atomic CAS)
    // Returns true if successful, false if insufficient balance
    DeductFunding(amountRT float64) (success bool, newBalance float64)
    
    // Get current balance (for metrics/health)
    GetBalance() float64
    
    // Get balance in micro-RT (internal representation)
    GetBalanceMicro() int64
}
```

**Implementation**:
- Use `atomic.Int64` for micro-RT storage (1 RT = 1,000,000 µRT)
- `AddInflow`: `atomic.Add(microRT)`
- `DeductFunding`: CAS loop with balance check
- Thread-safe, zero-allocation operations

---

### **2. MintEventReceiver**
**Responsibility**: Subscribe to `mint.batches`, process MintEvents

**Interface**:
```go
type MintEventReceiver interface {
    Start(ctx context.Context) error
    Shutdown() error
}
```

**Behavior**:
1. Subscribe to `mint.batches` NATS topic
2. Parse `MintEvent` from JSON
3. Extract `TotalRoboTorq` (discard metadata like batch_hash, batch_id)
4. Convert RT → micro-RT
5. Call `ReservoirManager.AddInflow()`
6. Update Prometheus metrics (inflows_total, inflows_robo_total)
7. Log structured event

**Error Handling**:
- Invalid JSON: Log error, NACK message (if NATS supports)
- Negative amount: Log warning, ignore
- ReservoirManager error: Log error, retry

---

### **3. ContractFunder**
**Responsibility**: Subscribe to `contracts.approved`, fund contracts

**Interface**:
```go
type ContractFunder interface {
    Start(ctx context.Context) error
    Shutdown() error
}
```

**Behavior**:
1. Subscribe to `contracts.approved` NATS topic
2. Parse `Contract` from JSON
3. Validate `Status == "approved"` and `RoboStake > 0`
4. Call `ReservoirManager.DeductFunding(RoboStake)`
5. If successful:
   - Set `contract.FundedAt = time.Now()`
   - Set `contract.FundedBy = damID`
   - Set `contract.Status = "funded"`
   - Publish to `contracts.funded` via EventPublisher
   - Update metrics (contracts_funded_total, contracts_funded_robo_total)
6. If insufficient balance:
   - Log warning (reservoir depleted)
   - Increment metric (contracts_rejected_insufficient_funds)
   - Do NOT publish (Trust will timeout and retry later)

**Critical**: All-or-nothing - either fund completely or don't fund at all

---

### **4. UBDFunder** (Future - Stub for Now)
**Responsibility**: Subscribe to `ubd.requests`, fund UBD payments

**Interface**:
```go
type UBDFunder interface {
    Start(ctx context.Context) error
    Shutdown() error
}
```

**Behavior**: Similar to ContractFunder, but for UBD requests

**Note**: Stub this component until Wallet service exists

---

### **5. FundingRouter** (Future - Interface Only)
**Responsibility**: Prioritize funding when multiple requests compete

**Interface** (for future pluggability):
```go
type FundingRouter interface {
    // Decide funding order for competing requests
    // Returns sorted slice (highest priority first)
    Prioritize(contracts []*Contract, ubdRequests []*UBDRequest) []FundingRequest
}

type FundingRequest struct {
    Type     string  // "contract" or "ubd"
    ID       string
    Amount   float64
    Priority int
}
```

**Implementations** (pick ONE for MVP):
- `SimpleRouter`: First-come-first-served (queue order)
- `ROIRouter`: Prioritize high-ROI contracts
- `BalancedRouter`: Alternate between contracts and UBD
- `FairShareRouter`: Equal distribution by count

**MVP Decision**: Use `SimpleRouter` (FCFS) - simplest, no config needed

---

### **6. EventPublisher**
**Responsibility**: Publish funded events to NATS topics

**Interface**:
```go
type EventPublisher interface {
    PublishContractFunded(contract *Contract) error
    PublishUBDFunded(event *UBDFundedEvent) error
}
```

**Behavior**:
- Serialize to JSON
- Publish to appropriate topic
- Retry on failure (exponential backoff, 3 retries)
- Update metrics (publish_success, publish_failures)

**Critical Decision Needed**: What if publish fails after deducting funds?
- **Option A**: Rollback to reservoir immediately (add funds back)
- **Option B**: Retry with exponential backoff (block until success)
- **Option C**: Write-ahead log, retry queue (complex)

**Recommendation**: **Option B** - Retry until success (NATS is reliable, failures rare)

---

### **7. MetricsCollector**
**Responsibility**: Prometheus metrics for observability

**Metrics** (minimum set):
```go
// Reservoir state
reservoir_balance_rt        Gauge    // Current balance in RT
reservoir_balance_micro     Gauge    // Current balance in micro-RT

// Inflows (from Mint)
inflows_total               Counter  // Number of MintEvents received
inflows_robo_total          Counter  // Total RT received from Mint
inflows_invalid             Counter  // Invalid MintEvents (parse errors)

// Contract funding
contracts_received_total    Counter  // Approved contracts received
contracts_funded_total      Counter  // Successfully funded
contracts_funded_robo_total Counter  // Total RT funded to contracts
contracts_rejected_insufficient_funds Counter

// UBD funding (future)
ubd_requests_received_total Counter
ubd_funded_total            Counter
ubd_funded_robo_total       Counter
ubd_rejected_insufficient_funds Counter

// Publishing
publish_success_total       Counter  // Successful NATS publishes
publish_failures_total      Counter  // Failed NATS publishes
publish_retry_total         Counter  // Publish retries

// Performance
funding_latency_seconds     Histogram // Time from receive to publish
```

---

### **8. Config**
**Responsibility**: Load and validate configuration from environment

**Environment Variables**:
```bash
# HTTP server
HTTP_PORT=8082              # Default: "8082" (avoid conflict with Trust on 8081)

# NATS connection
NATS_URL=nats://nats:4222   # Default: "nats://nats:4222"

# NATS topics (configurable for testing)
MINT_BATCHES_TOPIC=mint.batches           # Default
CONTRACTS_APPROVED_TOPIC=contracts.approved  # Default
CONTRACTS_FUNDED_TOPIC=contracts.funded   # Default
UBD_REQUESTS_TOPIC=ubd.requests           # Default
UBD_FUNDED_TOPIC=ubd.funded               # Default

# Reservoir configuration
INITIAL_BALANCE_RT=0.0      # Default: 0.0 (start empty)
LOW_WATER_MARK_RT=0.001     # Default: 0.001 RT (future rebalancing)
HIGH_WATER_MARK_RT=0.01     # Default: 0.01 RT (future rebalancing)

# Retry configuration
NATS_RETRIES=3              # Default: 3 retries
NATS_BACKOFF_BASE_MS=100    # Default: 100ms

# Logging
LOG_LEVEL=info              # Default: "info" (debug, info, warn, error)

# Identity
DAM_ID=distodam-001         # Default: "distodam-001"
```

**Validation**:
- HTTP_PORT: Must be valid port (1-65535)
- NATS_URL: Must be valid URL
- INITIAL_BALANCE_RT: >= 0
- LOW_WATER_MARK_RT: > 0, < HIGH_WATER_MARK_RT
- NATS_RETRIES: >= 0
- LOG_LEVEL: Must be valid slog level

---

## 🧪 **Testing Strategy**

Follow Mint service testing patterns:

### **Unit Tests** (Component Isolation)
- `ReservoirManager`: Atomic operations, concurrent access, overflow/underflow
- `MintEventReceiver`: Parse MintEvent, handle invalid JSON, reservoir updates
- `ContractFunder`: Funding logic, all-or-nothing, insufficient balance handling
- `EventPublisher`: Serialization, retry logic, error handling
- `Config`: Load defaults, parse env vars, validation errors

**Target**: 95%+ code coverage

---

### **Integration Tests** (Embedded NATS)
- Full pipeline: MintEvent → Reservoir → ContractFunding → Publish
- NATS subscription and publishing (use embedded NATS server like Mint)
- Concurrent funding requests (race condition testing)
- Graceful shutdown with pending operations

---

### **Benchmark Tests**
- Reservoir operations throughput (target: >1M ops/sec)
- Concurrent funding (100 goroutines, 1000 contracts)
- Large batch processing (1000 contracts in single batch)

---

### **E2E Tests** (After BidNet Integration)
Full flow test script (PowerShell):
```powershell
# 1. Start all services (Mint, BidNet, DistoDam, Trust)
# 2. Send ores to Refinery → Mint → DistoDam (reservoir fills)
# 3. Create opportunity → Trust → BidNet → DistoDam (contract funded)
# 4. Verify: Contract reaches Executor with "funded" status
# 5. Check metrics: inflows, outflows, reservoir balance
```

---

## 🚧 **Key Decisions Pending**

### **Priority 1: Publish Failure Handling**
**Question**: If `PublishContractFunded()` fails after deducting reservoir funds, what do we do?

**Options**:
- A) Rollback immediately (add funds back to reservoir)
- B) Retry with exponential backoff (block until success)
- C) Write-ahead log + retry queue (complex)

**Recommendation**: **Option B** (retry until success)

**Reasoning**: 
- NATS failures are rare in production
- Rollback could cause double-funding if message was actually delivered
- WAL adds complexity, overkill for initial MVP

**Action**: Implement exponential backoff retry (3 attempts, 100ms base, 2x multiplier)

---

### **Priority 2: Funding Priority Algorithm**
**Question**: If multiple contracts arrive simultaneously, which order to fund?

**Options**:
- A) First-come-first-served (queue order)
- B) Highest ROI first (prioritize profitable contracts)
- C) Balanced (alternate contracts and UBD)
- D) Fair share (equal distribution)

**Recommendation**: **Option A** (FCFS) for MVP

**Reasoning**:
- Simplest to implement and test
- No configuration needed
- Predictable behavior
- Can be replaced later with pluggable router

**Action**: Implement `SimpleRouter` (FCFS), design interface for future swapping

---

### **Priority 3: Persistence Strategy**
**Question**: How to ensure "data is never lost" if DistoDam crashes?

**Options**:
- A) No persistence (rely on distributed replication across multiple dams)
- B) Write-ahead log (WAL) to disk before every operation
- C) Event sourcing (append-only event log, rebuild state on restart)
- D) Periodic snapshots (save reservoir state every N seconds)

**Recommendation**: **Option A** (no persistence) for MVP, **Option C** (event sourcing) for production

**Reasoning**:
- MVP: Single dam, no persistence needed (reservoir starts at 0, rebuilds from Mint inflows)
- Production: Multiple dams ensure redundancy (if one crashes, others continue)
- Event sourcing provides full audit trail (useful for debugging and compliance)

**Action**: 
- MVP: No persistence, document in ARCHITECTURE.md
- Production: Add event sourcing in Phase 3 (after multi-dam deployment)

---

### **Priority 4: Reservoir Initial Balance**
**Question**: Should reservoir start at 0 or have a seed balance?

**Options**:
- A) Always start at 0 (rely on Mint inflows)
- B) Configurable via INITIAL_BALANCE_RT env var
- C) Restore from disk/DB on restart

**Recommendation**: **Option B** (configurable)

**Reasoning**:
- Development: Seed balance allows testing funding without waiting for Mint
- Production: Start at 0, wait for Mint inflows (proper flow)
- Testing: Set high balance for stress testing

**Action**: Add `INITIAL_BALANCE_RT` env var (default: 0.0)

---

## 📝 **Implementation Order**

### **Phase 1: Core Components** (Days 1-2)
1. ✅ Config (environment variables, validation)
2. ✅ ReservoirManager (atomic operations, micro-RT conversions)
3. ✅ MetricsCollector (Prometheus setup)
4. ✅ EventPublisher (NATS publishing with retries)

### **Phase 2: Input Receivers** (Days 2-3)
5. ✅ MintEventReceiver (subscribe mint.batches, add inflows)
6. ✅ ContractFunder (subscribe contracts.approved, fund contracts)
7. ✅ UBDFunder (stub - interface only, no implementation)

### **Phase 3: Orchestration** (Day 3)
8. ✅ Main.go (wire all components, graceful shutdown)
9. ✅ HTTP server (health, status, metrics endpoints)

### **Phase 4: Testing** (Days 4-5)
10. ✅ Unit tests (all components, 95%+ coverage)
11. ✅ Integration tests (embedded NATS, full pipeline)
12. ✅ Benchmark tests (throughput, concurrency)

### **Phase 5: Deployment & Documentation** (Day 5)
13. ✅ Dockerfile (multi-stage build like Mint)
14. ✅ docker-compose.yaml (add distodam service)
15. ✅ ARCHITECTURE.md (comprehensive design doc)
16. ✅ TESTING_PLAN.md (test matrix, execution commands)
17. ✅ E2E test script (PowerShell, full flow validation)

---

## 🔗 **Dependencies on Other Services**

### **Required Before DistoDam Refactor**
- ✅ **BidNet MVP**: Must define `contracts.approved` schema
- ✅ **Mint Refactor**: MintEvent structure must be stable (already done)
- ✅ **Trust Service**: Must subscribe to `contracts.funded` topic

### **Optional (Can Stub)**
- ⏳ **Wallet Service**: For UBD funding (stub UBDFunder for now)
- ⏳ **Giskard/Calvin/Daneel**: BidNet enhancements (doesn't affect DistoDam)

---

## 📚 **Reference Documentation**

### **Similar Services (Use as Templates)**
- `src/mint/` - Modular component architecture, NATS integration
- `src/mint/MINT_ARCHITECTURE.md` - Documentation structure
- `src/mint/TESTING_PLAN.md` - Testing strategy

### **Key Design Decisions from Mint**
- ✅ Embedded NATS server for integration tests
- ✅ Atomic operations with `atomic.Int64`
- ✅ Structured JSON logging (slog)
- ✅ Comprehensive Prometheus metrics
- ✅ Graceful shutdown with context cancellation
- ✅ Docker multi-stage builds
- ✅ Environment-based configuration

---

## 🚀 **Next Steps**

### **Immediate Actions** (After BidNet MVP Complete)
1. ✅ Review BidNet's `contracts.approved` schema
2. ✅ Finalize Contract struct (add DistoDam-specific fields)
3. ✅ Create feature branch: `feature/distodam-refactor`
4. ✅ Expand this TODO into detailed implementation checklist
5. ✅ Begin Phase 1 implementation

### **Before Starting**
- [ ] BidNet E2E tests passing
- [ ] BidNet documentation complete
- [ ] Contract schema frozen (no more field changes)
- [ ] Team alignment on pending decisions (publish failures, priority, persistence)

---

## 📖 **Notes for Future Maintainers**

### **Why All-or-Nothing Funding?**
Partial funding adds complexity:
- Need to track partial amounts per contract
- Bondholders expect full funding before execution starts
- Refund logic needed if contract cancels after partial funding

All-or-nothing is simpler, safer, and matches bondholder expectations.

---

### **Why Micro-RT Internal Representation?**
Floating-point precision issues:
- `0.1 + 0.2 != 0.3` in float64
- Atomic operations require integer types
- Micro-RT (1 RT = 1,000,000 µRT) gives 6 decimal places of precision
- All reservoir math is exact integer arithmetic

---

### **Why No Rebalancing in MVP?**
Rebalancing requires:
- Multiple DistoDam instances (horizontal scaling)
- Inter-dam communication protocol
- Consensus on reservoir state
- Handling of in-flight transfers

Single-dam MVP doesn't need this complexity. Add in Phase 3.

---

### **Why Remove BRLa Labor Routing?**
Original 40% labor routing was placeholder logic:
- Hard-coded rate (not realistic)
- No actual BRLa service to receive funds
- Confuses funding flow (contracts vs. labor are different)

Will be re-added properly when BRLa service exists, with:
- Dynamic routing based on BRLa performance
- Configurable rates per BRLa
- Priority/scheduling algorithms

---

**Last Updated**: November 14, 2025  
**Author**: GitHub Copilot (with Jon's guidance)  
**Status**: Ready for Phase 2 (pending BidNet MVP completion)
