# StashVault Implementation - Low-Risk Savings

## 📋 **Document Metadata**
- **Component**: StashVault (within Vault Service)
- **Purpose**: Low-risk savings with instant liquidity + demurrage protection
- **Version**: v0.1.0 (MVP)
- **Author**: Jonathan Clark (@GrokkingGrok)
- **Date**: November 14, 2025
- **Parent Document**: [VAULT_ARCHITECTURE.md](VAULT_ARCHITECTURE.md)

---

## 🎯 **StashVault Overview**

### **What is a StashVault?**

A **StashVault** is the RoboTorq network's equivalent of a **savings account**, but better:

| Traditional Savings | StashVault |
|---------------------|------------|
| Bank holds your money | You hold your keys |
| 0.01% interest (if lucky) | 0.5%/month base + demurrage pool |
| Bank lends it out for profit | **You** earn from others' demurrage |
| Can freeze/seize funds | Instant withdrawal, always |
| Inflation erodes value | Physics-backed RT |

### **Core Value Proposition**:

1. **Demurrage Protection**: RT in StashVault is **exempt from demurrage**
2. **Yield Source**: Pro-rata share of **100% of network demurrage**
   - No guaranteed "base yield" - yield varies with network demurrage
   - High network demurrage (idle wallets) = high StashVault yield
   - Low network demurrage (everyone vaulting) = low yield BUT cheap credit
3. **Instant Liquidity**: Withdraw anytime, no penalty, no lock
4. **No Counterparty Risk**: Non-custodial, you control the vault

---

## 🏗️ **StashVault Architecture**

### **Component: StashVaultManager**

**Responsibility**: Manage all StashVault operations (create, deposit, withdraw, yield)

**Interface**:
```go
type StashVaultManager interface {
    // Vault operations
    CreateVault(ctx context.Context, walletID string) (*StashVault, error)
    GetVault(ctx context.Context, vaultID string) (*StashVault, error)
    GetVaultsByWallet(ctx context.Context, walletID string) ([]*StashVault, error)
    
    // Balance operations
    Deposit(ctx context.Context, vaultID string, amountMicroRT int64, source string) error
    Withdraw(ctx context.Context, vaultID string, amountMicroRT int64) error
    GetBalance(ctx context.Context, vaultID string) (int64, error)
    
    // Yield operations
    DistributeDemurragePool(ctx context.Context, totalDemurrageMicroRT int64) error
    
    // Statistics
    GetStats() StashVaultStats
    GetTotalBalance() int64  // Sum of all StashVault balances
}

type StashVault struct {
    ID                      string    `json:"id" db:"id"`
    WalletID                string    `json:"wallet_id" db:"wallet_id"`
    BalanceMicroRT          int64     `json:"balance" db:"balance_micro_rt"`
    TotalDepositedMicroRT   int64     `json:"total_deposited" db:"total_deposited_micro_rt"`
    TotalWithdrawnMicroRT   int64     `json:"total_withdrawn" db:"total_withdrawn_micro_rt"`
    LastYieldAt             time.Time `json:"last_yield_at" db:"last_yield_at"`
    CreatedAt               time.Time `json:"created_at" db:"created_at"`
    UpdatedAt               time.Time `json:"updated_at" db:"updated_at"`
}

type StashVaultStats struct {
    TotalVaults          int64
    TotalBalanceMicroRT  int64
    TotalDeposits        int64
    TotalWithdrawals     int64
    TotalYieldsDistributed int64
    LastYieldDistribution  time.Time
}
```

---

## 💰 **Yield Mechanics**

### **Demurrage Pool Distribution** (100% of network demurrage)

**Source**: Demurrage collected from wallet balances (calculated on phones, settled daily)
**Distribution**: Daily batch, pro-rata to all StashVaults
**Formula**:
```
StashVault_Share = (StashVault_Balance / Total_StashVault_Balance) × Total_Demurrage

Example (Daily):
- Your StashVault: 1,000 RT
- Total StashVaults: 100,000 RT
- Daily Demurrage Collected: 50 RT
- Your Share: (1,000 / 100,000) × 50 = 0.5 RT

Example (Monthly accumulation):
- Daily demurrage: 50 RT/day
- Monthly total: 50 × 30 = 1,500 RT
- Your monthly share: (1,000 / 100,000) × 1,500 = 15 RT
- Effective yield: 15 / 1,000 = 1.5%/month
```

**Key Insight**: Yield is NOT guaranteed - it's a market signal!
- High demurrage collected = lots of idle wallets = high StashVault yield
- Low demurrage collected = everyone vaulting = low yield BUT cheap credit available

**Implementation**:
```go
func (m *StashVaultManager) DistributeDemurragePool(ctx context.Context, totalDemurrageMicroRT int64) error {
    // Get all StashVaults
    vaults, err := m.repo.ListAllStashVaults(ctx)
    if err != nil {
        return err
    }
    
    // Calculate total StashVault balance
    totalBalanceMicroRT := int64(0)
    for _, vault := range vaults {
        totalBalanceMicroRT += vault.BalanceMicroRT
    }
    
    // No StashVaults? No distribution (should never happen)
    if totalBalanceMicroRT == 0 {
        return fmt.Errorf("no StashVault balance to distribute demurrage to")
    }
    
    now := time.Now()
    distributedTotal := int64(0)
    
    for _, vault := range vaults {
        // Calculate pro-rata share
        // share = (vault_balance / total_balance) × total_demurrage
        shareMicroRT := (vault.BalanceMicroRT * totalDemurrageMicroRT) / totalBalanceMicroRT
        
        // Credit vault
        vault.BalanceMicroRT += shareMicroRT
        vault.UpdatedAt = now
        
        // Update database
        if err := m.repo.UpdateStashVault(ctx, vault); err != nil {
            return err
        }
        
        // Log yield event
        yield := &VaultYield{
            VaultID:        vault.ID,
            VaultType:      "stash",
            AmountMicroRT:  shareMicroRT,
            YieldType:      "demurrage_pool",
            EarnedAt:       now,
        }
        if err := m.repo.LogYield(ctx, yield); err != nil {
            return err
        }
        
        distributedTotal += shareMicroRT
        
        // Update metrics
        m.metrics.IncrementYieldDistributed("demurrage_pool", shareMicroRT)
    }
    
    // Sanity check: distributed should equal total demurrage (within rounding)
    if distributedTotal != totalDemurrageMicroRT {
        // Log warning if difference > 1 micro-RT per vault (rounding tolerance)
        tolerance := int64(len(vaults))
        if abs(distributedTotal - totalDemurrageMicroRT) > tolerance {
            m.logger.Warn("Demurrage distribution mismatch",
                zap.Int64("expected", totalDemurrageMicroRT),
                zap.Int64("distributed", distributedTotal),
                zap.Int64("difference", distributedTotal - totalDemurrageMicroRT))
        }
    }
    
    return nil
}
```

**Tests** (10 tests):
- ✅ Distribute to single StashVault (gets 100%)
- ✅ Distribute to multiple StashVaults (pro-rata)
- ✅ Correct share calculation (various balances)
- ✅ Handle zero demurrage (no distribution, normal in high-vault-ratio economy)
- ✅ Handle zero StashVault balance (error)
- ✅ Rounding tolerance (micro-RT precision)
- ✅ Log yield events correctly
- ✅ Database transaction atomicity
- ✅ Metrics updated correctly
- ✅ Distribution sum equals total demurrage

**Where Demurrage Comes From**:
Phones calculate demurrage continuously (tiered based on balance/daily_disto ratio), then publish `demurrage.paid` events daily. Vault Service aggregates these and distributes to StashVaults.

---

## 🔄 **Deposit & Withdrawal Operations**

### **Deposit**

**Sources**:
1. **DistoDam Auto-Diversion**: User configures "X% of UBD → StashVault" (happens BEFORE wallet)
2. **Manual Phone Transfer**: User moves RT from phone wallet to StashVault
3. **Lump Sum**: One-time deposit from any source

**Implementation**:
```go
func (m *StashVaultManager) Deposit(ctx context.Context, vaultID string, amountMicroRT int64, source string) error {
    // Validate amount
    if amountMicroRT <= 0 {
        return fmt.Errorf("deposit amount must be positive: %d", amountMicroRT)
    }
    
    // Get vault
    vault, err := m.repo.GetStashVault(ctx, vaultID)
    if err != nil {
        return err
    }
    
    // Update balance atomically
    vault.BalanceMicroRT += amountMicroRT
    vault.TotalDepositedMicroRT += amountMicroRT
    vault.UpdatedAt = time.Now()
    
    if err := m.repo.UpdateStashVault(ctx, vault); err != nil {
        return err
    }
    
    // Log transaction
    tx := &VaultTransaction{
        VaultID:        vaultID,
        VaultType:      "stash",
        Type:           "deposit",
        AmountMicroRT:  amountMicroRT,
        BalanceAfter:   vault.BalanceMicroRT,
        Source:         source,  // "ubd_auto", "wallet_transfer", "manual"
        CreatedAt:      time.Now(),
    }
    if err := m.repo.LogTransaction(ctx, tx); err != nil {
        return err
    }
    
    // Update metrics
    m.metrics.IncrementDeposits("stash")
    m.metrics.AddDepositAmount("stash", amountMicroRT)
    
    // Publish confirmation
    m.nats.PublishJSON("vault.deposited", VaultDeposited{
        VaultID:    vaultID,
        VaultType:  "stash",
        WalletID:   vault.WalletID,
        Amount:     microRTToRT(amountMicroRT),
        NewBalance: microRTToRT(vault.BalanceMicroRT),
        DepositedAt: time.Now(),
    })
    
    return nil
}
```

**Tests** (7 tests):
- ✅ Deposit positive amount
- ✅ Reject negative/zero amount
- ✅ Update balance correctly
- ✅ Update total deposited correctly
- ✅ Log transaction
- ✅ Publish confirmation event
- ✅ Metrics updated

---

### **Withdrawal**

**Rules**:
- ✅ Instant (no lock period)
- ✅ No penalty
- ✅ Cannot withdraw more than balance
- ✅ Minimum withdrawal: 0.01 RT (configurable)

**Implementation**:
```go
func (m *StashVaultManager) Withdraw(ctx context.Context, vaultID string, amountMicroRT int64) error {
    // Validate amount
    if amountMicroRT <= 0 {
        return fmt.Errorf("withdrawal amount must be positive: %d", amountMicroRT)
    }
    
    // Get vault
    vault, err := m.repo.GetStashVault(ctx, vaultID)
    if err != nil {
        return err
    }
    
    // Check sufficient balance
    if vault.BalanceMicroRT < amountMicroRT {
        return fmt.Errorf("insufficient balance: have %d, need %d", 
            vault.BalanceMicroRT, amountMicroRT)
    }
    
    // Update balance atomically
    vault.BalanceMicroRT -= amountMicroRT
    vault.TotalWithdrawnMicroRT += amountMicroRT
    vault.UpdatedAt = time.Now()
    
    if err := m.repo.UpdateStashVault(ctx, vault); err != nil {
        return err
    }
    
    // Log transaction
    tx := &VaultTransaction{
        VaultID:        vaultID,
        VaultType:      "stash",
        Type:           "withdrawal",
        AmountMicroRT:  -amountMicroRT,  // Negative for withdrawals
        BalanceAfter:   vault.BalanceMicroRT,
        CreatedAt:      time.Now(),
    }
    if err := m.repo.LogTransaction(ctx, tx); err != nil {
        return err
    }
    
    // Update metrics
    m.metrics.IncrementWithdrawals("stash")
    m.metrics.AddWithdrawalAmount("stash", amountMicroRT)
    
    // Publish confirmation
    m.nats.PublishJSON("vault.withdrawn", VaultWithdrawn{
        VaultID:     vaultID,
        VaultType:   "stash",
        WalletID:    vault.WalletID,
        Amount:      microRTToRT(amountMicroRT),
        NewBalance:  microRTToRT(vault.BalanceMicroRT),
        WithdrawnAt: time.Now(),
    })
    
    return nil
}
```

**Tests** (6 tests):
- ✅ Withdraw valid amount
- ✅ Reject negative/zero amount
- ✅ Reject insufficient balance
- ✅ Update balance correctly
- ✅ Log transaction
- ✅ Publish confirmation event

---

## 📡 **NATS Integration**

### **Subscribe: `ubd.vault_deposit`** (from DistoDam)

**Handler**:
```go
func (m *StashVaultManager) HandleUBDVaultDeposit(msg *nats.Msg) {
    var event UBDVaultDeposit
    if err := json.Unmarshal(msg.Data, &event); err != nil {
        m.logger.Error("Failed to parse UBD vault deposit", zap.Error(err))
        msg.Nak()
        return
    }
    
    // Validate vault type
    if event.VaultType != "stash" {
        // Not for StashVault, let TorqedPledgeManager handle it
        msg.Ack()
        return
    }
    
    // Check for existing vault or create new
    vaultID := event.VaultID
    if vaultID == "" || vaultID == "new" {
        // Create new StashVault for this wallet
        vault, err := m.CreateVault(context.Background(), event.WalletID)
        if err != nil {
            m.logger.Error("Failed to create StashVault", zap.Error(err))
            msg.Nak()
            return
        }
        vaultID = vault.ID
    }
    
    // Deposit to vault
    amountMicroRT := rtToMicroRT(event.Amount)
    if err := m.Deposit(context.Background(), vaultID, amountMicroRT, "ubd_auto"); err != nil {
        m.logger.Error("Failed to deposit to StashVault", 
            zap.String("vault_id", vaultID),
            zap.Error(err))
        msg.Nak()
        return
    }
    
    m.logger.Info("UBD auto-deposited to StashVault",
        zap.String("vault_id", vaultID),
        zap.String("wallet_id", event.WalletID),
        zap.Float64("amount", event.Amount))
    
    msg.Ack()
}
```

**Tests** (5 tests):
- ✅ Create new StashVault on first UBD
- ✅ Deposit to existing StashVault
- ✅ Handle invalid JSON
- ✅ Handle deposit errors
- ✅ Idempotency (duplicate events)

---

### **Subscribe: `phone.vault_transfer`** (from Phone App)

**Handler**:
```go
func (m *StashVaultManager) HandlePhoneVaultTransfer(msg *nats.Msg) {
    var event PhoneVaultTransfer
    if err := json.Unmarshal(msg.Data, &event); err != nil {
        m.logger.Error("Failed to parse phone vault transfer", zap.Error(err))
        msg.Nak()
        return
    }
    
    // Validate vault type
    if event.VaultType != "stash" {
        msg.Ack()
        return
    }
    
    amountMicroRT := rtToMicroRT(event.Amount)
    
    switch event.Direction {
    case "deposit":
        if err := m.Deposit(context.Background(), event.VaultID, amountMicroRT, "phone_transfer"); err != nil {
            m.logger.Error("Failed to deposit from phone", zap.Error(err))
            msg.Nak()
            return
        }
        
    case "withdraw":
        if err := m.Withdraw(context.Background(), event.VaultID, amountMicroRT); err != nil {
            m.logger.Error("Failed to withdraw to phone", zap.Error(err))
            msg.Nak()
            return
        }
        
    default:
        m.logger.Error("Invalid transfer direction", zap.String("direction", event.Direction))
        msg.Ack()  // Don't retry invalid messages
        return
    }
    
    msg.Ack()
}
```

**Tests** (4 tests):
- ✅ Handle deposit from phone
- ✅ Handle withdrawal to phone
- ✅ Reject invalid direction
- ✅ Handle operation errors

---

### **Subscribe: `demurrage.paid`** (from Phones - Daily Settlement)

**Handler**:
```go
func (m *StashVaultManager) HandleDemurragePaid(msg *nats.Msg) {
    var event DemurragePaid
    if err := json.Unmarshal(msg.Data, &event); err != nil {
        m.logger.Error("Failed to parse demurrage paid", zap.Error(err))
        msg.Nak()
        return
    }
    
    // Accumulate for daily distribution
    m.mu.Lock()
    m.dailyDemurrageTotal += event.AmountMicroRT
    m.mu.Unlock()
    
    m.logger.Info("Demurrage received from phone",
        zap.String("wallet_id", event.WalletID),
        zap.Int64("amount", event.AmountMicroRT))
    
    msg.Ack()
}

// Cron job runs at end of day (midnight)
func (m *StashVaultManager) DistributeDailyDemurrage(ctx context.Context) error {
    m.mu.Lock()
    totalDemurrage := m.dailyDemurrageTotal
    m.dailyDemurrageTotal = 0  // Reset for next day
    m.mu.Unlock()
    
    if totalDemurrage == 0 {
        m.logger.Info("No demurrage to distribute today")
        return nil
    }
    
    return m.DistributeDemurragePool(ctx, totalDemurrage)
}
```

**Tests** (5 tests):
- ✅ Accumulate demurrage from multiple phones
- ✅ Distribute at end of day
- ✅ Reset daily total after distribution
- ✅ Handle zero demurrage days
- ✅ Thread-safe accumulation

---

## 🌐 **HTTP API Endpoints**

### **1. Create StashVault**

```http
POST /api/v1/vault/stash
Content-Type: application/json

{
  "wallet_id": "wallet-abc123"
}

Response 201:
{
  "id": "stash-xyz789",
  "wallet_id": "wallet-abc123",
  "balance": 0.0,
  "total_deposited": 0.0,
  "total_withdrawn": 0.0,
  "last_yield_at": "2025-11-14T12:00:00Z",
  "created_at": "2025-11-14T12:00:00Z"
}
```

---

### **2. Get StashVault**

```http
GET /api/v1/vault/stash/:id

Response 200:
{
  "id": "stash-xyz789",
  "wallet_id": "wallet-abc123",
  "balance": 150.5,
  "total_deposited": 200.0,
  "total_withdrawn": 49.5,
  "last_yield_at": "2025-11-14T12:00:00Z",
  "created_at": "2025-11-14T10:00:00Z",
  "updated_at": "2025-11-14T12:30:00Z"
}
```

---

### **3. Deposit to StashVault**

```http
POST /api/v1/vault/stash/:id/deposit
Content-Type: application/json

{
  "amount": 50.0,
  "source": "manual"
}

Response 200:
{
  "vault_id": "stash-xyz789",
  "amount": 50.0,
  "new_balance": 200.5,
  "deposited_at": "2025-11-14T12:35:00Z"
}
```

---

### **4. Withdraw from StashVault**

```http
POST /api/v1/vault/stash/:id/withdraw
Content-Type: application/json

{
  "amount": 25.0
}

Response 200:
{
  "vault_id": "stash-xyz789",
  "amount": 25.0,
  "new_balance": 175.5,
  "withdrawn_at": "2025-11-14T12:40:00Z"
}
```

---

### **5. Get Yield History**

```http
GET /api/v1/vault/stash/:id/yields?limit=10&offset=0

Response 200:
{
  "vault_id": "stash-xyz789",
  "yields": [
    {
      "id": "yield-001",
      "amount": 0.75,
      "yield_type": "base_yield",
      "earned_at": "2025-11-14T00:00:00Z"
    },
    {
      "id": "yield-002",
      "amount": 2.5,
      "yield_type": "demurrage_pool",
      "earned_at": "2025-11-14T00:00:00Z"
    }
  ],
  "total": 2,
  "limit": 10,
  "offset": 0
}
```

---

## 📊 **Example Scenarios**

### **Scenario 1: New User Auto-Saves UBD**

**Setup**:
- User receives 100 RT/month UBD
- Configures DistoDam: "20% → StashVault, 80% → Wallet"

**Month 1**:
```
DistoDam publishes:
- ubd.vault_deposit: 20 RT → StashVault (creates new)
- ubd.funded: 80 RT → Wallet

StashVault balance: 20 RT
Wallet balance: 80 RT
```

**Month 2**:
```
DistoDam publishes:
- ubd.vault_deposit: 20 RT → StashVault
- ubd.funded: 80 RT → Phone

Phone pays demurrage: 80 RT wallet balance at 1%/month tier = 0.8 RT
Network demurrage collected (all users): ~50 RT/day × 30 = 1,500 RT/month
Your StashVault share (40 RT of 10,000 RT total): 0.4% = 6 RT

StashVault balance: 20 + 20 + 6 = 46 RT
Phone balance: 80 + 80 - 0.8 = 159.2 RT
```

**Month 6**:
```
StashVault balance: ~145 RT (6 months × 20 RT + demurrage yields)
Effective yield: ~4.5% (varies with network demurrage)
Phone balance: ~470 RT (spending some, paying demurrage)
```

---

### **Scenario 2: User Manually Deposits Emergency Fund**

**Setup**:
- User has 500 RT in wallet
- Wants to save 200 RT for emergencies

**Action**:
```http
POST /api/v1/vault/stash
Body: {"wallet_id": "wallet-123"}

Response: {"id": "stash-456", ...}

POST /api/v1/vault/stash/stash-456/deposit
Body: {"amount": 200.0, "source": "manual"}

Response: {"new_balance": 200.0}
```

**Result**:
- StashVault: 200 RT (earning demurrage share from network)
- Phone Wallet: 300 RT (exposed to demurrage if held idle)

---

### **Scenario 3: Demurrage Pool Distribution**

**Network State**:
- Total StashVault balance: 100,000 RT
- Your StashVault: 1,000 RT (1% of total)
- Monthly demurrage collected: 500 RT

**Your Share**:
```
Share = (1,000 / 100,000) × 500 = 5 RT
Effective yield = 5 / 1,000 = 0.5% this month
APY ≈ 6% (varies with network conditions)
```

**Key Insights**: 
- The more idle RT in phone wallets, the higher your demurrage returns!
- If everyone vaults: yield drops BUT credit becomes super cheap (low demurrage)
- Self-balancing: high yield attracts vaulting → lowers yield → encourages spending → raises yield

---

## 🧪 **Testing Strategy**

### **Unit Tests** (25 tests):

**StashVaultManager** (15 tests):
- ✅ Create vault (valid/invalid wallet ID)
- ✅ Get vault (exists/not found)
- ✅ Deposit (valid/invalid amounts)
- ✅ Withdraw (valid/insufficient balance)
- ✅ Get balance
- ✅ List vaults by wallet
- ✅ Get total balance (all vaults)

**Yield Distribution** (10 tests):
- ✅ Distribute demurrage pool (pro-rata)
- ✅ Handle single StashVault (gets 100%)
- ✅ Handle multiple StashVaults
- ✅ Handle zero balances
- ✅ Handle zero demurrage (normal in high-vault economy)
- ✅ Rounding tolerance
- ✅ Yield logging
- ✅ Metrics updates
- ✅ Daily accumulation from phones
- ✅ End-of-day distribution

---

### **Integration Tests** (8 tests):

**Test 1: UBD Auto-Save Flow**
```go
func TestStashVault_UBDAutoSave(t *testing.T) {
    // Setup
    vault := setupVaultService(t)
    natsClient := setupNATS(t)
    
    // Publish UBD vault deposit
    natsClient.PublishJSON("ubd.vault_deposit", UBDVaultDeposit{
        WalletID:  "wallet-1",
        VaultType: "stash",
        VaultID:   "new",
        Amount:    20.0,
    })
    
    time.Sleep(100 * time.Millisecond)
    
    // Verify StashVault created and funded
    vaults, _ := vault.GetStashVaultsByWallet(ctx, "wallet-1")
    assert.Len(t, vaults, 1)
    assert.Equal(t, 20.0, microRTToRT(vaults[0].BalanceMicroRT))
}
```

**Test 2: Manual Deposit from Wallet**
**Test 3: Instant Withdrawal**
**Test 4: Base Yield Distribution**
**Test 5: Demurrage Pool Distribution**
**Test 6: Concurrent Deposits (race condition)**
**Test 7: Transaction Logging**
**Test 8: NATS Message Idempotency**

---

## 🔐 **Event Sourcing Architecture**

**⚠️ CRITICAL SECURITY FIX (Nov 18, 2025)**:
- **Database is cache only** (not source of truth)
- **NATS JetStream is source of truth** (replayable event log)
- **Prevents data loss**: Node restart → replay events from NATS → rebuild vault balances

**Event Schema**:
```go
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
```

**Event Replay**:
```go
func (m *StashVaultManager) ReplayEvents(ctx context.Context) error {
    m.logger.Info("replaying StashVault events from NATS")
    
    js, _ := m.natsClient.JetStream()
    
    sub, err := js.Subscribe("vault.stash.*", func(msg *nats.Msg) {
        var event StashVaultEvent
        json.Unmarshal(msg.Data, &event)
        
        // Apply event to in-memory state
        m.applyEvent(event)
        msg.Ack()
    }, nats.DeliverAll())  // Replay from beginning
    
    if err != nil {
        return err
    }
    
    time.Sleep(10 * time.Second)  // Wait for replay
    sub.Unsubscribe()
    
    m.logger.Info("StashVault event replay complete")
    return nil
}
```

---

## 📝 **Implementation Checklist**

### **Phase 1: Core StashVault** (Day 1)
- [ ] **StashVaultManager interface**
  - [ ] CreateVault
  - [ ] GetVault, GetVaultsByWallet
  - [ ] Deposit, Withdraw
  - [ ] GetBalance, GetTotalBalance
  
- [ ] **Database operations**
  - [ ] Insert StashVault
  - [ ] Update StashVault (balance, totals)
  - [ ] Query by ID, by wallet
  - [ ] Query all (for yield distribution)
  
- [ ] **Unit tests** (15 tests)

### **Phase 2: Yield Distribution** (Day 1-2)
- [ ] **Demurrage accumulation**
  - [ ] Subscribe to `demurrage.paid` from phones
  - [ ] Thread-safe daily accumulation
  - [ ] Cron job for end-of-day distribution
  
- [ ] **Demurrage pool distribution**
  - [ ] Calculate pro-rata shares
  - [ ] Credit all StashVaults
  - [ ] Verify distribution sum
  - [ ] Log yield events
  - [ ] Reset daily total
  
- [ ] **Unit tests** (10 tests)

### **Phase 3: NATS Integration** (Day 2)
- [ ] **Subscribe to `ubd.vault_deposit`**
  - [ ] Parse event
  - [ ] Create vault if needed
  - [ ] Deposit to StashVault
  - [ ] Ack/Nak handling
  
- [ ] **Subscribe to `phone.vault_transfer`**
  - [ ] Parse event
  - [ ] Handle deposit/withdraw
  - [ ] Publish confirmations
  
- [ ] **Subscribe to `demurrage.paid`**
  - [ ] Accumulate daily total
  - [ ] Thread-safe handling
  
- [ ] **Publish `vault.deposited`, `vault.withdrawn`**
  
- [ ] **Integration tests** (10 tests)

### **Phase 4: HTTP API** (Day 2-3)
- [ ] **Endpoints**
  - [ ] POST /vault/stash (create)
  - [ ] GET /vault/stash/:id (get)
  - [ ] POST /vault/stash/:id/deposit
  - [ ] POST /vault/stash/:id/withdraw
  - [ ] GET /vault/stash/:id/yields
  
- [ ] **API tests** (10 tests)

---

## 🎯 **Success Metrics**

- ✅ 95%+ test coverage
- ✅ Deposit/withdraw < 50ms (p99)
- ✅ Yield distribution < 1 second (100K vaults)
- ✅ Zero balance discrepancies
- ✅ 100% demurrage pool distribution (within rounding tolerance)

---

**END OF STASHVAULT IMPLEMENTATION**
