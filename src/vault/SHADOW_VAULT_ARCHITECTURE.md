# Shadow Vault Architecture

**Version**: 1.0  
**Date**: November 18, 2025  
**Status**: Design Complete  
**Author**: GitHub Copilot (Claude Sonnet 4.5)
**Editor**: Jon Clark

---

## Executive Summary

**Shadow Vaults** are DistoDam-controlled vault pairs (StakeVault + DistoVault) allocated to each TorqVault service in the network. They serve as **operational reserves** and **crisis liquidity buffers**.

**Key Characteristics**:
- **1 pair per TorqVault service**: Every TorqVault has exactly one Shadow StakeVault and one Shadow DistoVault
- **DistoDam-controlled**: Members cannot withdraw, only DistoDam can allocate/divert
- **Automatic rebalancing**: Urgent demurrage rerouting refills Shadow StakeVaults during crises
- **Distributed ledger storage**: Like member vaults, shadow vaults use event sourcing and consensus
- **Network-wide visibility**: All TorqVaults can query shadow vault balances (transparency)

**This is NOT**:
- ❌ Member-owned savings (those are StashVault/PledgeVault)
- ❌ TorqVault profit centers (shadow vaults are network reserves)
- ❌ Hidden slush funds (fully auditable via distributed ledger)

---

## Table of Contents

1. [Shadow Vault Types](#1-shadow-vault-types)
2. [DistoDam Integration](#2-distodam-integration)
3. [Funding Sources](#3-funding-sources)
4. [Urgent Demurrage Rerouting](#4-urgent-demurrage-rerouting)
5. [Shadow Vault Allocation Logic](#5-shadow-vault-allocation-logic)
6. [Event Sourcing Implementation](#6-event-sourcing-implementation)
7. [NATS Message Schemas](#7-nats-message-schemas)
8. [Monitoring & Alerts](#8-monitoring--alerts)
9. [Phase Roadmap](#9-phase-roadmap)

---

## 1. Shadow Vault Types

### 1.1 Shadow StakeVault

**Purpose**: Operational liquidity buffer for BidNet escrow

**Funding sources**:
1. **Transaction fees** (BidNet escrow winners pay 0.1-0.5% fee → Shadow StakeVault)
2. **Flow cancellation penalties** (100% of undelivered amount → Shadow StakeVault)
3. **Urgent demurrage rerouting** (50-80% of demurrage pool during crisis → Shadow StakeVaults)
4. **Initial network bootstrapping** (genesis allocation from DistoDam)

**Use cases**:
- **BidNet liquidity backstop**: If TorqVault's primary liquidity pool depleted, shadow vault provides emergency RT
- **Oracle settlement buffer**: Ensures Oracle can always complete drips (even if TorqVault offline)
- **Crisis recapitalization**: Urgent rerouting refills shadow vaults when network ratio drops below threshold

**Example**:
```
TorqVault Alpha's Shadow StakeVault balance: 150,000 RT

Sources:
  - Transaction fees:         50,000 RT (6 months of BidNet escrow fees)
  - Cancellation penalties:   30,000 RT (users who cancelled flows)
  - Urgent rerouting:         70,000 RT (demurrage diverted during crisis)
  
Usage:
  - Emergency liquidity fronting: 0 RT (never needed yet)
  - Held as reserve:             150,000 RT
```

### 1.2 Shadow DistoVault

**Purpose**: Network operational expenses and infrastructure funding

**Funding sources**:
1. **DistoDam operational fees** (small % of UBD distributions → Shadow DistoVaults)
2. **Oracle service fees** (TorqVaults pay Oracle operators → Shadow DistoVaults)
3. **Slashing penalties** (malicious TorqVault operators penalized → Shadow DistoVaults)
4. **Unused demurrage** (if urgent rerouting ends early, surplus → Shadow DistoVaults)

**Use cases**:
- **Oracle operator compensation**: Pay 3-5 independent Oracle operators for threshold signature service
- **Network infrastructure**: NATS servers, monitoring, logging, backups
- **Governance costs**: Quadratic voting systems, proposal management
- **Security audits**: Third-party code reviews, penetration testing

**Example**:
```
TorqVault Beta's Shadow DistoVault balance: 75,000 RT

Sources:
  - Operational fees:    25,000 RT (DistoDam allocations)
  - Oracle fees:         40,000 RT (6 months of Oracle compensation)
  - Slashing penalties:  10,000 RT (1 Byzantine TorqVault slashed)
  
Usage:
  - Oracle payments:     50,000 RT (compensate 5 Oracles @ 10k each)
  - Infrastructure:      15,000 RT (NATS JetStream storage costs)
  - Held as reserve:     10,000 RT
```

---

## 2. DistoDam Integration

### 2.1 Shadow Vault Ownership

**Who controls shadow vaults?** DistoDam service (exclusively)

**Why?**
- Members shouldn't access operational reserves (prevents bank run)
- TorqVaults can't self-allocate (prevents embezzlement)
- DistoDam acts as neutral arbiter (governance-driven)

**Data Structure**:
```go
// In TorqVaultService
type TorqVaultService struct {
    // ... other fields
    
    // Shadow vaults (DistoDam-controlled)
    shadowStakeVaultMicroRT atomic.Int64  // Liquidity buffer
    shadowDistoVaultMicroRT atomic.Int64  // Operational reserve
    
    // Ledger events
    shadowVaultEvents []*ShadowVaultEvent
}

type ShadowVaultEvent struct {
    EventID          string
    EventType        string  // "fee_deposit", "urgent_rerouting", "oracle_payment", etc.
    TorqVaultID      string
    VaultType        string  // "shadow_stake" or "shadow_disto"
    AmountMicroRT    int64
    Source           string  // "transaction_fee", "cancellation_penalty", "demurrage_diversion", etc.
    Timestamp        time.Time
    DistoDamSignature []byte  // Only DistoDam can modify shadow vaults
}
```

### 2.2 DistoDam Allocation Authority

**Only DistoDam can**:
- Allocate funds TO shadow vaults (from demurrage pool, fees)
- Withdraw funds FROM shadow vaults (to pay Oracles, infrastructure)
- Trigger urgent rerouting (divert demurrage to shadow StakeVaults)

**TorqVaults CANNOT**:
- Modify shadow vault balances directly
- Transfer shadow vault RT to liquidity pool
- Pay themselves from shadow vaults

**Enforcement**:
```go
// All shadow vault events require DistoDam signature
func (tv *TorqVaultService) handleShadowVaultEvent(msg *nats.Msg) {
    var event ShadowVaultEvent
    json.Unmarshal(msg.Data, &event)
    
    // Verify DistoDam signature (prevent tampering)
    if !tv.verifyDistoDamSignature(event) {
        tv.logger.Error("INVALID DISTODAM SIGNATURE - shadow vault event rejected",
            "event_id", event.EventID,
            "torqvault_id", event.TorqVaultID)
        return
    }
    
    // Apply event to shadow vault
    switch event.VaultType {
    case "shadow_stake":
        atomic.AddInt64(&tv.shadowStakeVaultMicroRT, event.AmountMicroRT)
    case "shadow_disto":
        atomic.AddInt64(&tv.shadowDistoVaultMicroRT, event.AmountMicroRT)
    }
    
    tv.logger.Info("shadow vault event applied",
        "event_type", event.EventType,
        "vault_type", event.VaultType,
        "amount_micro_rt", event.AmountMicroRT)
}
```

### 2.3 Shadow Vault Ratio Monitoring

**DistoDam continuously monitors**: `StakeVault / DistoVault` ratio across all TorqVaults

```go
// In DistoDam service
func (dd *DistoDam) calculateNetworkVaultRatio() float64 {
    totalStakeMicroRT := int64(0)
    totalDistoMicroRT := int64(0)
    
    // Query all TorqVaults for shadow vault balances
    for _, torqVaultID := range dd.activeTorqVaults {
        balances := dd.queryShadowVaultBalances(torqVaultID)
        totalStakeMicroRT += balances.ShadowStakeMicroRT
        totalDistoMicroRT += balances.ShadowDistoMicroRT
    }
    
    if totalDistoMicroRT == 0 {
        return 0.0  // Avoid division by zero
    }
    
    ratio := float64(totalStakeMicroRT) / float64(totalDistoMicroRT)
    
    dd.logger.Info("network vault ratio calculated",
        "total_stake_micro_rt", totalStakeMicroRT,
        "total_disto_micro_rt", totalDistoMicroRT,
        "ratio", ratio)
    
    return ratio
}
```

**Trigger thresholds** (configurable via governance):
- **Low ratio** (< 0.1): Urgent rerouting triggered (crisis mode)
- **Restore ratio** (> 0.2): Urgent rerouting ends (normal mode)
- **Optimal ratio** (0.3-0.5): Healthy network state

---

## 3. Funding Sources

### 3.1 Transaction Fees (→ Shadow StakeVault)

**When**: BidNet escrow awarded, winner fronts liquidity

**Fee structure**:
```
Standard RT payment:      0.1% of flow amount
Currency exchange:        0.2% of flow amount
Labor contract payment:   0.15% of flow amount
```

**Example**:
```
Alice sends 1000 RT to Bob (1-hour flow)
TorqVault Alpha wins bid (fee: 1 RT = 0.1%)

Flow:
  1. Alpha fronts 1000 RT to Bob (instant)
  2. Oracle drips 1000 RT from Alice → Alpha (over 1 hour)
  3. Alice pays 1 RT fee → Alpha's Shadow StakeVault
  
Alpha's Shadow StakeVault += 1 RT
```

**NATS Event**:
```go
type TransactionFeeEvent struct {
    EventID          string
    TorqVaultID      string
    FlowID           string
    FeeMicroRT       int64
    FeePercent       float64  // 0.001 = 0.1%
    Timestamp        time.Time
    DistoDamSignature []byte
}

// Published to: vault.shadow.fee_deposit
```

### 3.2 Cancellation Penalties (→ Shadow StakeVault)

**When**: User cancels flow (100% penalty on undelivered amount)

**Example**:
```
Alice sends 1000 RT to Bob (1-hour flow)
TorqVault Beta fronts 1000 RT to Bob instantly
30 minutes elapse → 500 RT delivered (via Weibull drip)
Alice cancels flow

Penalty calculation:
  Delivered:    500 RT (Bob keeps)
  Undelivered:  500 RT (penalty)
  
Alice loses:    500 RT
Bob receives:   500 RT (already credited)
Beta receives:  500 RT → Shadow StakeVault (penalty compensation)
```

**NATS Event**:
```go
type CancellationPenaltyEvent struct {
    EventID          string
    TorqVaultID      string
    FlowID           string
    PenaltyMicroRT   int64
    UndeliveredMicroRT int64
    Timestamp        time.Time
    DistoDamSignature []byte
}

// Published to: vault.shadow.cancellation_penalty
```

### 3.3 Urgent Demurrage Rerouting (→ Shadow StakeVaults)

**When**: Network vault ratio < 0.1 (crisis)

**Diversion**:
- Aggressive: 80% of daily demurrage → Shadow StakeVaults
- Moderate: 50% of daily demurrage → Shadow StakeVaults
- Gentle: 30% of daily demurrage → Shadow StakeVaults

**Example** (Moderate rerouting):
```
Daily demurrage collected: 100,000 RT (network-wide)

Normal distribution:
  StashVault yield:  50,000 RT (50%)
  PledgeVault yield: 50,000 RT (50%)
  
During crisis (50% diversion):
  Shadow StakeVaults:  50,000 RT (diverted, distributed across all TorqVaults)
  StashVault yield:    25,000 RT (reduced)
  PledgeVault yield:   25,000 RT (reduced)
```

**Distribution logic** (proportional to TorqVault member count):
```go
// In DemurrageOrchestrator
func (do *DemurrageOrchestrator) distributeUrgentRerouting(diversionMicroRT int64) {
    totalMembers := 0
    for _, tv := range do.torqVaults {
        totalMembers += tv.MemberCount
    }
    
    for _, tv := range do.torqVaults {
        // Proportional allocation
        allocationMicroRT := (diversionMicroRT * int64(tv.MemberCount)) / int64(totalMembers)
        
        // Publish allocation event
        event := ShadowVaultEvent{
            EventID:       uuid.New().String(),
            EventType:     "urgent_rerouting",
            TorqVaultID:   tv.ID,
            VaultType:     "shadow_stake",
            AmountMicroRT: allocationMicroRT,
            Source:        "demurrage_diversion",
            Timestamp:     time.Now(),
            DistoDamSignature: do.signWithDilithium3(event),
        }
        
        do.natsClient.Publish("vault.shadow.urgent_allocation", event.ToJSON())
        
        do.logger.Info("urgent rerouting allocated",
            "torqvault_id", tv.ID,
            "allocation_micro_rt", allocationMicroRT,
            "member_count", tv.MemberCount)
    }
}
```

**NATS Event**:
```go
type UrgentReroutingAllocation struct {
    EventID          string
    TorqVaultID      string
    AllocationMicroRT int64
    DiverionPercent  float64  // 0.5 = 50% of demurrage pool
    DurationDays     int      // How long rerouting active
    Timestamp        time.Time
    DistoDamSignature []byte
}

// Published to: vault.shadow.urgent_allocation
```

### 3.4 Operational Fees (→ Shadow DistoVault)

**When**: DistoDam allocates portion of UBD for network operations

**Fee structure**:
```
UBD distribution: 1000 RT
Operational fee:  0.5% = 5 RT → Shadow DistoVault
Net to member:    995 RT
```

**Example**:
```
Monthly UBD distribution to Alice: 5000 RT
Operational fee (0.5%):            25 RT → TorqVault Alpha's Shadow DistoVault
Alice receives:                    4975 RT
```

**NATS Event**:
```go
type OperationalFeeEvent struct {
    EventID          string
    TorqVaultID      string
    MemberID         string
    UBDAmountMicroRT int64
    FeeMicroRT       int64
    FeePercent       float64  // 0.005 = 0.5%
    Timestamp        time.Time
    DistoDamSignature []byte
}

// Published to: vault.shadow.operational_fee
```

---

## 4. Urgent Demurrage Rerouting

### 4.1 Crisis Detection

**DistoDam monitors** vault ratio every 10 minutes:

```go
func (dd *DistoDam) monitorVaultRatios() {
    ticker := time.NewTicker(10 * time.Minute)
    defer ticker.Stop()
    
    for {
        select {
        case <-ticker.C:
            ratio := dd.calculateNetworkVaultRatio()
            
            if ratio < dd.urgentTriggerRatioLow {
                dd.logger.Warn("URGENT REROUTING TRIGGERED",
                    "ratio", ratio,
                    "threshold", dd.urgentTriggerRatioLow)
                
                dd.publishUrgentFundingProposal(ratio)
            } else if ratio > dd.urgentRestoreRatioHigh && dd.urgentReroutingActive {
                dd.logger.Info("URGENT REROUTING ENDED - ratio restored",
                    "ratio", ratio,
                    "threshold", dd.urgentRestoreRatioHigh)
                
                dd.deactivateUrgentRerouting()
            }
            
        case <-dd.ctx.Done():
            return
        }
    }
}
```

### 4.2 Quadratic Voting on Duration

**Members vote** on rerouting duration (NOT whether to activate):

**Proposal published**:
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

**Quadratic vote weighting**:
```go
func calculateVotingPower(voluntaryFundingMicroRT int64) float64 {
    fundingRT := float64(voluntaryFundingMicroRT) / 1_000_000
    return math.Sqrt(fundingRT)  // sqrt(RT contributed)
}

// Example:
// Alice contributes 100 RT → voting power = sqrt(100) = 10
// Bob contributes 10,000 RT → voting power = sqrt(10,000) = 100
// Bob has 100x more RT but only 10x more voting power
```

**Winner selection**:
```go
func (dd *DistoDam) tallyVotes(proposalID string) *VotingResult {
    votes := dd.getVotes(proposalID)
    
    optionScores := make([]float64, 3)  // 3 duration options
    
    for _, vote := range votes {
        power := calculateVotingPower(vote.VoluntaryFundingMicroRT)
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

### 4.3 Voluntary Funding Acceleration

**Members can contribute RT** to reduce crisis duration:

```go
type VoluntaryFunding struct {
    ProposalID           string
    FunderID             string
    AmountMicroRT        int64
    Timestamp            time.Time
    FunderSignature      []byte
}

// Acceleration calculation
func (dd *DistoDam) calculateAcceleration(proposalID string) int {
    baseDuration := dd.winningDurationDays
    totalFundingMicroRT := dd.getTotalVoluntaryFunding(proposalID)
    targetStakeMicroRT := dd.targetStakeBalance
    
    // Funding percentage
    fundingPct := float64(totalFundingMicroRT) / float64(targetStakeMicroRT)
    
    // Adjusted duration = base * (1 - funding%)
    adjustedDuration := float64(baseDuration) * (1.0 - fundingPct)
    
    return int(adjustedDuration)
}
```

**Example**:
```
Winning option: Moderate (60 days, 50% diversion)
Target stake balance: 1,000,000 RT
Voluntary funding: 300,000 RT (30% of target)

Acceleration:
  Adjusted duration = 60 * (1 - 0.3) = 42 days

Result: Crisis resolves in 42 days instead of 60 (18 days saved)
```

### 4.4 Permanent Rebalancing (No Loan Tracking)

**Diverted demurrage** goes to shadow StakeVaults permanently (not a loan):

```go
// During urgent rerouting
func (do *DemurrageOrchestrator) distributeYieldWithRerouting() {
    totalPool := do.dailyCollectionsMicroRT.Load()
    
    if do.urgentReroutingActive && time.Now().Before(do.urgentReroutingEndDate) {
        // Divert portion to shadow StakeVaults
        diversionMicroRT := int64(float64(totalPool) * do.urgentReroutingDiversionPct)
        remainingMicroRT := totalPool - diversionMicroRT
        
        // Distribute diversion to shadow vaults (PERMANENT)
        do.distributeUrgentRerouting(diversionMicroRT)
        
        // Distribute remaining to members (reduced yield)
        do.distributeYieldToMembers(remainingMicroRT)
    } else {
        // Normal distribution (no rerouting)
        do.distributeYieldToMembers(totalPool)
    }
}
```

**No payback**: Shadow StakeVaults keep diverted RT forever (money supply rebalance, not debt)

---

## 5. Shadow Vault Allocation Logic

### 5.1 Proportional Distribution

**Why proportional?** Fairness (larger TorqVaults serve more members, need more reserves)

**Allocation formula**:
```
TorqVault_Allocation = Total_Amount × (TorqVault_Members / Network_Members)
```

**Example**:
```
Network state:
  TorqVault Alpha:  500 members
  TorqVault Beta:   300 members
  TorqVault Gamma:  200 members
  Total:            1000 members

Urgent rerouting: 100,000 RT diverted

Allocations:
  Alpha:  100,000 × (500/1000) = 50,000 RT
  Beta:   100,000 × (300/1000) = 30,000 RT
  Gamma:  100,000 × (200/1000) = 20,000 RT
```

### 5.2 Dynamic Reallocation

**If TorqVault membership changes**, reallocate future diversions:

```go
func (dd *DistoDam) updateMembershipCounts() {
    for _, tv := range dd.torqVaults {
        memberCount := dd.queryMemberCount(tv.ID)
        tv.MemberCount = memberCount
    }
    
    dd.logger.Info("membership counts updated",
        "torqvaults", len(dd.torqVaults),
        "total_members", dd.getTotalMembers())
}

// Run every 24 hours
func (dd *DistoDam) runMembershipUpdateJob(ctx context.Context) {
    ticker := time.NewTicker(24 * time.Hour)
    defer ticker.Stop()
    
    for {
        select {
        case <-ticker.C:
            dd.updateMembershipCounts()
        case <-ctx.Done():
            return
        }
    }
}
```

---

## 6. Event Sourcing Implementation

### 6.1 Shadow Vault Event Schema

```go
type ShadowVaultEvent struct {
    EventID          string    `json:"event_id"`
    EventType        string    `json:"event_type"`  // "fee_deposit", "urgent_rerouting", etc.
    TorqVaultID      string    `json:"torqvault_id"`
    VaultType        string    `json:"vault_type"`  // "shadow_stake" or "shadow_disto"
    AmountMicroRT    int64     `json:"amount_micro_rt"`
    Source           string    `json:"source"`      // "transaction_fee", "demurrage_diversion", etc.
    Metadata         map[string]interface{} `json:"metadata,omitempty"`  // Extra context
    Timestamp        time.Time `json:"timestamp"`
    DistoDamSignature []byte   `json:"distodam_signature"`
}
```

### 6.2 NATS JetStream Storage

**Durable stream**:
```bash
nats stream add vault-shadow \
  --subjects "vault.shadow.*" \
  --storage file \
  --retention limits \
  --max-age 365d \
  --max-msgs -1 \
  --discard old
```

**Event replay** (resync shadow vault balances):
```go
func (tv *TorqVaultService) resyncShadowVaults() error {
    tv.logger.Info("resyncing shadow vaults from event log")
    
    // Reset balances
    atomic.StoreInt64(&tv.shadowStakeVaultMicroRT, 0)
    atomic.StoreInt64(&tv.shadowDistoVaultMicroRT, 0)
    
    // Subscribe to all shadow vault events (replay from beginning)
    js, _ := tv.natsClient.JetStream()
    
    sub, err := js.Subscribe("vault.shadow.*", func(msg *nats.Msg) {
        tv.handleShadowVaultEvent(msg)
        msg.Ack()
    }, nats.DeliverAll())
    
    if err != nil {
        return err
    }
    
    time.Sleep(10 * time.Second)  // Wait for replay
    sub.Unsubscribe()
    
    tv.logger.Info("shadow vault resync complete",
        "shadow_stake_micro_rt", atomic.LoadInt64(&tv.shadowStakeVaultMicroRT),
        "shadow_disto_micro_rt", atomic.LoadInt64(&tv.shadowDistoVaultMicroRT))
    
    return nil
}
```

---

## 7. NATS Message Schemas

### 7.1 Shadow Vault Event Topics

| Topic | Publisher | Subscriber | Purpose |
|-------|-----------|------------|---------|
| `vault.shadow.fee_deposit` | DistoDam | All TorqVaults | Transaction fees → Shadow StakeVault |
| `vault.shadow.cancellation_penalty` | DistoDam | All TorqVaults | Flow cancellation penalties → Shadow StakeVault |
| `vault.shadow.urgent_allocation` | DistoDam | All TorqVaults | Urgent demurrage rerouting → Shadow StakeVaults |
| `vault.shadow.operational_fee` | DistoDam | All TorqVaults | UBD operational fees → Shadow DistoVault |
| `vault.shadow.oracle_payment` | DistoDam | All TorqVaults | Oracle compensation → Shadow DistoVault |
| `vault.shadow.balance_query` | Any | All TorqVaults | Query shadow vault balances |

### 7.2 Example Messages

#### 7.2.1 Transaction Fee Deposit

```json
{
  "event_id": "550e8400-e29b-41d4-a716-446655440000",
  "event_type": "fee_deposit",
  "torqvault_id": "torqvault-alpha-001",
  "vault_type": "shadow_stake",
  "amount_micro_rt": 1000000,
  "source": "transaction_fee",
  "metadata": {
    "flow_id": "payment-770g0622...",
    "fee_percent": 0.001
  },
  "timestamp": "2025-11-18T22:00:00Z",
  "distodam_signature": "base64-dilithium3-signature..."
}
```

#### 7.2.2 Urgent Rerouting Allocation

```json
{
  "event_id": "660f9511-f3ac-52e5-b827-557766551001",
  "event_type": "urgent_rerouting",
  "torqvault_id": "torqvault-beta-002",
  "vault_type": "shadow_stake",
  "amount_micro_rt": 30000000000,
  "source": "demurrage_diversion",
  "metadata": {
    "proposal_id": "urgent-2025-11-18-001",
    "diversion_percent": 0.5,
    "duration_days": 60,
    "member_count": 300
  },
  "timestamp": "2025-11-18T22:00:00Z",
  "distodam_signature": "base64-dilithium3-signature..."
}
```

#### 7.2.3 Oracle Payment

```json
{
  "event_id": "770h0733-g4bd-63f6-c938-668877662002",
  "event_type": "oracle_payment",
  "torqvault_id": "torqvault-gamma-003",
  "vault_type": "shadow_disto",
  "amount_micro_rt": -10000000000,
  "source": "oracle_compensation",
  "metadata": {
    "oracle_id": "oracle-001",
    "payment_period": "2025-11",
    "services_rendered": "threshold_signatures"
  },
  "timestamp": "2025-11-30T00:00:00Z",
  "distodam_signature": "base64-dilithium3-signature..."
}
```

---

## 8. Monitoring & Alerts

### 8.1 Prometheus Metrics

```go
type ShadowVaultMetrics struct {
    // Balances
    ShadowStakeBalance prometheus.GaugeVec  // Per TorqVault
    ShadowDistoBalance prometheus.GaugeVec  // Per TorqVault
    
    // Ratios
    NetworkVaultRatio  prometheus.Gauge     // StakeVault / DistoVault
    
    // Events
    FeeDepositsTotal   prometheus.CounterVec  // By TorqVault
    UrgentAllocationsTotal prometheus.Counter
    OraclePaymentsTotal prometheus.Counter
    
    // Urgent rerouting
    UrgentReroutingActive  prometheus.Gauge  // 0 or 1
    UrgentReroutingDaysRemaining prometheus.Gauge
}
```

### 8.2 Grafana Dashboard

**Panels**:
1. **Network Vault Ratio** (time series)
   - Red zone: < 0.1 (crisis)
   - Yellow zone: 0.1-0.2 (warning)
   - Green zone: > 0.2 (healthy)

2. **Shadow Vault Balances** (stacked area chart)
   - Per TorqVault: StakeVault + DistoVault

3. **Urgent Rerouting Status** (stat panel)
   - Active: Yes/No
   - Days remaining: X days
   - Diversion %: 50%

4. **Transaction Fees** (bar chart)
   - Per TorqVault: Daily fee income

5. **Oracle Payments** (table)
   - Oracle ID, Payment amount, Date

### 8.3 Alerting Rules

```yaml
groups:
  - name: shadow_vaults
    rules:
      - alert: LowVaultRatio
        expr: network_vault_ratio < 0.1
        for: 10m
        annotations:
          summary: "Network vault ratio critically low"
          description: "Ratio {{ $value }} < 0.1 threshold"
        
      - alert: ShadowStakeVaultDepleted
        expr: shadow_stake_balance{torqvault_id=~".*"} < 10000
        for: 5m
        annotations:
          summary: "Shadow StakeVault nearly empty"
          description: "TorqVault {{ $labels.torqvault_id }} has {{ $value }} RT remaining"
      
      - alert: UrgentReroutingStuck
        expr: urgent_rerouting_active == 1 and urgent_rerouting_days_remaining > 90
        for: 1h
        annotations:
          summary: "Urgent rerouting duration excessive"
          description: "Rerouting active for {{ $value }} days (check governance)"
```

---

## 9. Phase Roadmap

### Phase 1: Mock Shadow Vaults (Current)

**Goals**:
- ⏳ Implement shadow vault fields in TorqVaultService
- ⏳ Implement DistoDam allocation logic (proportional distribution)
- ⏳ Implement transaction fee deposits (BidNet winner → shadow StakeVault)
- ⏳ Implement basic monitoring (Prometheus metrics)

**Simplifications**:
- No urgent rerouting (shadow vaults static)
- No Oracle payments (shadow DistoVault unused)
- Single TorqVault (no distribution logic needed)

**Deliverable**: Functional shadow vault storage (testnet)

### Phase 2: Urgent Rerouting

**Goals**:
- Implement vault ratio monitoring (DistoDam)
- Implement quadratic voting on duration
- Implement voluntary funding acceleration
- Implement demurrage diversion (DemurrageOrchestrator)
- Implement automatic rerouting end (ratio > 0.2)

**New components**:
- `governance.urgent.funding.proposal` NATS topic
- `governance.urgent.funding.vote` NATS topic
- `vault.shadow.urgent_allocation` NATS topic

**Deliverable**: Crisis-responsive shadow vault system

### Phase 3: Oracle Payments

**Goals**:
- Implement multi-Oracle compensation (from shadow DistoVaults)
- Implement threshold signature verification (2-of-3 Oracles)
- Implement Oracle slashing (malicious behavior → shadow DistoVault)

**New components**:
- `OracleCompensationManager` (DistoDam component)
- `vault.shadow.oracle_payment` NATS topic

**Deliverable**: Self-sustaining Oracle network (funded by shadow DistoVaults)

### Phase 4: Advanced Features

**Goals**:
- Implement shadow vault insurance (protect against extreme crises)
- Implement cross-TorqVault shadow vault rebalancing
- Implement shadow vault transparency dashboard (public audit)
- Implement governance control of allocation ratios

**Deliverable**: Production-ready shadow vault ecosystem

---

## Conclusion

Shadow Vaults are the **operational backbone** of the RoboTorq network:

- **StakeVault**: Emergency liquidity buffer (BidNet escrow, crisis recapitalization)
- **DistoVault**: Network operations funding (Oracles, infrastructure, governance)
- **DistoDam-controlled**: Neutral arbiter (prevents embezzlement, ensures fairness)
- **Proportional allocation**: Larger TorqVaults get more reserves (fairness)
- **Urgent rerouting**: Quadratic governance + voluntary funding (democratic crisis response)
- **Permanent rebalancing**: No loan tracking (money supply adjustment)

**Implementation Status**: Design complete, Phase 1 (mock shadow vaults) ready to implement.

**Next Steps**: Integrate with DistoDam service, implement proportional allocation, deploy shadow vault monitoring.

---

*"Shadow vaults: the network's immune system. Crisis triggers → members decide → reserves deploy."* 🛡️⚡
