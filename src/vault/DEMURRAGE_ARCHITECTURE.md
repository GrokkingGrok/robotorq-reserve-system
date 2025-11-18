# RoboTorq Demurrage Architecture

**Version**: 1.0  
**Date**: November 18, 2025  
**Status**: Design Complete  
**Author**: GitHub Copilot (Claude Sonnet 4.5)

---

## Executive Summary

RoboTorq implements **demurrage** (idle currency tax) as the primary monetary policy tool. Holding RT costs money; using RT creates value. This inverts traditional banking incentives and drives economic velocity.

**Core Mechanics**:
- **Collection**: Wallets calculate demurrage hourly, send as short-duration flow to TorqVault
- **Aggregation**: Network-wide pool (all TorqVaults report to DemurrageOrchestrator)
- **Redistribution**: Proportional yield to StashVault + PledgeVault balances (50/50 split default)
- **Urgent Rerouting**: Emergency funding via quadratic governance (member votes on duration)

**Key Principles**:
- **No hoarding** - Idle RT decays at ~0.5%/month (configurable)
- **Equal percentage yield** - Rich and poor earn same % return (fairness)
- **Voluntary participation** - No vaults = no yield (must opt-in)
- **Transparent rates** - TorqVaults broadcast rates daily (max 0.01%/day change)
- **Crisis governance** - Members control emergency rerouting (not admins)

**This is NOT**:
- ❌ A tax collected by government (decentralized, algorithmic)
- ❌ Inflation (money supply fixed by physics: 1 kWh = 1 RT)
- ❌ Confiscation (voluntary, predictable, redistributed to participants)

---

## Table of Contents

1. [Economic Theory](#1-economic-theory)
2. [Demurrage Collection](#2-demurrage-collection)
3. [Rate Broadcasting](#3-rate-broadcasting)
4. [Network-Wide Aggregation](#4-network-wide-aggregation)
5. [Yield Distribution](#5-yield-distribution)
6. [Urgent Rerouting](#6-urgent-rerouting)
7. [NATS Message Schemas](#7-nats-message-schemas)
8. [DemurrageOrchestrator Service](#8-demurrageorchestrator-service)
9. [Governance Integration](#9-governance-integration)
10. [Phase Roadmap](#10-phase-roadmap)
11. [Appendix: Economic Simulations](#appendix-economic-simulations)

---

## 1. Economic Theory

### 1.1 Why Demurrage?

**Problem**: Traditional money rewards hoarding (interest on savings)

**Effect**: Wealth accumulates at top, velocity slows, deflation spirals

**RoboTorq Inversion**: Tax idle money, reward circulation

```
Traditional Banking:
  Hold $1000 in savings → Earn 5% interest/year → Incentive: HOARD

RoboTorq Demurrage:
  Hold 1000 RT idle → Pay 0.5% demurrage/month → Incentive: USE
  Stake 1000 RT in vaults → Earn 0.6% yield/month → Incentive: PARTICIPATE
```

**Outcome**: Money circulates (velocity ↑), economy grows, no deflation trap.

### 1.2 Demurrage vs Inflation

| Feature | Demurrage (RoboTorq) | Inflation (Fiat) |
|---------|----------------------|------------------|
| Money supply | Fixed by physics (1 kWh = 1 RT) | Arbitrary printing |
| Idle money | Decays (0.5%/month) | Debases (2-3%/year) |
| Active money | Immune (used in flows) | Debases equally |
| Savers | Penalized (must use or stake) | Penalized (silent tax) |
| Spenders | Neutral (no demurrage on flows) | Neutral |
| Vault participants | Rewarded (0.6% yield) | Unrewarded |
| Transparency | Explicit rate (broadcast daily) | Hidden (CPI manipulation) |

**Key difference**: Demurrage is **transparent** and **redistributed to participants**, not central banks.

### 1.3 Velocity Mechanics

**Formula**:
```
Velocity = (Total RT Spent) / (Average RT Supply)

With demurrage:
  Velocity ↑ (people spend faster to avoid decay)
  GDP ↑ (V * M = P * Q, if M fixed and V ↑, then Q ↑)
```

**Example**:
```
No demurrage:
  Alice holds 100 RT for 6 months → 100 RT (no change)
  Network velocity: Low (money sits idle)

With demurrage (0.5%/month):
  Alice holds 100 RT for 6 months → 97 RT (3 RT demurrage paid)
  Alice stakes 100 RT in PledgeVault → 103 RT (3 RT yield earned)
  Network velocity: High (Alice incentivized to stake or spend)
```

### 1.4 Target Rate

**Initial rate**: 0.5% per month (~6% per year)

**Rationale**:
- High enough to discourage hoarding
- Low enough to not panic users
- Competitive with staking yield (0.6% = slight premium)

**Dynamic adjustment** (Phase 2+):
- If velocity too low → Increase demurrage (max +0.01%/day)
- If velocity too high → Decrease demurrage (max -0.01%/day)
- Governance votes on target velocity range

---

## 2. Demurrage Collection

### 2.1 Collection Cycle

**Frequency**: Every 1 hour (configurable)

```
t=0hr:   Last collection timestamp recorded
         ↓
t=1hr:   Wallet calculates demurrage owed
         ↓ Create DemurragePaymentFlow
         ↓ Duration: 60 seconds (short flow, k=1.0 exponential)
         ↓ Amount: idle_balance * rate * time_elapsed
         ↓
         Publish to bidnet.escrow.request
         ↓
         TorqVault wins bid, receives flow
         ↓
         TorqVault verifies calculation (proof check)
         ↓
         TorqVault credits DemurragePool
```

**Why hourly?**
- Frequent enough to prevent large one-time charges
- Infrequent enough to avoid network spam
- Aligns with human timescales (1 hour = digestible)

### 2.2 Calculation Logic

**Demurrage owed** = `idle_balance * rate * time_elapsed`

```go
type DemurrageCalculation struct {
    // Inputs
    IdleBalanceMicroRT  int64     // Balance not in active flows
    DemurrageRate       float64   // e.g., 0.005 (0.5% per month)
    TimeElapsedSec      int       // Seconds since last collection
    
    // Calculation
    MonthsElapsed       float64   // TimeElapsedSec / (30 * 24 * 3600)
    DemurrageOwedMicroRT int64    // IdleBalance * Rate * MonthsElapsed
    
    // Merkle proof (for verification)
    BalanceProof        []byte    // Merkle branch proving balance
    TimestampProof      []byte    // Signature of last collection time
}

// Example: 1000 RT idle for 1 hour, rate = 0.005/month
idle := 1000.0  // RT
rate := 0.005   // 0.5% per month
elapsed := 3600.0  // 1 hour in seconds
months := elapsed / (30 * 24 * 3600)  // 0.00137 months

demurrage := idle * rate * months
// = 1000 * 0.005 * 0.00137
// = 0.00685 RT
// = 6,850 micro-RT
```

**Why MicroRT?** Prevents rounding errors (0.00685 RT = exact integer)

### 2.3 Idle Balance Definition

**Idle balance** = `total_balance - locked_in_flows - staked_in_vaults`

```go
func (w *Wallet) calculateIdleBalance() int64 {
    account := w.getAccount()
    
    totalMicroRT := account.StashBalanceMicroRT + account.PledgeBalanceMicroRT
    lockedMicroRT := account.LockedMicroRT  // Active outgoing flows
    stakedMicroRT := account.PledgeBalanceMicroRT  // Staked (earns yield, no demurrage)
    
    idleMicroRT := account.StashBalanceMicroRT - lockedMicroRT
    
    return idleMicroRT
}
```

**Key exemptions**:
- **Active flows**: Money in transit (locked) = no demurrage
- **Staked vaults**: PledgeVault balance = no demurrage (earns yield instead)
- **StashVault**: Earns yield but ALSO pays demurrage if idle (net yield = yield - demurrage)

**Example**:
```
Alice's balances:
  StashVault: 500 RT (idle, pays demurrage, earns yield)
  PledgeVault: 300 RT (staked, no demurrage, earns yield)
  Locked in flows: 200 RT (active payment, no demurrage)

Demurrage calculation:
  Idle balance = 500 RT (StashVault only)
  Demurrage owed = 500 * 0.005 * (1 hour / 720 hours) = 0.00347 RT
```

### 2.4 DemurragePaymentFlow Structure

```go
type DemurragePaymentFlow struct {
    FlowRequest  // Inherits standard flow fields
    
    // Demurrage-specific
    CalculationProof DemurrageCalculation
    LastCollectionTimestamp time.Time
    
    // Flow parameters (override defaults)
    DurationSec  int      // 60 seconds (short flow)
    ShapePreset  string   // "short" (k=1.0, front-loaded)
    
    // Recipient
    RecipientAddress string  // TorqVault's demurrage collection address
}
```

**Why short flow?**
- Demurrage is urgent (penalty, not payment)
- Front-loaded drip (Oracle settles quickly)
- Low risk (TorqVault validates before accepting)

### 2.5 TorqVault Verification

**TorqVault must verify** demurrage calculation (prevent fraud):

```go
func (tv *TorqVaultService) verifyDemurrageFlow(flow DemurragePaymentFlow) error {
    proof := flow.CalculationProof
    
    // 1. Verify balance proof (Merkle branch)
    valid := tv.verifyMerkleProof(proof.BalanceProof, flow.SenderAddress)
    if !valid {
        return ErrInvalidBalanceProof
    }
    
    // 2. Verify timestamp (check last collection time)
    lastCollection := tv.getLastCollectionTime(flow.SenderAddress)
    if flow.LastCollectionTimestamp != lastCollection {
        return ErrTimestampMismatch
    }
    
    // 3. Recalculate demurrage (independent verification)
    expectedDemurrage := tv.calculateDemurrage(
        proof.IdleBalanceMicroRT,
        proof.DemurrageRate,
        proof.TimeElapsedSec,
    )
    
    if flow.AmountMicroRT != expectedDemurrage {
        tv.logger.Error("demurrage calculation mismatch",
            "expected", expectedDemurrage,
            "claimed", flow.AmountMicroRT)
        return ErrInvalidDemurrageAmount
    }
    
    // 4. Accept flow
    tv.demurragePoolMicroRT.Add(flow.AmountMicroRT)
    tv.updateLastCollectionTime(flow.SenderAddress, time.Now())
    
    return nil
}
```

**Fraud prevention**:
- User can't claim lower balance (Merkle proof verified)
- User can't claim shorter elapsed time (timestamp verified)
- TorqVault recalculates independently (double-check math)

---

## 3. Rate Broadcasting

### 3.1 Minimum Demurrage Rate

**Each TorqVault** broadcasts its minimum accepted demurrage rate daily.

```go
type DemurrageRateBroadcast struct {
    TorqVaultID         string
    MinimumDemurrageRate float64   // e.g., 0.005 (0.5% per month)
    EffectiveDate        time.Time // When rate takes effect
    PreviousRate         float64   // For auditability
    Timestamp            time.Time
    Signature            []byte    // TorqVault's Dilithium3 signature
}

// NATS topic: demurrage.rate.broadcast
```

**Why minimum rate?**
- TorqVaults compete on fees (via BidNet), not demurrage rate
- Minimum prevents race-to-zero (must cover operational costs)
- Network-wide consistency (all vaults converge on similar rate)

### 3.2 Rate Change Limits

**Max change**: ±0.01% per day (prevents shock)

```go
func (tv *TorqVaultService) updateDemurrageRate(newRate float64) error {
    currentRate := tv.minimumDemurrageRate
    
    // Check change limit (0.01% per day = 0.0001 absolute)
    maxDelta := 0.0001
    delta := math.Abs(newRate - currentRate)
    
    if delta > maxDelta {
        return ErrRateChangeTooLarge
    }
    
    // Accept new rate
    tv.minimumDemurrageRate = newRate
    
    // Broadcast to network
    broadcast := DemurrageRateBroadcast{
        TorqVaultID:         tv.id,
        MinimumDemurrageRate: newRate,
        EffectiveDate:        time.Now().Add(24 * time.Hour),  // 1-day notice
        PreviousRate:         currentRate,
        Timestamp:            time.Now(),
    }
    
    tv.signAndPublish("demurrage.rate.broadcast", broadcast)
    
    tv.logger.Info("demurrage rate updated",
        "old_rate", currentRate,
        "new_rate", newRate,
        "effective_date", broadcast.EffectiveDate)
    
    return nil
}
```

**1-day notice period**: Users can react (stake more, spend idle RT, switch TorqVaults)

### 3.3 Rate Discovery

**Wallets subscribe** to `demurrage.rate.broadcast` and track all TorqVault rates:

```go
type WalletRateTracker struct {
    torqVaultRates map[string]float64  // torqvault_id → rate
    mu             sync.RWMutex
}

func (wrt *WalletRateTracker) handleRateBroadcast(msg *nats.Msg) {
    var broadcast DemurrageRateBroadcast
    json.Unmarshal(msg.Data, &broadcast)
    
    // Verify signature
    if !verifyTorqVaultSignature(broadcast) {
        return
    }
    
    // Update rate map
    wrt.mu.Lock()
    wrt.torqVaultRates[broadcast.TorqVaultID] = broadcast.MinimumDemurrageRate
    wrt.mu.Unlock()
}

func (wrt *WalletRateTracker) getMedianRate() float64 {
    wrt.mu.RLock()
    defer wrt.mu.RUnlock()
    
    rates := make([]float64, 0, len(wrt.torqVaultRates))
    for _, rate := range wrt.torqVaultRates {
        rates = append(rates, rate)
    }
    
    sort.Float64s(rates)
    return rates[len(rates)/2]  // Median (robust to outliers)
}
```

**Wallet displays**: "Network demurrage rate: 0.5%/month (median of 12 TorqVaults)"

---

## 4. Network-Wide Aggregation

### 4.1 DemurrageOrchestrator Service

**New standalone service** (separate from DistoDam, TorqVault):

```go
type DemurrageOrchestrator struct {
    // NATS
    natsClient *nats.Conn
    
    // Aggregation state
    dailyCollectionsMicroRT atomic.Int64  // Total demurrage collected today
    torqVaultReports        map[string]*DemurrageReport
    
    // Redistribution targets
    eligibleVaultBalances   map[string]*VaultBalances  // member_id → balances
    
    // Configuration
    stashPledgeSplitRatio   float64  // 0.5 = 50/50 default
    
    // Logging
    logger *slog.Logger
    metrics *Metrics
}
```

**Responsibilities**:
1. Receive daily reports from all TorqVaults
2. Aggregate total demurrage pool (network-wide)
3. Calculate yield distribution (proportional to vault balances)
4. Publish allocation instructions to TorqVaults
5. Monitor urgent rerouting triggers (vault ratio thresholds)

### 4.2 Daily Reporting Cycle

**Each TorqVault** reports total demurrage collected every 24 hours:

```go
type DemurrageReport struct {
    TorqVaultID            string
    ReportingPeriodStart   time.Time
    ReportingPeriodEnd     time.Time
    TotalCollectedMicroRT  int64     // Sum of all demurrage flows received
    MemberCount            int       // Number of unique members
    FlowCount              int       // Number of demurrage flows processed
    Timestamp              time.Time
    Signature              []byte    // TorqVault's Dilithium3 signature
}

// NATS topic: demurrage.orchestrator.report
```

**DemurrageOrchestrator aggregates**:

```go
func (do *DemurrageOrchestrator) handleDemurrageReport(msg *nats.Msg) {
    var report DemurrageReport
    json.Unmarshal(msg.Data, &report)
    
    // Verify signature
    if !verifyTorqVaultSignature(report) {
        do.logger.Error("invalid signature on demurrage report",
            "torqvault_id", report.TorqVaultID)
        return
    }
    
    // Store report
    do.torqVaultReports[report.TorqVaultID] = &report
    
    // Aggregate
    do.dailyCollectionsMicroRT.Add(report.TotalCollectedMicroRT)
    
    do.logger.Info("demurrage report received",
        "torqvault_id", report.TorqVaultID,
        "collected_micro_rt", report.TotalCollectedMicroRT,
        "network_total", do.dailyCollectionsMicroRT.Load())
}
```

### 4.3 Network-Wide Pool

**Total demurrage pool** = Sum of all TorqVault reports

```
Network Pool (Daily):
  TorqVault Alpha:  1,250 RT collected
  TorqVault Beta:   1,100 RT collected
  TorqVault Gamma:    950 RT collected
  -----------------------------------------
  Total Pool:       3,300 RT

Redistribution (next section):
  StashVaults:      1,650 RT (50%)
  PledgeVaults:     1,650 RT (50%)
```

**Why network-wide?**
- **Fairness**: All members earn same percentage yield (regardless of TorqVault)
- **No arbitrage**: Can't game system by switching TorqVaults
- **Simplicity**: Single global pool (no inter-vault transfers)

---

## 5. Yield Distribution

### 5.1 Eligible Balances

**Yield distributed to**:
- **StashVault balances** (all members)
- **PledgeVault balances** (all members)

**Formula**:
```
Yield Rate = Total Demurrage Pool / Total Eligible Balances

Member Yield = Member Eligible Balances * Yield Rate
```

**Example**:
```
Network state:
  Total demurrage collected: 3,300 RT (daily)
  Total StashVault balances: 500,000 RT
  Total PledgeVault balances: 300,000 RT
  Total eligible balances: 800,000 RT

Yield rate:
  = 3,300 / 800,000
  = 0.004125 (0.4125% daily)
  = 0.12375% monthly (if sustained)

Alice's balances:
  StashVault: 1,000 RT
  PledgeVault: 500 RT
  Total eligible: 1,500 RT

Alice's yield:
  = 1,500 * 0.004125
  = 6.1875 RT (daily)
  = 185.625 RT (monthly if sustained)
```

**Equal percentage**: Rich and poor earn same 0.4125% (fairness).

### 5.2 Stash/Pledge Split

**Default**: 50/50 split (configurable via governance)

```go
type YieldAllocation struct {
    TotalPoolMicroRT         int64
    StashAllocationMicroRT   int64  // 50% of pool
    PledgeAllocationMicroRT  int64  // 50% of pool
    
    // Per-member allocations (calculated)
    memberYields             map[string]*MemberYield
}

type MemberYield struct {
    MemberID                 string
    StashBalanceMicroRT      int64
    PledgeBalanceMicroRT     int64
    StashYieldMicroRT        int64
    PledgeYieldMicroRT       int64
    TotalYieldMicroRT        int64
}

func (do *DemurrageOrchestrator) calculateYieldAllocation() *YieldAllocation {
    totalPool := do.dailyCollectionsMicroRT.Load()
    
    // Split pool
    stashAllocation := int64(float64(totalPool) * do.stashPledgeSplitRatio)
    pledgeAllocation := totalPool - stashAllocation
    
    // Calculate total balances
    totalStashMicroRT := int64(0)
    totalPledgeMicroRT := int64(0)
    
    for _, vaults := range do.eligibleVaultBalances {
        totalStashMicroRT += vaults.StashBalanceMicroRT
        totalPledgeMicroRT += vaults.PledgeBalanceMicroRT
    }
    
    // Calculate yields per member
    memberYields := make(map[string]*MemberYield)
    
    for memberID, vaults := range do.eligibleVaultBalances {
        stashYield := int64(0)
        pledgeYield := int64(0)
        
        if totalStashMicroRT > 0 {
            stashYield = (vaults.StashBalanceMicroRT * stashAllocation) / totalStashMicroRT
        }
        
        if totalPledgeMicroRT > 0 {
            pledgeYield = (vaults.PledgeBalanceMicroRT * pledgeAllocation) / totalPledgeMicroRT
        }
        
        memberYields[memberID] = &MemberYield{
            MemberID:              memberID,
            StashBalanceMicroRT:   vaults.StashBalanceMicroRT,
            PledgeBalanceMicroRT:  vaults.PledgeBalanceMicroRT,
            StashYieldMicroRT:     stashYield,
            PledgeYieldMicroRT:    pledgeYield,
            TotalYieldMicroRT:     stashYield + pledgeYield,
        }
    }
    
    return &YieldAllocation{
        TotalPoolMicroRT:        totalPool,
        StashAllocationMicroRT:  stashAllocation,
        PledgeAllocationMicroRT: pledgeAllocation,
        memberYields:            memberYields,
    }
}
```

### 5.3 Zero Balance Handling

**No vaults = no yield** (must participate)

```go
// Member with no vault balances receives 0 yield
member := do.eligibleVaultBalances["alice-no-vaults"]
if member == nil || (member.StashBalanceMicroRT == 0 && member.PledgeBalanceMicroRT == 0) {
    // Alice receives 0 RT yield (still pays demurrage if idle balance exists)
}
```

**Incentive**: Encourages all members to stake in vaults (even small amounts).

### 5.4 Yield Distribution Workflow

```
Daily at 00:00 UTC:
  1. DemurrageOrchestrator receives final TorqVault reports
     ↓
  2. Calculate total pool (sum all reports)
     ↓
  3. Query distributed ledger (get all member vault balances)
     ↓
  4. Calculate yield allocation (proportional distribution)
     ↓
  5. Publish allocation to demurrage.orchestrator.allocation
     ↓
  6. All TorqVaults subscribe, apply updates to distributed ledger
     ↓
  7. Members see yield credited to StashVault + PledgeVault
```

**NATS Message**:

```go
type YieldDistribution struct {
    AllocationID             string    // UUID for this distribution
    TotalPoolMicroRT         int64
    StashAllocationMicroRT   int64
    PledgeAllocationMicroRT  int64
    MemberYields             []*MemberYield  // All members
    Timestamp                time.Time
    OrchestratorSignature    []byte
}

// NATS topic: demurrage.orchestrator.allocation
```

**TorqVault applies**:

```go
func (tv *TorqVaultService) handleYieldDistribution(msg *nats.Msg) {
    var dist YieldDistribution
    json.Unmarshal(msg.Data, &dist)
    
    // Verify Orchestrator signature
    if !verifyOrchestratorSignature(dist) {
        tv.logger.Error("invalid orchestrator signature")
        return
    }
    
    // Apply yield to distributed ledger
    for _, memberYield := range dist.MemberYields {
        member := tv.memberVaultBalances[memberYield.MemberID]
        
        atomic.AddInt64(&member.StashBalanceMicroRT, memberYield.StashYieldMicroRT)
        atomic.AddInt64(&member.PledgeBalanceMicroRT, memberYield.PledgeYieldMicroRT)
        
        tv.logger.Info("yield distributed",
            "member_id", memberYield.MemberID,
            "stash_yield", memberYield.StashYieldMicroRT,
            "pledge_yield", memberYield.PledgeYieldMicroRT)
    }
    
    tv.publishMerkleRoot()  // Update distributed ledger state
}
```

---

## 6. Urgent Rerouting

### 6.1 Crisis Trigger

**Configurable policy** detects low vault balances:

```go
type UrgentReroutingPolicy struct {
    Enabled              bool
    
    // Trigger conditions (OR logic)
    TriggerRatioLow      float64  // e.g., 0.1 (StakeVault / DistoVault < 10%)
    RestoreRatioHigh     float64  // e.g., 0.2 (hysteresis: stop at 20%)
    MinStakeBalanceMicroRT int64  // e.g., 100,000 RT absolute minimum
    MaxRejectionRate     int      // e.g., 10 rejections/hour
    
    TriggerCombinationLogic string  // "ratio_OR_absolute_OR_rejection"
}

func (do *DemurrageOrchestrator) checkUrgentTrigger() bool {
    policy := do.urgentReroutingPolicy
    
    if !policy.Enabled {
        return false
    }
    
    // Check vault ratio (across all TorqVaults)
    totalStakeMicroRT := int64(0)
    totalDistoMicroRT := int64(0)
    
    for _, tv := range do.torqVaults {
        totalStakeMicroRT += tv.shadowStakeVaultMicroRT.Load()
        totalDistoMicroRT += tv.shadowDistoVaultMicroRT.Load()
    }
    
    ratio := float64(totalStakeMicroRT) / float64(totalDistoMicroRT)
    
    if ratio < policy.TriggerRatioLow {
        do.logger.Warn("URGENT REROUTING TRIGGERED",
            "reason", "low_vault_ratio",
            "ratio", ratio,
            "threshold", policy.TriggerRatioLow)
        return true
    }
    
    // Check absolute balance
    if totalStakeMicroRT < policy.MinStakeBalanceMicroRT {
        do.logger.Warn("URGENT REROUTING TRIGGERED",
            "reason", "absolute_balance_low",
            "stake_balance", totalStakeMicroRT,
            "threshold", policy.MinStakeBalanceMicroRT)
        return true
    }
    
    // Check rejection rate (if BidNet reporting)
    // ... (implementation omitted for brevity)
    
    return false
}
```

**Auto-detect**: DistoDam monitors continuously (every 10 minutes).

### 6.2 Governance Voting (Quadratic Funding)

**When triggered**, DistoDam publishes funding proposal:

```go
type UrgentFundingProposal struct {
    ProposalID           string
    TriggerReason        string   // "low_vault_ratio", "absolute_balance", etc.
    CurrentRatio         float64
    CurrentStakeMicroRT  int64
    
    // Voting options (members choose duration)
    DurationOptions      []DurationOption
    
    // Voting period
    VotingDeadline       time.Time  // e.g., 48 hours
    
    Timestamp            time.Time
    DistoDamSignature    []byte
}

type DurationOption struct {
    Label               string   // "Aggressive", "Moderate", "Gentle"
    DurationDays        int      // 30, 60, 90
    DiversionPercent    float64  // 0.8, 0.5, 0.3 (% of demurrage rerouted)
}

// NATS topic: governance.urgent.funding.proposal
```

**Example proposal**:

```json
{
  "proposal_id": "urgent-2025-11-18-001",
  "trigger_reason": "low_vault_ratio",
  "current_ratio": 0.08,
  "current_stake_micro_rt": 50000000000,
  "duration_options": [
    {
      "label": "Aggressive (30 days, 80% diversion)",
      "duration_days": 30,
      "diversion_percent": 0.8
    },
    {
      "label": "Moderate (60 days, 50% diversion)",
      "duration_days": 60,
      "diversion_percent": 0.5
    },
    {
      "label": "Gentle (90 days, 30% diversion)",
      "duration_days": 90,
      "diversion_percent": 0.3
    }
  ],
  "voting_deadline": "2025-11-20T00:00:00Z"
}
```

### 6.3 Quadratic Voting

**Members vote** with quadratic weighting (prevents whale dominance):

```go
type UrgentFundingVote struct {
    ProposalID           string
    VoterID              string
    ChosenDurationOption int      // Index: 0 (aggressive), 1 (moderate), 2 (gentle)
    
    // Quadratic funding (optional)
    VoluntaryFundingMicroRT int64  // RT contributed to accelerate recovery
    
    Timestamp            time.Time
    VoterSignature       []byte   // Dilithium2/3
}

// Voting power calculation
func calculateVotingPower(vote UrgentFundingVote) float64 {
    // Base vote = sqrt(RT contributed)
    fundingRT := float64(vote.VoluntaryFundingMicroRT) / 1_000_000
    votingPower := math.Sqrt(fundingRT)
    
    return votingPower
}

// Tally votes
func (do *DemurrageOrchestrator) tallyVotes(proposalID string) *VotingResult {
    votes := do.getVotes(proposalID)
    
    optionScores := make([]float64, 3)  // 3 duration options
    
    for _, vote := range votes {
        power := calculateVotingPower(vote)
        optionScores[vote.ChosenDurationOption] += power
    }
    
    // Winner = highest score
    winningOption := 0
    maxScore := optionScores[0]
    for i, score := range optionScores {
        if score > maxScore {
            maxScore = score
            winningOption = i
        }
    }
    
    return &VotingResult{
        ProposalID:     proposalID,
        WinningOption:  winningOption,
        OptionScores:   optionScores,
    }
}
```

**Example**:
```
Votes received:
  Alice:   Option 0 (Aggressive), 100 RT contributed → sqrt(100) = 10 voting power
  Bob:     Option 1 (Moderate), 400 RT contributed → sqrt(400) = 20 voting power
  Charlie: Option 1 (Moderate), 900 RT contributed → sqrt(900) = 30 voting power
  Dave:    Option 2 (Gentle), 1600 RT contributed → sqrt(1600) = 40 voting power

Scores:
  Option 0 (Aggressive): 10
  Option 1 (Moderate):   50 ← WINNER
  Option 2 (Gentle):     40

Result: Moderate (60 days, 50% diversion) selected
```

**Prevents plutocracy**: 1000x more RT = only 31x more voting power.

### 6.4 Voluntary Funding Acceleration

**Members can contribute RT** to reduce duration:

```go
type VoluntaryFunding struct {
    ProposalID           string
    FunderID             string
    AmountMicroRT        int64
    Timestamp            time.Time
    FunderSignature      []byte
}

// Acceleration calculation
func (do *DemurrageOrchestrator) calculateAcceleration(proposalID string) int {
    baseDuration := do.winningDurationDays
    totalFundingMicroRT := do.getTotalVoluntaryFunding(proposalID)
    targetStakeMicroRT := do.targetStakeBalance
    
    // Funding percentage
    fundingPct := float64(totalFundingMicroRT) / float64(targetStakeMicroRT)
    
    // Adjusted duration = base * (1 - funding%)
    adjustedDuration := float64(baseDuration) * (1.0 - fundingPct)
    
    return int(adjustedDuration)
}
```

**Example**:
```
Winning option: Moderate (60 days base duration)
Target stake balance: 1,000,000 RT
Voluntary funding: 200,000 RT (20% of target)

Acceleration:
  Adjusted duration = 60 * (1 - 0.2) = 48 days

Result: Crisis resolves in 48 days instead of 60 (12 days saved)
```

### 6.5 Rerouting Mechanics

**Permanent rebalancing** (no loan tracking):

```go
func (do *DemurrageOrchestrator) executeUrgentRerouting(result VotingResult) {
    option := do.durationOptions[result.WinningOption]
    
    // Calculate daily diversion
    diversionPct := option.DiversionPercent  // e.g., 0.5 (50%)
    
    // Redirect demurrage during crisis
    do.urgentReroutingActive = true
    do.urgentReroutingDiversionPct = diversionPct
    do.urgentReroutingEndDate = time.Now().AddDate(0, 0, option.DurationDays)
    
    do.logger.Info("URGENT REROUTING ACTIVATED",
        "option", option.Label,
        "duration_days", option.DurationDays,
        "diversion_pct", diversionPct,
        "end_date", do.urgentReroutingEndDate)
}

func (do *DemurrageOrchestrator) calculateYieldAllocationWithRerouting() *YieldAllocation {
    totalPool := do.dailyCollectionsMicroRT.Load()
    
    if do.urgentReroutingActive && time.Now().Before(do.urgentReroutingEndDate) {
        // Divert portion to shadow StakeVaults
        diversionMicroRT := int64(float64(totalPool) * do.urgentReroutingDiversionPct)
        remainingMicroRT := totalPool - diversionMicroRT
        
        // Distribute diversion to shadow StakeVaults
        do.distributeDiversionToShadowVaults(diversionMicroRT)
        
        // Distribute remaining to members (reduced yield)
        return do.calculateYieldAllocation(remainingMicroRT)
    } else {
        // Normal distribution (no rerouting)
        return do.calculateYieldAllocation(totalPool)
    }
}
```

**Member impact** (reduced yield during crisis):

```
Normal yield (no crisis):
  Alice's 1,000 RT → 4.125 RT/day yield

During crisis (50% diversion):
  Alice's 1,000 RT → 2.0625 RT/day yield (50% of normal)

After crisis ends:
  Alice's 1,000 RT → 4.125 RT/day yield (restored)
```

**No payback**: Diverted RT goes to shadow StakeVaults permanently (money supply rebalance).

---

## 7. NATS Message Schemas

### 7.1 Demurrage Flow Topics

| Topic | Publisher | Subscriber | Purpose |
|-------|-----------|------------|---------|
| `demurrage.payment.flow.{torqvault_id}` | Wallet | Specific TorqVault | Hourly demurrage collection |
| `demurrage.rate.broadcast` | TorqVaults | Wallets | Daily rate updates |
| `demurrage.orchestrator.report` | TorqVaults | DemurrageOrchestrator | Daily collection totals |
| `demurrage.orchestrator.allocation` | DemurrageOrchestrator | All TorqVaults | Yield distribution |
| `governance.urgent.funding.proposal` | DistoDam | All members | Emergency rerouting vote |
| `governance.urgent.funding.vote` | Members | DemurrageOrchestrator | Quadratic votes |

### 7.2 Message Schemas (Detailed)

#### 7.2.1 DemurragePaymentFlow

```json
{
  "flow_id": "demurrage-550e8400-e29b-41d4-a716-446655440000",
  "nonce": 1337,
  "sender_address": "RT1a2b3c4d5e6f7g8h9i0j...",
  "recipient_address": "torqvault-alpha-demurrage-addr",
  "amount_micro_rt": 6850,
  "duration_sec": 60,
  "shape_preset": "short",
  "weibull_k": 1.0,
  "calculation_proof": {
    "idle_balance_micro_rt": 1000000000,
    "demurrage_rate": 0.005,
    "time_elapsed_sec": 3600,
    "months_elapsed": 0.00137,
    "demurrage_owed_micro_rt": 6850,
    "balance_proof": "base64-merkle-branch...",
    "timestamp_proof": "base64-signature..."
  },
  "last_collection_timestamp": "2025-11-18T21:00:00Z",
  "timestamp": "2025-11-18T22:00:00Z",
  "signature": "base64-dilithium-signature..."
}
```

#### 7.2.2 DemurrageRateBroadcast

```json
{
  "torqvault_id": "torqvault-alpha-001",
  "minimum_demurrage_rate": 0.005,
  "effective_date": "2025-11-19T00:00:00Z",
  "previous_rate": 0.0049,
  "timestamp": "2025-11-18T22:00:00Z",
  "signature": "base64-dilithium3-signature..."
}
```

#### 7.2.3 DemurrageReport

```json
{
  "torqvault_id": "torqvault-beta-002",
  "reporting_period_start": "2025-11-18T00:00:00Z",
  "reporting_period_end": "2025-11-18T23:59:59Z",
  "total_collected_micro_rt": 1100000000,
  "member_count": 450,
  "flow_count": 10800,
  "timestamp": "2025-11-19T00:00:30Z",
  "signature": "base64-dilithium3-signature..."
}
```

#### 7.2.4 YieldDistribution

```json
{
  "allocation_id": "yield-2025-11-19-001",
  "total_pool_micro_rt": 3300000000,
  "stash_allocation_micro_rt": 1650000000,
  "pledge_allocation_micro_rt": 1650000000,
  "member_yields": [
    {
      "member_id": "alice-001",
      "stash_balance_micro_rt": 1000000000,
      "pledge_balance_micro_rt": 500000000,
      "stash_yield_micro_rt": 3300000,
      "pledge_yield_micro_rt": 2750000,
      "total_yield_micro_rt": 6050000
    }
  ],
  "timestamp": "2025-11-19T00:01:00Z",
  "orchestrator_signature": "base64-dilithium3-signature..."
}
```

#### 7.2.5 UrgentFundingProposal

```json
{
  "proposal_id": "urgent-2025-11-18-001",
  "trigger_reason": "low_vault_ratio",
  "current_ratio": 0.08,
  "current_stake_micro_rt": 50000000000,
  "duration_options": [
    {
      "label": "Aggressive (30 days, 80% diversion)",
      "duration_days": 30,
      "diversion_percent": 0.8
    },
    {
      "label": "Moderate (60 days, 50% diversion)",
      "duration_days": 60,
      "diversion_percent": 0.5
    },
    {
      "label": "Gentle (90 days, 30% diversion)",
      "duration_days": 90,
      "diversion_percent": 0.3
    }
  ],
  "voting_deadline": "2025-11-20T00:00:00Z",
  "timestamp": "2025-11-18T22:30:00Z",
  "distodam_signature": "base64-dilithium3-signature..."
}
```

#### 7.2.6 UrgentFundingVote

```json
{
  "proposal_id": "urgent-2025-11-18-001",
  "voter_id": "bob-wallet-002",
  "chosen_duration_option": 1,
  "voluntary_funding_micro_rt": 400000000,
  "timestamp": "2025-11-19T12:00:00Z",
  "voter_signature": "base64-dilithium2-signature..."
}
```

---

## 8. DemurrageOrchestrator Service

### 8.1 Service Architecture

```go
package orchestrator

import (
    "sync/atomic"
    "time"
    "log/slog"
    "github.com/nats-io/nats.go"
)

type DemurrageOrchestrator struct {
    // Configuration
    stashPledgeSplitRatio float64  // 0.5 = 50/50
    urgentReroutingPolicy UrgentReroutingPolicy
    
    // State
    dailyCollectionsMicroRT atomic.Int64
    torqVaultReports        map[string]*DemurrageReport
    eligibleVaultBalances   map[string]*VaultBalances
    
    // Urgent rerouting state
    urgentReroutingActive     bool
    urgentReroutingDiversionPct float64
    urgentReroutingEndDate    time.Time
    
    // NATS
    natsClient *nats.Conn
    
    // Logging & Metrics
    logger  *slog.Logger
    metrics *Metrics
}

func (do *DemurrageOrchestrator) Start(ctx context.Context) error {
    // Subscribe to TorqVault reports
    do.natsClient.Subscribe("demurrage.orchestrator.report", do.handleDemurrageReport)
    
    // Subscribe to votes
    do.natsClient.Subscribe("governance.urgent.funding.vote", do.handleVote)
    
    // Daily allocation job (00:00 UTC)
    go do.runDailyAllocationJob(ctx)
    
    // Urgent trigger monitor (every 10 minutes)
    go do.runUrgentTriggerMonitor(ctx)
    
    do.logger.Info("DemurrageOrchestrator started")
    
    <-ctx.Done()
    return nil
}

func (do *DemurrageOrchestrator) runDailyAllocationJob(ctx context.Context) {
    ticker := time.NewTicker(24 * time.Hour)
    defer ticker.Stop()
    
    for {
        select {
        case <-ticker.C:
            do.processDailyAllocation()
        case <-ctx.Done():
            return
        }
    }
}

func (do *DemurrageOrchestrator) processDailyAllocation() {
    // 1. Calculate yield allocation
    allocation := do.calculateYieldAllocationWithRerouting()
    
    // 2. Publish to all TorqVaults
    do.publishYieldDistribution(allocation)
    
    // 3. Reset daily counter
    do.dailyCollectionsMicroRT.Store(0)
    do.torqVaultReports = make(map[string]*DemurrageReport)
    
    do.logger.Info("daily yield allocation complete",
        "total_pool", allocation.TotalPoolMicroRT,
        "members_paid", len(allocation.memberYields))
}
```

### 8.2 Deployment

**Standalone service** (separate container):

```yaml
# docker-compose.yaml
services:
  demurrage-orchestrator:
    build: ./src/demurrage-orchestrator
    environment:
      - NATS_URL=nats://nats:4222
      - STASH_PLEDGE_SPLIT_RATIO=0.5
      - URGENT_REROUTING_ENABLED=true
      - URGENT_TRIGGER_RATIO_LOW=0.1
      - URGENT_RESTORE_RATIO_HIGH=0.2
      - LOG_LEVEL=info
    depends_on:
      - nats
    restart: unless-stopped
```

**Why separate service?**
- Network-wide responsibility (not TorqVault-specific)
- Governance integration (neutral, not vault-affiliated)
- Scalability (single orchestrator, many TorqVaults)

---

## 9. Governance Integration

### 9.1 DistoDam Integration

**DistoDam** monitors vault ratios, triggers proposals:

```go
// In DistoDam service
func (dd *DistoDam) monitorVaultRatios() {
    ticker := time.NewTicker(10 * time.Minute)
    defer ticker.Stop()
    
    for {
        select {
        case <-ticker.C:
            ratio := dd.calculateNetworkVaultRatio()
            
            if ratio < dd.urgentTriggerRatioLow {
                // Trigger urgent funding proposal
                dd.publishUrgentFundingProposal(ratio)
            }
        case <-dd.ctx.Done():
            return
        }
    }
}
```

### 9.2 Voting Period

**Duration**: 48 hours (configurable)

**Why 48 hours?**
- Long enough for global participation (time zones)
- Short enough to respond to crisis quickly
- Balances urgency with deliberation

### 9.3 Vote Tallying

**After voting deadline**:

```go
func (do *DemurrageOrchestrator) finalizeVoting(proposalID string) {
    result := do.tallyVotes(proposalID)
    
    // Execute winning option
    do.executeUrgentRerouting(result)
    
    // Notify network
    do.publishVotingResult(result)
    
    do.logger.Info("voting finalized",
        "proposal_id", proposalID,
        "winning_option", result.WinningOption,
        "option_scores", result.OptionScores)
}
```

---

## 10. Phase Roadmap

### Phase 1: Mock Demurrage (Current)

**Goals**:
- ⏳ Implement hourly demurrage calculation (Wallet-side)
- ⏳ Implement DemurragePaymentFlow (short flow, 60 seconds)
- ⏳ Implement TorqVault verification (merkle proof check)
- ⏳ Implement fixed demurrage rate (0.5%/month, no adjustment)

**Simplifications**:
- No DemurrageOrchestrator (single TorqVault distributes yield)
- No network-wide pool (local distribution only)
- No urgent rerouting (DistoDam shadow vaults static)
- Fixed 50/50 Stash/Pledge split (no governance)

**Deliverable**: Functional demurrage collection and yield distribution (single-vault testnet)

### Phase 2: Network-Wide Aggregation

**Goals**:
- Implement DemurrageOrchestrator service
- Implement daily TorqVault reporting
- Implement network-wide pool aggregation
- Implement proportional yield distribution (all members)

**New components**:
- `DemurrageOrchestrator` service (standalone)
- `demurrage.orchestrator.report` NATS topic
- `demurrage.orchestrator.allocation` NATS topic

**Deliverable**: Multi-TorqVault network with fair global yield rates

### Phase 3: Dynamic Rate Adjustment

**Goals**:
- Implement rate broadcasting (daily updates)
- Implement rate change limits (max ±0.01%/day)
- Implement velocity-based rate tuning (governance-controlled)

**New components**:
- `RateAdjustmentEngine` (DistoDam component)
- `demurrage.rate.broadcast` NATS topic

**Deliverable**: Self-adjusting demurrage rates based on economic velocity

### Phase 4: Urgent Rerouting

**Goals**:
- Implement urgent trigger detection (vault ratio monitoring)
- Implement quadratic voting (governance proposals)
- Implement voluntary funding acceleration
- Implement permanent demurrage rerouting (shadow vaults)

**New components**:
- `UrgentFundingProposalEngine` (DistoDam)
- `QuadraticVotingTally` (DemurrageOrchestrator)
- `governance.urgent.*` NATS topics

**Deliverable**: Crisis-responsive demurrage system with member governance

### Phase 5: Advanced Features

**Goals**:
- Implement demurrage exemptions (contract-locked RT, emergency reserves)
- Implement demurrage smoothing (moving average instead of daily spikes)
- Implement yield reinvestment options (auto-compound to PledgeVault)
- Implement demurrage analytics dashboard (Grafana)

**Deliverable**: Production-ready demurrage system with advanced economic controls

---

## Appendix: Economic Simulations

### A.1 Velocity Impact Simulation

**Scenario**: 1000 members, 1,000,000 RT total supply

```python
# No demurrage
members_holding_idle = 700  # 70% hoard
average_hold_time = 180  # 6 months
velocity = 1000000 / (700 * 180) = 7.94  # Low velocity

# With 0.5%/month demurrage
members_holding_idle = 200  # 20% hoard (rest stake or spend)
average_hold_time = 30  # 1 month
velocity = 1000000 / (200 * 30) = 166.67  # High velocity

# Result: 21x velocity increase
```

### A.2 Yield vs Demurrage Breakeven

**Question**: At what staking % does yield offset demurrage?

```python
# Assumptions
demurrage_rate = 0.005  # 0.5%/month
total_supply = 1000000  # RT
staking_pct = 0.6  # 60% staked

# Demurrage collected (from idle 40%)
idle_supply = total_supply * 0.4
demurrage_collected = idle_supply * demurrage_rate
# = 400,000 * 0.005 = 2,000 RT

# Yield rate (distributed to stakers)
staked_supply = total_supply * 0.6
yield_rate = demurrage_collected / staked_supply
# = 2,000 / 600,000 = 0.00333 (0.333%/month)

# Net yield for staker
net_yield = yield_rate - demurrage_rate
# = 0.00333 - 0.005 = -0.00167 (-0.167%/month)

# Conclusion: 60% staking is BELOW breakeven
# Need higher staking % or higher demurrage on idle RT
```

**Breakeven calculation**:
```python
# Let staking_pct = x
# Demurrage collected = (1 - x) * total_supply * demurrage_rate
# Yield rate = demurrage_collected / (x * total_supply)
# Net yield = 0 when: yield_rate = demurrage_rate

# (1 - x) * demurrage_rate / x = demurrage_rate
# (1 - x) / x = 1
# 1 - x = x
# x = 0.5

# Breakeven: 50% staking (exactly offsets demurrage)
# Above 50%: Stakers profit
# Below 50%: Stakers lose (but still better than idle)
```

### A.3 Urgent Rerouting Impact

**Scenario**: Vault ratio drops to 0.08, moderate option (60 days, 50% diversion) selected

```python
# Normal yield
daily_demurrage = 3300  # RT/day
member_yield_pct = 0.004125  # 0.4125%/day

# During crisis (50% diverted)
daily_diversion = 3300 * 0.5  # 1650 RT/day to shadow StakeVaults
remaining_yield = 3300 * 0.5  # 1650 RT/day to members
member_yield_pct = 0.002062  # 0.2062%/day (50% reduction)

# Crisis duration
duration_days = 60

# Total diverted to shadow vaults
total_diverted = 1650 * 60 = 99,000 RT

# Vault ratio after crisis (assuming no other changes)
initial_stake_vault = 50,000 RT
final_stake_vault = 50,000 + 99,000 = 149,000 RT
disto_vault = 625,000 RT (unchanged)

final_ratio = 149,000 / 625,000 = 0.238 (23.8%)
# Result: Crisis resolved (ratio > 0.2 restore threshold)
```

**Voluntary funding acceleration**:
```python
# Members voluntarily contribute 30,000 RT (30% of diverted amount)
funding_pct = 30,000 / 99,000 = 0.303

# Adjusted duration
adjusted_duration = 60 * (1 - 0.303) = 41.8 days
# Result: Crisis resolves 18.2 days faster (30% acceleration)
```

---

## Conclusion

RoboTorq's demurrage system is a **self-balancing economic engine**:

- **Hourly collection** ensures frequent, small charges (no shock)
- **Network-wide aggregation** guarantees equal percentage yield (fairness)
- **Proportional distribution** rewards vault participation (incentive alignment)
- **Quadratic governance** gives members crisis control (democratic)
- **Permanent rerouting** rebalances money supply (no debt accumulation)

**Implementation Status**: Design complete, Phase 1 (mock demurrage) ready to implement.

**Next Steps**: Build `DemurrageOrchestrator` service, integrate with TorqVault distributed ledger, implement hourly collection flows.

---

*"Idle money decays. Active money thrives. RoboTorq rewards participation."* 💰⚡
