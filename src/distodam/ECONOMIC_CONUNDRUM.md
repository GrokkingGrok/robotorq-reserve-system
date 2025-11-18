# DistoDam Economic Conundrum: The Liquidity Balance Problem

**Date**: November 17, 2025  
**Status**: 🤔 **OPEN QUESTION** - Requires economic analysis before implementation  
**Priority**: 🔴 **CRITICAL** - Affects core monetary policy

---

## 🎯 The Problem

When **StakeVault** (robot labor payment pool) needs to borrow from **DistoVault** (UBD distribution pool), we face a monetary policy decision with profound implications:

### **Option A: Torq Bump (Increase Token Rate)**
Create extra RT to repay the loan by bumping the Torq ratio on the contract.

### **Option B: Natural Rebalancing (No Torq Bump)**
Treat the borrowing as evidence that the two money supplies are already balanced, requiring no additional minting.

---

## 📊 Scenario Analysis

### **Partial Borrowing Example**

```
Initial State:
├─ StakeVault: 2 RT
└─ DistoVault: 10 RT

Contract Request: 5 RT needed for robot labor

Borrowing Action:
├─ StakeVault provides: 2 RT (all it has)
└─ DistoVault lends: 3 RT (to make up shortfall)

Contract Execution:
├─ Total RoboStake: 5 RT
├─ Expected Output: 15 RT (Torq = 3.0, normal efficiency)
└─ Newly Minted: 10 RT (15 - 5 = value-add)

The Question: What happens to the 3 RT loan?
```

---

## 🔀 Option A: Torq Bump (Create Extra RT)

### **Mechanism**
Increase the effective token rate to mint extra RT specifically to repay the loan.

### **Flow**
```
Contract needs 5 RT:
├─ StakeVault: 2 RT (withdraw all available)
├─ DistoVault: 3 RT (LOAN)
└─ Total: 5 RT funding provided

Digger works with 5 RT RoboStake:
├─ Normal Torq: 3.0 (would mint 10 RT)
├─ Bumped Torq: 4.6 (mints 13 RT extra)
└─ Total output: 23 RT

Mint processes:
├─ RoboStake return: 5 RT → StakeVault
├─ Newly minted: 13 RT
│   ├─ Loan repayment: 3 RT → DistoVault
│   └─ UBD distribution: 10 RT → DistoVault
└─ Net result: Both vaults whole + 10 RT to members

Economics:
✅ Loan repaid immediately
✅ No vault imbalance
❌ Extra 3 RT created (inflationary?)
❌ Torq ratio distorted (not purely physics-based)
```

### **Arguments FOR Torq Bump**
1. **Preserves vault integrity** - Both vaults return to pre-loan state
2. **Self-correcting mechanism** - System rebalances automatically
3. **Incentivizes efficiency** - Enterprises must accept higher Torq to get funding
4. **Predictable repayment** - Loans always cleared within contract lifecycle

### **Arguments AGAINST Torq Bump**
1. **Inflation risk** - Creates RT not tied to real value creation
2. **Distorts price signals** - Torq no longer pure efficiency measurement
3. **Perverse incentive** - Could encourage vault depletion to trigger extra minting
4. **Physics dishonesty** - Minting should reflect actual work, not accounting needs

---

## 🌊 Option B: Natural Rebalancing (Accept Imbalance)

### **Mechanism**
Treat borrowing as evidence that consumer liquidity is already balanced with production capacity. No Torq bump needed.

### **Flow**
```
Contract needs 5 RT:
├─ StakeVault: 2 RT (withdraw all available)
├─ DistoVault: 3 RT (LOAN)
└─ Total: 5 RT funding provided

Digger works with 5 RT RoboStake:
├─ Normal Torq: 3.0 (honest efficiency)
└─ Total output: 15 RT (5 input + 10 value-add)

Mint processes:
├─ RoboStake return: 5 RT → StakeVault
├─ Newly minted: 10 RT → DistoVault
└─ Net result:
    ├─ StakeVault: 5 RT (replenished to 2 + returned 3)
    └─ DistoVault: 10 RT (down 3 from loan, up 10 from mint = +7)

Loan Status:
├─ Outstanding: 3 RT (still owed by StakeVault to DistoVault)
└─ Will be repaid as future contracts return RoboStake

Economics:
✅ Pure physics-based minting (Torq = real efficiency)
✅ Natural market signal (StakeVault depletion = high production demand)
✅ No artificial inflation
❌ Vaults remain imbalanced until loan repaid
❌ Loan repayment depends on future contract flow
```

### **Arguments FOR Natural Rebalancing**
1. **Economic signal interpretation** - Borrowing indicates:
   - High production activity (lots of contracts)
   - Consumer liquidity flowing into production (as it should)
   - StakeVault recycling working (RT circulating through economy)
2. **Physics-honest minting** - Torq remains pure efficiency metric
3. **Self-stabilizing** - Future contracts will replenish StakeVault naturally
4. **Prevents gaming** - Can't exploit system to mint extra RT

### **Arguments AGAINST Natural Rebalancing**
1. **Vault imbalance** - DistoVault perpetually smaller (until repaid)
2. **UBD reduction** - Less RT available for distribution to members
3. **Uncertainty** - No guarantee future contracts will repay loan quickly
4. **Cascading crisis** - If StakeVault stays depleted, could spiral

---

## 🧠 Economic Theory Analysis

### **What Does Borrowing Actually Signal?**

#### **Hypothesis 1: Excessive Consumer Liquidity**
If members have "too much" RT sitting in wallets (not circulating), production demand surges, depleting StakeVault. In this case:
- **Borrowing = Natural rebalancing mechanism**
- DistoVault → StakeVault transfer moves RT from consumption to production
- No new minting needed (RT already exists, just redistributed)
- System corrects via natural circulation

#### **Hypothesis 2: Insufficient Robot Payment Capacity**
If StakeVault is too small relative to production demand, borrowing indicates:
- **Borrowing = System undersupply signal**
- Need more RT in circulation to support economy
- Torq bump creates necessary liquidity
- Prevents production bottleneck

#### **Hypothesis 3: Timing Mismatch**
If StakeVault depleted due to lumpy contract execution (many contracts funded, ore not yet minted), borrowing indicates:
- **Borrowing = Temporary liquidity bridge**
- Not a fundamental imbalance, just timing
- Loan will self-repay when ore processes
- No intervention needed (neither Torq bump nor concern)

---

## 🔍 Key Questions to Resolve

### **1. What is StakeVault's Role?**
- **Pure robot payment pool** (should always be full)?
- **OR economic buffer** (allowed to fluctuate with demand)?

### **2. What Does Loan Represent?**
- **Short-term bridge** (timing mismatch, self-correcting)?
- **Structural imbalance** (requires intervention)?
- **Natural transfer** (consumption → production rebalancing)?

### **3. Should Vaults Be Balanced?**
- **Equal size** (50/50 split)?
- **Production-heavy** (80% Stake / 20% Disto)?
- **Consumption-heavy** (20% Stake / 80% Disto)?
- **Dynamic equilibrium** (no target, natural flow)?

### **4. Who Pays for the Loan?**
- **Future members** (via Torq bump = inflation)?
- **Current members** (via reduced UBD until loan repaid)?
- **System absorption** (loan remains outstanding indefinitely)?

---

## 🧪 Proposed Experiments

### **Experiment 1: Measure Loan Frequency**
- Track how often StakeVault borrows from DistoVault
- If rare (< 1% of contracts): **Timing mismatch** → Natural rebalancing
- If frequent (> 10% of contracts): **Structural issue** → Torq bump or other intervention

### **Experiment 2: Measure Repayment Time**
- Track how long loans stay outstanding
- If short (< 1 hour): **Self-correcting** → No action needed
- If long (> 24 hours): **Persistent imbalance** → Intervention required

### **Experiment 3: A/B Test Both Policies**
- Run two parallel DistoDams:
  - Dam A: Torq bump on borrowing
  - Dam B: Natural rebalancing
- Measure:
  - Total RT supply growth
  - Member UBD per capita
  - Contract funding success rate
  - Vault balance stability

---

## 💡 Potential Hybrid Solutions

### **Solution 1: Progressive Torq Bump**
- **Small loans** (< 10% of contract): No Torq bump (natural rebalancing)
- **Medium loans** (10-30%): Partial Torq bump (50% repayment)
- **Large loans** (> 30%): Full Torq bump (100% repayment)

**Rationale**: Small fluctuations are normal; large deviations signal real imbalance.

### **Solution 2: Time-Delayed Torq Bump**
- Borrow from DistoVault initially (no Torq bump)
- If loan outstanding > 24 hours, trigger Torq bump on NEXT contract
- Gives system time to self-correct before intervention

**Rationale**: Don't overreact to temporary imbalances.

### **Solution 3: Member-Funded Loan Repayment**
- Borrow from DistoVault (no Torq bump)
- Members who benefit from UBD distribution vote to repay loan via demurrage tax
- Democratizes the cost of borrowing

**Rationale**: Those who receive UBD should help maintain system stability.

### **Solution 4: Dynamic Reserve Ratio**
- Set target: StakeVault should maintain X% of total vault capacity
- If StakeVault < X%, trigger gradual Torq increase on all new contracts
- If StakeVault > X%, trigger gradual Torq decrease
- Self-balancing without explicit loans

**Rationale**: Continuous adjustment prevents large swings.

---

## 🎯 Decision Framework

### **Criteria for Choosing Policy**

| Criterion | Torq Bump | Natural Rebalancing | Winner |
|-----------|-----------|---------------------|--------|
| **Physics honesty** (Torq = real efficiency) | ❌ Distorted | ✅ Pure | Natural |
| **Vault stability** (both vaults whole) | ✅ Immediate | ❌ Delayed | Torq Bump |
| **Member UBD** (distribution consistency) | ✅ Maintained | ❌ Reduced | Torq Bump |
| **Inflation risk** (excess RT creation) | ❌ Risk | ✅ None | Natural |
| **Gaming resistance** (can't exploit) | ❌ Gameable | ✅ Resistant | Natural |
| **Economic signaling** (market feedback) | ❌ Obscured | ✅ Clear | Natural |
| **Simplicity** (easy to understand) | ❌ Complex rules | ✅ Simple | Natural |

### **✅ DECISION (Jon - Nov 17, 2025)**

**Natural Equilibrium by Default, With Policy Lever**

**Default Behavior**: Natural rebalancing (no Torq bump)
- Allow borrowing from DistoVault
- Track loan metrics comprehensively
- Let loans repay via natural RoboStake circulation

**Emergency Intervention Lever**: Configurable Torq bump policy
- Environment variable: `DISTODAM_LOAN_POLICY` (default: `natural`)
- Possible values:
  - `natural` - No intervention (MVP default)
  - `torq_bump_immediate` - Bump on every loan
  - `torq_bump_threshold` - Bump if loan > X% of contract
  - `torq_bump_delayed` - Bump if outstanding > Y hours
- Operator can flip policy without code changes

**Rationale**:
- ✅ Preserves physics-honest minting (default)
- ✅ Generates real-world data for policy decisions
- ✅ Provides emergency valve if crisis emerges
- ✅ Allows A/B testing different policies
- ✅ No premature optimization, but prepared for intervention

---

## 📋 Implementation Implications

### **For Phase 6 (Natural Equilibrium + Policy Lever)**

```go
type LoanPolicy string

const (
    LoanPolicyNatural            LoanPolicy = "natural"
    LoanPolicyTorqBumpImmediate  LoanPolicy = "torq_bump_immediate"
    LoanPolicyTorqBumpThreshold  LoanPolicy = "torq_bump_threshold"
    LoanPolicyTorqBumpDelayed    LoanPolicy = "torq_bump_delayed"
)

type Config struct {
    LoanPolicy              LoanPolicy
    TorqBumpThresholdPct    float64   // For threshold policy (e.g., 0.30 = 30%)
    TorqBumpDelayHours      float64   // For delayed policy (e.g., 24.0 hours)
    TorqBumpMultiplier      float64   // How much to bump (e.g., 1.0 = 100% extra)
}

type LoanMetrics struct {
    TotalLoansCount       int64
    TotalLoansRT          float64
    OutstandingLoansCount int64
    OutstandingLoansRT    float64
    AverageLoanDuration   time.Duration
    LongestLoanDuration   time.Duration
    LoansPerHour          float64
}

func (vm *VaultManager) FundContract(contractID string, amountRT float64) error {
    // Try StakeVault first
    available := vm.stakeVault.GetBalance()
    
    if available >= amountRT {
        // Sufficient funds - normal path
        return vm.stakeVault.WithdrawForContract(contractID, amountRT)
    }
    
    // Insufficient StakeVault - borrow from DistoVault
    shortfall := amountRT - available
    
    if vm.distoVault.GetBalance() < shortfall {
        return errors.New("insufficient funds in both vaults")
    }
    
    // Create loan
    loan := &Loan{
        LoanID:      uuid.New().String(),
        AmountRT:    shortfall,
        BorrowedAt:  time.Now(),
        ContractID:  contractID,
        Outstanding: shortfall,
    }
    
    // Transfer from DistoVault to StakeVault
    if err := vm.transferDistoToStake(shortfall, loan.LoanID); err != nil {
        return err
    }
    
    vm.recordLoan(loan)
    
    // Policy lever: Decide if we need Torq bump
    if vm.shouldTorqBump(loan, amountRT) {
        vm.requestTorqBump(contractID, shortfall)
    }
    
    // Now StakeVault has enough
    return vm.stakeVault.WithdrawForContract(contractID, amountRT)
}

func (vm *VaultManager) shouldTorqBump(loan *Loan, contractAmount float64) bool {
    switch vm.config.LoanPolicy {
    case LoanPolicyNatural:
        return false  // Default: no intervention
    
    case LoanPolicyTorqBumpImmediate:
        return true  // Always bump
    
    case LoanPolicyTorqBumpThreshold:
        loanPct := loan.AmountRT / contractAmount
        return loanPct > vm.config.TorqBumpThresholdPct
    
    case LoanPolicyTorqBumpDelayed:
        // Don't bump immediately; check duration in RepayOutstandingLoans()
        return false
    
    default:
        return false
    }
}

func (vm *VaultManager) requestTorqBump(contractID string, amountRT float64) {
    // Publish event to Digger/Ore pipeline to increase Torq on this contract
    // Implementation depends on how we want to communicate policy to execution layer
    // Options:
    // 1. Publish to contracts.torq_adjustments topic
    // 2. Store in contract metadata (TorqMultiplier field)
    // 3. Signal to Mint to expect extra RT from this contract
    
    vm.logger.Warn("torq bump requested",
        "contract_id", contractID,
        "loan_amount", amountRT,
        "policy", vm.config.LoanPolicy)
    
    vm.metrics.TorqBumpsRequestedTotal.Inc()
}

// Mint event handler returns RoboStake to StakeVault
// Loans repay naturally as StakeVault accumulates returned stakes
func (vm *VaultManager) RepayOutstandingLoans() error {
    // Called after each MintEvent deposits RoboStake to StakeVault
    // Prioritize loan repayment over new contract funding
    
    loans := vm.GetOutstandingLoans()
    for _, loan := range loans {
        // Check if delayed Torq bump policy triggered
        if vm.config.LoanPolicy == LoanPolicyTorqBumpDelayed {
            loanAge := time.Since(loan.BorrowedAt).Hours()
            if loanAge > vm.config.TorqBumpDelayHours && !loan.TorqBumpRequested {
                vm.requestTorqBump(loan.ContractID, loan.AmountRT)
                loan.TorqBumpRequested = true
            }
        }
        
        available := vm.stakeVault.GetBalance()
        repayAmount := math.Min(loan.Outstanding, available)
        
        if repayAmount < 0.001 {
            break  // No funds available for repayment
        }
        
        // Transfer back to DistoVault
        if err := vm.transferStakeToDisto(repayAmount, loan.LoanID); err != nil {
            return err
        }
        
        loan.Outstanding -= repayAmount
        loan.Repaid += repayAmount
        
        if loan.Outstanding < 0.001 {
            vm.markLoanPaid(loan.LoanID)
        }
    }
    
    return nil
}
```

---

## 🔮 Future Considerations

### **Phase 7: Economic Dashboard**
Add real-time monitoring:
- StakeVault / DistoVault ratio over time
- Loan frequency and duration trends
- Contract funding success rate
- Member UBD per capita trends

### **Phase 8: Governance Layer**

**Member-Voted Parameters** (bounded ranges to prevent gaming):

```go
// Members can vote to adjust circuit breaker sensitivity
// Values are BOUNDED - no crazy extremes allowed

type GovernanceParams struct {
    // How long to wait before Torq bump (delayed policy)
    // Range: 12-96 hours
    // Default: 24 hours
    TorqBumpDelayHours float64 `min:"12.0" max:"96.0" default:"24.0"`
    
    // Loan-to-contract ratio threshold (threshold policy)
    // Range: 0.10-0.50 (10% to 50%)
    // Default: 0.30 (30%)
    TorqBumpThresholdPct float64 `min:"0.10" max:"0.50" default:"0.30"`
}
```

**What Members CANNOT Vote On** (by design):
- ❌ Which policy to use (natural/immediate/threshold/delayed)
  - **Reason**: Too binary, becomes political instead of technical
  - **Set by**: Operator based on observed economic data
- ❌ Whether to allow loans at all
  - **Reason**: Fundamental to system operation
- ❌ Torq bump multiplier (how much to bump)
  - **Reason**: Should be calculated from physics, not voted
- ❌ Disabling the circuit breaker
  - **Reason**: Safety mechanism, not optional

**Governance Rationale** (Grok's insight):
> *"Never let them vote on 'should we bump or not' — that's too binary and political. Let them vote on how sensitive the circuit breaker is. This keeps it technical, not ideological."*

**Voting Mechanism** (Phase 8 implementation):
1. Proposal: Member submits new parameter value (within bounds)
2. Quorum: X% of members must vote (e.g., 30%)
3. Majority: Simple majority wins (>50%)
4. Cool-down: 7-day minimum between parameter changes
5. Effect: New value applied to future contracts only (no retroactive)

**Example Governance Flow**:
```
Member observes: "Loans taking 36 hours to repay, causing UBD delays"
↓
Proposal: Reduce TorqBumpDelayHours from 24h → 18h
↓
Vote: 67% approval (quorum met)
↓
Result: Future delayed policies trigger Torq bump after 18h instead of 24h
↓
Outcome: Loans repaid faster, UBD distribution more consistent
```

**Anti-Capture Design**:
- Bounded ranges prevent extreme values
- Technical parameters, not ideological switches
- Operator still controls which policy is active
- Economic data drives operator decisions, not politics

### **Phase 9: AI-Driven Policy**
Train model to optimize:
- Loan vs Torq bump decisions
- Dynamic reserve ratios
- Member UBD distribution timing

---

## 📝 Open Questions for Jon

1. **Initial hypothesis**: Which economic interpretation do you believe is most accurate?
   - Excessive consumer liquidity (natural rebalancing)? This is default. 
   - Insufficient robot capacity (Torq bump needed)?
   - Timing mismatch (no intervention)?

2. **Acceptable loan duration**: How long should loans stay outstanding before concern?
   - Hours? Days? Weeks?

3. **Vault balance target**: Should we aim for any particular StakeVault/DistoVault ratio?
   - Or let it float naturally?

4. **Member impact**: Is reduced UBD during loan outstanding acceptable?
   - Or should members always receive full expected distribution?

5. **Implementation priority**: 
   - Start with Natural Rebalancing (learn from data)?
   - OR implement Torq Bump from day 1 (prevent imbalances)?

---

## 🎛️ Configuration (Phase 6 Implementation)

### **Environment Variables**

```bash
# Loan Policy (default: natural)
DISTODAM_LOAN_POLICY=natural

# Torq Bump Threshold Policy Settings
DISTODAM_TORQ_BUMP_THRESHOLD_PCT=0.30    # 30% of contract
DISTODAM_TORQ_BUMP_MULTIPLIER=1.0        # 100% bump (doubles output)

# Torq Bump Delayed Policy Settings
DISTODAM_TORQ_BUMP_DELAY_HOURS=24.0      # Wait 24 hours before bumping

# Vault Genesis Bootstrap
DISTODAM_GENESIS_STAKE_VAULT_PCT=0.95    # 95% to StakeVault
DISTODAM_GENESIS_DISTO_VAULT_PCT=0.05    # 5% to DistoVault
```

### **Prometheus Metrics (Monitor Policy Effectiveness)**

```
# Loan behavior
distodam_loans_created_total
distodam_loans_outstanding_count
distodam_loans_outstanding_rt
distodam_loans_repaid_total
distodam_loan_duration_seconds (histogram)

# Policy interventions
distodam_torq_bumps_requested_total
distodam_torq_bumps_policy{policy="natural|torq_bump_*"}

# Vault health
distodam_stake_vault_balance_rt
distodam_disto_vault_balance_rt
distodam_vault_ratio{vault="stake|disto"}
```

### **Operational Dashboard**

```
| Metric | Target | Alert Threshold |
|--------|--------|----------------|
| Outstanding Loans | < 5% of StakeVault | > 20% |
| Avg Loan Duration | < 6 hours | > 48 hours |
| Loans per Day | < 10 | > 50 |
| Vault Ratio (Stake/Disto) | 0.80-0.95 | < 0.50 or > 0.98 |
| Torq Bumps per Week | 0 (natural policy) | > 5 |
```

---

**Status**: ✅ **POLICY DECIDED** - Natural equilibrium by default, configurable intervention lever built in.

**Implementation**: Phase 6 will include LoanPolicy enum, shouldTorqBump() logic, and comprehensive metrics.

**Last Updated**: November 17, 2025  
**Decision**: Jon Graves  
**Documentation**: GitHub Copilot
