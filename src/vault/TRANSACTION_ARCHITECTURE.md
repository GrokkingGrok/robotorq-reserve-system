# RoboTorq Transaction Architecture

**Version**: 1.0  
**Date**: November 18, 2025  
**Status**: Design Complete  
**Author**: GitHub Copilot (Claude Sonnet 4.5)

---

## Executive Summary

RoboTorq uses a **flow-based transaction model** where payments are time-shaped streams rather than atomic transfers. This design mirrors the physical reality of electricity flow and aligns with the system's core metaphor: "Watts > Wall Street."

**Key Characteristics**:
- **Account-based balances** (NOT UTXOs) - Users see one balance number
- **Time-shaped payments** - Weibull distributions control flow profiles
- **Oracle settlement** - 10-second ticks drip RT from sender to recipient
- **Flow locking** - Sender's balance locked during active flows
- **Fixed-point arithmetic** - MicroRT (int64) prevents floating-point drift
- **Quantum-safe signatures** - Dilithium2/3 + Kyber512

**This is NOT**:
- ❌ A UTXO model (no coin selection, no change addresses)
- ❌ Atomic transfers (payments drip over time)
- ❌ Blockchain-based (NATS message passing, not append-only ledger)
- ❌ Instant settlement (10-second oracle ticks)

---

## Table of Contents

1. [Core Concepts](#1-core-concepts)
2. [Transaction Model](#2-transaction-model)
3. [Flow-Based Payments](#3-flow-based-payments)
4. [Weibull Distribution Profiles](#4-weibull-distribution-profiles)
5. [Oracle Settlement](#5-oracle-settlement)
6. [Account State Management](#6-account-state-management)
7. [BidNet Escrow Integration](#7-bidnet-escrow-integration)
8. [NATS Message Schemas](#8-nats-message-schemas)
9. [Security & Cryptography](#9-security--cryptography)
10. [Phase Roadmap](#10-phase-roadmap)
11. [Appendix: Mathematical Foundations](#appendix-mathematical-foundations)

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
   Wallet signs with Dilithium2/3
   ↓
   Publish to bidnet.escrow.request

2. Escrow Bidding (BidNet Integration)
   ↓
   TorqVaults receive FlowRequest
   ↓
   Each TorqVault calculates fee (risk, liquidity, duration)
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
   Oracle begins 10-second ticks
   ↓
   Each tick: drip Δ RT from sender's locked balance to TorqVault
   ↓
   Flow completes after duration expires

4. Settlement
   ↓
   TorqVault publishes vault.ledger.update (final balances)
   ↓
   All TorqVaults apply update to distributed ledger (consensus)
   ↓
   Merkle root published for verification
   ↓
   Transaction complete
```

### 2.3 Flow Cancellation

**Allowed during**: First 50% of flow duration (configurable)

```go
type FlowCancellation struct {
    FlowID        string
    CancellerID   string  // Must be sender
    Timestamp     time.Time
    Signature     []byte  // Dilithium2/3
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
    Signature      []byte   // Dilithium2/3 (sender's private key)
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
}

func (fs *FlowSettler) CalculateDrip(currentTime time.Time) int64 {
    elapsed := currentTime.Sub(fs.StartTime).Seconds()
    
    if elapsed >= float64(fs.DurationSec) {
        // Flow complete - drip remaining balance
        remaining := fs.TotalMicroRT - fs.TotalDrippedMicroRT
        fs.TotalDrippedMicroRT = fs.TotalMicroRT
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

## 5. Oracle Settlement

### 5.1 Oracle Tick Cycle

**Frequency**: Every 10 seconds (configurable)

```
t=0s:   Flow starts, sender's balance locked
        ↓
t=10s:  Oracle tick #1
        ↓ Drip Δ₁ RT (calculated from Weibull P(10))
        ↓ Update: sender.locked -= Δ₁, recipient.balance += Δ₁
        ↓
t=20s:  Oracle tick #2
        ↓ Drip Δ₂ RT (calculated from Weibull P(20))
        ↓
...
        ↓
t=3600s: Flow completes
        ↓ Drip remaining balance
        ↓ Unlock sender, publish final ledger update
```

### 5.2 Oracle Service Architecture

```go
package oracle

import (
    "time"
    "log/slog"
)

type FlowOracle struct {
    tickInterval time.Duration  // 10 seconds default
    activeFlows  map[string]*FlowSettler
    natsClient   *nats.Conn
    logger       *slog.Logger
}

func (o *FlowOracle) Start(ctx context.Context) {
    ticker := time.NewTicker(o.tickInterval)
    defer ticker.Stop()
    
    for {
        select {
        case <-ticker.C:
            o.processTick()
        case <-ctx.Done():
            o.logger.Info("Oracle shutting down gracefully")
            return
        }
    }
}

func (o *FlowOracle) processTick() {
    currentTime := time.Now()
    
    for flowID, settler := range o.activeFlows {
        // Calculate drip for this flow
        dripMicroRT := settler.CalculateDrip(currentTime)
        
        if dripMicroRT == 0 {
            continue  // No drip (flow complete or edge case)
        }
        
        // Publish ledger update
        update := LedgerUpdate{
            FlowID:        flowID,
            SenderAddr:    settler.SenderAddr,
            RecipientAddr: settler.RecipientAddr,
            DripMicroRT:   dripMicroRT,
            Timestamp:     currentTime,
        }
        
        o.natsClient.Publish("vault.ledger.update", update.ToJSON())
        
        o.logger.Info("oracle tick",
            "flow_id", flowID,
            "drip_micro_rt", dripMicroRT,
            "total_dripped", settler.TotalDrippedMicroRT,
            "progress_pct", float64(settler.TotalDrippedMicroRT) / float64(settler.TotalMicroRT) * 100)
        
        // Remove completed flows
        if settler.TotalDrippedMicroRT >= settler.TotalMicroRT {
            delete(o.activeFlows, flowID)
            o.logger.Info("flow complete", "flow_id", flowID)
        }
    }
}
```

### 5.3 Ledger Update Message

```go
type LedgerUpdate struct {
    FlowID        string
    SenderAddr    string
    RecipientAddr string
    DripMicroRT   int64
    Timestamp     time.Time
    OracleSignature []byte  // Dilithium2/3 (Oracle's private key)
}

// NATS topic: vault.ledger.update
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

### 5.4 Oracle Redundancy & Fault Tolerance

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
    Address       string  // Dilithium public key hash (32 bytes)
    Nonce         int64   // Incremental (prevents replay attacks)
    
    // Balances (MicroRT)
    StashBalanceMicroRT   int64  // Available for spending
    PledgeBalanceMicroRT  int64  // Staked in DistoDam (yield-earning)
    LockedMicroRT         int64  // Locked in active outgoing flows
    
    // Metadata
    CreatedAt     time.Time
    LastActivity  time.Time
    
    // Cryptography
    PublicKey     []byte  // Dilithium2/3 (for signature verification)
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
    Timestamp     time.Time
    Signature     []byte    // TorqVault's Dilithium2/3 signature
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

### 7.4 Instant Recipient Credit (Liquidity Fronting)

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

## 8. NATS Message Schemas

### 8.1 Flow Lifecycle Topics

| Topic | Publisher | Subscriber | Purpose |
|-------|-----------|------------|---------|
| `bidnet.escrow.request` | Wallet | TorqVaults | User initiates payment flow |
| `bidnet.escrow.bid` | TorqVaults | BidNet Coordinator | TorqVaults submit fee bids |
| `bidnet.escrow.awarded` | BidNet Coordinator | All (winner, sender, recipient) | Announce winning bid |
| `vault.ledger.update` | Oracle | All TorqVaults | Drip RT from sender to recipient |
| `flow.cancelled` | Wallet or TorqVault | Oracle, TorqVaults | Cancel active flow (refund) |

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
  "signature": "base64-encoded-dilithium-signature..."
}
```

#### 8.2.2 EscrowBid (`bidnet.escrow.bid`)

```json
{
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "torqvault_id": "torqvault-alpha-001",
  "fee_micro_rt": 50000,
  "timestamp": "2025-11-18T22:30:00.500Z",
  "signature": "base64-encoded-dilithium-signature..."
}
```

#### 8.2.3 EscrowAwarded (`bidnet.escrow.awarded`)

```json
{
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "winner_torqvault_id": "torqvault-beta-002",
  "winning_fee_micro_rt": 45000,
  "timestamp": "2025-11-18T22:30:01Z",
  "coordinator_signature": "base64-encoded-dilithium-signature..."
}
```

#### 8.2.4 LedgerUpdate (`vault.ledger.update`)

```json
{
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "sender_address": "RT1a2b3c4d5e6f7g8h9i0j...",
  "recipient_address": "RT9z8y7x6w5v4u3t2s1r...",
  "drip_micro_rt": 138888,
  "timestamp": "2025-11-18T22:30:10Z",
  "oracle_signature": "base64-encoded-dilithium-signature..."
}
```

#### 8.2.5 FlowCancellation (`flow.cancelled`)

```json
{
  "flow_id": "550e8400-e29b-41d4-a716-446655440000",
  "canceller_address": "RT1a2b3c4d5e6f7g8h9i0j...",
  "reason": "incorrect recipient address",
  "elapsed_sec": 120,
  "amount_settled_micro_rt": 2777777,
  "refund_micro_rt": 46222222,
  "timestamp": "2025-11-18T22:32:00Z",
  "signature": "base64-encoded-dilithium-signature..."
}
```

### 8.3 Subscription Patterns

**Wallet**:
```go
// Subscribe to awarded flows (confirm acceptance)
nc.Subscribe("bidnet.escrow.awarded", handleEscrowAwarded)

// Subscribe to ledger updates (track flow progress)
nc.Subscribe("vault.ledger.update", handleLedgerUpdate)

// Subscribe to cancellation confirmations
nc.Subscribe("flow.cancelled", handleFlowCancelled)
```

**TorqVault**:
```go
// Subscribe to flow requests (participate in auctions)
nc.Subscribe("bidnet.escrow.request", handleEscrowRequest)

// Subscribe to ledger updates (maintain distributed ledger)
nc.Subscribe("vault.ledger.update", handleLedgerUpdate)

// Subscribe to cancellations (refund locked balances)
nc.Subscribe("flow.cancelled", handleFlowCancelled)
```

**Oracle**:
```go
// Subscribe to awarded flows (start dripping)
nc.Subscribe("bidnet.escrow.awarded", startFlowDrip)

// Subscribe to cancellations (stop dripping)
nc.Subscribe("flow.cancelled", stopFlowDrip)
```

---

## 9. Security & Cryptography

### 9.1 Quantum-Safe Signatures

**Algorithm**: NIST PQC Round 3 - Dilithium2/Dilithium3

```go
import "github.com/cloudflare/circl/sign/dilithium"

// Key generation (one-time per account)
publicKey, privateKey := dilithium.Mode2.GenerateKey(rand.Reader)

// Signing (every FlowRequest)
signature := dilithium.Mode2.Sign(privateKey, flowRequestBytes)

// Verification (by TorqVaults)
valid := dilithium.Mode2.Verify(publicKey, flowRequestBytes, signature)
```

**Signature sizes**:
- Dilithium2: ~2.4 KB (faster, less secure)
- Dilithium3: ~3.3 KB (slower, more secure)

**RoboTorq choice**: Dilithium2 for user transactions (speed), Dilithium3 for Oracle/TorqVault (security)

### 9.2 Address Derivation

**Account address** = `RT` + Base58(SHA256(Dilithium_PublicKey))

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

### 9.3 Nonce-Based Replay Prevention

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

### 9.4 Oracle Signature Verification

**Problem**: Malicious actor publishes fake `vault.ledger.update`

**Solution**: Oracle signs every LedgerUpdate with Dilithium3

```go
type LedgerUpdate struct {
    // ... fields
    OracleSignature []byte
}

// TorqVault verification
func (tv *TorqVaultService) verifyOracleSignature(update LedgerUpdate) bool {
    oraclePublicKey := tv.trustedOracleKeys[update.OracleID]
    
    messageBytes := serializeLedgerUpdate(update)
    
    valid := dilithium.Mode3.Verify(oraclePublicKey, messageBytes, update.OracleSignature)
    
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

## 10. Phase Roadmap

### Phase 1: Mock Settlement (Current)

**Goals**:
- ✅ Implement account-based balances (StashVault, PledgeVault)
- ✅ Implement fixed-point MicroRT arithmetic
- ✅ Implement Dilithium2 signatures
- ⏳ Implement instant settlement (no flow drips)
- ⏳ Implement single TorqVault (no BidNet)
- ⏳ Implement mock Oracle (no Weibull, instant transfer)

**Simplifications**:
- No flow drips (instant atomic transfers for now)
- No BidNet escrow (direct wallet → TorqVault)
- No Oracle ticks (TorqVault processes transfers immediately)

**Deliverable**: Functional payment system with quantum-safe signatures

### Phase 2: Flow-Based Payments

**Goals**:
- Implement FlowRequest with Weibull parameters
- Implement Oracle with 10-second ticks
- Implement FlowSettler (Weibull drip calculation)
- Implement balance locking during active flows
- Implement flow cancellation (refund logic)

**New components**:
- `FlowOracle` service (standalone, subscribes to `bidnet.escrow.awarded`)
- `FlowSettler` library (Weibull math)
- `vault.ledger.update` NATS topic

**Deliverable**: Time-shaped payment flows (user sends 50 RT over 1 hour)

### Phase 3: BidNet Escrow

**Goals**:
- Integrate payment flows with BidNet marketplace
- Implement TorqVault bidding (fee calculation, liquidity check)
- Implement instant recipient credit (liquidity fronting)
- Implement multi-TorqVault competition

**New components**:
- Extend `BidNet Coordinator` (handle payment flows, not just contracts)
- `EscrowBiddingEngine` (TorqVault component)

**Deliverable**: Market-driven payment fees, no TorqVault lock-in

### Phase 4: Advanced Features

**Goals**:
- Implement multi-Oracle consensus (Byzantine Fault Tolerance)
- Implement flow encryption (Kyber512 for memo privacy)
- Implement reputation system (sender history, cancel rate)
- Implement dynamic fee pricing (demand-based)

**New components**:
- `OraclePool` (3-5 Oracles, 2-of-3 quorum)
- `ReputationTracker` (per-account metrics)
- `FeeOracle` (market pricing data)

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

## Appendix: Mathematical Foundations

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

## Conclusion

RoboTorq's flow-based transaction model is a **physics-native** approach to digital payments:

- **Time-shaped flows** mirror electricity transmission (not teleportation)
- **Weibull distributions** provide flexible payment profiles (urgent, standard, scheduled)
- **Account-based balances** simplify UX (one number, not UTXO coin selection)
- **Oracle settlement** drips RT every 10 seconds (partial settlement, reversibility)
- **BidNet escrow** ensures market-driven fees and no lock-in
- **Quantum-safe crypto** future-proofs against Shor's algorithm

**Implementation Status**: Design complete, Phase 1 (mock settlement) in progress.

**Next Steps**: Implement `FlowOracle` service, `FlowSettler` library, and integrate with BidNet escrow marketplace.

---

*"Electricity doesn't teleport. Neither does RoboTorq."* ⚡💰
