# Vault System MVP – Final Design

**Version**: 1.1 (MVP – NATS, Single Node)
**Date**: November 24, 2025
**Status**: Updated to Reflect Actual Implementation
**Language**: Rust (tokio async runtime)

---

## 1. MVP Scope (Actual Implementation)

This MVP implements a **working UBD distribution system** with the following end-to-end capability:

> On Phase3 completion from Mint, the vault service stores newly minted RoboTorq certificates in CertVault, processes returned RoboStake, authorizes UBD distribution schedules, and delivers value to wallets either via scheduled drips to ShortVaults or direct single-package delivery to wallets.

### What's Actually Implemented:
- ✅ **Certificate Storage**: ShadowCertVault stores RoboTorqCertificates
- ✅ **Stake Management**: ShadowStakeVault tracks available/deployed RoboStake
- ✅ **UBD Distribution**: ShadowDistoVault with persistence and recovery
- ✅ **ShortVault System**: User vaults with demurrage-free balances and drip management
- ✅ **Single Package Delivery**: Direct wallet-to-vault UBD packages (alternative to drips)
- ✅ **Contract Approval**: Optional service for approving robotic labor contracts
- ✅ **NATS Integration**: Full event-driven communication
- ✅ **Persistence**: PostgreSQL storage for distostream schedules
- ✅ **Simulation Features**: Time compression, drip algorithms, variance

### What's Not Implemented (Stubs):
- ❌ **Bearer Bonds**: Structs exist but no methods implemented
- ❌ **Peer-to-Peer Transactions**: Endpoints exist but return 404
- ❌ **Multi-node Consensus**: Single-node only
- ❌ **Advanced Demurrage**: Basic demurrage implemented

---

## 2. Mint → Vault Interface (Actual NATS Events)

### 2.1 Phase3 Certificate Batch Event

**Subject**: `vault.phase3.completed`

**Actual Payload**:
```jsonc
{
  "event_type": "robotorqcert_batch_completed",
  "batch_id": "batch-0001",
  "created_at": 1732224000000000000,
  "cert_count": 12,
  "total_robostake": 10,  // Returned stake (may differ from cert_count)
  "canonical_total_jouletorq": 43200000,
  "certificates": [
    {
      "cert_id": "RT-20251121-123456.000000",
      "merkle_root": "abcdef1234...",
      "tree_height": 10,
      "contract_ids": ["printer-coin-42"],
      "minted_at": 1732224000000000000
    }
  ]
}
```

### 2.2 RoboStake Return Event

**Subject**: `vault.robostake.returned`

**Actual Payload**:
```jsonc
{
  "event_type": "robostake_returned",
  "batch_id": "batch-0001",
  "robostake": 10,
  "tokentorq_remainder": 0,  // Placeholder for future
  "jouletorq_remainder": 0   // Placeholder for future
}
```

---

## 3. Core Components (Actual Implementation)

### 3.1 ShadowCertVault (Certificate Storage & Authorization)

**Responsibilities**:
- Store `RoboTorqCertificate`s in DashMap
- Count certificates per contract for authorization
- Emit `vault.distostream.authorized` events

**Actual Code Structure**:
```rust
pub struct ShadowCertVault {
    certificates: DashMap<String, RoboTorqCertificate>,
    nats: Client,
    metrics: Option<Arc<VaultMetrics>>,
}
```

**Key Methods**:
- `store_certificate()`: Stores cert and publishes event
- `robotorq_count_for_contract()`: Counts certs for authorization
- `total_robotorq()`: Total certificates stored

### 3.2 ShadowStakeVault (Stake Reserve Management)

**Responsibilities**:
- Track `available_robostake` and `deployed_robostake` (whole units only)
- Process stake returns from Phase3 batches
- Allocate stake to approved contracts (future use)

**Actual Implementation**:
```rust
pub struct ShadowStakeVault {
    available_robostake: AtomicI64,
    deployed_robostake: AtomicI64,
    nats: Client,
    metrics: Option<Arc<VaultMetrics>>,
}
```

**Key Methods**:
- `increment_available()`: Add returned stake
- `allocate()`: Deploy stake to contracts
- `publish_robostake_return()`: Emit return events

### 3.3 ShadowDistoVault (UBD Distribution Engine)

**Responsibilities**:
- Subscribe to `vault.distostream.authorized`
- Execute distribution schedules with persistence
- Support both drip-based and single-package delivery
- Recover active schedules on restart

**Actual Code Structure**:
```rust
pub struct ShadowDistoVault {
    nats: Client,
    metrics: Option<Arc<VaultMetrics>>,
    tick_interval_millis: i64,
    schedules: Arc<DashMap<String, StreamState>>,
    persistence: Option<VaultPersistence>,
    time_compression_factor: f64,
}
```

**Key Features**:
- **Persistence**: Saves schedules to PostgreSQL
- **Recovery**: Restores active schedules on startup
- **Time Compression**: Simulation feature for faster testing
- **Dual Delivery**: Single packages OR drip-based distribution

### 3.4 ShortVault & ShortVaultRegistry (User Landing Zones)

**Responsibilities**:
- Provide demurrage-free reserve balances
- Manage drip schedules for gradual wallet funding
- Handle demurrage requests from wallets

**Actual Implementation**:
```rust
pub struct ShortVault {
    user_id: String,
    balance_canonical_jouletorq: AtomicI64,
    nats: Client,
    metrics: Option<Arc<VaultMetrics>>,
    active_drips: Arc<Mutex<HashMap<String, DripSchedule>>>,
}
```

**Key Methods**:
- `credit_ubd()`: Add UBD to reserve
- `request_demurrage_release()`: Process wallet demurrage requests
- `apply_demurrage_to_all()`: Simulation feature for demurrage processing

### 3.5 ContractApproval (Optional Service)

**Responsibilities**:
- Evaluate contract approval based on stake availability
- Track concurrent contract limits
- Allocate stake to approved contracts

**Actual Implementation**:
```rust
pub struct ContractApproval {
    stake_vault: Arc<ShadowStakeVault>,
    nats: NatsClient,
    config: ApprovalConfig,
    active_contracts: Arc<Mutex<HashSet<String>>>,
}
```

---

## 4. UBD Distribution Modes (Actual Implementation)

### 4.1 Single Package Delivery (New Default)

**Configuration**: `VAULT_SINGLE_PACKAGE_DELIVERY=true`

**Flow**:
1. DistoVault receives `vault.distostream.distribution` tick
2. Creates `UBDDistributionPackage` for each wallet
3. Publishes to `vault.ubd.package` subject
4. Wallets receive and credit balances
5. Wallets send confirmations via `wallet.package.confirmation`

**Package Structure**:
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UBDDistributionPackage {
    pub package_id: String,
    pub user_id: String,
    pub amount_canonical_jouletorq: i64,
    pub distribution_timestamp: DateTime<Utc>,
    pub provenance_cert_ids: Vec<String>,
    pub package_hash: String,
    pub vault_signature: Option<String>,
}
```

### 4.2 Drip-Based Delivery (Legacy)

**Configuration**: `VAULT_SINGLE_PACKAGE_DELIVERY=false`

**Flow**:
1. DistoVault credits ShortVault balances directly
2. ShortVaults manage drip schedules
3. Wallets request demurrage releases
4. ShortVaults send `DemurrageReleasePackage` to wallets

---

## 5. Data Models (Actual Implementation)

### 5.1 Hierarchical Triple (Implemented but Simplified)

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Triple {
    pub robotorq: i64,
    pub tokentorq_remainder: i64,
    pub jouletorq_remainder: i64,
}

pub fn canonical_jouletorq(triple: Triple) -> i64 {
    triple.robotorq * 3_600_000 + triple.tokentorq_remainder * 3_600 + triple.jouletorq_remainder
}
```

**Current Usage**: Remainder fields are placeholders (set to 0). Only whole RoboTorq units are processed.

### 5.2 Certificate Model

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoboTorqCertificate {
    pub cert_id: String,
    pub merkle_root: String,
    pub tree_height: Option<u32>,
    pub contract_ids: Vec<String>,
    pub minted_at: i64,
}
```

### 5.3 Package Models

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UBDDistributionPackage {
    pub package_id: String,
    pub user_id: String,
    pub amount_canonical_jouletorq: i64,
    pub distribution_timestamp: DateTime<Utc>,
    pub provenance_cert_ids: Vec<String>,
    pub package_hash: String,
    pub vault_signature: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemurrageReleasePackage {
    pub package_id: String,
    pub user_id: String,
    pub amount_canonical_jouletorq: i64,
    pub request_timestamp: DateTime<Utc>,
    pub release_timestamp: DateTime<Utc>,
    pub package_hash: String,
    pub vault_signature: Option<String>,
}
```

---

## 6. NATS Event Flow (Actual Implementation)

### 6.1 Certificate Processing Flow

```
Mint Phase3 → vault.phase3.completed
    ↓
Vault stores certificates → vault.cert.stored
Vault returns stake → vault.robostake.returned
Vault authorizes UBD → vault.distostream.authorized
    ↓
DistoVault starts distribution → vault.distostream.distribution (ticks)
    ↓
Single Package: vault.ubd.package → Wallets
OR
Drip-based: ShortVault credited → wallet.demurrage.request → vault.demurrage.package
    ↓
Wallet confirms → wallet.package.confirmation
```

### 6.2 All NATS Subjects (Actual)

```rust
// Core vault events
pub const PHASE3_COMPLETED: &str = "vault.phase3.completed";
pub const CERT_STORED: &str = "vault.cert.stored";
pub const ROBOSTAKE_RETURNED: &str = "vault.robostake.returned";
pub const STAKE_ALLOCATED: &str = "vault.stake.allocated";
pub const DISTOSTREAM_AUTHORIZED: &str = "vault.distostream.authorized";
pub const DISTOSTREAM_DISTRIBUTION_TICK: &str = "vault.distostream.distribution";

// ShortVault events
pub const SHORTVAULT_CREATED: &str = "vault.shortvault.created";
pub const SHORTVAULT_DEMURRAGE_APPLIED: &str = "vault.shortvault.demurrage.applied";
pub const SHORTVAULT_UBD_CREDITED: &str = "vault.shortvault.ubd.credited";
pub const SHORTVAULT_WALLET_TRANSFER: &str = "vault.shortvault.wallet.transfer";

// Wallet communication
pub const WALLET_DEMURRAGE_REQUEST: &str = "wallet.demurrage.request";
pub const SHORTVAULT_DEMURRAGE_RESPONSE: &str = "vault.shortvault.demurrage.response";
pub const SHORTVAULT_DRIP_RELEASED: &str = "vault.shortvault.drip.released";
pub const WALLET_BALANCE_REPLENISH: &str = "wallet.balance.replenish";

// Package delivery (new)
pub const VAULT_UBD_PACKAGE: &str = "vault.ubd.package";
pub const VAULT_DEMURRAGE_PACKAGE: &str = "vault.demurrage.package";
pub const WALLET_PACKAGE_CONFIRMATION: &str = "wallet.package.confirmation";
```

---

## 7. Configuration (Actual Environment Variables)

```rust
pub struct VaultConfig {
    pub nats_url: String,
    pub distostream_default_seconds: i64,
    pub distostream_tick_millis: i64,
    pub db_url: String,
    // Contract approval
    pub approval_enabled: bool,
    pub min_available_stake_ratio: f64,
    pub max_concurrent_contracts: usize,
    // Package delivery
    pub single_package_delivery: bool,
    pub package_signing_enabled: bool,
    pub package_hash_verification: bool,
    // Simulation features
    #[cfg(feature = "simulation")]
    pub simulation_mode: bool,
    #[cfg(feature = "simulation")]
    pub time_compression_factor: f64,
    #[cfg(feature = "simulation")]
    pub drip_processing_interval_seconds: i64,
    #[cfg(feature = "simulation")]
    pub drip_algorithm: String,
    #[cfg(feature = "simulation")]
    pub drip_duration_hours: f64,
    #[cfg(feature = "simulation")]
    pub drip_algorithm_param: f64,
    #[cfg(feature = "simulation")]
    pub economic_variance_factor: f64,
    #[cfg(feature = "simulation")]
    pub simulation_scenario_id: Option<String>,
}
```

---

## 8. HTTP API Endpoints (Actual Implementation)

### 8.1 Health & Metrics

- `GET /health`: System status with balances and drip stats
- `GET /metrics`: Prometheus metrics

### 8.2 UBD Distribution (Working)

- `POST /ubd/distribute`: Manual UBD distribution for testing
  ```json
  {
    "wallet_id": "wallet-123",
    "amount_jouletorq": 1000
  }
  ```

### 8.3 Transaction Endpoints (Stub Implementation)

- `POST /transaction/quote`: Returns 404 (not implemented)
- `POST /transaction/commit`: Returns 404 (not implemented)
- `POST /transaction/status`: Returns 404 (not implemented)

---

## 9. Persistence Layer (Actual Implementation)

### 9.1 PostgreSQL Schema

```sql
-- Active distribution schedules
CREATE TABLE disto_schedules (
    schedule_id TEXT PRIMARY KEY,
    contract_id TEXT NOT NULL,
    total BIGINT NOT NULL,
    planned_ticks BIGINT NOT NULL,
    base_per_tick BIGINT NOT NULL,
    remainder BIGINT NOT NULL,
    next_tick_index BIGINT NOT NULL,
    distributed_so_far BIGINT NOT NULL,
    start_ms BIGINT NOT NULL
);

-- Future: transaction logs, package confirmations, etc.
```

### 9.2 Recovery Process

1. On startup, `ShadowDistoVault::recover_active()` loads incomplete schedules
2. Spawns emitters for each recovered schedule
3. Continues distribution from `next_tick_index`

---

## 10. Simulation Features (Actual Implementation)

When `feature = "simulation"` is enabled:

- **Time Compression**: Speed up distribution (1000x faster for testing)
- **Drip Algorithms**: "uniform", "exponential", "linear"
- **Economic Variance**: Random factors in calculations
- **Scenario Tagging**: Track simulation runs

Configuration:
```bash
SIMULATION_MODE=true
SIMULATION_TIME_COMPRESSION=1000.0
VAULT_DRIP_ALGORITHM=uniform
VAULT_DRIP_DURATION_HOURS=24.0
```

---

## 11. Current Status & Next Steps

### ✅ **Working Features**
- Certificate storage and authorization
- UBD distribution (both single-package and drip-based)
- ShortVault management with demurrage
- Contract approval system
- Persistence and recovery
- NATS event-driven communication
- Comprehensive metrics and health checks

### ❌ **Not Yet Implemented**
- Bearer bond issuance/redemption (structs exist, no methods)
- Peer-to-peer transactions (endpoints return 404)
- Multi-node consensus
- Advanced demurrage algorithms

### 🎯 **Architecture Validation**
The vault system successfully implements a **working UBD distribution pipeline** with robust error handling, persistence, and monitoring. The core economic flow (Mint → Vault → Wallets) is functional and ready for production use.

**Key Insight**: UBD distribution is NOT a transaction - it's a direct economic distribution mechanism separate from peer-to-peer transfers, which is architecturally correct for a monetary system.
