# TorqedVault Implementation - Deferred Consumption Savings

## 📋 **Document Metadata**
- **Component**: TorqedPledge / TorqedVault (within Vault Service)
- **Purpose**: Save for big-ticket purchases via future UBD pledges
- **Version**: v0.1.0 (MVP)
- **Author**: Jonathan Clark (@GrokkingGrok)
- **Date**: November 14, 2025
- **Parent Document**: [VAULT_ARCHITECTURE.md](VAULT_ARCHITECTURE.md)

---

## 🎯 **TorqedPledge Overview**

### **What is a TorqedPledge?**

A **TorqedPledge** (also called **TorqedVault**) is how users **save for big purchases** without debt:

| Traditional Mortgage/Loan | TorqedPledge |
|---------------------------|--------------|
| Borrow money upfront | Save first, buy later |
| Pay interest to bank | **Earn** yield while saving |
| Debt on your balance sheet | **Zero debt**, just deferred consumption |
| Bank can foreclose | **You** control the pledge |
| Credit check required | Reputation-based (improves over time) |

### **Core Concept**:

**"I pledge X% of my future UBD stream to save for a house/car/robot fleet"**

When the pledge reaches its target, it **triggers a BRLA** (Bonded Robotic Labor Agreement) to produce the item.

---

## 🏗️ **TorqedPledge Architecture**

### **Component: TorqedPledgeManager**

**Responsibility**: Manage all TorqedPledge operations (create, fund, mature, trigger BRLA)

**Interface**:
```go
type TorqedPledgeManager interface {
    // Pledge operations
    CreatePledge(ctx context.Context, req *CreatePledgeRequest) (*TorqedPledge, error)
    GetPledge(ctx context.Context, pledgeID string) (*TorqedPledge, error)
    GetPledgesByWallet(ctx context.Context, walletID string) ([]*TorqedPledge, error)
    
    // Funding operations
    Deposit(ctx context.Context, pledgeID string, amountMicroRT int64, source string) error
    GetProgress(ctx context.Context, pledgeID string) (*PledgeProgress, error)
    
    // Maturity & BRLA
    CheckMaturity(ctx context.Context, pledgeID string) (bool, error)
    TriggerBRLA(ctx context.Context, pledgeID string) error
    LockPledge(ctx context.Context, pledgeID string, brlaID string) error
    
    // Reputation
    UpdateReputation(ctx context.Context, pledgeID string, delta int) error
    GetReputation(ctx context.Context, pledgeID string) (int, error)
    
    // Statistics
    GetStats() TorqedPledgeStats
}

type TorqedPledge struct {
    ID                    string    `json:"id" db:"id"`
    WalletID              string    `json:"wallet_id" db:"wallet_id"`
    TargetMicroRT         int64     `json:"target_amount" db:"target_micro_rt"`
    SavedMicroRT          int64     `json:"saved_amount" db:"saved_micro_rt"`
    MonthlyPledgeMicroRT  int64     `json:"monthly_pledge" db:"monthly_pledge_micro_rt"`
    BRLAID                string    `json:"brla_id,omitempty" db:"brla_id"`
    ReputationScore       int       `json:"reputation_score" db:"reputation_score"`
    Status                string    `json:"status" db:"status"`
    LockDurationMonths    int       `json:"lock_duration_months" db:"lock_duration_months"`
    MaturityAt            *time.Time `json:"maturity_at,omitempty" db:"maturity_at"`
    TriggeredAt           *time.Time `json:"triggered_at,omitempty" db:"triggered_at"`
    CompletedAt           *time.Time `json:"completed_at,omitempty" db:"completed_at"`
    CreatedAt             time.Time `json:"created_at" db:"created_at"`
    UpdatedAt             time.Time `json:"updated_at" db:"updated_at"`
}

type CreatePledgeRequest struct {
    WalletID             string  `json:"wallet_id"`
    TargetAmount         float64 `json:"target_amount"`      // RT
    MonthlyPledgeAmount  float64 `json:"monthly_pledge"`     // RT per month
    Purpose              string  `json:"purpose"`            // "house", "car", "robot_fleet"
}

type PledgeProgress struct {
    PledgeID      string    `json:"pledge_id"`
    TargetAmount  float64   `json:"target_amount"`
    SavedAmount   float64   `json:"saved_amount"`
    ProgressPct   float64   `json:"progress_pct"`    // 0-100
    MonthsElapsed int       `json:"months_elapsed"`
    EstimatedMonthsRemaining int `json:"estimated_months_remaining"`
    Status        string    `json:"status"`
}

type TorqedPledgeStats struct {
    TotalPledges         int64
    ActivePledges        int64
    MaturedPledges       int64
    TriggeredPledges     int64
    TotalTargetMicroRT   int64
    TotalSavedMicroRT    int64
    AverageProgressPct   float64
}
```

---

## 💰 **Pledge Mechanics**

### **1. Creating a Pledge**

**User Intent**: "I want to save 30,000 RT for a house over the next 2 years"

**Calculation**:
```
Target: 30,000 RT
Timeline: 24 months
Monthly Pledge: 30,000 / 24 = 1,250 RT/month

Assuming UBD: 5,000 RT/month
Required %: (1,250 / 5,000) × 100 = 25% of UBD
```

**Implementation**:
```go
func (m *TorqedPledgeManager) CreatePledge(ctx context.Context, req *CreatePledgeRequest) (*TorqedPledge, error) {
    // Validate
    if req.TargetAmount <= 0 {
        return nil, fmt.Errorf("target amount must be positive")
    }
    if req.MonthlyPledgeAmount <= 0 {
        return nil, fmt.Errorf("monthly pledge must be positive")
    }
    if req.MonthlyPledgeAmount > req.TargetAmount {
        return nil, fmt.Errorf("monthly pledge cannot exceed target")
    }
    
    // Calculate lock duration
    estimatedMonths := int(math.Ceil(req.TargetAmount / req.MonthlyPledgeAmount))
    
    // Create pledge
    pledge := &TorqedPledge{
        ID:                   generatePledgeID(),
        WalletID:             req.WalletID,
        TargetMicroRT:        rtToMicroRT(req.TargetAmount),
        SavedMicroRT:         0,
        MonthlyPledgeMicroRT: rtToMicroRT(req.MonthlyPledgeAmount),
        ReputationScore:      50,  // Default: neutral
        Status:               "active",
        LockDurationMonths:   estimatedMonths,
        CreatedAt:            time.Now(),
        UpdatedAt:            time.Now(),
    }
    
    // Save to database
    if err := m.repo.CreateTorqedPledge(ctx, pledge); err != nil {
        return nil, err
    }
    
    // Update metrics
    m.metrics.IncrementPledgesCreated()
    m.metrics.AddTargetAmount(pledge.TargetMicroRT)
    
    m.logger.Info("TorqedPledge created",
        zap.String("pledge_id", pledge.ID),
        zap.String("wallet_id", pledge.WalletID),
        zap.Float64("target", req.TargetAmount),
        zap.Int("estimated_months", estimatedMonths))
    
    return pledge, nil
}
```

**Tests** (8 tests):
- ✅ Create valid pledge
- ✅ Reject negative/zero target
- ✅ Reject negative/zero monthly pledge
- ✅ Reject monthly pledge > target
- ✅ Calculate lock duration correctly
- ✅ Default reputation = 50
- ✅ Database insertion
- ✅ Metrics updated

---

### **2. Funding a Pledge (Auto-Diversion from UBD)**

**Flow**:
1. User configures DistoDam: "25% of my UBD → TorqedPledge#123"
2. DistoDam publishes `ubd.vault_deposit` → Vault Service
3. TorqedPledgeManager receives event, deposits to pledge
4. Progress checked: saved >= target?
5. If matured → publish `vault.pledge_ready` → Trust

**Implementation**:
```go
func (m *TorqedPledgeManager) Deposit(ctx context.Context, pledgeID string, amountMicroRT int64, source string) error {
    // Validate
    if amountMicroRT <= 0 {
        return fmt.Errorf("deposit amount must be positive")
    }
    
    // Get pledge
    pledge, err := m.repo.GetTorqedPledge(ctx, pledgeID)
    if err != nil {
        return err
    }
    
    // Check status (can only deposit to active pledges)
    if pledge.Status != "active" {
        return fmt.Errorf("cannot deposit to pledge with status: %s", pledge.Status)
    }
    
    // Update balance
    pledge.SavedMicroRT += amountMicroRT
    pledge.UpdatedAt = time.Now()
    
    // Check if reached target
    if pledge.SavedMicroRT >= pledge.TargetMicroRT {
        pledge.Status = "matured"
        now := time.Now()
        pledge.MaturityAt = &now
        
        m.logger.Info("TorqedPledge matured!",
            zap.String("pledge_id", pledgeID),
            zap.Int64("target", pledge.TargetMicroRT),
            zap.Int64("saved", pledge.SavedMicroRT))
        
        // Publish maturity event to Trust
        m.nats.PublishJSON("vault.pledge_ready", PledgeReady{
            PledgeID:     pledgeID,
            WalletID:     pledge.WalletID,
            TargetAmount: microRTToRT(pledge.TargetMicroRT),
            SavedAmount:  microRTToRT(pledge.SavedMicroRT),
            MaturedAt:    now,
        })
        
        // Update metrics
        m.metrics.IncrementPledgesMatured()
    }
    
    // Save to database
    if err := m.repo.UpdateTorqedPledge(ctx, pledge); err != nil {
        return err
    }
    
    // Log transaction
    tx := &VaultTransaction{
        VaultID:        pledgeID,
        VaultType:      "torqed",
        Type:           "deposit",
        AmountMicroRT:  amountMicroRT,
        BalanceAfter:   pledge.SavedMicroRT,
        Source:         source,
        CreatedAt:      time.Now(),
    }
    if err := m.repo.LogTransaction(ctx, tx); err != nil {
        return err
    }
    
    // Update metrics
    m.metrics.AddPledgeSaved(amountMicroRT)
    m.metrics.UpdateAverageProgress(m.calculateAverageProgress(ctx))
    
    // Publish confirmation
    m.nats.PublishJSON("vault.deposited", VaultDeposited{
        VaultID:     pledgeID,
        VaultType:   "torqed",
        WalletID:    pledge.WalletID,
        Amount:      microRTToRT(amountMicroRT),
        NewBalance:  microRTToRT(pledge.SavedMicroRT),
        DepositedAt: time.Now(),
    })
    
    return nil
}
```

**Tests** (10 tests):
- ✅ Deposit to active pledge
- ✅ Reject deposit to matured pledge
- ✅ Reject deposit to triggered pledge
- ✅ Update saved amount correctly
- ✅ Detect maturity (saved >= target)
- ✅ Publish `vault.pledge_ready` on maturity
- ✅ Update status to "matured"
- ✅ Log transaction
- ✅ Publish confirmation
- ✅ Metrics updated

---

### **3. Checking Maturity & Progress**

**Progress Calculation**:
```go
func (m *TorqedPledgeManager) GetProgress(ctx context.Context, pledgeID string) (*PledgeProgress, error) {
    pledge, err := m.repo.GetTorqedPledge(ctx, pledgeID)
    if err != nil {
        return nil, err
    }
    
    progressPct := (float64(pledge.SavedMicroRT) / float64(pledge.TargetMicroRT)) * 100.0
    
    // Calculate months elapsed
    monthsElapsed := int(time.Since(pledge.CreatedAt).Hours() / (24 * 30))
    
    // Estimate months remaining
    monthsRemaining := 0
    if pledge.SavedMicroRT < pledge.TargetMicroRT {
        remaining := pledge.TargetMicroRT - pledge.SavedMicroRT
        monthsRemaining = int(math.Ceil(float64(remaining) / float64(pledge.MonthlyPledgeMicroRT)))
    }
    
    return &PledgeProgress{
        PledgeID:                  pledgeID,
        TargetAmount:              microRTToRT(pledge.TargetMicroRT),
        SavedAmount:               microRTToRT(pledge.SavedMicroRT),
        ProgressPct:               progressPct,
        MonthsElapsed:             monthsElapsed,
        EstimatedMonthsRemaining:  monthsRemaining,
        Status:                    pledge.Status,
    }, nil
}
```

**Tests** (5 tests):
- ✅ Calculate progress correctly (0%, 50%, 100%)
- ✅ Calculate months elapsed
- ✅ Estimate months remaining
- ✅ Handle completed pledges (0 months remaining)
- ✅ Handle database errors

---

### **4. BRLA Trigger & Pledge Lock**

**When pledge matures** → Trust Service creates BRLA → publishes `brla.triggered`

**Vault Service response**:
```go
func (m *TorqedPledgeManager) LockPledge(ctx context.Context, pledgeID string, brlaID string) error {
    pledge, err := m.repo.GetTorqedPledge(ctx, pledgeID)
    if err != nil {
        return err
    }
    
    // Validate status
    if pledge.Status != "matured" {
        return fmt.Errorf("can only lock matured pledges, got status: %s", pledge.Status)
    }
    
    // Lock pledge
    pledge.Status = "triggered"
    pledge.BRLAID = brlaID
    now := time.Now()
    pledge.TriggeredAt = &now
    pledge.UpdatedAt = now
    
    if err := m.repo.UpdateTorqedPledge(ctx, pledge); err != nil {
        return err
    }
    
    m.logger.Info("TorqedPledge locked to BRLA",
        zap.String("pledge_id", pledgeID),
        zap.String("brla_id", brlaID))
    
    // Update metrics
    m.metrics.IncrementPledgesTriggered()
    
    return nil
}
```

**NATS Handler**:
```go
func (m *TorqedPledgeManager) HandleBRLATriggered(msg *nats.Msg) {
    var event BRLATriggered
    if err := json.Unmarshal(msg.Data, &event); err != nil {
        m.logger.Error("Failed to parse BRLA triggered", zap.Error(err))
        msg.Nak()
        return
    }
    
    if err := m.LockPledge(context.Background(), event.PledgeID, event.BRLAID); err != nil {
        m.logger.Error("Failed to lock pledge", zap.Error(err))
        msg.Nak()
        return
    }
    
    msg.Ack()
}
```

**Tests** (6 tests):
- ✅ Lock matured pledge
- ✅ Reject locking active pledge
- ✅ Reject locking already triggered pledge
- ✅ Update status to "triggered"
- ✅ Set BRLA ID
- ✅ Update TriggeredAt timestamp

---

### **5. Reputation System**

**Reputation Score**: 0-100 (default 50 = neutral)

**Impact**:
- Higher reputation → better collateral credit for future pledges
- Lower reputation → higher insurance premiums (BufferPool)

**Events that affect reputation**:

| Event | Reputation Δ | Consequence |
|-------|--------------|-------------|
| Pledge met on time | +2 | Future pledges easier to fulfill |
| Pledge met early | +3 | Shows financial discipline |
| Pledge missed (insured) | -1 | Minor penalty, insurance covers |
| Pledge missed (uninsured) | -5 | Major penalty, pledge frozen 90 days |

**Implementation**:
```go
func (m *TorqedPledgeManager) UpdateReputation(ctx context.Context, pledgeID string, delta int) error {
    pledge, err := m.repo.GetTorqedPledge(ctx, pledgeID)
    if err != nil {
        return err
    }
    
    // Update score (clamp to 0-100)
    newScore := pledge.ReputationScore + delta
    if newScore < 0 {
        newScore = 0
    }
    if newScore > 100 {
        newScore = 100
    }
    
    pledge.ReputationScore = newScore
    pledge.UpdatedAt = time.Now()
    
    if err := m.repo.UpdateTorqedPledge(ctx, pledge); err != nil {
        return err
    }
    
    m.logger.Info("Reputation updated",
        zap.String("pledge_id", pledgeID),
        zap.Int("old_score", pledge.ReputationScore - delta),
        zap.Int("new_score", newScore),
        zap.Int("delta", delta))
    
    return nil
}
```

**Tests** (7 tests):
- ✅ Increase reputation (positive delta)
- ✅ Decrease reputation (negative delta)
- ✅ Clamp to 0 (minimum)
- ✅ Clamp to 100 (maximum)
- ✅ Update database
- ✅ Log reputation changes
- ✅ Handle database errors

---

## 💸 **Yield Calculation**

**TorqedPledge Yield**: 1.2% - 2.0% per month (based on lock duration)

**Formula**:
```
Lock_Duration_Months = Target / Monthly_Pledge
Yield_Rate_BPS = 120 + (80 × min(Lock_Duration_Months / 36, 1))

Example 1: 12-month pledge
Yield_Rate = 120 + (80 × 12/36) = 120 + 27 = 147 BPS = 1.47%/month

Example 2: 36-month pledge
Yield_Rate = 120 + (80 × 36/36) = 120 + 80 = 200 BPS = 2.0%/month
```

**Implementation**:
```go
func (m *TorqedPledgeManager) DistributeYield(ctx context.Context) error {
    // Get all active TorqedPledges
    pledges, err := m.repo.ListActivePledges(ctx)
    if err != nil {
        return err
    }
    
    now := time.Now()
    
    for _, pledge := range pledges {
        // Calculate yield rate based on lock duration
        lockMonths := float64(pledge.LockDurationMonths)
        yieldRateBPS := int64(120 + (80 * math.Min(lockMonths/36.0, 1.0)))
        yieldRate := float64(yieldRateBPS) / 10000.0  // BPS to decimal
        
        // Calculate months since last yield
        monthsSinceLastYield := now.Sub(pledge.UpdatedAt).Hours() / (24 * 30)
        
        // Calculate yield
        yieldMicroRT := int64(float64(pledge.SavedMicroRT) * yieldRate * monthsSinceLastYield)
        
        // Credit pledge
        pledge.SavedMicroRT += yieldMicroRT
        pledge.UpdatedAt = now
        
        // Check if yield pushed pledge to maturity
        if pledge.SavedMicroRT >= pledge.TargetMicroRT && pledge.Status == "active" {
            pledge.Status = "matured"
            pledge.MaturityAt = &now
            
            m.nats.PublishJSON("vault.pledge_ready", PledgeReady{
                PledgeID:     pledge.ID,
                WalletID:     pledge.WalletID,
                TargetAmount: microRTToRT(pledge.TargetMicroRT),
                SavedAmount:  microRTToRT(pledge.SavedMicroRT),
                MaturedAt:    now,
            })
        }
        
        // Update database
        if err := m.repo.UpdateTorqedPledge(ctx, pledge); err != nil {
            return err
        }
        
        // Log yield
        yield := &VaultYield{
            VaultID:       pledge.ID,
            VaultType:     "torqed",
            AmountMicroRT: yieldMicroRT,
            YieldType:     "pledge_yield",
            EarnedAt:      now,
        }
        if err := m.repo.LogYield(ctx, yield); err != nil {
            return err
        }
        
        // Update metrics
        m.metrics.IncrementYieldDistributed("pledge_yield", yieldMicroRT)
    }
    
    return nil
}
```

**Tests** (8 tests):
- ✅ Calculate yield rate (12, 24, 36 months)
- ✅ Calculate yield amount correctly
- ✅ Credit pledge balance
- ✅ Detect maturity after yield
- ✅ Publish `vault.pledge_ready` if matured
- ✅ Log yield events
- ✅ Handle empty pledges
- ✅ Metrics updated

---

## 📡 **NATS Integration**

### **Subscribe: `ubd.vault_deposit`** (from DistoDam)

**Handler**:
```go
func (m *TorqedPledgeManager) HandleUBDVaultDeposit(msg *nats.Msg) {
    var event UBDVaultDeposit
    if err := json.Unmarshal(msg.Data, &event); err != nil {
        m.logger.Error("Failed to parse UBD vault deposit", zap.Error(err))
        msg.Nak()
        return
    }
    
    // Only handle torqed vault types
    if event.VaultType != "torqed" {
        msg.Ack()
        return
    }
    
    // Deposit to pledge
    amountMicroRT := rtToMicroRT(event.Amount)
    if err := m.Deposit(context.Background(), event.VaultID, amountMicroRT, "ubd_auto"); err != nil {
        m.logger.Error("Failed to deposit to TorqedPledge",
            zap.String("pledge_id", event.VaultID),
            zap.Error(err))
        msg.Nak()
        return
    }
    
    m.logger.Info("UBD deposited to TorqedPledge",
        zap.String("pledge_id", event.VaultID),
        zap.Float64("amount", event.Amount))
    
    msg.Ack()
}
```

**Tests** (4 tests):
- ✅ Deposit to existing pledge
- ✅ Ignore non-torqed vault types
- ✅ Handle invalid JSON
- ✅ Handle deposit errors

---

### **Publish: `vault.pledge_ready`** (to Trust)

**Published when**:
- Pledge reaches target via UBD deposits
- Pledge reaches target via yield distribution
- Manual deposit pushes pledge to target

**Message**:
```go
type PledgeReady struct {
    PledgeID     string    `json:"pledge_id"`
    WalletID     string    `json:"wallet_id"`
    TargetAmount float64   `json:"target_amount"`
    SavedAmount  float64   `json:"saved_amount"`
    Purpose      string    `json:"purpose"`  // "house", "car", etc.
    MaturedAt    time.Time `json:"matured_at"`
}
```

**Trust Service response**:
1. Receive `vault.pledge_ready`
2. Create BRLA for house/car/etc.
3. Submit to BidNet for approval
4. If approved → publish `brla.triggered` back to Vault
5. Vault locks pledge

---

### **Subscribe: `brla.triggered`** (from Trust)

Already covered in "BRLA Trigger & Pledge Lock" section above.

---

## 🌐 **HTTP API Endpoints**

### **1. Create TorqedPledge**

```http
POST /api/v1/vault/pledge
Content-Type: application/json

{
  "wallet_id": "wallet-abc123",
  "target_amount": 30000.0,
  "monthly_pledge": 1250.0,
  "purpose": "house"
}

Response 201:
{
  "id": "pledge-xyz789",
  "wallet_id": "wallet-abc123",
  "target_amount": 30000.0,
  "saved_amount": 0.0,
  "monthly_pledge": 1250.0,
  "reputation_score": 50,
  "status": "active",
  "lock_duration_months": 24,
  "created_at": "2025-11-14T12:00:00Z"
}
```

---

### **2. Get TorqedPledge**

```http
GET /api/v1/vault/pledge/:id

Response 200:
{
  "id": "pledge-xyz789",
  "wallet_id": "wallet-abc123",
  "target_amount": 30000.0,
  "saved_amount": 15000.0,
  "monthly_pledge": 1250.0,
  "brla_id": null,
  "reputation_score": 52,
  "status": "active",
  "lock_duration_months": 24,
  "maturity_at": null,
  "triggered_at": null,
  "created_at": "2025-11-14T12:00:00Z",
  "updated_at": "2025-11-26T12:00:00Z"
}
```

---

### **3. Get Pledge Progress**

```http
GET /api/v1/vault/pledge/:id/progress

Response 200:
{
  "pledge_id": "pledge-xyz789",
  "target_amount": 30000.0,
  "saved_amount": 15000.0,
  "progress_pct": 50.0,
  "months_elapsed": 12,
  "estimated_months_remaining": 12,
  "status": "active"
}
```

---

### **4. Manual Deposit to Pledge**

```http
POST /api/v1/vault/pledge/:id/deposit
Content-Type: application/json

{
  "amount": 5000.0,
  "source": "manual"
}

Response 200:
{
  "pledge_id": "pledge-xyz789",
  "amount": 5000.0,
  "new_balance": 20000.0,
  "progress_pct": 66.7,
  "deposited_at": "2025-11-26T12:30:00Z"
}
```

---

## 📊 **Example Scenarios**

### **Scenario 1: Saving for a House**

**Setup**:
- Target: 30,000 RT
- Monthly UBD: 5,000 RT
- Pledge: 25% (1,250 RT/month)
- Timeline: 24 months

**Month 1**:
```
UBD received: 5,000 RT
DistoDam diverts: 25% = 1,250 RT → TorqedPledge
Pledge balance: 1,250 RT
Progress: 4.2%
```

**Month 12**:
```
Cumulative UBD deposits: 12 × 1,250 = 15,000 RT
Yield earned (1.47%/mo avg): ~2,200 RT
Pledge balance: 17,200 RT
Progress: 57.3%
```

**Month 24**:
```
Cumulative UBD deposits: 24 × 1,250 = 30,000 RT
Yield earned (1.47%/mo avg): ~5,300 RT
Pledge balance: 35,300 RT (exceeded target!)
Status: "matured"
vault.pledge_ready published → Trust
```

**Trust Response**:
```
Trust receives pledge_ready
Creates house BRLA (30k RT collateral)
BidNet approves
Trust publishes brla.triggered
Vault locks pledge
Robots build house
```

---

### **Scenario 2: Saving for a Car (Accelerated)**

**Setup**:
- Target: 5,000 RT
- Monthly UBD: 5,000 RT
- Pledge: 50% (2,500 RT/month)
- Timeline: 2 months (accelerated)

**Month 1**:
```
UBD received: 5,000 RT
DistoDam diverts: 50% = 2,500 RT → TorqedPledge
Pledge balance: 2,500 RT
Progress: 50%
```

**Month 2**:
```
UBD received: 5,000 RT
DistoDam diverts: 50% = 2,500 RT → TorqedPledge
Pledge balance: 5,000 RT + yield
Status: "matured"
vault.pledge_ready published → Trust
```

**Result**: Car production BRLA triggered after just 2 months!

---

### **Scenario 3: Manual Lump-Sum Deposit**

**Setup**:
- User has 10,000 RT in wallet
- Target: 20,000 RT
- Current pledge balance: 12,000 RT
- Shortfall: 8,000 RT

**Action**:
```http
POST /api/v1/vault/pledge/pledge-123/deposit
Body: {"amount": 8000.0, "source": "manual"}
```

**Result**:
```
Pledge balance: 12,000 + 8,000 = 20,000 RT
Status: "matured"
vault.pledge_ready published → Trust
Reputation: +3 (met early)
```

---

## 🧪 **Testing Strategy**

### **Unit Tests** (30 tests):

**TorqedPledgeManager** (20 tests):
- ✅ Create pledge (valid/invalid)
- ✅ Get pledge (exists/not found)
- ✅ Deposit (active/matured/triggered)
- ✅ Calculate progress
- ✅ Check maturity
- ✅ Lock pledge (valid/invalid status)
- ✅ Update reputation (clamp 0-100)
- ✅ List pledges by wallet

**Yield Distribution** (10 tests):
- ✅ Calculate yield rate (various lock durations)
- ✅ Distribute yield to active pledges
- ✅ Detect maturity after yield
- ✅ Handle empty pledges
- ✅ Log yield events
- ✅ Metrics updates

---

### **Integration Tests** (10 tests):

**Test 1: Full Pledge Lifecycle**
```go
func TestTorqedPledge_FullLifecycle(t *testing.T) {
    // 1. Create pledge
    pledge := createPledge(t, 1000.0, 100.0)  // Target 1000, monthly 100
    assert.Equal(t, "active", pledge.Status)
    
    // 2. Simulate 10 months of UBD deposits
    for i := 0; i < 10; i++ {
        deposit(t, pledge.ID, 100.0)
    }
    
    // 3. Check progress
    progress := getProgress(t, pledge.ID)
    assert.Equal(t, 100.0, progress.ProgressPct)
    assert.Equal(t, "matured", progress.Status)
    
    // 4. Verify vault.pledge_ready published
    assertEventPublished(t, "vault.pledge_ready", pledge.ID)
    
    // 5. Simulate BRLA trigger
    lockPledge(t, pledge.ID, "brla-123")
    assert.Equal(t, "triggered", getPledge(t, pledge.ID).Status)
}
```

**Test 2: UBD Auto-Deposit**
**Test 3: Manual Deposit Maturity**
**Test 4: Yield Distribution Maturity**
**Test 5: Reputation Updates**
**Test 6: NATS Message Idempotency**
**Test 7: Concurrent Deposits (race condition)**
**Test 8: Progress Calculation Edge Cases**
**Test 9: Lock Invalid Status**
**Test 10: Database Transaction Rollback**

---

## 📝 **Implementation Checklist**

### **Phase 1: Core TorqedPledge** (Day 1)
- [ ] **TorqedPledgeManager interface**
  - [ ] CreatePledge
  - [ ] GetPledge, GetPledgesByWallet
  - [ ] Deposit
  - [ ] GetProgress, CheckMaturity
  - [ ] UpdateReputation
  
- [ ] **Database operations**
  - [ ] Insert TorqedPledge
  - [ ] Update TorqedPledge (balance, status, reputation)
  - [ ] Query by ID, by wallet
  - [ ] Query active pledges (for yield distribution)
  
- [ ] **Unit tests** (20 tests)

### **Phase 2: Maturity & BRLA** (Day 2)
- [ ] **Maturity detection**
  - [ ] Check on each deposit
  - [ ] Check after yield distribution
  - [ ] Update status to "matured"
  - [ ] Set MaturityAt timestamp
  
- [ ] **BRLA trigger**
  - [ ] Publish `vault.pledge_ready`
  - [ ] Subscribe to `brla.triggered`
  - [ ] Lock pledge (status = "triggered")
  - [ ] Set BRLA ID
  
- [ ] **Unit tests** (6 tests)

### **Phase 3: Yield Distribution** (Day 2)
- [ ] **Yield calculation**
  - [ ] Calculate yield rate (lock duration based)
  - [ ] Credit pledge balance
  - [ ] Check maturity after yield
  - [ ] Log yield events
  
- [ ] **Unit tests** (8 tests)

### **Phase 4: NATS Integration** (Day 3)
- [ ] **Subscribe to `ubd.vault_deposit`**
  - [ ] Parse event
  - [ ] Deposit to TorqedPledge
  - [ ] Ack/Nak handling
  
- [ ] **Publish `vault.pledge_ready`**
  - [ ] On deposit maturity
  - [ ] On yield maturity
  
- [ ] **Subscribe to `brla.triggered`**
  - [ ] Lock pledge
  
- [ ] **Integration tests** (10 tests)

### **Phase 5: HTTP API** (Day 3)
- [ ] **Endpoints**
  - [ ] POST /vault/pledge (create)
  - [ ] GET /vault/pledge/:id (get)
  - [ ] GET /vault/pledge/:id/progress
  - [ ] POST /vault/pledge/:id/deposit
  
- [ ] **API tests** (8 tests)

---

## 🎯 **Success Metrics**

- ✅ 95%+ test coverage
- ✅ Pledge creation < 50ms (p99)
- ✅ Deposit + maturity check < 100ms (p99)
- ✅ Yield distribution < 2 seconds (100K pledges)
- ✅ 100% maturity detection accuracy
- ✅ Zero missed `vault.pledge_ready` events

---

**END OF TORQEDVAULT IMPLEMENTATION**
