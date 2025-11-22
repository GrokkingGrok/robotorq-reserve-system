# RoboTorq Reserve System Architecture

**Version**: 2.0 (Corrected - RoboStake as Network Collateral)  
**Date**: November 21, 2025  
**Status**: Design Complete  
**Author**: GitHub Copilot (Claude Haiku 4.5)  
**Editor**: Jon Clark

---

## Executive Summary

The **RoboTorq Reserve System** is a three-tier capital framework where network-minted collateral (RoboStake) flows through Trust validators, CertVault registries, and DistoVault distributors to fund the entire economic loop.

**Critical Correction**:
- ❌ OLD: RoboStake = Payment TO robots
- ✅ NEW: RoboStake = Network's COLLATERAL (internal-loop JTUs) FOR robotic labor

**The Three Shadow Vaults per Vault Node**:
1. **Shadow StakeVault** (Reserve Holder): Holds RoboStake collateral (excess supply)
2. **Shadow CertVault** (Contract Registry & Bearer Bonds): Validates contracts, manages certificates, issues bearer bonds
3. **Shadow DistoVault** (Distribution Stream): Produces JTUs deterministically, routes to user StashVaults equally

**Trust Service**: Capital allocator that requests RoboStake when contracts need it (validated by CertVault)

**User Vaults per Node** (hosted alongside shadow vaults):
- **StashVaults** (savings/reserve, first landing zone for ALL value)
- **PledgeVaults** (commitment/locked funds)

**Business Model**:
- Business pays for **goods** (embed robot labor cost)
- Robot labor creates **value** (measured in kWh × tokens/sec)
- Network provides **RoboStake collateral** (internal loop, never leaves network)
- Value flows back via **crypto pipeline** (Phase3 → Mint → UBD distribution)

---

## Table of Contents

1. [The RoboTorq Loop](#1-the-robotorq-loop)
2. [RoboStake: Network Collateral](#2-robostake-network-collateral)
3. [Trust Service: Capital Allocation](#3-trust-service-capital-allocation)
4. [CertVault: Contract Registry](#4-certvault-contract-registry)
5. [DistoVault: JTU Distribution](#5-distovault-jtu-distribution)
6. [Shadow Vaults: Operational Reserves](#6-shadow-vaults-operational-reserves)
7. [Printer Bearer Bonds](#7-printer-bearer-bonds)
8. [Data Flow & Messaging](#8-data-flow--messaging)
9. [Implementation Roadmap](#9-implementation-roadmap)

---

## 1. The RoboTorq Loop

### 1.1 Complete Capital Flow

```
CONTRACT COMMISSIONED
  │
  ├─ Business requests robot labor (e.g., "3D print 100 coins")
  │
  └─ Trust Service:
     ├─ Queries CertVault: "Is this contract real?"
     │  └─ CertVault: "Yes, validates against schema. Need 1,000 RT RoboStake (10 kW × 100 tokens/sec × 1h)."
     │
     └─ Requests from StakeVault: "I need 1,000 RT RoboStake and my torq factor is 1.5"
        ├─ StakeVault: "Releasing 1,000 RT from collateral reserve"
        │
        └─ Robot operates under RoboStake
           ├─ Stake Paid: 10 kW (Robot Power) × 100 tokens/sec (Tokens Processed) × 1 hour = 1,000 RT = 3.6 billion JTUs
           ├─Value Created: 3.6 billion JTUs x 1.5 torq factor = 5.4 billtion JTUs
           │
           ├─ Phase1: Digger harvests ore (kinetic energy → tokens)
           ├─ Phase2: Refinery processes ore (JouleTorqOre → Phase2Ingot)
           ├─ Phase3: Mint validates (Phase2Ingot → Phase3RoboTorqUnit)
           │
           └─ CRYPTO PIPELINE: RoboTorqUnit arrives at DistoDam
              ├─ Extract RoboStake (1,000 RT) → travels back to StakeVault reserves
              ├─ Extract Certificates → stored in CertVault (permanent ledger)
              │
              └─ DistoVault: Produces 5,400,000,000 JTUs
                 └─ Distributes EQUALLY to ALL user StashVaults (UBD)
                    ├─ Every network member receives equal share
                    ├─ StashVault → first landing zone (savings)
                    ├─ Vault system manages StashVault → Wallet transfers
                    └─ Economic activity shifts value around naturally
```

### 1.2 The Key Insight

**Money flows in two directions**:

```
FORWARD (Collateral):    RoboStake → Trust → Digger → Refinery → Mint → DistoDam
                         ↑                                                    ↓
                         └────────────── Returns to StakeVault ───────────────┘

REVERSE (Value):         Certificates → CertVault (locked)
                         DistoVault → Produces JTUs (stake × torq factor = value of cert the certvault just stored, distoVault gets told how much to make by CertVault) → Signs & distributes equally to ALL StashVaults (UBD)
```

**RoboStake never leaves the network**. It travels WITH the proof chain:
1. Minted at startup (network's capital in Shadow StakeVault)
2. Released by Trust when contracts need it (validated by Shadow CertVault)
3. Digger operates under that collateral (RoboStake attached to each JouleTorq)
4. RoboStake travels through entire pipeline: Ore → Refinery → Mint → DistoDam
5. DistoDam deposits RoboStake BACK to Shadow StakeVault (when minting completes)
6. Cycle repeats infinitely (mathematically cannot run dry due to demurrage or sipping from DistoStream to the StashVaults)

---

## 2. RoboStake: Network Collateral

### 2.1 What is RoboStake?

**Definition**: JTUs representing the network's investment in robotic labor capacity.

**Measurement Formula** (Cross Products):
```
1 RoboTorq (RT) = 1 kWh × 1 token/sec = 3,600,000 JTUs

RoboStake (RT) = Robot kW × Robot Tokens/sec × Hours

Example:
  Robot: 10 kW, tokens/sec rating = 100
  Task duration: 1 hour
  
  RoboStake (RT) = 10 kW × 100 tokens/sec × 1 hour
  RoboStake (RT) = 1,000 RT
  
  In JTUs:
  RoboStake (JTUs) = 1,000 RT × 3,600,000 JTU/RT
  RoboStake (JTUs) = 3,600,000,000 JTUs

Physics breakdown:
  1 kWh = 3.6 MJ (3,600,000 Joules)
  1 RT = 1 kWh × 1 token/sec
  Over 3600 seconds: 3.6 MJ × 1 token = 3,600,000 JouleTorqs
  A 2 kWh robot with a max token throughput of 50 tokens/sec would cost 2 kWh x 50 tokens/sec = 100 RT/hour, paid by the network, not the business that hires the bot (they pay a bot provider for access)
```

### 2.2 StakeVault: RoboStake Reserve

**Purpose**: Hold network's RoboStake collateral, release on-demand

**Architecture**:
```go
type StakeVault struct {
    // Reserve capacity
    TotalRoboStakeMicroRT      int64  // Total collateral available (network-wide)
    AvailableRoboStakeMicroRT  int64  // Not yet deployed to contracts
    DeployedRoboStakeMicroRT   int64  // Currently funding active contracts
    
    // Tracking
    ContractAllocations map[string]int64  // contractID → deployed RoboStake
    AllocationHistory   []*AllocationEvent
}

type AllocationEvent struct {
    EventID           string
    Timestamp         time.Time
    ContractID        string
    TrustID           string
    AllocationMicroRT int64  // RoboStake requested
    Status            string // "requested", "approved", "released", "returned"
    Reason            string
}
```

**Events**:
```
vault.robostake.requested  → Trust asks for RoboStake (validates against CertVault first)
vault.robostake.released   → StakeVault releases to contract
vault.robostake.returned   → Robot finishes, RoboStake returned from Phase3Proof
vault.robostake.slashed    → Malicious contract behavior → RoboStake penalty
```

### 2.3 RoboStake Lifecycle

**Timeline**:
```
T=0: StakeVault holds 10,000 RT RoboStake (network reserve)

T=5min: Trust requests 1,000 RT for "3D Printer Contract #42" (10 kW × 100 tokens/sec × 1 hour)
        └─ Queries CertVault: "Is contract #42 registered?"
           └─ CertVault: "Yes, with 1,000 RT allocation schema (10 kW × 100 tokens/sec)"
        └─ StakeVault: Transitions 1,000 RT from Available → Deployed

T=5min-65min: Robot operates under 1,000 RT RoboStake collateral for 60 minutes
              ├─ Stake Paid: 10 kW × 100 tokens/sec × 1 hour = 1,000 RT = 3,600,000,000 JTUs
              └─ Value Created: 3.6B JTUs × 1.5 torq factor = 5,400,000,000 JTUs

T=65min: Phase3 proof arrives at DistoDam (minting complete)
         ├─ RoboTorqUnit: contains RoboStake (1,000 RT) + Certificates + Torq Factor (1.5)
         ├─ DistoDam extracts RoboStake → returns to Shadow StakeVault (available again)
         ├─ DistoDam extracts Certificates → stores in Shadow CertVault (permanent ledger)
         └─ DistoVault: Produces 5,400,000,000 JTUs (stake × torq factor)
            └─ Distributes EQUALLY to all user StashVaults (UBD)
               └─ Vault system manages StashVault → Wallet transfers

Final state: StakeVault still holds 10,000 RT
             ├─ 9,000 RT available
             └─ 1,000 RT deployed (if next contract running)
```

### 2.4 RoboStake Funding Sources

**Initial allocation** (genesis):
```
Network bootstraps with N RT of RoboStake
└─ Divided proportionally among TorqVault services
   └─ Stored in their StakeVaults
      └─ Available for allocation to contracts
```

**Replenishment** (from value creation):
```
Phase3Proofs return RoboStake to StakeVault
└─ Continuous cycle: allocate → operate → return → reallocate
   └─ Network capacity grows with successful contracts
      └─ More value created = more RoboStake available
```

**Emergency reserves** (crisis mode):
```
If StakeVault depleted (too many contracts, not enough returned):
  └─ Urgent demurrage rerouting (50-80% of daily demurrage)
     └─ Diverted to Shadow StakeVaults
        └─ Can front emergency RoboStake if needed
           └─ Resolved when Phase3Proofs return capital
```

---

## 3. Trust Service: Capital Allocation

### 3.1 Trust Architecture

**Purpose**: Validate contracts and allocate RoboStake from StakeVault

**Responsibilities**:
```go
type TrustService struct {
    // Validation
    CertVaultClient  *grpc.Client      // Query contracts
    StakeVaultClient *grpc.Client      // Request/release RoboStake
    
    // Request processing
    PendingRequests map[string]*AllocationRequest
    
    // Logging
    AllocationLog []AllocationEvent
}

type AllocationRequest struct {
    RequestID        string
    ContractID       string
    Metadata         map[string]string
}
```

### 3.2 Trust Allocation Flow

**Request → Validate → Release → Track**:

```
1. RECEIVE REQUEST
   └─ "Contract #42 needs 1,000 RT RoboStake (10 kW × 100 tokens/sec × 1 hour) with torq factor 1.5"
      └─ From: Business/Platform integration

2. VALIDATE AGAINST CERTVAULT
   ├─ CertVault.QueryContract(contractID)
   └─ Verify:
      ├─ Contract registered? ✓
      ├─ Owner wallet authenticated? ✓
      ├─ Allocation schema allows 1,000 RT? ✓
      └─ No suspicious activity? ✓

3. REQUEST FROM STAKEVAULT
   ├─ StakeVault.AllocateRoboStake(contractID, 1000 MRT)
   └─ Verify:
      ├─ Available >= 1,000 MRT? ✓
      ├─ Transition: Available → Deployed
      └─ Log event: "vault.robostake.released"

4. RETURN TO CONTRACT
   └─ Signal contract service:
      ├─ RoboStake allocated: 1,000 RT
      ├─ Ready to start robot labor
      └─ Await Phase3 return

5. TRACK DEPLOYMENT
   └─ Monitor contract:
      ├─ Is robot running? (healthcheck)
      ├─ Any errors? (log monitoring)
      ├─ Misbehavior? (slash RoboStake)
      └─ Completion? (return RoboStake)
```

**Go Implementation** (pseudocode):
```go
func (ts *TrustService) AllocateRoboStake(contractID string, requiredMRT int64) error {
    // 1. Query CertVault
    contract, err := ts.CertVaultClient.QueryContract(contractID)
    if err != nil {
        return fmt.Errorf("contract not found in CertVault: %w", err)
    }
    
    // 2. Verify allocation schema
    if requiredMRT > contract.AllocationSchemaMRT {
        return fmt.Errorf("requested %d exceeds schema limit %d", 
            requiredMRT, contract.AllocationSchemaMRT)
    }
    
    // 3. Request from StakeVault
    allocation, err := ts.StakeVaultClient.RequestAllocation(
        &pb.AllocationRequest{
            ContractID: contractID,
            AmountMicroRT: requiredMRT,
        })
    
    if err != nil {
        ts.logger.Error("StakeVault request failed", "error", err)
        return err
    }
    
    // 4. Log and track
    event := AllocationEvent{
        EventID: uuid.New().String(),
        ContractID: contractID,
        TrustID: ts.serviceID,
        AllocationMicroRT: requiredMRT,
        Status: "released",
        Timestamp: time.Now(),
    }
    ts.AllocationLog = append(ts.AllocationLog, event)
    
    ts.logger.Info("RoboStake allocated",
        "contract_id", contractID,
        "amount_micro_rt", requiredMRT,
        "allocation_id", allocation.ID)
    
    return nil
}

func (ts *TrustService) ReturnRoboStake(contractID string, returnedMRT int64) error {
    // Called when Phase3Proof arrives with RoboStake in it
    err := ts.StakeVaultClient.ReturnAllocation(
        &pb.ReturnRequest{
            ContractID: contractID,
            AmountMicroRT: returnedMRT,
        })
    
    if err != nil {
        return err
    }
    
    ts.logger.Info("RoboStake returned to StakeVault",
        "contract_id", contractID,
        "amount_micro_rt", returnedMRT)
    
    return nil
}
```

---

## 4. CertVault: Contract Registry

### 4.1 CertVault Purpose

**Definition**: Authoritative registry of contracts accepted on the network.

**Core Functions**:
1. **Register contracts** with allocation schema (how many JTUs they can produce)
2. **Validate contracts** when Trust requests RoboStake (contract really exists)
3. **Generate deterministic JTU distribution** curves (how JTUs flow to wallets)
4. **Issue bearer bonds** for printer commissioning (physical coins)

### 4.2 Contract Registration

**Schema**:
```go
type RegisteredContract struct {
    ContractID           string
    OwnerWallet          string
    ContractType         string  // "labor", "goods", "services", "physical"
    AllocationSchemaMRT  int64   // Max RoboStake for this contract
    ExpectedOutputJTU    int64   // How many JTUs expected from labor
    TorqFactor           float64
    DistributionSchema   *DistributionCurve
    Status               string  // "active", "paused", "completed", "slashed"
    CreatedAt            time.Time
    CertVaultSignature   []byte  // Dilithium5 signature
}

type DistributionCurve struct {
    // Deterministic function: time → JTU amount
    // Example: Weibull distribution for labor completion
    ContractID     string
    CurveType      string    // "weibull", "linear", "exponential"
    Parameters     map[string]float64
    
    // Economic info
    TotalJTU       int64
    MarketDate     time.Time // the day this good is expected to hit market
    Confidence     float64 // how confident the firm is that it will hit that date. for a delivery bot that already did the work this is 100, for an airframe that could get delayed massively it would be relatively low
}
```

**Example Contract**:
```
RegisteredContract {
    ContractID: "printer-coin-42",
    OwnerWallet: "alice@wallet",
    ContractType: "physical",
    AllocationSchemaMRT: 1,000 RT (10 kW × 100 tokens/sec × 1 hour)
    TorqFactor: 1.5 (50% efficiency/quality bonus)
    ExpectedOutputJTU: 5,400,000,000 (3.6B stake × 1.5 torq factor)
    
    Note: UBD distributes ALL 5.4B JTUs equally to network members
    Economic activity redistributes value to producer through market forces
}
```

### 4.3 CertVault Validation Flow

```
Trust requests: "Validate contract-42"
  └─ CertVault: Query RegisteredContracts
     ├─ Found? Yes
     ├─ Active? Yes
     ├─ Signature valid (Dilithium5)? Yes
     └─ Response: {
          approved: true,
          allocationSchemaMRT: 50,
          distributionCurve: {...}
        }
```

---

## 5. DistoVault: JTU Distribution

### 5.1 DistoVault Architecture

**Purpose**: Produce JTUs deterministically based on CertVault distribution schemas

**Key Feature**: Self-replenishing wallets from contract labor

```go
type DistoVault struct {
    // Contract tracking
    ActiveContracts map[string]*ContractStream
    CompletedContracts map[string]*CompletedContractRecord
    
    // Wallet management
    WalletStreams map[string]*JTUStream
    
    // Monitoring
    TotalJTUProducedMicroRT int64
    TotalJTUDistributedMicroRT int64
}

type ContractStream struct {
    ContractID          string
    Schema              *DistributionCurve
    StreamStartTime     time.Time
    CurrentJTUProduced  int64
    Status              string  // "streaming", "completed", "failed"
}

type JTUStream struct {
    WalletID          string
    IncomingJTUPerMin float64  // Drip rate from active contracts
    CurrentBalanceMRT int64    // Accumulated JTUs
    WithdrawalQueue   []*WithdrawalRequest
}
```

### 5.2 Deterministic JTU Production

**Mathematical integration** (wallets are self-replenishing):

```
For each active contract:
  1. Query CertVault distribution schema
  2. Calculate current JTU position (based on elapsed time)
  3. Route to owner wallet:
     owner_jtu = current_position × owner_allocation %
  4. Route to fees (Shadow DistoVault):
     fees_jtu = current_position × fees_allocation %
  5. Repeat every 1 second (check elapsed time)
  6. When contract complete, mark as historical
```

**Example** (5.4 billion JTU contract = 1,000 RT stake × 1.5 torq factor from 10 kW robot @ 100 tokens/sec × 1 hour):
```
UBD Distribution: Equal to ALL StashVaults

T=65min: Minting complete, DistoDam receives Phase3RoboTorqUnit
         ├─ Extract RoboStake: 1,000 RT → back to Shadow StakeVault
         ├─ Extract Certificates → Shadow CertVault (permanent)
         ├─ Extract Torq Factor: 1.5
         └─ DistoVault produces: 5,400,000,000 JTUs (3.6B stake × 1.5 torq factor)

Distribution (assuming 1000 network members):
  Each member's StashVault receives: 5,400,000 JTUs (5,400,000,000 / 1000)
  
Economic activity:
  - Members shift value through buying/selling
  - Eventually someone purchases the physical goods
  - Value reaccumulates to producer through market forces
  - Producer earned premium due to 1.5x torq factor (efficiency/quality bonus)
```

### 5.3 Wallet Self-Replenishment

**Key insight**: Wallets automatically receive JTUs as contracts produce them.

```go
func (dv *DistoVault) processTick() {
    now := time.Now()
    
    for contractID, stream := range dv.ActiveContracts {
        // Calculate elapsed time since contract start
        elapsed := now.Sub(stream.StreamStartTime)
        
        // Get distribution curve from CertVault
        curve := stream.Schema
        
        // Calculate current position on curve (deterministic)
        currentJTU := curve.CalculateJTUAtTime(elapsed)
        
        if currentJTU > stream.CurrentJTUProduced {
            // New JTUs available
            newJTU := currentJTU - stream.CurrentJTUProduced
            
            // Split according to allocation schema
            ownerJTU := int64(float64(newJTU) * curve.OwnerAllocation)
            feesJTU := newJTU - ownerJTU
            
            // Route to wallets
            dv.creditWallet(curve.OwnerWallet, ownerJTU)
            dv.creditWallet(curve.FeesDestination, feesJTU)  // Shadow DistoVault
            
            stream.CurrentJTUProduced = currentJTU
        }
        
        // Check if complete
        if currentJTU >= curve.TotalJTU {
            stream.Status = "completed"
            dv.logger.Info("contract stream complete",
                "contract_id", contractID,
                "total_jtu_produced", currentJTU)
        }
    }
}

func (dv *DistoVault) creditWallet(walletID string, jtuAmount int64) {
    if stream, ok := dv.WalletStreams[walletID]; ok {
        atomic.AddInt64(&stream.CurrentBalanceMRT, jtuAmount)
        
        dv.logger.Debug("wallet credited",
            "wallet_id", walletID,
            "jtu_amount", jtuAmount,
            "new_balance", stream.CurrentBalanceMRT)
    }
}
```

---

## 6. Shadow Vaults: Operational Reserves

### 6.1 Shadow StakeVault (RoboStake Collateral Reserve)

**Purpose**: Hold network's RoboStake collateral for authorizing robotic labor

**Funding**:
- Genesis allocation (network bootstraps with N RT of RoboStake)
- RoboStake returns (from DistoDam when minting completes)
- Urgent demurrage rerouting (50-80% during crisis, if ratio < 0.1)

**Use cases**:
- Authorize robotic labor contracts (Trust allocates from this pool)
- Self-replenishing (RoboStake travels through proof chain, returns to vault)
- Crisis-proof (demurrage ensures mathematically cannot run dry)

### 6.2 Shadow CertVault (Certificate Ledger & Bearer Bonds)

**Purpose**: Permanent certificate storage and bearer bond registry

**Responsibilities**:
- Store RoboTorqUnit certificates (never leave CertVault)
- Validate contracts (registry of approved contracts)
- Issue bearer bonds (physical coin backing via merkle roots)
- Track off-grid inventory (real-time physical RoboTorq supply)

**Critical**: Certificates are the permanent asset record, status transitions:
- `digital` (normal, in CertVault)
- `off_grid` (locked for physical coin)
- `digital` (redeemed back from physical)

### 6.3 Shadow DistoVault (JTU Distribution)

**Purpose**: Produce and distribute JTUs to all StashVaults equally (UBD)

**Process**:
1. Receives signal from DistoDam when minting completes
2. Produces JTUs based on validated labor (kWh × tokens/sec × time)
3. Distributes EQUALLY to all user StashVaults (not wallets)
4. Vault system manages StashVault → Wallet transfers

**Key**: DistoVault ONLY distributes disto (JTUs). Nothing else.

### 6.4 Proportional Allocation (for Demurrage Rerouting)

**Formula**: `TorqVault_Allocation = Total_Amount × (Members / Network_Members)`

**Example**:
```
Network: 1000 total members
  TorqVault Alpha: 500 members (50%)
  TorqVault Beta: 300 members (30%)
  TorqVault Gamma: 200 members (20%)

Urgent rerouting: 100,000 RT diverted
  Alpha: 50,000 RT
  Beta: 30,000 RT
  Gamma: 20,000 RT
```

---

## 7. Printer Bearer Bonds

### 7.1 Physical Coin Minting

**Flow**:
```
1. COMMISSION
   └─ Business: "Print 1,000 physical RoboTorq coins (1 RT each)"
      └─ Wallet: alice@wallet (has 3,600,000,000 JTU = 1,000 RT from completed contract)

2. QUERY CERTVAULT
   └─ Printer: "I need bearer bonds for 1,000 coins @ 3.6M JTU each = 3.6B JTU total"
      └─ Verify: alice@wallet has 3,600,000,000 JTU available? ✓

3. REQUEST BEARER BONDS
   └─ Printer → CertVault.IssueBearerBonds(
        wallet: "alice@wallet",
        amount_jtu: 3_600_000_000,
        coin_count: 1_000,
        nfc_count: 1_000)

4. CERTIFICATE VERIFICATION
   └─ CertVault:
      ├─ Query DistoVault: alice@wallet balance = 3.6B JTU? ✓
      ├─ Extract WHOLE certificates only (no fractional certs)
      │  └─ 1,000 coins × 3.6M JTU/coin = 3.6 billion whole certificates
      ├─ Sign each certificate with SPHINCS+ (post-quantum signature)
      │  └─ cert[i].signature = SPHINCS_Sign(alice_wallet || coin_serial[i] || 3.6M_JTU)
      └─ Generate 1,000 unique NFC private keys (bearer bonds)
         └─ Each key = SPHINCS+ private key for that coin

5. STORE PROOFS
   └─ CertVault database:
      ├─ bearer_bonds[nfc_key] = {
      │    wallet_id: "alice@wallet",
      │    jtu_value: 1.8,
      │    coin_serial: "1000000-of-1000000",
      │    sphincs_signature: "...",
      │    issued_at: "2025-11-21T22:00:00Z",
      │    status: "active"
      │  }
      └─ Store all proofs for audit/recovery

6. SEND TO PRINTER
   └─ CertVault → Printer:
      ├─ nfc_private_keys: [key0, key1, ..., key999999]
      ├─ certificates: [cert0, cert1, ..., cert999999]
      └─ Installation instructions:
         └─ Write NFC key to NFC tag on each coin
            └─ Key = bearer bond (proof of JTU value)

7. PHYSICAL COIN CREATION
   └─ Printer:
      ├─ 3D print 1,000,000 coins
      ├─ Install NFC tag on each coin
      ├─ Write NFC private key to each tag
      └─ Seal coin with certificate
         └─ NFC tag now contains: "I am 1.8 JTU, signed by CertVault"

8. COIN READY
   └─ Physical coin with NFC:
      ├─ Value: 1.8 JTU
      ├─ Bearer: Whoever holds the coin
      ├─ Proof: SPHINCS+ signature in CertVault database
      ├─ Recovery: If lost, prove ownership via blockchain audit
      └─ Redemption: Scan NFC, verify signature, credit wallet
```

### 7.2 Bearer Bond Security

**Why bearer bonds work**:

```
Old model (no bearer bonds):
  Coin has value, but no way to prove it
  └─ Coin could be counterfeited
  └─ Coin could be double-spent
  └─ No audit trail

New model (with bearer bonds):
  Coin has NFC key (SPHINCS+ private key)
  ├─ Only one key per coin (not copyable without CertVault)
  ├─ CertVault has record of every key issued
  ├─ Scanning NFC proves: "I hold the key for 180 JTU"
  └─ Can't forge without SPHINCS+ master key (network only has it)

Redemption:
  1. User scans NFC coin with phone
  2. Wallet app: "Verify this NFC key against CertVault"
  3. CertVault: "Yes, this key is valid, issued to alice@wallet, 180 JTU"
  4. StashVault: "Credit alice's StashVault with 180 JTU" (first landing zone)
  5. User chooses: Keep in StashVault (savings) or transfer to Wallet (spending)
  6. CertVault: Mark this bearer bond as "redeemed" (only once)
```

### 7.3 CertVault Bearer Bond Implementation

**Critical Architecture**:
- RoboTorqUnits (certs) **NEVER leave CertVault** — they are the permanent ledger
- They back value by their existence in CertVault's database
- Printer requests offline bearer bonds, CertVault locks certs and merkle-signs
- Certs stay registered, status transitions: `digital` → `off_grid` → `digital`
- JTUs are slashed when coin goes off-grid, regenerated when coin redeemed

```go
type RoboTorqUnit struct {
// must be copied exactly from mint
}

type BearerBondCert struct { //stays on chain
    BondID              string  // UUID
    DownloadedByWalletID            string //the wallet that created this bearer bond
    UploadedByWalletID              string //the wallet that returned this bearer bond to the grid
    CertCount           int64   // Number of RoboTorqUnits in this bond
    MerkleRoot          string  // Root of merkle tree of all cert hashes
    CertificateHashes   []string  // All 100M leaf hashes (for proof chain)
    SPHINCSSignature    []byte  // SPHINCS+ signature of merkle_root
    NFCPrivateKey       []byte  // Key written to NFC tag
    Status              string  // "digital", "off_grid", "redeemed"
    IssuedAt            time.Time
    RedeemedAt          *time.Time
}

type BearerBondRegistry struct {
    BondsByID           map[string]*BearerBond
    BondsByMerkleRoot   map[string]*BearerBond  // Fast lookup by root
    OffGridInventory    map[string]int64  // walletID → cert_count off-grid
    ProofLog            []BearerBondProof
}

type BearerBondKey struct {
    ProofID         string
    BondID          string
    EventType       string  // "issued", "redeemed", "cancelled"
    WalletID        string
    CertCount       int64
    Timestamp       time.Time
    CertVaultSig    []byte
}

// Issue bearer bond (certs go off-grid, but stay in CertVault locked)
func (cv *CertVault) IssueBearerBond(
    walletID string,
    certCount int64,
) (*BearerBond, error) {
    
    // Step 1: Verify wallet has certCount uncertified RoboTorqUnits
    certs := cv.database.GetCertificatesForWallet(walletID, certCount, status="digital")
    if int64(len(certs)) < certCount {
        return nil, fmt.Errorf("insufficient uncertified certs: have %d, need %d", 
            len(certs), certCount)
    }
    
    // Step 2: Build merkle tree from all cert hashes
    leafHashes := make([]string, len(certs))
    for i, cert := range certs {
        leafHashes[i] = sha256Hash(cert.Hash)  // Hash of cert hash
    }
    
    merkleRoot := cv.buildMerkleTree(leafHashes)
    
    // Step 3: Sign merkle root with SPHINCS+
    sphincsSignature := cv.sphincsKey.Sign([]byte(merkleRoot))
    
    // Step 4: Generate NFC bearer bond key
    nfcPrivateKey := cv.generateNFCPrivateKey()
    
    // Step 5: Create bearer bond record
    bond := &BearerBond{
        BondID:            uuid.New().String(),
        WalletID:          walletID,
        CertCount:         certCount,
        MerkleRoot:        merkleRoot,
        CertificateHashes: leafHashes,
        SPHINCSSignature:  sphincsSignature,
        NFCPrivateKey:     nfcPrivateKey,
        Status:            "off_grid",
        IssuedAt:          time.Now(),
    }
    
    // Step 6: Update cert status in CertVault (locked, not moved)
    for _, cert := range certs {
        cert.Status = "off_grid"
        cert.BearerBondID = bond.BondID
    }
    
    // Step 7: Store bearer bond
    cv.registry.BondsByID[bond.BondID] = bond
    cv.registry.BondsByMerkleRoot[merkleRoot] = bond
    cv.registry.OffGridInventory[walletID] += certCount
    
    // Step 8: Slash JTUs from DistoVault (reserved for when coin redeemed)
    cv.distoVaultClient.SlashJTUs(walletID, certCount)
    
    // Step 9: Log proof
    proof := BearerBondProof{
        ProofID: uuid.New().String(),
        BondID: bond.BondID,
        EventType: "issued",
        WalletID: walletID,
        CertCount: certCount,
        Timestamp: time.Now(),
        CertVaultSig: cv.dilithiumKey.Sign([]byte(bond.BondID)),
    }
    cv.registry.ProofLog = append(cv.registry.ProofLog, proof)
    
    cv.logger.Info("bearer bond issued (certs locked off-grid)",
        "bond_id", bond.BondID,
        "wallet_id", walletID,
        "cert_count", certCount,
        "merkle_root", merkleRoot[:16]+"...")
    
    // Step 10: Return to printer (only NFC key + merkle root, not certs)
    return bond, nil
}

// Redeem bearer bond (certs come back online, JTUs regenerated)
func (cv *CertVault) RedeemBearerBond(merkleRoot string) (string, int64, error) {
    // Step 1: Look up bond by merkle root
    bond, ok := cv.registry.BondsByMerkleRoot[merkleRoot]
    if !ok {
        return "", 0, fmt.Errorf("bearer bond not found for merkle_root")
    }
    
    // Step 2: Verify SPHINCS+ signature (ensure no tampering)
    if !cv.sphincsKey.Verify([]byte(merkleRoot), bond.SPHINCSSignature) {
        return "", 0, fmt.Errorf("merkle root signature invalid")
    }
    
    // Step 3: Verify not already redeemed
    if bond.Status == "redeemed" {
        return "", 0, fmt.Errorf("bearer bond already redeemed at %v", bond.RedeemedAt)
    }
    
    // Step 4: Update cert status back to "digital"
    for _, cert := range cv.database.GetCertsByBearerBond(bond.BondID) {
        cert.Status = "digital"
        cert.BearerBondID = ""
    }
    
    // Step 5: Mark bond as redeemed
    bond.Status = "redeemed"
    now := time.Now()
    bond.RedeemedAt = &now
    
    // Step 6: Update off-grid inventory
    cv.registry.OffGridInventory[bond.WalletID] -= bond.CertCount
    
    // Step 7: Regenerate JTUs in DistoVault (from backing certs)
    cv.distoVaultClient.RegenerateJTUs(bond.WalletID, bond.CertCount)
    
    // Step 8: Log proof
    proof := BearerBondProof{
        ProofID: uuid.New().String(),
        BondID: bond.BondID,
        EventType: "redeemed",
        WalletID: bond.WalletID,
        CertCount: bond.CertCount,
        Timestamp: time.Now(),
        CertVaultSig: cv.dilithiumKey.Sign([]byte(bond.BondID)),
    }
    cv.registry.ProofLog = append(cv.registry.ProofLog, proof)
    
    cv.logger.Info("bearer bond redeemed (certs back online, JTUs regenerated)",
        "bond_id", bond.BondID,
        "wallet_id", bond.WalletID,
        "cert_count", bond.CertCount)
    
    return bond.WalletID, bond.CertCount, nil
}

// Query off-grid inventory instantly
func (cv *CertVault) GetOffGridInventory() map[string]int64 {
    return cv.registry.OffGridInventory
}

// Example usage
func ExamplePhysicalCoinIssuance() {
    wallet := "alice@wallet"
    certCount := int64(100_000_000)  // 100M certs on one coin
    
    // Step 1: Printer requests bearer bond
    bond, err := certVault.IssueBearerBond(wallet, certCount)
    if err != nil {
        log.Fatal(err)
    }
    
    // Step 2: CertVault returns only NFC key + merkle root to printer
    nfcData := map[string]interface{}{
        "nfc_key": bond.NFCPrivateKey,
        "merkle_root": bond.MerkleRoot,
        "cert_count": certCount,
    }
    printerAPI.SendNFCData(nfcData)
    
    // Step 3: Printer embeds NFC key on coin
    // Physical coin now worth exactly 100M certs (backed by merkle root in CertVault)
    
    // Step 4: When coin redeemed
    merkleRoot := nfcTag.ReadMerkleRoot()
    walletID, certCount, err := certVault.RedeemBearerBond(merkleRoot)
    // Certs marked "digital" again, JTUs regenerated to wallet
    
    // Step 5: Real-time off-grid inventory
    offGrid := certVault.GetOffGridInventory()
    // {"alice@wallet": 0}  (all 100M certs are back online)
}
```

### 7.4 Physical Coin Lifecycle: Off-Grid to Digital

**The Complete Flow**:

```
1. COMMISSION PHYSICAL COIN
   └─ Alice: "I want 100M RoboTorq on a physical coin"
      └─ Wallet: alice@wallet has 100M RoboTorqUnits (certs)

2. PRINTER REQUESTS BEARER BOND
   └─ Printer → CertVault: "Alice wants all 100M certs off-grid"
      └─ CertVault:
         ├─ Verify: alice has 100M digital certs ✓
         ├─ Merkle-tree all 100M cert hashes
         ├─ Sign merkle root with SPHINCS+
         ├─ Lock certs: status = "off_grid"
         ├─ Slash 100M JTUs from alice@wallet (reserved)
         └─ Return to printer: {nfc_key, merkle_root}

3. PHYSICAL COIN MANUFACTURED
   └─ Printer:
      ├─ 3D-print coin (normal or massive size)
      ├─ Write NFC key to embedded tag
      ├─ Embed merkle root in QR code
      ├─ Coin now represents 100M locked certs
      └─ Coin is bearer: whoever holds it owns the certs

4. COIN IN CIRCULATION
   └─ Coin physically transported, traded, stored
      └─ In CertVault database:
         ├─ 100M certs: status = "off_grid"
         ├─ bearer_bonds[merkle_root] = {nfc_key, cert_hashes, ...}
         ├─ alice@wallet: off_grid_inventory = 100M
         └─ JTUs: 100M reserved (not available to wallet)

5. REAL-TIME OFF-GRID QUERY
   └─ Query CertVault: "How much is off-grid?"
      └─ Result: {"alice@wallet": 100_000_000, ...}
      └─ Network always knows physical supply

6. COIN REDEEMED
   └─ User scans NFC tag with phone
      └─ Phone extracts: merkle_root + NFC key
      └─ Phone → CertVault: "Redeem this merkle_root"
      
7. ATOMIC REDEMPTION
   └─ CertVault:
      ├─ Verify SPHINCS+ signature on merkle_root ✓
      ├─ Verify not already redeemed ✓
      ├─ Update cert status: "off_grid" → "digital"
      ├─ Remove from off-grid inventory
      ├─ Tell DistoVault: "Regenerate 100M JTUs for alice's StashVault"
      └─ Return: "Success!"
      
8. JTUS REGENERATED
   └─ DistoVault:
      ├─ Calculate: 100M certs × 1 RT = 100M x 3.6 million JTUs
      ├─ Credit alice's StashVault (first landing zone)
      └─ Vault system manages StashVault → Wallet transfer if user chooses

9. COIN IS NOW WORTHLESS
   └─ NFC key can be scanned again, but:
      └─ CertVault: "Bearer bond already redeemed"
         └─ No regeneration, coin has no value
```

**Key Insight: Certs Never Leave**
```
ALICE'S INVENTORY AT EACH STAGE:

Start:
  ├─ Certs (CertVault): 100M, status="digital"
  ├─ JTUs (StashVault): 100M
  └─ Physical: 0

After coin minted:
  ├─ Certs (CertVault): 100M, status="off_grid"
  ├─ JTUs (StashVault): 0 (slashed, reserved)
  └─ Physical: 1 coin

Coin in circulation:
  ├─ Certs (CertVault): 100M, status="off_grid", locked
  ├─ JTUs (StashVault): 0
  └─ Physical: 1 coin (bearer owned)

After redemption:
  ├─ Certs (CertVault): 100M, status="digital"
  ├─ JTUs (StashVault): 100M (regenerated)
  └─ Physical: 1 worthless coin
```

### 7.5 Bearer Bond Architecture Benefits

**Why Certs Never Leave CertVault**:

✅ **Source of Truth**: CertVault is the permanent ledger (like blockchain, but queryable)  
✅ **Instant Audit**: Query off-grid inventory anytime (no scanning all coins)  
✅ **No Double-Spend**: Merkle root can only be redeemed once (SPHINCS+ signature proves ownership)  
✅ **Any Amount on One Coin**: Merkle tree scales to 100M+ certs  
✅ **Post-Quantum Secure**: SPHINCS+ withstands quantum computers  
✅ **Recoverable**: Lost coin? Prove wallet ownership via blockchain, recover value  
✅ **Regenerable Value**: JTUs recreated when coin redeemed (backing certs lock/unlock)  
✅ **Real-Time Transparency**: Off-grid supply visible to entire network  

**Why Bearer Bonds Work**:

```
Bearer Bond = Merkle-Proofed Cert Ownership

Traditional banknote:
  └─ Physical + hard to counterfeit
  └─ Serial number (auditable)
  └─ NO blockchain (can't verify ownership)

RoboTorq Bearer Bond:
  ├─ Physical (NFC tag)
  ├─ IMPOSSIBLE to counterfeit (SPHINCS+ needs master key)
  ├─ Merkle root (auditable in CertVault)
  ├─ Blockchain proof chain (complete history)
  └─ Instant verification (scan + verify signature)
```  

---

## 8. Off-Grid Inventory System

### 8.1 Real-Time Supply Tracking

**The Problem Bearer Bonds Solve**:
- Physical currency is normally opaque (governments print without telling public)
- RoboTorq is different: **complete transparency of off-grid supply**

**The Solution**:
```
CertVault maintains off_grid_inventory:
  {
    "alice@wallet": 100_000_000,  // 100M certs on coins
    "bob@wallet": 500,             // 500 certs on coffee maker
    "charlie@wallet": 1_000_000,   // 1M certs on collector coin
  }

Query anytime: "How much RoboTorq is in physical circulation?"
Answer: 101_500_500 certs total off-grid

This is INSTANT, no scanning or counting coins.
CertVault knows because it signed every bearer bond.
```

**How It Works**:
1. Printer requests bearer bond (certs go off-grid)
   └─ CertVault: `off_grid_inventory[wallet] += certCount`
   
2. Someone redeems coin (certs come back online)
   └─ CertVault: `off_grid_inventory[wallet] -= certCount`
   
3. Audit anytime: `query CertVault.GetOffGridInventory()`
   └─ Returns exact count of physical RoboTorq

**Ledger Integration**:
```go
// Every bearer bond event is recorded
type OffGridEvent struct {
    EventID         string
    EventType       string  // "issued_off_grid", "redeemed_online"
    WalletID        string
    CertCount       int64
    MerkleRoot      string
    Timestamp       time.Time
    CertVaultSig    []byte
}

// Published to NATS JetStream (durable)
// Stream: off_grid_supply
// Subjects: off-grid.events.*

// Can replay to reconstruct off-grid state
func (cv *CertVault) RecalculateOffGridInventory() {
    // Replay all off-grid events from beginning
    // Calculate current state deterministically
    // Verify against database
}
```

**Example Queries**:
```
// How much off-grid total?
SELECT SUM(cert_count) FROM off_grid_inventory
Result: 101,500,500 certs

// How much per wallet?
SELECT wallet_id, cert_count FROM off_grid_inventory WHERE cert_count > 0
Result:
  alice@wallet: 100,000,000
  bob@wallet: 500
  charlie@wallet: 1,000,000

// Is this coin real?
SELECT * FROM bearer_bonds WHERE merkle_root = '0x...'
Result: {issued_at: 2025-11-21, wallet_id: alice@wallet, ...}

// How much does Alice have off-grid?
SELECT SUM(cert_count) FROM bearer_bonds WHERE wallet_id = alice@wallet AND status = 'off_grid'
Result: 100,000,000
```

---

## 8. Data Flow & Messaging

### 8.1 NATS Topics

```
RoboStake Lifecycle:
  vault.robostake.requested    → Trust asks StakeVault
  vault.robostake.released     → StakeVault releases to contract
  vault.robostake.returned     → Phase3 returns RoboStake
  vault.robostake.slashed      → Malicious behavior penalty

Contract Lifecycle:
  contract.registered          → CertVault: New contract
  contract.validated           → Trust: Contract approved
  contract.active              → Contract operating
  contract.complete            → Contract finished
  
JTU Distribution:
  jtu.stream.start            → DistoVault: Start producing JTU
  jtu.stream.tick             → DistoVault: Produce next batch
  jtu.stream.complete         → DistoVault: Contract done
  wallet.credited             → Wallet: Received JTU

Bearer Bonds:
  bond.issued                 → CertVault: Issued N bonds
  bond.redeemed               → CertVault: Physical coin redeemed
  bond.cancelled              → CertVault: Bond revoked
```

### 8.2 Event Sourcing

**All state changes** are immutable events:
```go
type ReserveSystemEvent struct {
    EventID          string
    EventType        string  // "robostake_released", "jtu_streamed", "bond_issued", etc.
    AggregateID      string  // contractID, bondID, walletID, etc.
    Timestamp        time.Time
    Signature        []byte  // Dilithium5 or SPHINCS+ depending on source
    Metadata         map[string]interface{}
}
```

**Event storage**:
```
NATS JetStream (durable):
  Stream: reserve-system-events
  Subjects: reserve.events.*
  Retention: 1 year
  Replicas: 3 (for redundancy)
```

---

## 9. Implementation Roadmap

### Phase 1: Core RoboStake (Weeks 1-2)

**Goals**:
- [x] StakeVault data structure (hold RoboStake collateral)
- [x] Trust allocation flow (validate → release → track)
- [x] CertVault contract registry (store contracts, schemas)
- [ ] Integration: Mint → Trust → CertVault → DistoVault (verify flow)

**Deliverable**: RoboStake flows from StakeVault through contracts

### Phase 2: DistoVault & Wallets (Weeks 3-4)

**Goals**:
- [ ] DistoVault deterministic JTU production
- [ ] Wallet self-replenishment (JTU streams)
- [ ] Integration: Phase3Proof → DistoVault credit

**Deliverable**: JTUs produced and credited to wallets

### Phase 3: Shadow Vaults (Weeks 5-6)

**Goals**:
- [ ] Shadow StakeVault (BidNet escrow buffer)
- [ ] Shadow DistoVault (operations reserve)
- [ ] Urgent demurrage rerouting

**Deliverable**: Network resilience (crisis mode)

### Phase 4: Bearer Bonds (Weeks 7-8)

**Goals**:
- [ ] CertVault bearer bond issuance
- [ ] NFC key generation (SPHINCS+ private keys)
- [ ] Printer integration (issue bonds → print coins)
- [ ] Redemption flow (scan coin → verify → credit)

**Deliverable**: Physical coins backed by JTU bearer bonds

### Phase 5: UI & Integration (Weeks 9-10)

**Goals**:
- [ ] UBD: Display user RoboStake allocations
- [ ] UBD: Display JTU streams (incoming contracts)
- [ ] UBD: Bearer bond redemption status
- [ ] Printer dashboard: Monitor issued bonds

**Deliverable**: End-to-end reserve system visible to users

---

## Conclusion

The **RoboTorq Reserve System** is an elegant three-tier architecture:

1. **StakeVault** holds network's RoboStake collateral (supply)
2. **Trust** validates & allocates collateral to contracts (demand)
3. **CertVault** registers contracts & issues bearer bonds (governance)
4. **DistoVault** produces JTUs deterministically (value realization)

**Key insight**: Business pays for goods, not labor. Network provides collateral (internal loop), labor creates value (external loop via Phase3 crypto → wallet distributions).

**Physical coins** backed by bearer bonds (SPHINCS+ signatures) ensure coins can be:
- ✅ Verified (CertVault audit trail)
- ✅ Redeemed (scan NFC → credit wallet)
- ✅ Protected against counterfeiting (post-quantum signatures)
- ✅ Recovered if lost (blockchain ownership proof)

---

*"Three tiers, one loop: Collateral → Labor → Value → Wallets. The network's heart beats."* 💰⚡🔐

