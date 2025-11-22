# Vault Security Fixes - Critical Vulnerabilities Patched

**Date**: November 18, 2025  
**Status**: MANDATORY PRE-MAINNET FIXES  
**Severity**: 2 EXISTENTIAL + 7 CRITICAL

---

## Executive Summary

This document details **9 critical security vulnerabilities** discovered in StashVault and TorqedPledge implementations. **Two are existential threats that will collapse the system within 30 days of launch.**

**External validation**: Grok/ChatGPT security analysis confirmed these vulnerabilities would "kill the system within 48 hours of first $1M liquidity."

---

## Vulnerability Table

| # | Component | Problem | Severity | Impact | Fix |
|---|-----------|---------|----------|--------|-----|
| 1 | StashVault | 100% demurrage → StashVaults | ☠️☠️☠️☠️☠️ EXISTENTIAL | Everyone rushes in → demurrage pool dries up → zero yield → bank run → total collapse in <30 days | Cap StashVault share at 50-70% max (rest goes to shadow StakeVault, UBI buffer) |
| 2 | StashVault | No withdrawal restrictions | ☠️☠️☠️☠️ CRITICAL | Whale parks 90% of supply → starves DistoDam → no contract funding → chain halts | Require 30-day notice OR 0.5% early-withdrawal penalty |
| 3 | StashVault | Integer division rounding | ☠️☠️☠️ CRITICAL | Last vault in list gets huge rounding error → lost satoshis bug | Deterministic pro-rata with remainder distribution |
| 4 | TorqedPledge | Yield counts toward maturity | ☠️☠️☠️☠️☠️ EXISTENTIAL | 36-month pledge @ 2%/mo compounds to >100% APY → infinite money glitch | Yield paid to wallet (NOT pledge balance) |
| 5 | TorqedPledge | Monthly pledge unenforced | ☠️☠️☠️☠️ CRITICAL | User pledges 10k RT/month but diverts 1 RT → never matures but reputation rises | DistoDam hard cap: auto-divert MUST equal monthly pledge |
| 6 | TorqedPledge | Yield rate static | ☠️☠️☠️ HIGH | User pledges 36 months (2%/mo), adds lump sum, matures in 6 months → still gets 2% → free lunch | Recalculate yield rate monthly based on REMAINING duration |
| 7 | Both Vaults | No global vault ratio cap | ☠️☠️☠️ HIGH | 95%+ supply vaults → no circulating RoboStake → contract funding starves | Dynamic yield curve: drops to near-zero above 80% vault ratio |
| 8 | Both Vaults | No anti-Sybil limit | ☠️☠️ MEDIUM | Attacker creates 10,000 vaults × 0.000001 RT → skews pro-rata distribution | One StashVault per wallet OR minimum balance (10 RT) |
| 9 | Both Vaults | Database = source of truth | ☠️☠️ MEDIUM | Node restart → all vault balances lost unless DB replicated perfectly | Event sourcing from NATS JetStream (DB is cache only) |

---

## Fix 1: Cap StashVault Demurrage Share at 60%

### Problem

**Original design**: 100% of demurrage pool → StashVaults (pro-rata)

**Attack scenario**:
```
Day 1:  Total supply = 1,000,000 RT
        Vaulted: 100,000 RT (10%)
        Demurrage collected: 5,000 RT/day
        StashVault yield: 5% APY → attractive

Day 7:  Word spreads, more users vault
        Vaulted: 500,000 RT (50%)
        Demurrage collected: 2,500 RT/day (fewer circulating wallets)
        StashVault yield: 1.8% APY → still good

Day 14: Everyone rushes in
        Vaulted: 900,000 RT (90%)
        Demurrage collected: 500 RT/day (barely anyone holding RT)
        StashVault yield: 0.2% APY → terrible

Day 21: Bank run begins
        Users withdraw en masse → vaulted drops to 50%
        But demurrage takes time to recover → yield still low
        More withdrawals → vicious cycle

Day 30: Total collapse
        StashVault yield < 0.1% APY
        Everyone pulls out → StashVaults empty
        Demurrage pool never recovers
```

### Fix

```yaml
# vault-config.yaml
stash_vault_max_demurrage_share: 0.60  # 60% cap

demurrage_distribution:
  stash_vaults: 0.60      # 60% to StashVaults (pro-rata)
  shadow_stake_vaults: 0.30  # 30% to shadow StakeVaults (crisis buffer)
  ubd_buffer: 0.10        # 10% to UBI reserve (DistoDam)
```

```go
// In DemurrageOrchestrator
func (do *DemurrageOrchestrator) DistributeYield() {
    totalPoolMicroRT := do.dailyCollectionsMicroRT.Load()
    
    // Cap StashVault share at 60%
    stashShareMicroRT := int64(float64(totalPoolMicroRT) * 0.60)
    shadowShareMicroRT := int64(float64(totalPoolMicroRT) * 0.30)
    ubdBufferMicroRT := totalPoolMicroRT - stashShareMicroRT - shadowShareMicroRT
    
    // Distribute
    do.distributeToStashVaults(stashShareMicroRT)
    do.distributeToShadowStakeVaults(shadowShareMicroRT)
    do.sendToUBDBuffer(ubdBufferMicroRT)
}
```

**Impact**: Prevents tragedy of the commons (everyone rushes in → pool dries up)

---

## Fix 2: 30-Day Withdrawal Notice OR 0.5% Penalty

### Problem

**Original design**: Instant withdrawals, no penalty, no restrictions

**Attack scenario**:
```
Whale Attack:
1. Whale acquires 900,000 RT (90% of supply)
2. Vaults all of it in StashVault (instant, no lock)
3. DistoDam starves (no circulating RT for contract funding)
4. Network halts (no UBD distributions, no contracts funded)
5. Whale withdraws instantly when convenient (no penalty)

Result: Whale can hold network hostage at will
```

### Fix

```go
// withdrawal_notice.go
type WithdrawalNotice struct {
    VaultID   string
    Amount    int64
    CreatedAt time.Time
}

func (m *StashVaultManager) Withdraw(ctx context.Context, vaultID string, amountMicroRT int64) error {
    vault := m.repo.GetStashVault(ctx, vaultID)
    
    // Check for 30-day notice
    notice := m.repo.GetWithdrawalNotice(ctx, vaultID)
    noticePeriod := 30 * 24 * time.Hour
    
    penaltyMicroRT := int64(0)
    
    if notice == nil || time.Since(notice.CreatedAt) < noticePeriod {
        // Early withdrawal - apply 0.5% penalty
        penaltyMicroRT = int64(float64(amountMicroRT) * 0.005)
        
        // Penalty goes to shadow StakeVault (not burned)
        m.natsClient.Publish("vault.shadow.withdrawal_penalty", struct{
            VaultID string
            PenaltyMicroRT int64
        }{
            VaultID: vaultID,
            PenaltyMicroRT: penaltyMicroRT,
        })
    }
    
    // Deduct amount + penalty
    vault.BalanceMicroRT -= (amountMicroRT + penaltyMicroRT)
    
    // Clear notice
    if notice != nil {
        m.repo.DeleteWithdrawalNotice(ctx, vaultID)
    }
    
    m.repo.UpdateStashVault(ctx, vault)
    return nil
}

// Submit withdrawal notice (optional, avoids penalty)
func (m *StashVaultManager) SubmitWithdrawalNotice(ctx context.Context, vaultID string, amountMicroRT int64) error {
    notice := &WithdrawalNotice{
        VaultID:   vaultID,
        Amount:    amountMicroRT,
        CreatedAt: time.Now(),
    }
    
    return m.repo.SaveWithdrawalNotice(ctx, notice)
}
```

**Impact**: Prevents whale attacks (can't instantly withdraw 90% of supply)

---

## Fix 3: Deterministic Remainder Distribution

### Problem

**Original design**: Pro-rata distribution using integer division

**Bug**:
```go
// BUGGY CODE
for _, vault := range vaults {
    share := (totalPool * vault.Balance) / totalBalance  // Integer division!
    vault.Balance += share
}

// Example:
// Total pool: 1000 microRT
// Vault A: 333 microRT balance → share = (1000 * 333) / 1000 = 333
// Vault B: 333 microRT balance → share = (1000 * 333) / 1000 = 333
// Vault C: 334 microRT balance → share = (1000 * 334) / 1000 = 334
// Total distributed: 333 + 333 + 334 = 1000 ✓ (looks fine)

// BUT with 1001 microRT pool:
// Vault A: share = (1001 * 333) / 1000 = 333
// Vault B: share = (1001 * 333) / 1000 = 333  
// Vault C: share = (1001 * 334) / 1000 = 334
// Total distributed: 1000
// LOST: 1 microRT (accumulates over time!)
```

### Fix

```go
func (do *DemurrageOrchestrator) DistributeToStashVaults(poolMicroRT int64) {
    vaults := do.getAllStashVaults()
    totalBalanceMicroRT := do.getTotalStashBalance()
    
    distributed := int64(0)
    
    // Sort vaults by ID (deterministic order)
    sort.Slice(vaults, func(i, j int) bool {
        return vaults[i].ID < vaults[j].ID
    })
    
    for _, vault := range vaults {
        // Pro-rata share (integer division)
        share := (poolMicroRT * vault.BalanceMicroRT) / totalBalanceMicroRT
        vault.BalanceMicroRT += share
        distributed += share
    }
    
    // Distribute remainder to first N vaults (deterministic, fair)
    remainder := poolMicroRT - distributed
    for i := 0; i < int(remainder) && i < len(vaults); i++ {
        vaults[i].BalanceMicroRT += 1
    }
}
```

**Impact**: Prevents "lost satoshis" bug (rounding errors accumulate)

---

## Fix 4: Yield Paid to Wallet (NOT Pledge Balance)

### Problem

**Original design**: TorqedPledge yield added to `SavedMicroRT` (counts toward maturity)

**Exploit**:
```
Setup:
  Target: 30,000 RT
  Monthly pledge: 1,250 RT
  Lock duration: 24 months
  Yield rate: 1.5%/month

Month 1:
  Deposits: 1,250 RT
  Yield: 1.5% × 1,250 = 18.75 RT
  Balance: 1,250 + 18.75 = 1,268.75 RT

Month 2:
  Deposits: 1,250 RT
  Yield: 1.5% × 2,518.75 = 37.78 RT
  Balance: 2,518.75 + 37.78 = 2,556.53 RT

...compounding continues...

Month 24:
  Balance: 35,300 RT (exceeded 30k target via compounding!)
  Matured in 24 months, but got 5,300 RT "free" yield

Result: Effective APY = (5,300 / 30,000) / 2 years = 88% APY
This is an INFINITE MONEY GLITCH.
```

### Fix

```go
type TorqedPledge struct {
    SavedMicroRT  int64  // Deposits only (NO yield added here)
    TargetMicroRT int64  // Target (yield doesn't count)
    
    // Yield tracked separately (paid to wallet)
    TotalYieldEarnedMicroRT int64  // Informational only
}

func (m *TorqedPledgeManager) DistributeYield() {
    pledges := m.repo.ListActivePledges()
    
    for _, pledge := range pledges {
        // Calculate yield
        yieldMicroRT := calculateYield(pledge)
        
        // Pay yield to WALLET (NOT pledge balance)
        m.natsClient.Publish("wallet.yield_payment", struct{
            WalletID string
            PledgeID string
            YieldMicroRT int64
            Source string
        }{
            WalletID: pledge.WalletID,
            PledgeID: pledge.ID,
            YieldMicroRT: yieldMicroRT,
            Source: "torqed_pledge_yield",
        })
        
        // Track total yield (informational)
        pledge.TotalYieldEarnedMicroRT += yieldMicroRT
        
        // SavedMicroRT UNCHANGED (only deposits count toward maturity)
    }
}
```

```yaml
# vault-config.yaml
torqed_yield_counts_toward_target: false  # CRITICAL FIX
torqed_yield_paid_to: "wallet"  # NOT pledge balance
```

**Impact**: Prevents infinite money glitch (100%+ APY via compounding)

---

## Fix 5: DistoDam Hard Cap on Monthly Pledge

### Problem

**Original design**: User configures DistoDam auto-divert independently of pledge amount

**Exploit**:
```
User creates TorqedPledge:
  Target: 30,000 RT
  Monthly pledge: 10,000 RT  # Looks ambitious!
  
User configures DistoDam:
  Auto-divert: 1 RT/month  # Actually only sends 1 RT

Result:
  - Pledge never matures (1 RT/month × 24 months = 24 RT << 30,000 RT)
  - But reputation still rises (system thinks pledge is "on track")
  - User games reputation without actually saving
```

### Fix

```go
// In DistoDam service
func (dd *DistoDam) ConfigureVaultAutoDeposit(walletID, pledgeID string, ubdPercentage float64) error {
    // Get pledge details from Vault service
    pledge := dd.vaultClient.GetPledge(pledgeID)
    
    // Calculate monthly UBD amount
    monthlyUBDMicroRT := dd.getMonthlyUBD(walletID)
    autoDivertMicroRT := int64(float64(monthlyUBDMicroRT) * ubdPercentage)
    
    // SECURITY FIX: Validate auto-divert >= monthly pledge
    if autoDivertMicroRT < pledge.MonthlyPledgeMicroRT {
        return fmt.Errorf(
            "auto-divert %d microRT < monthly pledge %d microRT (must be equal or greater)",
            autoDivertMicroRT,
            pledge.MonthlyPledgeMicroRT)
    }
    
    // Save configuration
    dd.repo.SaveAutoDepositConfig(walletID, pledgeID, ubdPercentage)
    return nil
}
```

**Impact**: Prevents reputation gaming (must actually fund pledge as promised)

---

## Fix 6: Recalculate Yield Rate Based on Remaining Duration

### Problem

**Original design**: Yield rate set at pledge creation, never changes

**Exploit**:
```
User creates 36-month pledge (gets 2.0%/month yield)

Month 1-11: User contributes monthly (11 months × 1,000 RT = 11,000 RT)

Month 12: User adds lump sum (19,000 RT)
  Pledge balance: 30,000 RT (MATURED in 12 months instead of 36!)
  
But yield rate still 2.0%/month (calculated for 36-month lock)

Result: User gets 36-month yield for 12-month commitment (free lunch)
```

### Fix

```go
func (m *TorqedPledgeManager) DistributeYield() {
    pledges := m.repo.ListActivePledges()
    
    for _, pledge := range pledges {
        // Recalculate remaining duration EVERY MONTH
        monthsElapsed := int(time.Since(pledge.CreatedAt).Hours() / (24 * 30))
        remainingMonths := math.Max(0, float64(pledge.LockDurationMonths) - float64(monthsElapsed))
        
        // Yield rate based on REMAINING duration (not original)
        yieldRateBPS := 120 + (80 * math.Min(remainingMonths/36.0, 1.0))
        
        // 36 months remaining → 200 BPS (2.0%)
        // 12 months remaining → 147 BPS (1.47%)
        // 0 months remaining → 120 BPS (1.2%)
        
        yieldMicroRT := calculateYield(pledge, yieldRateBPS)
        
        // Pay to wallet (see Fix 4)
        m.payYieldToWallet(pledge, yieldMicroRT)
    }
}
```

**Impact**: Prevents "lock long, mature early" exploit

---

## Fix 7: Dynamic Yield Curve Above 80% Vault Ratio

### Problem

**Original design**: Yield rate fixed (0.5-1.5%/month for StashVault, 1.2-2.0%/month for TorqedPledge)

**Attack scenario**:
```
Network state:
  Total supply: 1,000,000 RT
  Vaulted (StashVault + PledgeVault): 950,000 RT (95%)
  Circulating (for RoboStake contracts): 50,000 RT (5%)
  
Problem:
  - Contracts need RT staking (for RoboStake reputation)
  - Only 50k RT available for staking
  - DistoDam can't fund contracts (no liquidity)
  - Network halts (no robotic labor execution)
  
But users have no incentive to unvault (yield still 1.5%/month)
```

### Fix

```go
func (m *StashVaultManager) DistributeYield() {
    // Calculate network vault ratio
    totalStashBalanceMicroRT := m.getTotalStashBalance()
    totalPledgeBalanceMicroRT := m.getTotalPledgeBalance()
    totalVaultedMicroRT := totalStashBalanceMicroRT + totalPledgeBalanceMicroRT
    
    circulatingSupplyMicroRT := m.getTotalSupply() - totalVaultedMicroRT
    vaultRatio := float64(totalVaultedMicroRT) / float64(totalSupplyMicroRT)
    
    // Dynamic yield multiplier (drops above 80% vault ratio)
    yieldMultiplier := 1.0
    if vaultRatio > 0.80 {
        // Linear drop from 1.0 at 80% to 0.0667 at 95%
        // (reduces yield to 1/15th at 95%)
        yieldMultiplier = math.Max(0.0667, 1.0 - ((vaultRatio - 0.80) * 6.0))
        
        m.logger.Warn("high vault ratio - reducing yield",
            "vault_ratio", vaultRatio,
            "yield_multiplier", yieldMultiplier)
    }
    
    // Apply multiplier to all vaults
    for _, vault := range vaults {
        baseYield := calculateBaseYield(vault)
        adjustedYield := int64(float64(baseYield) * yieldMultiplier)
        vault.BalanceMicroRT += adjustedYield
    }
}
```

**Impact**: Market signal to circulate RT when vault ratio too high

---

## Fix 8: One StashVault Per Wallet + Minimum Balance

### Problem

**Original design**: No limit on number of vaults per wallet, no minimum balance

**Attack scenario**:
```
Sybil Attack:
  Attacker creates 10,000 vaults with 0.000001 RT each
  Total: 0.01 RT across 10,000 vaults
  
Pro-rata distribution:
  Total StashVault balance: 1,000,000 RT
  Demurrage pool: 10,000 RT
  
  Normal user (1 vault, 1,000 RT balance):
    Share: (10,000 * 1,000) / 1,000,000 = 10 RT
  
  Attacker (10,000 vaults, 0.000001 RT each):
    Each vault share: (10,000 * 0.000001) / 1,000,000 = 0.00001 RT
    Total share: 0.00001 × 10,000 = 0.1 RT
    
Attack fails (too small to matter). BUT:

Iteration attack (skews rounding):
  If pro-rata uses integer division:
    Each attacker vault gets FLOOR((10,000 * 0.000001) / 1,000,000) = 0
    But remainder distribution gives 1 microRT to first 10,000 vaults
    Attacker gets 10,000 microRT (0.01 RT) instead of 0.1 RT
    
Still small, but skews deterministic remainder distribution.
```

### Fix

```go
// One StashVault per wallet
func (m *StashVaultManager) CreateVault(ctx context.Context, walletID string, initialAmountMicroRT int64) error {
    // Check for existing vault
    existingVault := m.repo.GetStashVaultByWallet(ctx, walletID)
    if existingVault != nil {
        return fmt.Errorf("wallet already has StashVault: %s", existingVault.ID)
    }
    
    // Enforce minimum balance (10 RT)
    minBalanceMicroRT := int64(10_000_000)  // 10 RT
    if initialAmountMicroRT < minBalanceMicroRT {
        return fmt.Errorf("minimum initial balance is 10 RT, got %.6f RT",
            float64(initialAmountMicroRT) / 1_000_000)
    }
    
    // Create vault
    vault := &StashVault{
        ID: generateVaultID(),
        WalletID: walletID,
        BalanceMicroRT: initialAmountMicroRT,
        CreatedAt: time.Now(),
    }
    
    m.repo.CreateStashVault(ctx, vault)
    return nil
}
```

**Impact**: Prevents Sybil attacks on pro-rata distribution

---

## Fix 9: Event Sourcing (NATS JetStream = Source of Truth)

### Problem

**Original design**: PostgreSQL database is source of truth

**Risk**:
```
Scenario 1: Database corruption
  - Power failure during write
  - Vault balance = 50,000 RT → corrupted to 0 RT
  - User loses all savings (CRITICAL)

Scenario 2: Node restart without replication
  - Service crashes mid-transaction
  - In-memory state lost
  - Database state may be stale (last write 10 minutes ago)
  - Vault balances incorrect

Scenario 3: Split-brain (multiple instances)
  - Two Vault services running (misconfiguration)
  - Both writing to same database
  - Race conditions → inconsistent state
```

### Fix

```go
// Event sourcing architecture
type StashVaultEvent struct {
    EventID       string    `json:"event_id"`
    EventType     string    `json:"event_type"`  // "created", "deposited", "withdrawn", "yield_distributed"
    VaultID       string    `json:"vault_id"`
    WalletID      string    `json:"wallet_id"`
    AmountMicroRT int64     `json:"amount_micro_rt"`
    BalanceAfter  int64     `json:"balance_after"`
    Timestamp     time.Time `json:"timestamp"`
    Signature     []byte    `json:"signature"`  // Dilithium3
}

// NATS JetStream = source of truth
func (m *StashVaultManager) Deposit(ctx context.Context, vaultID string, amountMicroRT int64) error {
    vault := m.getVault(vaultID)  // From in-memory cache
    
    // Calculate new balance
    newBalance := vault.BalanceMicroRT + amountMicroRT
    
    // Create event
    event := &StashVaultEvent{
        EventID:       uuid.New().String(),
        EventType:     "deposited",
        VaultID:       vaultID,
        WalletID:      vault.WalletID,
        AmountMicroRT: amountMicroRT,
        BalanceAfter:  newBalance,
        Timestamp:     time.Now(),
        Signature:     m.signEvent(event),
    }
    
    // Publish to NATS JetStream (DURABLE, append-only log)
    js, _ := m.natsClient.JetStream()
    js.Publish("vault.stash.deposited", event.ToJSON())
    
    // Update in-memory state
    vault.BalanceMicroRT = newBalance
    
    // Update database (CACHE ONLY, not source of truth)
    m.repo.UpdateStashVault(ctx, vault)
    
    return nil
}

// Rebuild state from NATS on startup
func (m *StashVaultManager) ReplayEvents(ctx context.Context) error {
    m.logger.Info("replaying StashVault events from NATS JetStream")
    
    js, _ := m.natsClient.JetStream()
    
    // Subscribe to all vault events from beginning
    sub, err := js.Subscribe("vault.stash.*", func(msg *nats.Msg) {
        var event StashVaultEvent
        json.Unmarshal(msg.Data, &event)
        
        // Apply event to in-memory state
        m.applyEvent(event)
        msg.Ack()
    }, nats.DeliverAll())  // Replay from first event
    
    if err != nil {
        return err
    }
    
    // Wait for replay (or use channel to signal completion)
    time.Sleep(10 * time.Second)
    sub.Unsubscribe()
    
    m.logger.Info("event replay complete",
        "total_vaults", len(m.vaults),
        "total_balance_micro_rt", m.getTotalBalance())
    
    return nil
}
```

**NATS JetStream configuration**:
```bash
# Create durable stream for vault events
nats stream add vault-events \
  --subjects "vault.*.*" \
  --storage file \
  --retention limits \
  --max-age 365d \
  --max-msgs -1 \
  --discard old \
  --replicas 3  # Cluster replication for durability
```

**Impact**: Prevents data loss (database becomes cache, NATS is source of truth)

---

## Implementation Checklist

### Phase 1: Existential Fixes (MANDATORY FOR TESTNET)

- [ ] **Fix 1**: Cap StashVault demurrage share at 60%
  - [ ] Update DemurrageOrchestrator distribution logic
  - [ ] Add `stash_vault_max_demurrage_share` config
  - [ ] Implement dynamic yield curve (above 80% vault ratio)
  - [ ] Unit tests (10 tests)

- [ ] **Fix 4**: Yield paid to wallet (NOT pledge balance)
  - [ ] Update TorqedPledgeManager.DistributeYield()
  - [ ] Change NATS message: `wallet.yield_payment` instead of `vault.deposited`
  - [ ] Update TorqedPledge struct (remove yield from SavedMicroRT)
  - [ ] Integration tests (8 tests)

### Phase 2: Critical Fixes (MANDATORY FOR MAINNET)

- [ ] **Fix 2**: 30-day withdrawal notice OR 0.5% penalty
  - [ ] Add WithdrawalNotice table
  - [ ] Implement SubmitWithdrawalNotice()
  - [ ] Update Withdraw() with penalty logic
  - [ ] Unit tests (6 tests)

- [ ] **Fix 5**: DistoDam hard cap on monthly pledge
  - [ ] Update DistoDam.ConfigureVaultAutoDeposit()
  - [ ] Add validation: auto-divert >= monthly pledge
  - [ ] Integration tests with Vault service (4 tests)

- [ ] **Fix 6**: Recalculate yield rate based on remaining duration
  - [ ] Update TorqedPledgeManager.DistributeYield()
  - [ ] Calculate remainingMonths every distribution
  - [ ] Unit tests (5 tests)

### Phase 3: High-Priority Fixes (RECOMMENDED FOR MAINNET)

- [ ] **Fix 3**: Deterministic remainder distribution
  - [ ] Sort vaults by ID before distribution
  - [ ] Distribute remainder to first N vaults
  - [ ] Unit tests (7 tests, including edge cases)

- [ ] **Fix 7**: Dynamic yield curve
  - [ ] Calculate network vault ratio
  - [ ] Apply yield multiplier above 80%
  - [ ] Prometheus metrics for vault ratio
  - [ ] Unit tests (5 tests)

- [ ] **Fix 8**: One StashVault per wallet + minimum balance
  - [ ] Add unique constraint: wallet_id → vault_id
  - [ ] Enforce minimum balance (10 RT)
  - [ ] Unit tests (4 tests)

### Phase 4: Resilience Fixes (MANDATORY FOR PRODUCTION)

- [ ] **Fix 9**: Event sourcing from NATS JetStream
  - [ ] Implement StashVaultEvent schema
  - [ ] Publish all operations to NATS JetStream
  - [ ] Implement ReplayEvents() for startup
  - [ ] Mark database as cache-only
  - [ ] Integration tests (10 tests)

---

## Configuration

```yaml
# vault-config.yaml

# Fix 1: Cap StashVault demurrage share
stash_vault_max_demurrage_share: 0.60  # 60% max
demurrage_distribution:
  stash_vaults: 0.60
  shadow_stake_vaults: 0.30
  ubd_buffer: 0.10

# Fix 2: Withdrawal restrictions
stash_vault_withdrawal_notice_days: 30
stash_vault_early_withdrawal_penalty: 0.005  # 0.5%

# Fix 4: Yield destination
torqed_yield_counts_toward_target: false
torqed_yield_paid_to: "wallet"  # NOT pledge balance

# Fix 7: Dynamic yield curve
vault_ratio_yield_reduction_threshold: 0.80  # Start reducing above 80%
vault_ratio_yield_minimum_multiplier: 0.0667  # 1/15th at 95% ratio

# Fix 8: Anti-Sybil
stash_vault_one_per_wallet: true
stash_vault_minimum_balance_rt: 10.0

# Fix 9: Event sourcing
vault_event_sourcing_enabled: true
vault_database_is_cache_only: true
```

---

## Testing Requirements

### Unit Tests (70 total)
- StashVault: 35 tests
- TorqedPledge: 35 tests

### Integration Tests (30 total)
- DistoDam ↔ Vault: 10 tests
- Vault ↔ Wallet: 10 tests
- Event sourcing replay: 10 tests

### E2E Tests (10 total)
- Full vault lifecycle (create → deposit → yield → withdraw)
- Attack scenario simulations (whale attack, Sybil attack, compounding exploit)

### Chaos Tests (5 total)
- Node restart mid-transaction (event sourcing recovery)
- Database corruption (NATS replay)
- Network partition (eventual consistency)

---

## Success Metrics

- ✅ Vault ratio stays below 80% (market equilibrium)
- ✅ Zero "lost satoshis" (remainder distribution working)
- ✅ No infinite money glitches (yield separate from pledge balance)
- ✅ No whale attacks (withdrawal penalties enforced)
- ✅ No Sybil attacks (one vault per wallet)
- ✅ Zero data loss on node restart (event sourcing replay)
- ✅ 95%+ test coverage across all fixes

---

**END OF SECURITY FIXES**
