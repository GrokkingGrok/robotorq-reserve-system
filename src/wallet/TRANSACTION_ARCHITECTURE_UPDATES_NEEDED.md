# Transaction Architecture - Remaining Updates

**Status**: Partially updated (header + TOC + some crypto fixes complete)  
**Date**: November 18, 2025

---

## ✅ Completed Updates

1. **Header section** - Updated to match vault plan format with:
   - Related docs links (VAULT_IMPLEMENTATION_PLAN.md, PHASE5_VERIFICATION_PLAN.md)
   - Achieved properties with checkboxes
   - Tech stack including Falcon-1024 + SPHINCS+
   - Security fixes callouts (event sourcing, 30-day notice, nonce replay prevention)

2. **Table of Contents** - Restructured with:
   - Linked sections (clickable headers)
   - Organized by: Core Architecture, Security & Integration, Implementation, Appendices
   - Added new sections: Security Fixes (Vault-Inspired), Event Sourcing, API Endpoints, Metrics & Observability
   - Added Appendix B (Vault Security Lessons) and Appendix C (Deployment Checklist)

3. **Crypto fixes (partial)** - Fixed 1 Dilithium reference in flow initiation docs

---

## 🔧 Remaining Updates Needed

### 1. Fix Remaining Dilithium References (~20 remaining)

**Search for**: `Dilithium` (case-insensitive)

**Replace with**:
- `Dilithium2` → `Falcon-1024` (user transactions, fast)
- `Dilithium3` → `SPHINCS+` (Oracle signatures, archival long-term proofs)
- `Dilithium5` → `SPHINCS+` (if found)

**Locations** (from grep results):
- Line ~659: Account struct address comment
- Line ~672: Account struct PublicKey comment  
- Line ~819: EscrowBid struct signature comment
- Line ~911: FlowRequest JSON example signature
- Line ~923: EscrowBid JSON example signature
- Line ~935: EscrowAwarded JSON example signature
- Line ~948: LedgerUpdate JSON example signature (use SPHINCS+)
- Line ~963: FlowCancelled JSON example signature
- Line ~1008: Cryptography section algorithm description
- Line ~1011-1027: Code example using circl/dilithium (replace entire block)
- Line ~1031: Address derivation comment
- Line ~1079: Oracle signature comment
- Line ~1093: Oracle verification code (dilithium.Mode3)
- Line ~1137: Phase 1 roadmap item

**Replacement code block** (for lines ~1008-1027):

```go
import (
    "github.com/open-quantum-safe/liboqs-go/oqs"  // Falcon-1024 + SPHINCS+
)

// Key generation (one-time per account)
signer := oqs.Signature{}
signer.Init("Falcon-1024", nil)
defer signer.Clean()

publicKey, _ := signer.GenerateKeyPair()
privateKey := signer.ExportSecretKey()

// Signing (every FlowRequest)
signature, _ := signer.Sign(flowRequestBytes)

// Verification (by TorqVaults)
isValid, _ := signer.Verify(flowRequestBytes, signature, publicKey)
```

**Signature sizes**:
- Falcon-1024: ~1.3 KB (fast verification, compact)
- SPHINCS+-SHAKE256-128f: ~17 KB (stateless, long-term secure)

**RoboTorq choice**: Falcon-1024 for user transactions (speed), SPHINCS+ for Oracle/TorqVault long-term archival proofs

---

### 2. Add New Section 8: Security Fixes (Vault-Inspired)

**Insert before**: Current "NATS Message Schemas" section (around line ~900)

**Content**:

#### Security Fix #1: 30-Day Cancellation Notice (Large Flows)

**Problem**: Flash cancellation attacks (whales cancel simultaneously, drain TorqVault liquidity)

**Solution**: Large flow cancellations require 30-day notice period

```go
type FlowCancellationNotice struct {
    FlowID        string
    CancellerID   string  // Must be sender
    AmountMicroRT int64   // Amount to cancel (locked at submission)
    SubmittedAt   time.Time
    MaturesAt     time.Time  // SubmittedAt + 30 days (if amount > threshold)
    Signature     []byte     // Falcon-1024
}

const CANCELLATION_THRESHOLD = 10_000_000  // 10 RT

func ValidateCancellation(flow *FlowRequest, notice *FlowCancellationNotice) error {
    // Small flows: Cancel immediately
    if flow.AmountMicroRT < CANCELLATION_THRESHOLD {
        return nil
    }
    
    // Large flows: Require matured notice
    if notice == nil || time.Now().Before(notice.MaturesAt) {
        return errors.New("large flow cancellation requires 30-day notice")
    }
    
    // Amount binding (can't change amount after submission)
    if notice.AmountMicroRT < flow.AmountMicroRT {
        return errors.New("cancellation amount exceeds notice")
    }
    
    return nil
}
```

**Why**: Prevents coordinated "bank run" on TorqVault liquidity  
**Threshold**: 10 RT (~$50 USD) - Small payments cancel instantly, large transfers require notice  
**Ref**: VAULT_SECURITY_FIXES_PART_2.md Fix #2

#### Security Fix #2: Event Sourcing + Sequence Numbers

**Problem**: State corruption, replay attacks, lack of audit trails

**Solution**: All state changes event sourced to PostgreSQL

```go
type FlowEvent struct {
    EventID       uuid.UUID  `db:"event_id" json:"event_id"`
    FlowID        string     `db:"flow_id" json:"flow_id"`
    SequenceNumber int64     `db:"sequence_number" json:"sequence_number"`
    EventType     string     `db:"event_type" json:"event_type"`
    Payload       jsonb      `db:"payload" json:"payload"`
    Timestamp     time.Time  `db:"timestamp" json:"timestamp"`
    NATSMessageID string     `db:"nats_msg_id" json:"nats_msg_id"`  // Idempotency
}

// Database schema
CREATE TABLE flow_events (
    event_id UUID PRIMARY KEY,
    flow_id VARCHAR NOT NULL,
    sequence_number BIGINT NOT NULL,
    event_type VARCHAR NOT NULL,
    payload JSONB NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    nats_msg_id VARCHAR UNIQUE,
    UNIQUE(flow_id, sequence_number)
);

// Indexes (learned from Vault Todo #43)
CREATE INDEX idx_flow_events_timestamp ON flow_events(timestamp DESC);
CREATE INDEX idx_flow_events_nats_msg_id ON flow_events(nats_msg_id);
CREATE INDEX idx_flow_events_flow_id_seq ON flow_events(flow_id, sequence_number);
```

**Event Types**:
- `flow.initiated` - User creates FlowRequest
- `balance.locked` - Sender balance locked
- `escrow.awarded` - TorqVault wins bid
- `liquidity.fronted` - Recipient credited
- `flow.drip` - Oracle tick (drip event)
- `liquidity.recovered` - TorqVault recovers from sender
- `flow.completed` - Final settlement
- `flow.cancelled` - Cancellation (with notice)

**Why**: Enables audit trails, crash recovery, dispute resolution  
**Ref**: Vault Fix #9 (VAULT_SECURITY_FIXES_PART_2.md)

#### Security Fix #3: Nonce-Based Idempotency (NATS Messages)

**Problem**: NATS message replays cause duplicate state changes

**Solution**: Store NATS message IDs, reject duplicates

```go
func (w *Wallet) handleLedgerUpdate(msg *nats.Msg) {
    var update LedgerUpdate
    json.Unmarshal(msg.Data, &update)
    
    // Check if already processed (idempotency)
    exists, err := w.repo.CheckNATSMessageID(msg.Header.Get(nats.MsgIdHdr))
    if err != nil {
        w.logger.Error("idempotency check failed", "error", err)
        return
    }
    
    if exists {
        w.logger.Warn("duplicate NATS message ignored",
            "nats_msg_id", msg.Header.Get(nats.MsgIdHdr),
            "flow_id", update.FlowID)
        return
    }
    
    // Process update (apply to account balances)
    w.applyLedgerUpdate(update)
    
    // Store NATS message ID (prevent future replays)
    w.repo.StoreNATSMessageID(msg.Header.Get(nats.MsgIdHdr), update)
}
```

**Ref**: Vault idempotency pattern (VAULT_IMPLEMENTATION_PLAN.md Fix #9)

---

### 3. Add New Section 11: Event Sourcing

**Insert after**: "Cryptography & Signatures" section (around line ~1100)

**Content**:

## 11. Event Sourcing

### 11.1 Event Types & Schemas

```go
// Base event structure
type FlowEvent struct {
    EventID       uuid.UUID
    FlowID        string
    SequenceNumber int64
    EventType     string
    Timestamp     time.Time
}

// Specific event payloads
type FlowInitiatedEvent struct {
    FlowEvent
    SenderAddr    string
    RecipientAddr string
    AmountMicroRT int64
    DurationSec   int
}

type FlowDripEvent struct {
    FlowEvent
    DripMicroRT int64
    TotalDripped int64
    IsComplete  bool
}

type FlowCancelledEvent struct {
    FlowEvent
    CancellerAddr string
    NoticeID      string  // Reference to cancellation notice
    RefundMicroRT int64
}
```

### 11.2 Event Storage & Replay

**PostgreSQL Schema**:
```sql
CREATE TABLE flow_events (
    event_id UUID PRIMARY KEY,
    flow_id VARCHAR NOT NULL,
    sequence_number BIGINT NOT NULL,
    event_type VARCHAR NOT NULL,
    payload JSONB NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    nats_msg_id VARCHAR UNIQUE,
    UNIQUE(flow_id, sequence_number)
);

-- Indexes for performance
CREATE INDEX idx_flow_events_timestamp ON flow_events(timestamp DESC);
CREATE INDEX idx_flow_events_nats_msg_id ON flow_events(nats_msg_id);
CREATE INDEX idx_flow_events_flow_id_seq ON flow_events(flow_id, sequence_number);
```

**Replay Logic**:
```go
func (w *Wallet) RebuildFlowState(flowID string) (*FlowState, error) {
    // Query all events for flow (ordered by sequence)
    events, err := w.db.Query(`
        SELECT event_id, sequence_number, event_type, payload, timestamp
        FROM flow_events
        WHERE flow_id = $1
        ORDER BY sequence_number ASC
    `, flowID)
    
    if err != nil {
        return nil, err
    }
    defer events.Close()
    
    // Replay events to rebuild state
    state := &FlowState{FlowID: flowID}
    for events.Next() {
        var event FlowEvent
        events.Scan(&event.EventID, &event.SequenceNumber, &event.EventType, &event.Payload, &event.Timestamp)
        
        state.Apply(event)  // Apply event to state machine
    }
    
    return state, nil
}
```

**Why Event Sourcing?**
1. **Audit trail** - Every state change logged
2. **Crash recovery** - Rebuild state from events
3. **Dispute resolution** - Prove flow history
4. **Time travel** - Query state at any point in past

**Ref**: Vault Fix #9 (VAULT_SECURITY_FIXES_PART_2.md)

---

### 4. Update Oracle Settlement Section

**Find**: "5. Oracle Settlement" (around line ~460)

**Add subsection**: "5.2 Oracle Service Architecture with Leader Election"

```go
type FlowOracle struct {
    tickInterval  time.Duration  // 10 seconds default
    activeFlows   map[string]*FlowSettler
    natsClient    *nats.Conn
    js            nats.JetStreamContext
    kv            nats.KeyValue
    logger        *slog.Logger
    
    // Leader election (prevents duplicate ticks)
    isLeader      atomic.Bool
    leaderLockTTL time.Duration  // 5 minutes
}

func (o *FlowOracle) maintainLeaderLock(ctx context.Context) {
    ticker := time.NewTicker(o.leaderLockTTL / 2)
    defer ticker.Stop()
    
    // Hostname fallback (Docker safety - learned from Vault Todo #42)
    hostname := os.Getenv("HOSTNAME")
    if hostname == "" {
        var err error
        hostname, err = os.Hostname()
        if err != nil || hostname == "" {
            hostname = uuid.New().String()
        }
    }
    
    for {
        select {
        case <-ctx.Done():
            o.releaseLeaderLock()
            return
        case <-ticker.C:
            // Try to acquire leader lock (fixed key name)
            _, err := o.kv.Create("wallet_oracle_leader", []byte(hostname))
            if err == nil {
                o.isLeader.Store(true)
                slog.Info("acquired oracle leader lock", "hostname", hostname)
            } else {
                o.isLeader.Store(false)
            }
        }
    }
}
```

**Ref**: Leader election pattern from Vault Todo #42

---

### 5. Update FlowSettler Code Examples

**Find**: FlowSettler CalculateDrip function (around line ~380)

**Add** after each drip calculation:

```go
// EVENT SOURCING: Record drip event
fs.Events = append(fs.Events, FlowDripEvent{
    FlowID: fs.FlowID,
    DripMicroRT: dripMicroRT,
    Timestamp: currentTime,
    IsComplete: false,
})
```

**Add** when flow completes:

```go
// EVENT SOURCING: Record completion event
fs.Events = append(fs.Events, FlowDripEvent{
    FlowID: fs.FlowID,
    DripMicroRT: remaining,
    Timestamp: currentTime,
    IsComplete: true,
})
```

---

### 6. Add New Section 13: API Endpoints

**Insert after**: "Phase Roadmap" section (around line ~1200)

**Content**:

## 13. API Endpoints

### 13.1 Wallet API

**POST `/api/v1/wallet/flow/initiate`**
- **Purpose**: Create new payment flow
- **Auth**: Falcon-1024 signature
- **Body**:
```json
{
  "recipient_address": "RT9z8y...",
  "amount_micro_rt": 50000000,
  "duration_preset": "default",
  "memo": "Payment for services"
}
```
- **Response**: `{ "flow_id": "550e8400...", "status": "pending_escrow" }`

**POST `/api/v1/wallet/flow/cancel`**
- **Purpose**: Cancel active flow (requires notice for >10 RT)
- **Auth**: Falcon-1024 signature
- **Body**:
```json
{
  "flow_id": "550e8400...",
  "notice_id": "uuid..." // Required for large flows
}
```

**GET `/api/v1/wallet/account/:address`**
- **Purpose**: Get account balance and state
- **Response**:
```json
{
  "address": "RT1a2b...",
  "stash_balance_micro_rt": 100000000,
  "locked_micro_rt": 50000000,
  "nonce": 42,
  "active_flows": ["550e8400...", "abc123..."]
}
```

---

### 7. Add New Section 14: Metrics & Observability

**Insert after**: "API Endpoints" section

**Content**:

## 14. Metrics & Observability

### 14.1 Prometheus Metrics

```go
type WalletMetrics struct {
    // Flows
    FlowsInitiatedTotal         prometheus.Counter
    FlowsCompletedTotal         prometheus.Counter
    FlowsCancelledTotal         prometheus.Counter
    FlowsActiveCurrent          prometheus.Gauge
    
    // Balances
    TotalStashBalanceMicroRT    prometheus.Gauge
    TotalLockedMicroRT          prometheus.Gauge
    
    // Oracle
    OracleTicksTotal            prometheus.Counter
    OracleDripsTotal            prometheus.Counter
    OracleDripAmountMicroRT     prometheus.Counter
    
    // Cancellation Notices
    CancellationNoticesPending  prometheus.Gauge
    CancellationNoticesMatured  prometheus.Counter
    
    // Performance
    FlowInitiationLatency       prometheus.Histogram
    OracleTickLatency           prometheus.Histogram
}
```

### 14.2 Grafana Dashboard

**Panels**:
1. Active flows (time series)
2. Total locked RT (gauge)
3. Oracle tick rate (graph)
4. Cancellation notices pending (table)
5. Flow completion rate (success/cancelled ratio)
6. Liquidity fronted vs recovered (balance chart)

**Alerts**:
- **High locked balance**: Alert if locked_micro_rt > 80% of stash_balance (liquidity risk)
- **Cancellation spike**: Alert if cancellations > 10% of flows in 1 hour (attack?)
- **Oracle lag**: Alert if oracle tick interval > 15 seconds (performance issue)

---

### 8. Add Appendix B: Vault Security Lessons Applied

**Insert after**: "Appendix A: Mathematical Foundations"

**Content**:

## Appendix B: Vault Security Lessons Applied

### B.1 Security Fixes Ported from Vault

| Vault Fix | Wallet Application |
|-----------|-------------------|
| **Fix #2: 30-day withdrawal notice** | Large flow cancellations require 30-day notice (>10 RT threshold) |
| **Fix #4: Yield to wallet only** | Flow drips go to recipient balance (not locked, prevents compounding) |
| **Fix #9: Event sourcing** | All flow state changes event sourced to PostgreSQL with sequence numbers |
| **Todo #42: Leader election** | Oracle uses NATS KV leader election (prevents duplicate ticks) |
| **Todo #43: PostgreSQL indexes** | flow_events table indexed on timestamp, nats_msg_id, (flow_id, sequence_number) |

### B.2 Why These Fixes Matter

**30-Day Cancellation Notice**:
- Prevents "flash cancellation" attacks (coordinated whale exits)
- Gives TorqVaults time to reallocate liquidity
- Small transactions (<10 RT) unaffected (instant cancellation)

**Event Sourcing**:
- Complete audit trail for disputes
- Crash recovery (rebuild state from events)
- Idempotency (NATS message deduplication via nats_msg_id)

**Leader Election**:
- Prevents duplicate oracle ticks (multiple instances)
- Auto-recovery on leader failure (TTL expiry)
- Hostname fallback for Docker (learned from Vault Todo #42 polish)

**Ref**: VAULT_SECURITY_FIXES_PART_2.md, VAULT_IMPLEMENTATION_PLAN.md

---

### 9. Add Appendix C: Deployment Checklist

**Insert after**: "Appendix B"

**Content**:

## Appendix C: Deployment Checklist

Before deploying Wallet service:

- [ ] Event sourcing tables created (flow_events, cancellation_notices)
- [ ] PostgreSQL indexes applied (timestamp, nats_msg_id)
- [ ] Falcon-1024 library integrated (liboqs-go)
- [ ] SPHINCS+ archival signatures working
- [ ] Oracle leader election tested (3+ instances)
- [ ] Flow drip calculation validated (Weibull math correct)
- [ ] 30-day cancellation notice enforcement tested
- [ ] Nonce replay prevention tested
- [ ] NATS idempotency working (duplicate message rejection)
- [ ] Prometheus metrics exported
- [ ] Grafana dashboard deployed
- [ ] Integration tests passing (flow lifecycle)
- [ ] E2E test: User → TorqVault → Oracle → Settlement
- [ ] Performance benchmark: 1000 flows/sec target
- [ ] Docker image builds successfully
- [ ] docker-compose up works (all services healthy)

**Critical Dependencies**:
- Vault service deployed (for TorqVault balances)
- BidNet service deployed (for escrow marketplace)
- Mint service deployed (for circulating supply data)
- NATS JetStream configured (KV buckets for leader election)

---

### 10. Update Phase Roadmap Section

**Find**: "Phase 1: Mock Settlement" (around line ~1130)

**Replace** checkboxes:

```markdown
### Phase 1: Mock Settlement (Testnet v0.1)

**Goals**:
- ✅ Implement account-based balances
- ✅ Implement fixed-point MicroRT arithmetic
- ✅ Implement Falcon-1024 signatures
- ⏳ Implement instant settlement (no flow drips yet)
- ⏳ Implement single TorqVault (no BidNet)
- ⏳ Implement event sourcing (PostgreSQL)

**Simplifications**:
- No flow drips (instant atomic transfers)
- No BidNet escrow (direct wallet → TorqVault)
- No Oracle ticks (TorqVault processes transfers immediately)

**Deliverable**: Functional payment system with quantum-safe signatures
```

---

## Summary of Updates

**Completed**:
- ✅ Header (vault-style format with related docs, achieved properties, tech stack)
- ✅ TOC (linked sections, reorganized)
- ✅ Fixed 1 Dilithium reference (flow initiation)

**Remaining** (9 major items):
1. Fix ~20 remaining Dilithium references (Falcon-1024 / SPHINCS+)
2. Add Section 8: Security Fixes (30-day notice, event sourcing, idempotency)
3. Add Section 11: Event Sourcing (schemas, replay, PostgreSQL)
4. Update Section 5: Oracle with leader election
5. Update FlowSettler: Add event sourcing to drip calculations
6. Add Section 13: API Endpoints
7. Add Section 14: Metrics & Observability
8. Add Appendix B: Vault Security Lessons
9. Add Appendix C: Deployment Checklist
10. Update Phase Roadmap: Fix crypto refs + checkboxes

**Estimated effort**: 2-3 hours manual editing OR 1 large replace operation

**Reference docs**:
- `src/vault/VAULT_IMPLEMENTATION_PLAN.md` (structure template)
- `src/vault/VAULT_SECURITY_FIXES_PART_2.md` (security patterns)
- `docs/crypto refactor docs/PHASE5_VERIFICATION_PLAN.md` (Falcon/SPHINCS+ docs)
