# Wallet Transaction Architecture

**Branch**: `feature/wallet` (future)  
**Date**: November 18, 2025  
**Status**: Design Complete, Implementation Pending  
**Related Docs**: 
- [VAULT_IMPLEMENTATION_PLAN.md](../vault/VAULT_IMPLEMENTATION_PLAN.md)  
- [VAULT_SECURITY_FIXES_PART_2.md](../vault/VAULT_SECURITY_FIXES_PART_2.md)  
- [docs/crypto refactor docs/PHASE5_VERIFICATION_PLAN.md](../../docs/crypto refactor docs/PHASE5_VERIFICATION_PLAN.md)

---

## Executive Summary

RoboTorq uses a **flow-based transaction model** where payments are time-shaped streams rather than atomic transfers. This design mirrors the physical reality of electricity flow and aligns with the system's core metaphor: "Watts > Wall Street."

**Achieved Properties**:
- ✅ Account-based balances (NOT UTXOs) - Users see one balance number
- ✅ Time-shaped payments - Weibull distributions control flow profiles
- ✅ **DistoDam settlement** - Covenant-enforced drips with cryptographic proof (NO centralized Oracle!)
- ✅ Flow locking - Sender's balance locked during active flows
- ✅ Fixed-point arithmetic - MicroRT (int64) prevents floating-point drift
- ✅ **Quantum-safe signatures** - Falcon-1024 (operational), SPHINCS+ (archival)
- ✅ **Event sourcing** - All transactions replayable (learned from Vault Fix #9)
- ✅ **Nonce-based replay prevention** - Incremental nonces prevent double-spend
- ✅ **30-day cancellation notice** - Inspired by Vault withdrawal notice security (Fix #2)
- ✅ **Economic security** - TorqVault collateral + slashing prevents drip failures

**This is NOT**:
- ❌ A UTXO model (no coin selection, no change addresses)
- ❌ Atomic transfers (payments drip over time)
- ❌ Blockchain-based (NATS message passing, not append-only ledger)
- ❌ Centralized Oracle (DistoDam ledger enforces settlement, TorqVaults execute)
- ❌ Instant settlement (Weibull drips over duration)

**Technology Stack**:
- Go 1.24
- NATS 2.10+ (pub/sub messaging)
- PostgreSQL (event sourcing, transaction history)
- Prometheus (metrics)
- slog (structured logging)
- **Falcon-1024** (fast signatures for real-time flows)
- **SPHINCS+** (archival signatures for long-term proofs)
- Docker + docker-compose

---

## Table of Contents

### Core Architecture
- [1. Core Concepts](#1-core-concepts)
- [2. Transaction Model](#2-transaction-model)
- [3. Flow-Based Payments](#3-flow-based-payments)
- [4. Weibull Distribution Profiles](#4-weibull-distribution-profiles)
- [5. Oracle Settlement](#5-oracle-settlement)
- [6. Account State Management](#6-account-state-management)
- [7. BidNet Escrow Integration](#7-bidnet-escrow-integration)

### Security & Integration
- [8. Security Fixes (Vault-Inspired)](#8-security-fixes-vault-inspired)
- [9. NATS Message Schemas](#9-nats-message-schemas)
- [10. Cryptography & Signatures](#10-cryptography--signatures)
- [11. Event Sourcing](#11-event-sourcing)

### Implementation
- [12. Phase Roadmap](#12-phase-roadmap)
- [13. API Endpoints](#13-api-endpoints)
- [14. Metrics & Observability](#14-metrics--observability)

### Appendices
- [Appendix A: Mathematical Foundations](#appendix-a-mathematical-foundations)
- [Appendix B: Vault Security Lessons Applied](#appendix-b-vault-security-lessons-applied)
- [Appendix C: Deployment Checklist](#appendix-c-deployment-checklist)

---

## 1. Core Concepts

### 1.1 Why Flow-Based Transactions?

**Physical Reality**: Electricity doesn't teleport from power plant to home—it flows over time through transmission lines. RoboTorq mirrors this:

```
Traditional Payment (Instant Transfer):
  Alice: 100 RT  →  Alice: 50 RT, Bob: 50 RT  (atomic swap)

RoboTorq Payment (Time-Shaped Flow):
  t=0s:    Alice: 100 RT (50 locked), Bob: 0 RT
  t=10s:   Alice: 100 RT (40 locked), Bob: 10 RT
  t=20s:   Alice: 100 RT (30 locked), Bob: 20 RT
  ...
  t=60s:   Alice: 50 RT, Bob: 50 RT  (flow complete)
```

**Benefits**:
1. **Natural UX** - Users understand "payment over time" (like electricity bills)
2. **Reversibility window** - Cancel flow before completion (fraud protection)
3. **Partial settlement** - Bob receives value even if Alice goes offline mid-flow
4. **Load balancing** - Network smooths payment traffic over time (no spikes)
5. **Economic signaling** - Flow shape reveals urgency (front-loaded = urgent, uniform = scheduled)

### 1.2 Account-Based vs UTXO

**RoboTorq Choice**: Account-based (like Ethereum, NOT Bitcoin)

| Feature | Account-Based (RoboTorq) | UTXO (Bitcoin) |
|---------|--------------------------|----------------|
| User sees | Single balance (100 RT) | List of coins (25 RT + 50 RT + 25 RT) |
| Sending RT | Lock portion of balance | Select coins, create change output |
| UX complexity | Low (one number) | High (coin management, change addresses) |
| Flow model | Natural (lock 50 RT for flow) | Awkward (flows don't map to discrete coins) |
| Smart contracts | Easy (account state) | Complex (stateless scripts) |

**Example**:
```go
// Account-based (RoboTorq)
type Account struct {
    Address      string
    BalanceMicroRT    int64  // 100,000,000 micro-RT = 100 RT
    LockedMicroRT     int64  // 50,000,000 locked in active flows
}

// UTXO-based (Bitcoin-style) - NOT USED
type UTXO struct {
    TxID         string
    OutputIndex  int
    AmountMicroRT int64
    ScriptPubKey []byte
}
```

### 1.3 MicroRT Fixed-Point Arithmetic

**Problem**: Floating-point math causes drift (0.1 + 0.2 ≠ 0.3 in IEEE 754)

**Solution**: Fixed-point integer arithmetic

```go
const MicroRTPerRT = 1_000_000  // 1 RT = 1,000,000 micro-RT

type MicroRT int64  // Range: -9.2 trillion to +9.2 trillion RT

// Conversions
func RTToMicroRT(rt float64) MicroRT {
    return MicroRT(rt * MicroRTPerRT)
}

func MicroRTToRT(microRT MicroRT) float64 {
    return float64(microRT) / MicroRTPerRT
}

// Arithmetic (no drift!)
balance := MicroRT(100_000_000)  // 100 RT
payment := MicroRT(50_000_000)   // 50 RT
balance -= payment               // 50 RT exactly
```

**Display**: UIs convert to float for humans (`50.000000 RT`), but all calculations use `int64`.

---

## 2. Transaction Model

### 2.1 Transaction Types

RoboTorq supports four transaction categories:

1. **Standard Payment Flow** - RT from Alice to Bob
2. **Demurrage Payment Flow** - Hourly wallet → TorqVault (special duration/shape)
3. **Currency Exchange Flow** - RT ↔ USD via TorqVault escrow
4. **Physical RT Redemption** - NFC tag → account balance (one-time, instant)

### 2.2 Transaction Lifecycle

```
1. Flow Initiation
   ↓
   User creates FlowRequest
   ↓
   Wallet validates (sufficient balance, no pending nonce conflict)
   ↓
   Wallet signs with Falcon-1024
   ↓
   Publish to bidnet.escrow.request

2. Escrow Bidding (BidNet Integration)
   ↓
   TorqVaults receive FlowRequest
   ↓
   Each TorqVault calculates fee (risk, liquidity, duration)
   ↓
   TorqVaults post 10% collateral (slashed if drips fail)
   ↓
   TorqVaults publish bids to bidnet.escrow.bid
   ↓
   Lowest fee wins (auction settles in <1 second)
   ↓
   Winner publishes bidnet.escrow.awarded

3. Flow Execution
   ↓
   Winning TorqVault fronts liquidity to recipient (instant credit)
   ↓
   TorqVault submits covenant to DistoDam with drip schedule
   ↓
   DistoDam creates merkle commitment for flow
   ↓
   Each drip interval: DistoDam enforces Weibull drip from sender → TorqVault
   ↓
   DistoDam signs each drip with SPHINCS+ (archival proof)
   ↓
   Flow completes after duration expires

4. Settlement
   ↓
   DistoDam publishes distodam.flow.complete (merkle proof of all drips)
   ↓
   All Wallets verify merkle root + SPHINCS+ signatures
   ↓
   If TorqVault executed correctly: collateral returned
   ↓
   If TorqVault failed: DistoDam slashes collateral, activates fallback TorqVault
   ↓
   Transaction complete (cryptographically proven)
```

### 2.3 Flow Cancellation

**Allowed during**: First 50% of flow duration (configurable)

```go
type FlowCancellation struct {
    FlowID        string
    CancellerID   string  // Must be sender
    Timestamp     time.Time
    Signature     []byte  // Falcon-1024
}

// Refund calculation
elapsed := time.Since(flow.StartTime)
if elapsed < flow.Duration * 0.5 {
    amountSettled := calculateWeibullDrip(elapsed, flow)
    amountRefunded := flow.AmountMicroRT - amountSettled
    
    // Sender receives refund minus cancellation fee (e.g., 1%)
    refund := amountRefunded * 0.99
}
```

**Use cases**:
- User sent to wrong address (caught within 30 seconds)
- Merchant disputes charge (refund window)
- Fraud detection (automatic cancellation)

---

## 3. Flow-Based Payments

### 3.1 FlowRequest Structure

```go
type FlowRequest struct {
    // Identity
    FlowID        string    // UUID v4
    Nonce         int64     // Incremental (prevents replay)
    
    // Participants
    SenderAddress   string  // Public key hash
    RecipientAddress string
    
    // Amount
    AmountMicroRT  int64    // 50,000,000 = 50 RT
    FeeMicroRT     int64    // Set by winning TorqVault bid
    
    // Time shaping
    DurationSec    int      // 3600 = 1 hour (default)
    ShapePreset    string   // "short" / "default" / "long"
    WeibullK       float64  // Calculated from preset (or custom)
    
    // Metadata
    Memo           string   // Optional (max 256 bytes)
    ContractID     string   // If payment linked to Digger contract
    
    // Cryptography
    Timestamp      time.Time
    Signature      []byte   // Falcon-1024 (sender's private key)
}
```

### 3.2 Duration Presets

Users select human-friendly presets (Wallet calculates Weibull parameters):

| Preset | Duration | Weibull k | Profile | Use Case |
|--------|----------|-----------|---------|----------|
| **short** | 10 minutes | 1.0 | Exponential decay (front-loaded) | Urgent payments, merchant transactions |
| **default** | 1 hour | 2.0 | Bell curve (peak at 30 min) | Standard peer-to-peer |
| **long** | 6 hours | 3.5 | Near-uniform (steady drip) | Scheduled payments, payroll |
| **custom** | User-defined | User-defined | Any Weibull curve | Advanced users, contract-defined |

**Example**:
```python
# User selects "short" preset in Wallet UI
preset = "short"

# Wallet translates to parameters
duration_sec = 600      # 10 minutes
weibull_k = 1.0         # Exponential decay
weibull_lambda = 600    # Scale parameter = duration

# Oracle will drip using P(t) = (k/λ) * (t/λ)^(k-1) * exp(-(t/λ)^k)
```

### 3.3 Front-Loaded vs Uniform Flows

**Front-Loaded (k=1.0, "short")**:
```
 RT/sec
   |  *
   |   *
   |    *
   |     *
   |      *___
   |          ----____
   +-------------------> time
   0s  2min  5min  10min
```
- **Use case**: Merchant payment (fast settlement reduces risk)
- **Example**: 50 RT over 10 min → 25 RT in first 2 min, 25 RT in remaining 8 min

**Bell Curve (k=2.0, "default")**:
```
 RT/sec
   |        /\
   |       /  \
   |      /    \
   |     /      \
   |    /        \
   |___/          \___
   +-------------------> time
   0s  15min  30min  60min
```
- **Use case**: Standard P2P payment (balanced risk)
- **Example**: 50 RT over 1 hour → peak drip at 30 min

**Uniform (k=3.5, "long")**:
```
 RT/sec
   |   __________
   |  /          \
   | /            \
   |/              \
   +-------------------> time
   0s  2hr  4hr  6hr
```
- **Use case**: Payroll, scheduled payments (predictable)
- **Example**: 50 RT over 6 hours → ~0.14 RT/min steady

---

## 4. Weibull Distribution Profiles

### 4.1 Mathematical Definition

The **Weibull probability density function (PDF)** controls the flow profile:

```
P(t) = (k/λ) * (t/λ)^(k-1) * exp(-(t/λ)^k)

where:
  t = time elapsed (seconds)
  k = shape parameter (controls profile: exponential, bell, uniform)
  λ = scale parameter (typically = duration for normalization)
```

**Drip calculation** (amount sent in time interval Δt):

```
Δ RT = Total_Amount * P(t) * Δt

where:
  Total_Amount = AmountMicroRT (e.g., 50 RT)
  Δt = 10 seconds (oracle tick interval)
```

### 4.2 Shape Parameter (k) Examples

**k = 1.0 (Exponential Decay)**:
```go
// Front-loaded (e.g., merchant payments)
P(t) = (1/λ) * exp(-t/λ)

// Example: 50 RT over 600 seconds (10 min)
// At t=0s:   P(0) = 1/600 = 0.00167 RT/sec → 0.0167 RT per 10s tick
// At t=300s: P(300) = 0.00091 RT/sec → 0.0091 RT per 10s tick
// At t=600s: P(600) = 0.00062 RT/sec → 0.0062 RT per 10s tick
```

**k = 2.0 (Rayleigh / Bell Curve)**:
```go
// Balanced (e.g., standard payments)
P(t) = (2/λ^2) * t * exp(-(t/λ)^2)

// Example: 50 RT over 3600 seconds (1 hour)
// At t=0s:    P(0) = 0 RT/sec
// At t=1800s: P(1800) = peak (maximum drip rate)
// At t=3600s: P(3600) ≈ 0 RT/sec
```

**k = 3.5 (Near-Uniform)**:
```go
// Steady (e.g., payroll)
P(t) = (3.5/λ) * (t/λ)^2.5 * exp(-(t/λ)^3.5)

// Example: 50 RT over 21600 seconds (6 hours)
// Drip rate stays near-constant from t=2hr to t=5hr
```

### 4.3 Implementation (FlowSettler Component)

```go
package settlement

import (
    "math"
    "time"
)

type FlowSettler struct {
    FlowID          string
    TotalMicroRT    int64
    DurationSec     int
    WeibullK        float64
    WeibullLambda   float64  // Usually = DurationSec
    StartTime       time.Time
    
    // State
    TotalDrippedMicroRT int64
    LastTickTime        time.Time
    
    // Event sourcing
    Events []FlowDripEvent
}

func (fs *FlowSettler) CalculateDrip(currentTime time.Time) int64 {
    elapsed := currentTime.Sub(fs.StartTime).Seconds()
    
    if elapsed >= float64(fs.DurationSec) {
        // Flow complete - drip remaining balance
        remaining := fs.TotalMicroRT - fs.TotalDrippedMicroRT
        fs.TotalDrippedMicroRT = fs.TotalMicroRT
        
        // EVENT SOURCING: Record completion event
        fs.Events = append(fs.Events, FlowDripEvent{
            FlowID: fs.FlowID,
            DripMicroRT: remaining,
            Timestamp: currentTime,
            IsComplete: true,
        })
        
        return remaining
    }
    
    // Weibull PDF at time t
    t := elapsed
    k := fs.WeibullK
    λ := fs.WeibullLambda
    
    // P(t) = (k/λ) * (t/λ)^(k-1) * exp(-(t/λ)^k)
    tOverLambda := t / λ
    pdf := (k / λ) * math.Pow(tOverLambda, k-1) * math.Exp(-math.Pow(tOverLambda, k))
    
    // Δt = time since last tick (typically 10 seconds)
    deltaT := currentTime.Sub(fs.LastTickTime).Seconds()
    
    // Δ RT = Total * P(t) * Δt
    dripMicroRT := int64(float64(fs.TotalMicroRT) * pdf * deltaT)
    
    // Update state
    fs.TotalDrippedMicroRT += dripMicroRT
    fs.LastTickTime = currentTime
    
    // EVENT SOURCING: Record drip event
    fs.Events = append(fs.Events, FlowDripEvent{
        FlowID: fs.FlowID,
        DripMicroRT: dripMicroRT,
        Timestamp: currentTime,
        IsComplete: false,
    })
    
    return dripMicroRT
}
```

### 4.4 Custom Flows (Advanced)

Power users can specify custom Weibull parameters:

```go
type CustomFlowRequest struct {
    FlowRequest  // Inherits standard fields
    
    // Override defaults
    WeibullK      float64  // Custom shape (e.g., 4.2 for sharp peak)
    WeibullLambda float64  // Custom scale (e.g., 1800 for midpoint shift)
}

// Example: Payment peaks at 15 minutes (not 30)
customFlow := CustomFlowRequest{
    AmountMicroRT: 50_000_000,  // 50 RT
    DurationSec:   3600,         // 1 hour
    WeibullK:      2.0,          // Bell curve
    WeibullLambda: 900,          // Scale = 15 min → peak shifts left
}
```

**Use cases**:
- Contract-defined flows (e.g., Digger payment schedules)
- Currency exchange hedging (front-load to lock rate)
- Custom business logic (tiered milestones)

---

## 5. DistoDam Settlement Layer

**NO centralized Oracle service** - DistoDam ledger enforces drips via covenants, TorqVaults execute.

### 5.1 Covenant Structure

When a TorqVault wins BidNet auction, it posts a **covenant** to DistoDam:

```go
type FlowCovenant struct {
    CovenantID    string    // Hash of all fields
    FlowID        string    // Links to original FlowRequest
    TorqVaultID   string    // Executor
    SenderAddr    string
    RecipientAddr string
    
    // Drip schedule (generated from Weibull parameters)
    DripSchedule  []DripInstruction
    
    // Economic security
    CollateralMicroRT int64   // 10% of TotalMicroRT
    SlashConditions   []string  // ["missed_drip", "wrong_amount", "late_execution"]
    
    // Cryptographic proof
    TorqVaultSignature []byte  // Falcon-1024 (operational)
    DistoDamSignature  []byte  // SPHINCS+ (archival commitment)
    
    Timestamp time.Time
}

type DripInstruction struct {
    SequenceNum   int       // Drip #1, #2, #3...
    DripMicroRT   int64     // Amount to drip (calculated from Weibull)
    ExecuteAtTime time.Time // When to execute
    MerkleProof   []byte    // Proof this drip is in schedule
}
```

**Covenant Creation**:
```
TorqVault wins BidNet auction
  ↓
Calculate complete Weibull drip schedule (e.g., 360 drips @ 10s each = 1 hour)
  ↓
Build merkle tree from all DripInstructions
  ↓
Sign covenant with Falcon-1024
  ↓
Publish to distodam.covenant.submit
  ↓
DistoDam validates:
  - Sender has sufficient balance
  - Collateral posted (10%)
  - Drip schedule sums to TotalMicroRT
  - TorqVault signature valid
  ↓
DistoDam counter-signs with SPHINCS+ (archival commitment)
  ↓
Covenant active, sender balance LOCKED
```

### 5.2 Drip Enforcement Cycle

**DistoDam enforces drips** - TorqVault executes but DistoDam verifies:

```
t=0s:   Covenant active, sender's balance locked
        ↓
t=10s:  Drip #1 scheduled
        ↓ TorqVault publishes distodam.drip.execute
        ↓ Message contains: FlowID, SequenceNum=1, DripMicroRT, MerkleProof
        ↓ DistoDam validates:
        ↓   - Drip matches covenant schedule (amount, time)
        ↓   - MerkleProof valid (drip is in tree)
        ↓   - Sequence number correct (no skipped drips)
        ↓ If valid: Update ledger (sender.locked -= Δ₁, recipient.balance += Δ₁)
        ↓ If invalid: Mark TorqVault for slashing
        ↓ DistoDam signs drip execution with SPHINCS+ (permanent record)
        ↓
t=20s:  Drip #2 scheduled
        ↓ (repeat validation)
        ↓
...
        ↓
t=3600s: Final drip (#360)
        ↓ DistoDam validates complete execution
        ↓ If all drips successful: Return collateral to TorqVault
        ↓ If any drips failed: Slash collateral, activate fallback TorqVault
        ↓ Publish distodam.flow.complete (merkle root of all drips)
```

**Key Difference from Centralized Oracle**:
- **Oracle**: Centralized service calculates and publishes drips (single point of failure)
- **DistoDam**: Decentralized verification - TorqVaults compete to execute, DistoDam enforces correctness

### 5.3 Slashing Mechanism

**Collateral Requirements**:
- TorqVault posts **10% of TotalMicroRT** as collateral when covenant created
- Collateral held by DistoDam for flow duration
- Returned if execution perfect, slashed if failures occur

**Slashing Conditions**:

```go
type SlashCondition int

const (
    MissedDrip SlashCondition = iota  // Drip not executed within 30s window
    WrongAmount                        // Drip amount doesn't match covenant
    LateExecution                      // Drip >30s late (partial slash)
    InvalidProof                       // Merkle proof verification failed
    SkippedSequence                    // Tried to skip drip #N
)

type SlashingRules struct {
    MissedDrip:      100%  // Full collateral slashed
    WrongAmount:     100%  // Full collateral slashed
    LateExecution:   10%   // Partial slash per late drip
    InvalidProof:    100%  // Full collateral slashed
    SkippedSequence: 100%  // Full collateral slashed
}
```

**Slashing Process**:
```
TorqVault fails to execute Drip #23 within 30s window
  ↓
DistoDam detects timeout (monitors covenant schedules)
  ↓
DistoDam publishes distodam.slash.alert
  ↓ Message: FlowID, TorqVaultID, SlashReason="MissedDrip", SlashedAmount
  ↓
DistoDam transfers slashed collateral:
  ↓ 50% to sender (compensation for delayed payment)
  ↓ 50% burned (penalty for network unreliability)
  ↓
DistoDam activates **fallback TorqVault** (2nd place bidder from BidNet)
  ↓ Fallback TorqVault receives covenant with remaining drips (#24-360)
  ↓ Flow continues without interruption
```

### 5.4 Fallback TorqVault Activation

**BidNet stores top 3 bidders**:
```go
type BidNetAuction struct {
    FlowID       string
    WinnerBid    TorqVaultBid  // Primary executor
    FallbackBids []TorqVaultBid  // 2nd and 3rd place (sorted by fee)
}
```

**Activation on Primary Failure**:
```
Primary TorqVault slashed
  ↓
DistoDam publishes distodam.fallback.activate
  ↓ Message: FlowID, NewExecutorID (2nd place bidder), RemainingDrips
  ↓
Fallback TorqVault receives covenant transfer
  ↓ Must post NEW collateral (10% of remaining amount)
  ↓ Continues execution from next drip
  ↓
If fallback also fails: Activate 3rd place bidder
  ↓
If all fallbacks exhausted: Emergency mode (refund sender minus completed drips)
```

**Economic Incentives**:
- TorqVaults compete on reliability (missed drips = reputation damage + slashing)
- Fallback bidders earn fees if primary fails (incentive to monitor)
- Sender gets compensation (50% of slashed collateral) for delays

### 5.5 DistoDam Message Schemas

**Covenant Submission** (`distodam.covenant.submit`):
```go
type CovenantSubmission struct {
    Covenant        FlowCovenant
    CollateralProof []byte  // Merkle proof of collateral deposit
}
```

**Drip Execution** (`distodam.drip.execute`):
```go
type DripExecution struct {
    FlowID        string
    SequenceNum   int
    DripMicroRT   int64
    ExecutedAtTime time.Time
    MerkleProof   []byte      // Proves drip is in covenant schedule
    TorqVaultSignature []byte  // Falcon-1024
}
```

**Drip Confirmation** (`distodam.drip.confirmed`):
```go
type DripConfirmation struct {
    FlowID          string
    SequenceNum     int
    LedgerUpdate    LedgerUpdate  // Updated balances
    DistoDamSignature []byte       // SPHINCS+ (archival proof)
}
```

**Slashing Alert** (`distodam.slash.alert`):
```go
type SlashAlert struct {
    FlowID          string
    TorqVaultID     string
    SlashReason     SlashCondition
    SlashedMicroRT  int64
    SenderCompensation int64  // 50% of slashed amount
    BurnedAmount       int64  // 50% of slashed amount
    DistoDamSignature  []byte  // SPHINCS+ proof
}
```

**Flow Completion** (`distodam.flow.complete`):
```go
type FlowCompletion struct {
    FlowID              string
    TotalDripsExecuted  int
    MerkleRootAllDrips  []byte  // Root of all drip execution proofs
    CollateralReturned  bool    // True if no slashing occurred
    FinalBalances       map[string]int64  // All affected accounts
    DistoDamSignature   []byte  // SPHINCS+ (archival settlement proof)
}
```

**All TorqVaults** subscribe to `vault.ledger.update` and apply updates to distributed ledger:

```go
func (tv *TorqVaultService) handleLedgerUpdate(msg *nats.Msg) {
    var update LedgerUpdate
    json.Unmarshal(msg.Data, &update)
    
    // Verify Oracle signature (prevent tampering)
    if !verifyOracleSignature(update) {
        tv.logger.Error("invalid oracle signature", "flow_id", update.FlowID)
        return
    }
    
    // Apply update to distributed ledger
    tv.mu.Lock()
    defer tv.mu.Unlock()
    
    sender := tv.memberVaultBalances[update.SenderAddr]
    recipient := tv.memberVaultBalances[update.RecipientAddr]
    
    // Unlock from sender, credit to recipient
    atomic.AddInt64(&sender.StashBalanceMicroRT, -update.DripMicroRT)
    atomic.AddInt64(&sender.LockedMicroRT, -update.DripMicroRT)
    atomic.AddInt64(&recipient.StashBalanceMicroRT, update.DripMicroRT)
    
    tv.logger.Info("ledger update applied",
        "flow_id", update.FlowID,
        "drip_micro_rt", update.DripMicroRT)
}
```

### 5.5 Oracle Redundancy & Fault Tolerance

**Problem**: Single Oracle = single point of failure

**Solution** (Phase 2+): Multi-Oracle consensus

```go
type OraclePool struct {
    oracles       []*FlowOracle  // 3-5 Oracle instances
    quorumSize    int            // 2 out of 3 (majority)
}

// Each Oracle calculates drip independently
// TorqVaults accept update if 2+ Oracles agree (Byzantine Fault Tolerance)
```

**Phase 1 simplification**: Single Oracle (acceptable for testnet)

---

## 6. Account State Management

### 6.1 Account Structure

```go
type Account struct {
    // Identity
    Address       string  // Falcon-1024 public key hash (32 bytes)
    Nonce         int64   // Incremental (prevents replay attacks)
    
    // Balances (MicroRT)
    StashBalanceMicroRT   int64  // Available for spending
    PledgeBalanceMicroRT  int64  // Staked in DistoDam (yield-earning)
    LockedMicroRT         int64  // Locked in active outgoing flows
    
    // Metadata
    CreatedAt     time.Time
    LastActivity  time.Time
    
    // Cryptography
    PublicKey     []byte  // Falcon-1024 (for signature verification)
}

// Available balance = StashBalanceMicroRT - LockedMicroRT
// Total net worth = StashBalanceMicroRT + PledgeBalanceMicroRT
```

### 6.2 Balance Locking (Flow Initiation)

**When user sends FlowRequest**:

```go
func (w *Wallet) InitiateFlow(req FlowRequest) error {
    account := w.getAccount(req.SenderAddress)
    
    // Check sufficient available balance
    availableMicroRT := account.StashBalanceMicroRT - account.LockedMicroRT
    totalNeeded := req.AmountMicroRT + req.FeeMicroRT
    
    if availableMicroRT < totalNeeded {
        return ErrInsufficientBalance
    }
    
    // Lock balance (prevents double-spend)
    atomic.AddInt64(&account.LockedMicroRT, totalNeeded)
    
    // Publish FlowRequest to BidNet
    w.natsClient.Publish("bidnet.escrow.request", req.ToJSON())
    
    w.logger.Info("flow initiated",
        "flow_id", req.FlowID,
        "amount_micro_rt", req.AmountMicroRT,
        "locked_micro_rt", account.LockedMicroRT)
    
    return nil
}
```

### 6.3 Balance Unlocking (Flow Completion)

**After Oracle drips final tick**:

```go
func (tv *TorqVaultService) finalizeFlow(flowID string) {
    flow := tv.activeEscrows[flowID]
    
    // Unlock sender's remaining balance (should be 0 if flow completed)
    sender := tv.memberVaultBalances[flow.FromMember]
    remaining := flow.AmountMicroRT - flow.TotalDrippedMicroRT
    
    if remaining > 0 {
        // Partial completion (e.g., cancelled flow)
        atomic.AddInt64(&sender.LockedMicroRT, -remaining)
    }
    
    // Publish final ledger update
    finalUpdate := LedgerUpdate{
        FlowID:     flowID,
        SenderAddr: flow.FromMember,
        Status:     "complete",
        Timestamp:  time.Now(),
    }
    
    tv.natsClient.Publish("vault.ledger.update", finalUpdate.ToJSON())
    
    delete(tv.activeEscrows, flowID)
}
```

### 6.4 Nonce Management (Replay Prevention)

**Every FlowRequest increments nonce**:

```go
type FlowRequest struct {
    Nonce int64  // Must be account.Nonce + 1
    // ... other fields
}

// Validation
func (tv *TorqVaultService) validateNonce(req FlowRequest) error {
    account := tv.memberVaultBalances[req.SenderAddress]
    
    if req.Nonce != account.Nonce + 1 {
        return ErrInvalidNonce
    }
    
    // Increment nonce (accept request)
    atomic.AddInt64(&account.Nonce, 1)
    return nil
}
```

**Prevents**:
- Replay attacks (can't resubmit old FlowRequest)
- Double-spend (only one pending flow per nonce)

---

## 7. BidNet Escrow Integration

### 7.1 Why BidNet for Payments?

**Original BidNet design**: Marketplace for Digger contract execution

**Extension**: Use same marketplace for payment settlement

**Benefits**:
1. **No account lock-in** - Users not tied to specific TorqVault
2. **Market-driven fees** - Competition reduces costs
3. **Liquidity provision** - TorqVaults front RT to recipients (instant credit)
4. **Risk pricing** - Vaults bid based on sender reputation, flow duration

### 7.2 Escrow Lifecycle

```
1. User publishes FlowRequest → bidnet.escrow.request
   ↓
2. TorqVaults evaluate:
   - Sender reputation (has completed flows without cancellation?)
   - Flow duration (longer = higher risk)
   - Current liquidity (can we front this amount?)
   ↓
3. Each TorqVault calculates fee:
   fee = base_fee + risk_premium + liquidity_cost
   ↓
4. TorqVaults publish bids → bidnet.escrow.bid
   ↓
5. Lowest fee wins (auction settles in <1 second)
   ↓
6. Winner publishes bidnet.escrow.awarded
   ↓
7. Winner fronts liquidity (instant credit to recipient)
   ↓
8. Oracle drips RT from sender's locked balance to TorqVault
   ↓
9. Flow completes, TorqVault publishes final ledger update
```

### 7.3 Escrow Bid Structure

```go
type EscrowBid struct {
    FlowID        string
    TorqVaultID   string
    FeeMicroRT    int64     // Bid amount
    
    // NEW: Collateral commitment (10% of flow amount)
    CollateralMicroRT int64  // Locked if bid wins
    
    Timestamp     time.Time
    Signature     []byte    // TorqVault's Falcon-1024 signature
}

// Example fee calculation
func (tv *TorqVaultService) calculateFee(req FlowRequest) int64 {
    baseFee := req.AmountMicroRT * 0.001  // 0.1% base
    
    // Risk premium (sender history)
    senderRep := tv.getSenderReputation(req.SenderAddress)
    riskPremium := baseFee * (1.0 - senderRep)  // Bad rep = higher fee
    
    // Liquidity cost (do we have enough RT available?)
    liquidityNeeded := req.AmountMicroRT
    availableLiquidity := atomic.LoadInt64(&tv.availableLiquidityMicroRT)
    
    liquidityCost := 0.0
    if liquidityNeeded > availableLiquidity * 0.5 {
        liquidityCost = baseFee * 0.5  // Charge extra if low liquidity
    }
    
    totalFee := baseFee + riskPremium + liquidityCost
    return int64(totalFee)
}
```

### 7.4 Collateral Posting (Economic Security)

**NEW**: TorqVaults post **10% collateral** when they win BidNet auction.

```go
func (tv *TorqVaultService) awardEscrow(flowID string) error {
    flow := tv.activeEscrows[flowID]
    
    // Calculate required collateral (10% of flow amount)
    requiredCollateral := flow.AmountMicroRT / 10
    
    // Check TorqVault has sufficient collateral
    if atomic.LoadInt64(&tv.collateralPoolMicroRT) < requiredCollateral {
        return errors.New("insufficient collateral - cannot execute flow")
    }
    
    // Lock collateral (held until flow completes)
    atomic.AddInt64(&tv.collateralPoolMicroRT, -requiredCollateral)
    atomic.AddInt64(&tv.lockedCollateralMicroRT, requiredCollateral)
    
    // Publish covenant to DistoDam
    covenant := FlowCovenant{
        CovenantID:    generateCovenantID(flowID),
        FlowID:        flowID,
        TorqVaultID:   tv.id,
        SenderAddr:    flow.FromMember,
        RecipientAddr: flow.ToMember,
        DripSchedule:  generateWeibullDripSchedule(flow),
        CollateralMicroRT: requiredCollateral,
        SlashConditions: []string{"missed_drip", "wrong_amount", "late_execution"},
    }
    
    // Sign covenant with Falcon-1024
    covenant.TorqVaultSignature = tv.signWithFalcon(covenant)
    
    // Submit to DistoDam
    tv.natsClient.Publish("distodam.covenant.submit", covenant.ToJSON())
    
    tv.logger.Info("collateral posted",
        "flow_id", flowID,
        "collateral_micro_rt", requiredCollateral,
        "locked_collateral_total", tv.lockedCollateralMicroRT)
    
    return nil
}
```

**Why Collateral?**
- **Prevents drip failures**: TorqVaults lose money if they miss drips
- **No centralized Oracle**: DistoDam enforces, TorqVaults execute (economic incentive ensures reliability)
- **User compensation**: If TorqVault fails, 50% of slashed collateral goes to sender

### 7.5 Slashing on Drip Failures

**DistoDam monitors covenant execution** - if TorqVault fails, collateral is slashed:

```go
// TorqVault receives slashing alert from DistoDam
func (tv *TorqVaultService) handleSlashAlert(alert SlashAlert) {
    flow := tv.activeEscrows[alert.FlowID]
    
    // DistoDam has already slashed collateral (trustless enforcement)
    // TorqVault updates local state
    atomic.AddInt64(&tv.lockedCollateralMicroRT, -alert.SlashedMicroRT)
    
    tv.logger.Error("collateral slashed",
        "flow_id", alert.FlowID,
        "slash_reason", alert.SlashReason,
        "slashed_micro_rt", alert.SlashedMicroRT,
        "remaining_collateral", tv.lockedCollateralMicroRT)
    
    // Update reputation (slashing damages trust score)
    tv.reputationScore -= 0.1  // Lose 10% reputation per slash
    
    // Remove flow from active escrows (fallback TorqVault now handling)
    delete(tv.activeEscrows, alert.FlowID)
}
```

**Slashing Distribution**:
- **50%** to sender (compensation for delayed/failed payment)
- **50%** burned (penalty for network unreliability)

**Impact on TorqVault**:
- **Reputation damage**: Lower reputation = fewer bid wins in future auctions
- **Capital loss**: Collateral slashed, not recoverable
- **Opportunity cost**: Could have earned fees on successful flows

### 7.6 Reputation System

**Track TorqVault reliability** - successful flows increase trust score, slashing decreases:

```go
type TorqVaultReputation struct {
    TorqVaultID         string
    ReputationScore     float64  // 0.0 (worst) to 1.0 (perfect)
    FlowsCompleted      int64
    FlowsSlashed        int64
    TotalCollateralLost int64
    AvgDripLatency      time.Duration  // How fast do they execute?
}

func (tv *TorqVaultService) updateReputationOnSuccess(flowID string) {
    tv.mu.Lock()
    defer tv.mu.Unlock()
    
    tv.reputationScore = math.Min(1.0, tv.reputationScore + 0.01)  // +1% per success
    tv.flowsCompleted++
    
    tv.logger.Info("reputation increased",
        "flow_id", flowID,
        "new_reputation", tv.reputationScore,
        "flows_completed", tv.flowsCompleted)
}

func (tv *TorqVaultService) updateReputationOnSlash(alert SlashAlert) {
    tv.mu.Lock()
    defer tv.mu.Unlock()
    
    tv.reputationScore = math.Max(0.0, tv.reputationScore - 0.1)  // -10% per slash
    tv.flowsSlashed++
    tv.totalCollateralLost += alert.SlashedMicroRT
    
    tv.logger.Error("reputation damaged",
        "flow_id", alert.FlowID,
        "new_reputation", tv.reputationScore,
        "total_slashes", tv.flowsSlashed)
}
```

**How Reputation Affects Bids**:
- **High reputation (0.9-1.0)**: Users trust this TorqVault → more likely to manually select it → more bid wins
- **Medium reputation (0.5-0.9)**: Competes normally in auctions
- **Low reputation (<0.5)**: Users avoid → fewer bid wins → less revenue → economic pressure to improve or exit

**Public Visibility**:
- Reputation scores published to `bidnet.vault.reputation` topic
- Users can view TorqVault track records before accepting bids
- Reputation becomes competitive advantage (high-rep vaults charge premium fees)
```

### 7.7 Instant Recipient Credit (Liquidity Fronting)

**Problem**: If Oracle drips over 1 hour, recipient waits 1 hour for full amount

**Solution**: TorqVault fronts RT immediately (recipient sees balance instantly)

```go
func (tv *TorqVaultService) awardEscrow(flowID string) {
    flow := tv.activeEscrows[flowID]
    
    // Deduct from TorqVault's liquidity pool
    atomic.AddInt64(&tv.availableLiquidityMicroRT, -flow.AmountMicroRT)
    
    // Credit recipient immediately
    recipient := tv.memberVaultBalances[flow.ToMember]
    atomic.AddInt64(&recipient.StashBalanceMicroRT, flow.AmountMicroRT)
    
    tv.logger.Info("recipient credited instantly",
        "flow_id", flowID,
        "amount_micro_rt", flow.AmountMicroRT,
        "recipient", flow.ToMember)
    
    // TorqVault will recover liquidity as Oracle drips from sender
}
```

**As Oracle drips**:
```go
// Oracle tick: sender → TorqVault (recovering fronted liquidity)
dripMicroRT := calculateWeibullDrip(elapsed, flow)
atomic.AddInt64(&tv.availableLiquidityMicroRT, dripMicroRT)
```

**Net effect**: Recipient gets instant credit, TorqVault recovers liquidity over time.

---

## 8. Security Fixes (Vault-Inspired)

### Security Fix #1: 30-Day Cancellation Notice (Large Flows)

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

---

### Security Fix #2: Event Sourcing + Sequence Numbers

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

---

### Security Fix #3: Nonce-Based Idempotency (NATS Messages)

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

## 9. NATS Message Schemas

### 9.1 Flow Lifecycle Topics

| Topic | Publisher | Subscriber | Purpose |
|-------|-----------|------------|---------|
| `bidnet.escrow.request` | Wallet | TorqVaults | User initiates payment flow |
| `bidnet.escrow.bid` | TorqVaults | BidNet Coordinator | TorqVaults submit fee bids + collateral |
| `bidnet.escrow.awarded` | BidNet Coordinator | All (winner, sender, recipient) | Announce winning bid |
| `distodam.covenant.submit` | TorqVault (winner) | DistoDam | Submit drip schedule + collateral proof |
| `distodam.drip.execute` | TorqVault | DistoDam | Execute scheduled drip (with merkle proof) |
| `distodam.drip.confirmed` | DistoDam | All Wallets | DistoDam confirms drip valid (updates ledger) |
| `distodam.slash.alert` | DistoDam | All (BidNet, TorqVaults) | TorqVault slashed for failed drip |
| `distodam.fallback.activate` | DistoDam | Fallback TorqVault | Primary failed, activate 2nd place bidder |
| `distodam.flow.complete` | DistoDam | All (sender, recipient, TorqVault) | Flow complete, merkle proof published |
| `flow.cancelled` | Wallet or TorqVault | DistoDam, TorqVaults | Cancel active flow (refund) |

### 8.2 Message Schemas

#### 8.2.1 FlowRequest (`bidnet.escrow.request`)

```json
{
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "nonce": 42,
  "sender_address": "RT1a2b3c4d5e6f7g8h9i0j...",
  "recipient_address": "RT9z8y7x6w5v4u3t2s1r...",
  "amount_micro_rt": 50000000,
  "fee_micro_rt": 0,
  "duration_sec": 3600,
  "shape_preset": "default",
  "weibull_k": 2.0,
  "memo": "Payment for services rendered",
  "contract_id": "",
  "timestamp": "2025-11-18T22:30:00Z",
  "signature": "base64-encoded-falcon-1024-signature..."
}
```

#### 9.2.2 EscrowBid (`bidnet.escrow.bid`)

```json
{
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "torqvault_id": "torqvault-alpha-001",
  "fee_micro_rt": 50000,
  "collateral_micro_rt": 5000000,
  "timestamp": "2025-11-18T22:30:00.500Z",
  "signature": "base64-encoded-falcon-1024-signature..."
}
```

#### 9.2.3 EscrowAwarded (`bidnet.escrow.awarded`)

```json
{
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "winner_torqvault_id": "torqvault-beta-002",
  "winning_fee_micro_rt": 45000,
  "fallback_torqvaults": [
    {"torqvault_id": "torqvault-gamma-003", "fee_micro_rt": 47000},
    {"torqvault_id": "torqvault-delta-004", "fee_micro_rt": 48000}
  ],
  "timestamp": "2025-11-18T22:30:01Z",
  "coordinator_signature": "base64-encoded-falcon-1024-signature..."
}
```

#### 9.2.4 CovenantSubmission (`distodam.covenant.submit`)

```json
{
  "covenant_id": "covenant-550e8400-e29b-41d4",
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "torqvault_id": "torqvault-beta-002",
  "sender_address": "RT1a2b3c4d5e6f7g8h9i0j...",
  "recipient_address": "RT9z8y7x6w5v4u3t2s1r...",
  "drip_schedule": [
    {"sequence_num": 1, "drip_micro_rt": 138888, "execute_at_time": "2025-11-18T22:30:10Z"},
    {"sequence_num": 2, "drip_micro_rt": 139200, "execute_at_time": "2025-11-18T22:30:20Z"}
  ],
  "merkle_root_schedule": "base64-encoded-sha256-merkle-root...",
  "collateral_micro_rt": 5000000,
  "slash_conditions": ["missed_drip", "wrong_amount", "late_execution"],
  "torqvault_signature": "base64-encoded-falcon-1024-signature...",
  "timestamp": "2025-11-18T22:30:02Z"
}
```

#### 9.2.5 DripExecution (`distodam.drip.execute`)

```json
{
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "sequence_num": 1,
  "drip_micro_rt": 138888,
  "executed_at_time": "2025-11-18T22:30:10.050Z",
  "merkle_proof": ["base64-hash-1", "base64-hash-2", "base64-hash-3"],
  "torqvault_signature": "base64-encoded-falcon-1024-signature..."
}
```

#### 9.2.6 DripConfirmation (`distodam.drip.confirmed`)

```json
{
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "sequence_num": 1,
  "ledger_update": {
    "sender_address": "RT1a2b3c4d5e6f7g8h9i0j...",
    "recipient_address": "RT9z8y7x6w5v4u3t2s1r...",
    "drip_micro_rt": 138888,
    "sender_new_balance_micro_rt": 49861112,
    "recipient_new_balance_micro_rt": 138888
  },
  "distodam_signature": "base64-encoded-sphincs-plus-signature...",
  "timestamp": "2025-11-18T22:30:10.100Z"
}
```

#### 9.2.7 SlashAlert (`distodam.slash.alert`)

```json
{
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "torqvault_id": "torqvault-beta-002",
  "slash_reason": "missed_drip",
  "missed_sequence_num": 23,
  "slashed_micro_rt": 5000000,
  "sender_compensation_micro_rt": 2500000,
  "burned_micro_rt": 2500000,
  "fallback_torqvault_id": "torqvault-gamma-003",
  "distodam_signature": "base64-encoded-sphincs-plus-signature...",
  "timestamp": "2025-11-18T22:34:40Z"
}
```

#### 9.2.8 FlowCompletion (`distodam.flow.complete`)

```json
{
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "total_drips_executed": 360,
  "merkle_root_all_drips": "base64-encoded-sha256-merkle-root...",
  "collateral_returned": true,
  "final_balances": {
    "RT1a2b3c4d5e6f7g8h9i0j...": 0,
    "RT9z8y7x6w5v4u3t2s1r...": 50000000
  },
  "distodam_signature": "base64-encoded-sphincs-plus-signature...",
  "timestamp": "2025-11-18T23:30:10Z"
}
```

#### 9.2.9 FlowCancellation (`flow.cancelled`)

```json
{
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "canceller_address": "RT1a2b3c4d5e6f7g8h9i0j...",
  "reason": "incorrect recipient address",
  "elapsed_sec": 120,
  "amount_settled_micro_rt": 2777777,
  "refund_micro_rt": 46222222,
  "timestamp": "2025-11-18T22:32:00Z",
  "signature": "base64-encoded-falcon-1024-signature..."
}
```

### 9.3 Subscription Patterns

**Wallet**:
```go
// Subscribe to awarded flows (confirm acceptance)
nc.Subscribe("bidnet.escrow.awarded", handleEscrowAwarded)

// Subscribe to drip confirmations (track flow progress)
nc.Subscribe("distodam.drip.confirmed", handleDripConfirmed)

// Subscribe to flow completion (final settlement proof)
nc.Subscribe("distodam.flow.complete", handleFlowComplete)

// Subscribe to cancellation confirmations
nc.Subscribe("flow.cancelled", handleFlowCancelled)
```

**TorqVault**:
```go
// Subscribe to flow requests (participate in auctions)
nc.Subscribe("bidnet.escrow.request", handleEscrowRequest)

// Subscribe to auction results (did we win?)
nc.Subscribe("bidnet.escrow.awarded", handleEscrowAwarded)

// Subscribe to drip confirmations (track ledger updates)
nc.Subscribe("distodam.drip.confirmed", handleDripConfirmed)

// Subscribe to slashing alerts (did we fail execution?)
nc.Subscribe("distodam.slash.alert", handleSlashAlert)

// Subscribe to fallback activations (backup executor)
nc.Subscribe("distodam.fallback.activate", handleFallbackActivation)

// Subscribe to flow completions (release collateral)
nc.Subscribe("distodam.flow.complete", handleFlowComplete)

// Subscribe to cancellations (refund locked balances)
nc.Subscribe("flow.cancelled", handleFlowCancelled)
```

**DistoDam**:
```go
// Subscribe to covenant submissions (new flow initiated)
nc.Subscribe("distodam.covenant.submit", handleCovenantSubmission)

// Subscribe to drip executions (validate TorqVault drips)
nc.Subscribe("distodam.drip.execute", handleDripExecution)

// Subscribe to cancellations (stop covenant enforcement)
nc.Subscribe("flow.cancelled", handleFlowCancelled)
```

---

## 10. Cryptography & Signatures

### 10.1 Quantum-Safe Signatures

**Algorithms**: NIST PQC - Falcon-1024 (operational), SPHINCS+ (archival)

```go
import "github.com/open-quantum-safe/liboqs-go/oqs"

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

**RoboTorq choice**: 
- **Falcon-1024** for user transactions + TorqVault bids (speed, compact signatures)
- **SPHINCS+** for DistoDam settlement proofs (stateless, long-term archival security)

### 10.2 Signature Requirements by Actor

| Actor | Operation | Algorithm | Reason |
|-------|-----------|-----------|--------|
| **Wallet** | FlowRequest, FlowCancellation | Falcon-1024 | Fast user operations |
| **TorqVault** | EscrowBid, CovenantSubmission, DripExecution | Falcon-1024 | Operational signatures (speed matters) |
| **DistoDam** | DripConfirmation, SlashAlert, FlowCompletion | SPHINCS+ | Archival proofs (long-term verification) |

**Why DistoDam uses SPHINCS+**:
- Settlement proofs must be verifiable for DECADES (tax audits, legal disputes)
- SPHINCS+ is stateless (no state corruption risk)
- SPHINCS+ has strongest security guarantees (only relies on hash functions)
- TorqVaults use Falcon-1024 for speed (operational), DistoDam provides archival layer with SPHINCS+

### 10.3 Signature Verification Flow

```go
// TorqVault verifies user's FlowRequest (Falcon-1024)
func (tv *TorqVaultService) verifyFlowRequest(req FlowRequest) error {
    verifier := oqs.Signature{}
    verifier.Init("Falcon-1024", nil)
    defer verifier.Clean()
    
    // Load user's public key from ledger
    publicKey := tv.ledger.GetPublicKey(req.SenderAddress)
    
    // Verify signature
    isValid, err := verifier.Verify(req.ToBytes(), req.Signature, publicKey)
    if err != nil || !isValid {
        return errors.New("invalid signature on FlowRequest")
    }
    
    return nil
}

// Wallet verifies DistoDam's settlement proof (SPHINCS+)
func (w *Wallet) verifyFlowCompletion(completion FlowCompletion) error {
    verifier := oqs.Signature{}
    verifier.Init("SPHINCS+-SHAKE-256f-simple", nil)  // Archival verification
    defer verifier.Clean()
    
    // Load DistoDam's public key (well-known, baked into genesis)
    publicKey := w.config.DistoDamPublicKey
    
    // Verify SPHINCS+ signature
    isValid, err := verifier.Verify(completion.ToBytes(), completion.DistoDamSignature, publicKey)
    if err != nil || !isValid {
        return errors.New("invalid DistoDam settlement proof")
    }
    
    return nil
}
```

### 10.2 Address Derivation

**Account address** = `RT` + Base58(SHA256(Falcon_PublicKey))

```go
func DeriveAddress(publicKey []byte) string {
    hash := sha256.Sum256(publicKey)
    encoded := base58.Encode(hash[:])
    return "RT" + encoded
}

// Example address:
// RT1a2b3c4d5e6f7g8h9i0jK1L2M3N4O5P6Q7R8S9T0U
```

**Benefits**:
- **Collision-resistant** (SHA256 = 256-bit hash space)
- **Human-readable** (Base58 = no ambiguous chars like 0/O, 1/l)
- **Prefix-distinguishable** (RT = RoboTorq, vs BTC, ETH)

### 10.3 Nonce-Based Replay Prevention

**Every account maintains nonce** (increments with each FlowRequest):

```go
type Account struct {
    Nonce int64  // Current: 42
}

type FlowRequest struct {
    Nonce int64  // Must be: 43 (current + 1)
}

// Validation
if req.Nonce != account.Nonce + 1 {
    return ErrInvalidNonce  // Reject (replay attack or out-of-order)
}

// Accept and increment
account.Nonce += 1
```

**Prevents**:
- Replaying old signed transactions
- Double-spending via duplicate requests

### 10.4 Certificate Verification with Mint

**Problem**: Wallet receives Phase3RoboTorqUnit certificates from DistoDam - how to verify they're legitimate?

**Solution**: Query Mint's verification API (POST `/verify/certificate`) with merkle_root

**Architecture**:
```
DistoDam distributes certificate to Wallet
  ↓
Wallet receives certificate via NATS (wallet.distribution)
  ↓
Wallet IMMEDIATELY pings Mint verification API
    ↓ HTTP POST http://mint:8080/verify/certificate
  ↓ Body: { "merkle_root": "abc123...", "unit_id": "RT-20251117-001" }
  ↓
Mint checks ProofCache (merkle_root exists?)
  ↓ If found: Return valid=true, unit metadata
  ↓ If not found: Return valid=false, error
  ↓
Wallet accepts certificate ONLY if Mint confirms valid
  ↓ If valid: Store in wallet.rt_units[]
  ↓ If invalid: Reject, log fraud alert
```

**Implementation** (Wallet verification):
```rust
// src/wallet/src/main.rs
async fn verify_certificate_with_mint(
    merkle_root: &str,
    unit_id: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mint_verification_url = std::env::var("MINT_VERIFICATION_URL")
        .unwrap_or_else(|_| "http://mint:8080/verify/certificate".to_string());

    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "merkle_root": merkle_root,
        "unit_id": unit_id,
    });

    let response = client
        .post(&mint_verification_url)
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;

    if response.status().is_success() {
        let verification_result: serde_json::Value = response.json().await?;
        if let Some(valid) = verification_result.get("valid").and_then(|v| v.as_bool()) {
            return Ok(valid);
        }
    }

    Ok(false)
}

// Called immediately when certificate received
async fn start_distribution_subscriber(...) {
    while let Some(message) = subscriber.next().await {
        let dist_msg: WalletDistributionMessage = serde_json::from_slice(&message.payload)?;
        
        // FIRST: Verify certificate with Mint before accepting
        match verify_certificate_with_mint(
            &dist_msg.rt_unit.merkle_root,
            &dist_msg.rt_unit.unit_id,
        ).await {
            Ok(true) => {
                info!("✅ Certificate VERIFIED by Mint: {}", dist_msg.rt_unit.unit_id);
                // Store certificate
                wallet.rt_units.push(dist_msg.rt_unit);
            }
            Ok(false) => {
                error!("❌ Certificate REJECTED by Mint: {} - potential fraud!", dist_msg.rt_unit.unit_id);
            }
            Err(e) => {
                error!("⚠️  Failed to verify certificate: {} - network error", e);
            }
        }
    }
}
```

**Implementation** (Mint verification endpoint):
```go
// src/mint/internal/mint/verification_handler.go
func (h *VerificationHandler) handleCertificateVerification(w http.ResponseWriter, r *http.Request) {
    var req struct {
        MerkleRoot string `json:"merkle_root"`
        UnitID     string `json:"unit_id"`  // Optional
    }
    json.NewDecoder(r.Body).Decode(&req)
    
    // Look up by merkle_root in ProofCache
    unitID := h.proofCache.FindByMerkleRoot(req.MerkleRoot)
    if unitID == "" {
        h.respondJSON(w, http.StatusOK, map[string]interface{}{
            "valid":       false,
            "merkle_root": req.MerkleRoot,
            "error":       "certificate not found in Mint records",
        })
        return
    }
    
    // Certificate is valid (Mint minted it)
    merkleResult := h.proofCache.Get(unitID)
    h.respondJSON(w, http.StatusOK, map[string]interface{}{
        "valid":        true,
        "merkle_root":  req.MerkleRoot,
        "unit_id":      unitID,
        "tree_height":  merkleResult.TreeHeight,
        "ingot_count":  len(merkleResult.HashEntries),
        "verified_by":  "RoboTorq Mint",
    })
}

// ProofCache method
func (pc *ProofCache) FindByMerkleRoot(merkleRoot string) string {
    pc.mu.RLock()
    defer pc.mu.RUnlock()
    
    // Linear search (acceptable - cache bounded, infrequent verification)
    for unitID, result := range pc.results {
        if result.MerkleRoot == merkleRoot {
            return unitID
        }
    }
    
    return ""
}
```

**Why This Matters**:
1. **Fraud prevention** - Wallet doesn't blindly trust DistoDam (verify with source of truth)
2. **Double-spend detection** - If merkle_root not in Mint, certificate is fake
3. **Proof chain validation** - Mint confirms: "Yes, I minted this unit"
4. **Transparency** - Users can independently query Mint to verify their certificates

**Security Properties**:
- **Mint is authoritative** - Only Mint can create new RT units (via Phase3 assembly)
- **ProofCache is tamper-evident** - Mint stores all merkle_roots it has minted
- **Verification is stateless** - No write access needed, read-only API
- **Fast lookup** - O(n) linear search acceptable (cache size bounded, verification infrequent)

**Future Enhancement** (Phase 5):
- Add reverse index: `merkleRootIndex map[string]string` (merkleRoot → unitID)
- O(1) lookup instead of O(n) linear search
- Populated when Phase3RoboTorqUnit assembled, stored in ProofCache

**Environment Configuration**:
```yaml
# docker-compose.yaml
wallet:
  environment:
    - MINT_VERIFICATION_URL=http://mint:8080/verify/certificate
```

### 10.5 Oracle Signature Verification

**Problem**: Malicious actor publishes fake `vault.ledger.update`

**Solution**: Oracle signs every LedgerUpdate with SPHINCS+ (archival security)

```go
type LedgerUpdate struct {
    // ... fields
    OracleSignature []byte
}

// TorqVault verification
func (tv *TorqVaultService) verifyOracleSignature(update LedgerUpdate) bool {
    oraclePublicKey := tv.trustedOracleKeys[update.OracleID]
    
    messageBytes := serializeLedgerUpdate(update)
    
    // SPHINCS+ verification for long-term archival proofs
    verifier := oqs.Signature{}
    verifier.Init("SPHINCS+-SHAKE256-128f", nil)
    defer verifier.Clean()
    
    valid, _ := verifier.Verify(messageBytes, update.OracleSignature, oraclePublicKey)
    
    if !valid {
        tv.logger.Error("INVALID ORACLE SIGNATURE - POSSIBLE ATTACK",
            "oracle_id", update.OracleID,
            "flow_id", update.FlowID)
    }
    
    return valid
}
```

**Trusted key distribution**: Hardcoded in TorqVault config (bootstrapped via governance)

### 9.5 Flow Encryption (Optional, Phase 2+)

**Privacy concern**: NATS messages visible to all subscribers

**Solution**: Encrypt sensitive fields with Kyber512 (post-quantum KEM)

```go
import "github.com/cloudflare/circl/kem/kyber"

// Encrypt memo field
recipientPublicKey := getRecipientKyberKey(flow.RecipientAddress)
ciphertext, sharedSecret := kyber.Kyber512.Encapsulate(recipientPublicKey)

encryptedMemo := AES256_GCM_Encrypt(sharedSecret, flow.Memo)

// Only recipient can decrypt
plainMemo := AES256_GCM_Decrypt(sharedSecret, encryptedMemo)
```

**Phase 1 simplification**: No encryption (public NATS topics)

---

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

## 12. Phase Roadmap

### Phase 1: Core Flow Implementation (Current)

**Goals**:
- ✅ Implement account-based balances (StashVault, PledgeVault)
- ✅ Implement fixed-point MicroRT arithmetic
- ✅ Implement Falcon-1024 signatures
- ⏳ Implement Wallet service with flow creation
- ⏳ Implement DistoDam covenant submission + validation
- ⏳ Implement TorqVault drip execution (self-enforced initially)
- ⏳ Implement Weibull drip calculation
- ⏳ Implement event sourcing (PostgreSQL + NATS JetStream)

**New services**:
- `Wallet` (flow creation, balance tracking)
- `DistoDam Settlement Layer` (covenant enforcement, slashing)
- Extend `TorqVault` (add escrow, drip execution)

**Deliverable**: Time-shaped payment flows (user sends 50 RT over 1 hour)

### Phase 2: DistoDam Enforcement

**Goals**:
- Implement DistoDam covenant enforcement (validate drips against schedule)
- Implement collateral locking (10% of flow amount)
- Implement slashing mechanism (missed drip → slash collateral)
- Implement fallback TorqVault activation
- Implement merkle tree verification (drip proofs)

**New components**:
- `CovenantValidator` (DistoDam component)
- `SlashingEngine` (DistoDam component)
- `FallbackActivator` (DistoDam component)

**Deliverable**: Trustless drip enforcement (NO centralized Oracle)

### Phase 3: BidNet Escrow Integration

**Goals**:
- Integrate payment flows with BidNet marketplace
- Implement TorqVault bidding (fee calculation, liquidity check, collateral posting)
- Implement instant recipient credit (liquidity fronting)
- Implement multi-TorqVault competition + fallback bidders

**New components**:
- Extend `BidNet Coordinator` (handle payment flows, not just contracts)
- `EscrowBiddingEngine` (TorqVault component with collateral management)

**Deliverable**: Market-driven payment fees, no TorqVault lock-in

### Phase 4: Reputation & Advanced Features

**Goals**:
- Implement TorqVault reputation system (track slashing, completion rate)
- Implement flow encryption (Kyber512 for memo privacy)
- Implement sender reputation (history, cancel rate)
- Implement dynamic fee pricing (demand-based, reputation-weighted)

**New components**:
- `ReputationTracker` (per-TorqVault and per-sender metrics)
- `FeeOracle` (market pricing data)

**Deliverable**: Production-ready, economically secured payment network

**Deliverable**: Production-ready, fault-tolerant payment network

### Phase 5: Physical RT Integration

**Goals**:
- Implement NFC redemption (physical RT coin → digital balance)
- Implement one-time redemption codes (prevent double-spend)
- Implement Mint integration (physical coin issuance)

**New components**:
- `RedemptionService` (validates NFC signatures, credits accounts)
- `PhysicalRTRegistry` (tracks redeemed coins)

**Deliverable**: Physical-digital RT bridge (tap NFC, receive RT in wallet)

---

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

## Appendix A: Mathematical Foundations

### A.1 Weibull CDF and Integral Validation

**Cumulative Distribution Function (CDF)**:

```
F(t) = 1 - exp(-(t/λ)^k)

Meaning: F(t) = fraction of total flow delivered by time t
```

**Validation** (total flow = 1.0):

```
∫₀^∞ P(t) dt = ∫₀^∞ (k/λ) * (t/λ)^(k-1) * exp(-(t/λ)^k) dt = 1.0
```

**Proof**:
Let u = (t/λ)^k, then du = k/λ * (t/λ)^(k-1) dt

```
∫₀^∞ exp(-u) du = [-exp(-u)]₀^∞ = 0 - (-1) = 1.0  ✓
```

### A.2 Drip Calculation Pseudocode

```python
def calculate_drip(flow, current_time):
    elapsed = current_time - flow.start_time
    
    if elapsed >= flow.duration:
        # Flow complete - return remaining balance
        return flow.total_amount - flow.total_dripped
    
    # Weibull PDF at time t
    t = elapsed.total_seconds()
    k = flow.weibull_k
    λ = flow.weibull_lambda
    
    # P(t) = (k/λ) * (t/λ)^(k-1) * exp(-(t/λ)^k)
    t_over_lambda = t / λ
    pdf = (k / λ) * (t_over_lambda ** (k - 1)) * math.exp(-(t_over_lambda ** k))
    
    # Δt = 10 seconds (oracle tick interval)
    delta_t = 10
    
    # Δ RT = Total * P(t) * Δt
    drip_amount = flow.total_amount * pdf * delta_t
    
    return drip_amount
```

### A.3 Shape Parameter Effects

| k | Distribution Name | Peak Location | Use Case |
|---|-------------------|---------------|----------|
| k < 1 | Heavy-tailed exponential | t=0 (immediate) | Extreme urgency (rare) |
| k = 1 | Pure exponential | t=0 | Merchant payments (front-loaded) |
| 1 < k < 2 | Skewed right | Early | Slightly urgent |
| k = 2 | Rayleigh (bell curve) | t = λ/√2 | Standard P2P |
| 2 < k < 3.5 | Approaching uniform | Middle | Scheduled |
| k = 3.5 | Near-uniform | Plateau | Payroll (steady) |
| k > 3.5 | Sharp peak | Middle (narrow) | Custom (rare) |

### A.4 Fixed-Point Precision Analysis

**MicroRT = int64** (range: ±9.2 trillion RT)

**Smallest unit**: 1 micro-RT = 0.000001 RT

**Precision**: 6 decimal places (sufficient for $0.01 USD ≈ 0.002 RT)

**Example**:
```
1 RT = $5 USD (hypothetical exchange rate)
1 micro-RT = $0.000005 USD (negligible)

Payment: 50.123456 RT = 50,123,456 micro-RT (exact representation)
```

**No drift**: All arithmetic uses int64 (addition, subtraction exact)

---

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

## Conclusion

RoboTorq's flow-based transaction model is a **physics-native** approach to digital payments:

- **Time-shaped flows** mirror electricity transmission (not teleportation)
- **Weibull distributions** provide flexible payment profiles (urgent, standard, scheduled)
- **Account-based balances** simplify UX (one number, not UTXO coin selection)
- **DistoDam settlement** enforces drips with cryptographic proofs (NO centralized Oracle!)
- **TorqVault competition** ensures market-driven fees and no lock-in
- **Economic security** via collateral + slashing (10% collateral, reputation system)
- **Quantum-safe crypto** (Falcon-1024 + SPHINCS+) future-proofs against quantum attacks
- **Event sourcing** enables audit trails, crash recovery, dispute resolution
- **30-day cancellation notice** prevents flash attacks (learned from Vault)

**Key Architecture Wins**:
- **No single point of failure**: DistoDam ledger replaces centralized Oracle service
- **Trustless enforcement**: Covenants + merkle proofs + slashing mechanism
- **Leverages existing infrastructure**: No new service needed (BidNet + DistoDam already exist)
- **Economic incentives align**: TorqVaults lose collateral if they fail (self-regulating reliability)

**Implementation Status**: Design complete, Oracle removed in favor of DistoDam settlement.

**Next Steps**: 
1. Implement DistoDam covenant validation infrastructure
2. Build TorqVault collateral posting + drip execution
3. Implement slashing engine (missed drip detection, collateral distribution)
4. Add fallback TorqVault activation logic
5. Integrate Falcon-1024 + SPHINCS+ signatures
6. Deploy testnet v0.1 (covenant-based settlement)

---

*"Electricity doesn't teleport. Neither does RoboTorq."* ⚡💰  
*"Decentralization isn't a feature. It's the foundation."* 🏛️⚖️
