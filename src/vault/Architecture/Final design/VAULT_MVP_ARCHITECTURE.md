# Vault System MVP Architecture (Rust)

**Version**: 1.1 MVP  
**Date**: November 21, 2025  
**Status**: Aligned with Final Design (v1.1 – Design Frozen)  
**Language**: Rust (tokio async runtime)  
**Focus**: Single-node MVP, NATS, uniform UBD stream

---

## Executive Summary

The **Vault System** MVP hosts four logical components on a single node:
1. **ShadowCertVault** – Permanent certificate ledger. Certificates never leave.
2. **ShadowStakeVault** – Holds unassigned backed currency reserves and produces signed **StakeEvents** allocating existing RoboStake to approved robotic labor contracts. Outbound transfers ONLY occur for robotic labor. Return flows DO NOT use StakeEvents; they arrive indirectly as minted certificate batches from Mint (Phase3) which replenish system value at the CertVault layer.
3. **ShadowDistoVault** – Does not hold value directly; consumes authorizations and produces signed **IssuanceEvents** (batches of backed currency payments to ShortVaults). CertVault may arrange a transfer from a Disto issuance stream into StakeVault reserve (top‑up) but StakeVault never sends value to DistoVault.
4. **ShortVaults** – Vector of user landing vaults local to the node (only customers of this node, not global user set).

### Backed Currency Representation (Hierarchical Triple)
Backed currency is tracked like time (hours : minutes : seconds) but at economic aggregation layers:

| Layer | Meaning | Constant | Relationship |
|-------|---------|----------|--------------|
| RoboTorq (R) | Fully minted certificates | `INGOTS_PER_ROBOTORQ = 1000` | 1 R = 1000 TokenTorq ingots |
| TokenTorq (T) | Complete ingots not yet forming a full certificate | `ORE_PER_INGOT = 3600` | 1 T ingot = 3600 JouleTorq ore units |
| JouleTorq (J) | Atomic ore units (token × joule × second) | — | 1 R = 1000 T = 3,600,000 J |

We store balances as three fixed‑point integer fields where **TokenTorq and JouleTorq are remainders** relative to the next higher layer:
- `robotorq_balance: i128` (whole RoboTorq certificates)
- `tokentorq_remainder: i128` (0 ≤ remainder < 1000)
- `jouletorq_remainder: i128` (0 ≤ remainder < 3600)

Canonical Total Conversion (for comparisons / signatures):

```
total_jouletorq = robotorq_balance * 3_600_000
                + tokentorq_remainder * 3_600
                + jouletorq_remainder
```

Invariants:
```
0 <= tokentorq_remainder < INGOTS_PER_ROBOTORQ
0 <= jouletorq_remainder < ORE_PER_INGOT
```

All arithmetic uses deterministic fixed‑point integers (no floats). A global scaling factor (e.g. MICRO = 1e6) applies uniformly if sub‑micro precision is required; scaling lives in a constants module and is **never embedded inside events**. Remainder normalization occurs after every mutation (allocate / issue / return) ensuring invariants hold and preventing drift.

### Events Instead of Direct Balance Mutation
- **StakeEvents** move hierarchical value (R,T,J) from the **reserve contract** (special contract with `builder_id = CertVault`) into per‑contract allocation staging.
- **IssuanceEvents** stream hierarchical value (R,T,J) into ShortVaults; each event lists total (R,T,J) issued this tick plus deterministic membership snapshot for reproducible splits.
- Certificates never move; events may include `provenance_cert_ids` linking issued or allocated value back to source certificates for audit.

### MVP Goals
1. ✅ Deterministic fixed‑point accounting (triple balance schema)
2. ✅ Certificate storage & immutability guarantees (ShadowCertVault)
3. ✅ Reserve → contract allocation via signed StakeEvents (ShadowStakeVault)
4. ✅ Authorization from certificates → uniform issuance schedule (CertVault → DistoVault)
5. ✅ UBD distribution via signed IssuanceEvents (ShadowDistoVault)
6. ✅ Concurrency & atomicity (tokio + atomics + DashMap)

### Explicit Exclusions (Not in MVP)
- ❌ ShadowBondVault (BearerBond off‑grid management)
- ❌ LongVaults (long‑term savings tier)
- ❌ Multi-node consensus / replication
- ❌ Demurrage rerouting / crisis mode
- ❌ Slashing & advanced Trust logic
- ❌ Oracle payments & quadratic governance

### Terminology Shift
Legacy `robostake_micro_rt` and JTU vectoring are replaced by the hierarchical triple (R, T remainder, J remainder). Outbound StakeEvents (Reserve → Contract) are ONLY for robotic labor allocations. Value "returns" enter the system exclusively as Phase3 Mint batches (new certificates) — the StakeVault itself does not process Contract → Reserve StakeEvents in MVP. No other outbound destinations (e.g. DistoVault) are permitted.

---

## Table of Contents

1. [Rust Architecture](#1-rust-architecture)
2. [Shadow StakeVault](#2-shadow-stakevault)
3. [Shadow CertVault](#3-shadow-certvault)
4. [Shadow DistoVault](#4-shadow-distovault)
5. [User Vaults](#5-user-vaults)
6. [Data & Event Schema](#6-data--event-schema)
7. [Concurrency Model](#7-concurrency-model)
8. [NATS Integration](#8-nats-integration)
9. [MVP Implementation Plan](#9-mvp-implementation-plan)

---

## 1. Rust Architecture

### 1.1 Project Structure

```
src/vault/
├── Cargo.toml
├── src/
│   ├── main.rs               # Entry point, service orchestration
│   ├── lib.rs                # Library exports
│   ├── shadow_vaults/
│   │   ├── mod.rs
│   │   ├── stake_vault.rs    # ShadowStakeVault (RoboStake collateral, receive-only in MVP)
│   │   ├── cert_vault.rs     # ShadowCertVault (certificates, bearer-bond-ready)
│   │   └── disto_vault.rs    # ShadowDistoVault (UBD distribution engine)
│   ├── user_vaults/
│   │   ├── mod.rs
│   │   └── short_vault.rs    # User ShortVault (short-term savings / first landing zone)
│   ├── models/
│   │   ├── mod.rs
│   │   ├── robostake.rs      # RoboStake data structures
│   │   ├── certificate.rs    # RoboTorqCertificate data structures
│   │   └── bearer_bond.rs    # Bearer bond structures
│   ├── events/
│   │   ├── mod.rs
│   │   └── vault_events.rs   # Event sourcing
│   ├── nats_client.rs        # NATS async client
│   └── config.rs             # Configuration (VaultConfig, env-driven knobs)
└── tests/
    └── integration_tests.rs  # Vault system integration tests
```

### 1.2 Core Dependencies

```toml
[package]
name = "robotorq-vault"
version = "0.1.0"
edition = "2024"

[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }
tokio-util = "0.7"

# Concurrency primitives
dashmap = "5.5"              # Concurrent HashMap
arc-swap = "1.6"             # Atomic Arc swapping
parking_lot = "0.12"         # Fast mutexes

# NATS messaging
async-nats = "0.33"
futures = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bincode = "1.3"

# Cryptography (post-quantum)
oqs = "0.8"                  # liboqs for SPHINCS+, Falcon
sha2 = "0.10"                # SHA-256 for merkle trees

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Metrics
prometheus = "0.13"

# UUID generation
uuid = { version = "1.6", features = ["v4", "serde"] }
```

### 1.3 Configuration (VaultConfig)

```rust
// src/config.rs

use anyhow::Result;

/// VaultConfig: environment-driven configuration for the vault MVP
pub struct VaultConfig {
    /// Default duration for a distostream (seconds)
    /// env: VAULT_DISTOSTREAM_DEFAULT_SECONDS, default 60
    pub distostream_default_seconds: i64,

    /// Tick interval for UniformDistoStream (milliseconds)
    /// env: VAULT_DISTOSTREAM_TICK_MILLIS, default 1000 (1s)
    pub distostream_tick_millis: i64,
}

impl VaultConfig {
    pub fn from_env() -> Result<Self> {
        let distostream_default_seconds = std::env::var("VAULT_DISTOSTREAM_DEFAULT_SECONDS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(60);

        let distostream_tick_millis = std::env::var("VAULT_DISTOSTREAM_TICK_MILLIS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(1000);

        Ok(Self {
            distostream_default_seconds,
            distostream_tick_millis,
        })
    }
}
```

---

## 2. Shadow StakeVault

### 2.1 Purpose

Maintain the node’s backed currency reserve (triple balances) and produce signed **StakeEvents** allocating value to approved robotic labor contracts. All unassigned reserves live under a dedicated **reserve contract** whose `builder_id` is the CertVault (sentinel identity). Contract completion / cancellation does not generate a reverse StakeEvent in MVP; instead any economic output or unspent value manifests upstream as minted certificates delivered in Phase3 batches to the CertVault.

### 2.2 Rust Implementation

```rust
// src/shadow_vaults/stake_vault.rs

use dashmap::DashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::RwLock;
use uuid::Uuid;
use anyhow::Result;

/// Shadow StakeVault: Holds backed currency reserve (hierarchical RoboTorq + remainders)
#[derive(Debug)]
pub struct ShadowStakeVault {
    /// Available reserve (R, T remainder, J remainder) at rest in reserve contract
    available_robotorq: AtomicI64,
    available_tokentorq_remainder: AtomicI64,
    available_jouletorq_remainder: AtomicI64,

    /// Currently staged (allocated) to active contracts (R, T remainder, J remainder)
    deployed_robotorq: AtomicI64,
    deployed_tokentorq_remainder: AtomicI64,
    deployed_jouletorq_remainder: AtomicI64,

    /// Contract allocations (contract_id → (R, T remainder, J remainder))
    allocations: DashMap<String, (i64, i64, i64)>,
    
    /// Allocation history (event sourcing)
    history: RwLock<Vec<AllocationEvent>>,
    
    /// NATS client for publishing events
    nats_client: async_nats::Client,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StakeEvent {
    pub stake_event_id: String,
    pub contract_id: String,
    pub robotorq: i64,              // whole RoboTorq units allocated (one-way)
    pub tokentorq_remainder: i64,   // ingot remainder (<1000)
    pub jouletorq_remainder: i64,   // ore remainder (<3600)
    pub provenance_cert_ids: Vec<String>, // optional provenance linkage
    pub timestamp_nanos: i64,
    pub signature: Vec<u8>,         // Falcon signature over canonical fields
}

impl ShadowStakeVault {
    /// Create new vault with genesis allocation
    pub fn new(genesis_robotorq: i64, genesis_tokentorq_rem: i64, genesis_jouletorq_rem: i64, nats_client: async_nats::Client) -> Self {
        Self {
            available_robotorq: AtomicI64::new(genesis_robotorq),
            available_tokentorq_remainder: AtomicI64::new(genesis_tokentorq_rem),
            available_jouletorq_remainder: AtomicI64::new(genesis_jouletorq_rem),
            deployed_robotorq: AtomicI64::new(0),
            deployed_tokentorq_remainder: AtomicI64::new(0),
            deployed_jouletorq_remainder: AtomicI64::new(0),
            allocations: DashMap::new(),
            history: RwLock::new(Vec::new()),
            nats_client,
        }
    }
    // Example allocation function (pseudo – full deterministic fixed-point omitted for brevity)
    pub async fn allocate(&self, contract_id: String, robotorq: i64, tokentorq_rem: i64, jouletorq_rem: i64) -> Result<()> {
        // Bounds check on remainders
        if tokentorq_rem >= 1000 || jouletorq_rem >= 3600 { anyhow::bail!("remainder out of bounds"); }
        let ar = self.available_robotorq.load(Ordering::SeqCst);
        let at = self.available_tokentorq_remainder.load(Ordering::SeqCst);
        let aj = self.available_jouletorq_remainder.load(Ordering::SeqCst);
        if ar < robotorq || at < tokentorq_rem || aj < jouletorq_rem { anyhow::bail!("insufficient reserve"); }
        self.available_robotorq.fetch_sub(robotorq, Ordering::SeqCst);
        self.available_tokentorq_remainder.fetch_sub(tokentorq_rem, Ordering::SeqCst);
        self.available_jouletorq_remainder.fetch_sub(jouletorq_rem, Ordering::SeqCst);
        self.deployed_robotorq.fetch_add(robotorq, Ordering::SeqCst);
        self.deployed_tokentorq_remainder.fetch_add(tokentorq_rem, Ordering::SeqCst);
        self.deployed_jouletorq_remainder.fetch_add(jouletorq_rem, Ordering::SeqCst);
        self.allocations.insert(contract_id.clone(), (robotorq, tokentorq_rem, jouletorq_rem));
        let event = StakeEvent { stake_event_id: Uuid::new_v4().to_string(), contract_id: contract_id.clone(), robotorq, tokentorq_remainder: tokentorq_rem, jouletorq_remainder: jouletorq_rem, provenance_cert_ids: vec![], timestamp_nanos: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0), signature: vec![] };
        self.history.write().await.push(event.clone());
        self.nats_client.publish("vault.stake.event", serde_json::to_vec(&event)?.into()).await?;
        Ok(())
    }
}
```

---

## 3. Shadow CertVault

### 3.1 Purpose
Store RoboTorqCertificates as an immutable ledger. Certificates never leave the CertVault; all allocation and issuance events only reference their IDs for provenance.

Each RoboTorqCertificate = 1 whole RoboTorq unit (R). CertVault does not store ingots (TokenTorq) or ore units (JouleTorq); those lower layers have already been aggregated upstream. Thus CertVault is authoritative only for the count of whole RoboTorq units available per contract.

Hierarchical derivation:
```
total_robotorq_for_contract = count(certificates WHERE contract_id ∈ cert.contract_ids)
// TokenTorq and JouleTorq remainders are always 0 at CertVault layer.
```

Economic role in MVP:
1. Accept Phase3 batches and persist certificates.
2. For a contract, compute available whole RoboTorq (R) by counting its certificates.
3. Authorize a distribution schedule by specifying total RoboTorq (R) to issue. (TokenTorq and JouleTorq remainders remain zero at authorization; downstream ticks may result in remainder handling, but CertVault is not involved.)

Removed legacy concepts:
- No `robostake_micro_rt` numeric collateral field.
- No per‑certificate JTU or derived torq_factor multiplication inside CertVault.
- No JTU streaming list; IssuanceEvents handle hierarchical (R,T,J) emission.

CertVault publishes `vault.distostream.authorized` events containing:
```
{
    event_type: "distostream_authorized",
    contract_id: <id>,
    robotorq_total: <R>,
    tokentorq_remainder: 0,
    jouletorq_remainder: 0,
    duration_seconds: <cfg or payload>,
    starts_at_nanos: <now>,
    provenance_cert_ids: [ ... ]
}
```
Downstream, DistoVault converts this whole RoboTorq total into per‑tick IssuanceEvents; any partial ticks that cannot form a whole RoboTorq unit are expressed as TokenTorq / JouleTorq remainders inside the IssuanceEvents, never by mutating CertVault state.

### 3.2 Rust Implementation

```rust
// src/shadow_vaults/cert_vault.rs

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use oqs::sig::Sig;  // SPHINCS+ post-quantum signatures
use sha2::{Sha256, Digest};
use anyhow::Result;

/// RoboTorqCertificate (canonical wire shape; MUST match Mint exactly)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoboTorqCertificate {
    pub cert_id: String,
    pub robotorq_proof_id: String,
    pub merkle_root: String,
    pub contract_ids: Vec<String>,
    pub timestamp_nanos: i64,
    pub hash: String,
    pub status: CertStatus,
    pub bearer_bond_id: Option<String>,
}

/// RoboTorqBatch (canonical wire shape; MUST match Mint exactly)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoboTorqBatch {
    pub batch_id: String,
    pub created_at_nanos: i64,
    /// Atomic distribution certificates (JTUs) created from minted value.
    /// Replaces legacy numeric field `robostake_micro_rt`.
    pub joule_torq_certificates: Vec<JouleTorqCertificate>,
    /// Permanent backing certificates.
    pub certificates: Vec<RoboTorqCertificate>,
}

/// Registered contract (validated by CertVault)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegisteredContract {
    pub contract_id: String,
    pub owner_wallet: String,
    pub contract_type: String,  // "labor", "goods", "services", "physical"
    pub allocation_schema_mrt: i64,  // Max RoboStake for this contract
    pub expected_output_jtu: i64,    // How many JTUs expected from labor
    pub torq_factor: f64,             // Value multiplier (e.g., 1.5 = 50% bonus)
    pub status: String,               // "active", "paused", "completed", "slashed"
    pub created_at: i64,
    pub cert_vault_signature: Vec<u8>,  // Falcon signature
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CertStatus {
    Digital,      // Normal, on-grid
    OffGrid,      // Locked for physical coin
}

/// Bearer bond certificate (physical coin backing, stays on chain)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BearerBondCert {
    pub bond_id: String,
    pub downloaded_by_wallet_id: String,  // Wallet that created this bearer bond
    pub uploaded_by_wallet_id: Option<String>,  // Wallet that returned it to grid
    pub cert_count: i64,
    pub merkle_root: String,
    pub certificate_hashes: Vec<String>,  // All cert hashes for proof chain
    pub sphincs_signature: Vec<u8>,       // SPHINCS+ signature of merkle_root
    pub nfc_private_key: Vec<u8>,         // Key written to NFC tag
    pub status: BondStatus,
    pub issued_at: i64,
    pub redeemed_at: Option<i64>,
}

/// Bearer bond key (proof log entry)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BearerBondKey {
    pub proof_id: String,
    pub bond_id: String,
    pub event_type: String,  // "issued", "redeemed", "cancelled"
    pub wallet_id: String,
    pub cert_count: i64,
    pub timestamp: i64,
    pub cert_vault_sig: Vec<u8>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BondStatus {
    OffGrid,
    Redeemed,
}

/// Shadow CertVault: Certificate storage and bearer bond registry
pub struct ShadowCertVault {
    /// All certificates (certID → certificate)
    certificates: DashMap<String, RoboTorqCertificate>,
    
    /// Bearer bonds (bondID → bond)
    bonds: DashMap<String, BearerBondCert>,
    
    /// Merkle root index (merkle_root → bondID)
    merkle_index: DashMap<String, String>,
    
    /// Proof log (bearer bond events)
    proof_log: parking_lot::RwLock<Vec<BearerBondKey>>,
    
    /// Off-grid inventory (walletID → cert_count)
    off_grid_inventory: DashMap<String, i64>,
    
    /// SPHINCS+ signing key (post-quantum)
    sphincs_signer: Arc<Sig>,
    
    /// NATS client
    nats_client: async_nats::Client,
}

impl ShadowCertVault {
    pub fn new(nats_client: async_nats::Client) -> Result<Self> {
        // Initialize SPHINCS+ signer (post-quantum secure)
        let sphincs_signer = Sig::new(oqs::sig::Algorithm::Sphincssha256256frobust)?;
        
        Ok(Self {
            certificates: DashMap::new(),
            bonds: DashMap::new(),
            merkle_index: DashMap::new(),
            off_grid_inventory: DashMap::new(),
            proof_log: parking_lot::RwLock::new(Vec::new()),
            sphincs_signer: Arc::new(sphincs_signer),
            nats_client,
        })
    }
    
    /// Store certificate (from Mint)
    pub async fn store_certificate(&self, cert: RoboTorqCertificate) -> Result<()> {
        self.certificates.insert(cert.cert_id.clone(), cert.clone());
        
        // Publish event
        self.nats_client
            .publish(
                "vault.cert.stored",
                serde_json::to_vec(&cert)?.into(),
            )
            .await?;
        
        tracing::debug!(cert_id = %cert.cert_id, "Certificate stored");
        
        Ok(())
    }

    /// Compute hierarchical total (R,T,J) for a given contract from certificates.
    /// - 1 RoboTorqCertificate = 1 RoboTorq (R)
    /// - TokenTorq & JouleTorq remainders derived here are zero because certificates are whole units.
    pub async fn compute_triple_for_contract(&self, contract_id: &str) -> (i64, i64, i64) {
        let robotorq_count = self
            .certificates
            .iter()
            .filter(|entry| entry.value().contract_ids.iter().any(|c| c == contract_id))
            .count() as i64;
        // Remainders zero at this layer.
        (robotorq_count, 0, 0)
    }

    /// Authorize a uniform distostream for a contract using hierarchical triple.
    /// Publishes `vault.distostream.authorized` with (robotorq_total, tokentorq_remainder, jouletorq_remainder).
    /// NOTE: tokentorq_remainder & jouletorq_remainder from certificates are zero; non-zero remainders appear only in RoboStake lifecycle or partial allocation contexts.
    pub async fn authorize_disto_stream(
        &self,
        contract_id: &str,
        default_duration_seconds: i64,
    ) -> Result<()> {
        let (robotorq_total, tokentorq_remainder, jouletorq_remainder) =
            self.compute_triple_for_contract(contract_id).await;

        let duration_seconds = default_duration_seconds;
        let event = serde_json::json!({
            "event_type": "distostream_authorized",
            "contract_id": contract_id,
            "robotorq_total": robotorq_total,
            "tokentorq_remainder": tokentorq_remainder,
            "jouletorq_remainder": jouletorq_remainder,
            "duration_seconds": duration_seconds,
            "schedule_type": "uniform",
            "timestamp_nanos": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        });

        self.nats_client
            .publish(
                "vault.distostream.authorized",
                serde_json::to_vec(&event)?.into(),
            )
            .await?;

        tracing::info!(
            contract_id = %contract_id,
            robotorq_total = robotorq_total,
            duration_seconds = duration_seconds,
            "Disto stream authorized by CertVault (hierarchical triple; cert layer whole R)",
        );

        Ok(())
    }

    /// Publish a RoboStake return event correlated with a cert batch.
    /// Certificates are already minted upstream; this return recycles allocated stake capacity.
    pub async fn publish_robostake_return(
        &self,
        batch_id: &str,
        contract_ids: Vec<String>,
        robotorq: i64,
        tokentorq_remainder: i64,
        jouletorq_remainder: i64,
        provenance_cert_ids: Vec<String>,
    ) -> Result<()> {
        if tokentorq_remainder >= 1000 || jouletorq_remainder >= 3600 {
            anyhow::bail!("remainder out of bounds for RoboStake return");
        }
        let canonical_total_jouletorq = robotorq * 3_600_000 + tokentorq_remainder * 3_600 + jouletorq_remainder;
        let event = serde_json::json!({
            "event_type": "robostake_returned",
            "batch_id": batch_id,
            "contract_ids": contract_ids,
            "robotorq": robotorq,
            "tokentorq_remainder": tokentorq_remainder,
            "jouletorq_remainder": jouletorq_remainder,
            "canonical_total_jouletorq": canonical_total_jouletorq,
            "provenance_cert_ids": provenance_cert_ids,
            "timestamp_nanos": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        });
        self.nats_client
            .publish(
                "vault.robostake.returned",
                serde_json::to_vec(&event)?.into(),
            )
            .await?;
        tracing::info!(batch_id=%batch_id, robotorq=robotorq, tokentorq_remainder=tokentorq_remainder, jouletorq_remainder=jouletorq_remainder, "RoboStake recycled to reserve (cert batch correlated)");
        Ok(())
    }
    
    /// Issue bearer bond (physical coin)
    /// NOTE: Wallet-level selection of certificates is **future work**.
    /// For now, this uses a simple global selection of digital certs.
    pub async fn issue_bearer_bond(
        &self,
        wallet_id: String,
        cert_count: i64,
    ) -> Result<BearerBondCert> {
        // FUTURE: When wallet indexing exists, filter by wallet ownership.
        // MVP placeholder: take the first N digital certificates regardless of owner.
        let certs: Vec<_> = self.certificates
            .iter()
            .filter(|entry| matches!(entry.value().status, CertStatus::Digital))
            .take(cert_count as usize)
            .map(|entry| entry.value().clone())
            .collect();

        if certs.len() < cert_count as usize {
            anyhow::bail!(
                "Insufficient digital certificates: have {}, need {}", 
                certs.len(), 
                cert_count
            );
        }
        
        // Build merkle tree from cert hashes
        let cert_hashes: Vec<String> = certs.iter().map(|c| c.hash.clone()).collect();
        let merkle_root = Self::build_merkle_root(&cert_hashes);
        
        // Sign merkle root with SPHINCS+
        let (pk, sk) = self.sphincs_signer.keypair()?;
        let signature = self.sphincs_signer.sign(merkle_root.as_bytes(), &sk)?;
        
        // Generate NFC private key (for physical coin)
        let nfc_private_key = sk.into_vec();
        
        // Create bearer bond certificate
        let bond = BearerBondCert {
            bond_id: Uuid::new_v4().to_string(),
            downloaded_by_wallet_id: wallet_id.clone(),
            uploaded_by_wallet_id: None,
            cert_count,
            merkle_root: merkle_root.clone(),
            certificate_hashes: cert_hashes,
            sphincs_signature: signature.into_vec(),
            nfc_private_key,
            status: BondStatus::OffGrid,
            issued_at: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            redeemed_at: None,
        };
        
        // Lock certificates (digital → off_grid)
        for cert in &certs {
            if let Some(mut entry) = self.certificates.get_mut(&cert.cert_id) {
                entry.status = CertStatus::OffGrid;
                entry.bearer_bond_id = Some(bond.bond_id.clone());
            }
        }
        
        // Store bearer bond
        self.bonds.insert(bond.bond_id.clone(), bond.clone());
        self.merkle_index.insert(merkle_root, bond.bond_id.clone());
        
        // Update off-grid inventory
        self.off_grid_inventory
            .entry(wallet_id.clone())
            .and_modify(|count| *count += cert_count)
            .or_insert(cert_count);
        
        // Log proof (BearerBondKey)
        let proof = BearerBondKey {
            proof_id: Uuid::new_v4().to_string(),
            bond_id: bond.bond_id.clone(),
            event_type: "issued".to_string(),
            wallet_id: wallet_id.clone(),
            cert_count,
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            cert_vault_sig: vec![],  // TODO: Sign with Falcon
        };
        self.proof_log.write().push(proof);
        
        // Publish event
        self.nats_client
            .publish(
                "vault.bond.issued",
                serde_json::to_vec(&bond)?.into(),
            )
            .await?;
        
        tracing::info!(
            bond_id = %bond.bond_id,
            wallet_id = %wallet_id,
            cert_count = cert_count,
            "Bearer bond issued (certs locked off-grid)"
        );
        
        Ok(bond)
    }
    
    /// Redeem bearer bond (scan physical coin)
    pub async fn redeem_bearer_bond(&self, merkle_root: &str) -> Result<(String, i64)> {
        // Look up bond by merkle root
        let bond_id = self.merkle_index
            .get(merkle_root)
            .ok_or_else(|| anyhow::anyhow!("Bearer bond not found"))?
            .clone();
        
        let mut bond = self.bonds
            .get_mut(&bond_id)
            .ok_or_else(|| anyhow::anyhow!("Bearer bond not found"))?;
        
        // Verify not already redeemed
        if matches!(bond.status, BondStatus::Redeemed) {
            anyhow::bail!("Bearer bond already redeemed at {:?}", bond.redeemed_at);
        }
        
        // Verify SPHINCS+ signature
        let (pk, _) = self.sphincs_signer.keypair()?;
        self.sphincs_signer.verify(
            merkle_root.as_bytes(),
            &bond.sphincs_signature,
            &pk,
        )?;
        
        // Unlock certificates (off_grid → digital)
        for cert_id in self.certificates
            .iter()
            .filter(|e| e.value().bearer_bond_id == Some(bond_id.clone()))
            .map(|e| e.key().clone())
            .collect::<Vec<_>>()
        {
            if let Some(mut entry) = self.certificates.get_mut(&cert_id) {
                entry.status = CertStatus::Digital;
                entry.bearer_bond_id = None;
            }
        }
        
        // Mark bond as redeemed
        bond.status = BondStatus::Redeemed;
        bond.redeemed_at = Some(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        bond.uploaded_by_wallet_id = Some(bond.downloaded_by_wallet_id.clone());  // Could be different wallet in future
        
        // Log proof (BearerBondKey)
        let proof = BearerBondKey {
            proof_id: Uuid::new_v4().to_string(),
            bond_id: bond_id.clone(),
            event_type: "redeemed".to_string(),
            wallet_id: bond.downloaded_by_wallet_id.clone(),
            cert_count: bond.cert_count,
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            cert_vault_sig: vec![],  // TODO: Sign with Falcon
        };
        self.proof_log.write().push(proof);
        
        // Update off-grid inventory
        self.off_grid_inventory
            .entry(bond.wallet_id.clone())
            .and_modify(|count| *count -= bond.cert_count);
        
        // Publish event
        self.nats_client
            .publish(
                "vault.bond.redeemed",
                serde_json::to_vec(&*bond)?.into(),
            )
            .await?;
        
        tracing::info!(
            bond_id = %bond.bond_id,
            wallet_id = %bond.downloaded_by_wallet_id,
            cert_count = bond.cert_count,
            "Bearer bond redeemed (certs unlocked)"
        );
        
        Ok((bond.downloaded_by_wallet_id.clone(), bond.cert_count))
    }
    
    /// Build merkle root from certificate hashes
    fn build_merkle_root(hashes: &[String]) -> String {
        if hashes.is_empty() {
            return String::new();
        }
        
        let mut current_level: Vec<String> = hashes.to_vec();
        
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in current_level.chunks(2) {
                let combined = if chunk.len() == 2 {
                    format!("{}{}", chunk[0], chunk[1])
                } else {
                    chunk[0].clone()
                };
                
                let mut hasher = Sha256::new();
                hasher.update(combined.as_bytes());
                next_level.push(format!("{:x}", hasher.finalize()));
            }
            
            current_level = next_level;
        }
        
        current_level[0].clone()
    }
    
    /// Get off-grid inventory (real-time physical supply)
    pub fn get_off_grid_inventory(&self) -> Vec<(String, i64)> {
        self.off_grid_inventory
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect()
    }
}
```

---

## 4. Shadow DistoVault

### 4.1 Purpose

Generate **IssuanceEvents** (signed) according to authorization schedules derived from certificates (via CertVault). DistoVault holds no persistent backed currency balance; it computes deterministic splits per tick and credits ShortVaults. Late joiners receive remaining ticks only.

### 4.2 Rust Implementation

```rust
// src/shadow_vaults/disto_vault.rs

use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

/// In-memory uniform distostream state (one per authorized stream)
#[derive(Debug, Clone)]
pub struct UniformDistoStream {
    pub stream_id: String,
    pub contract_id: String,
    pub total_jouletorq: i64,
    pub total_tokentorq: i64,
    pub total_robotorq: i64,
    pub duration_seconds: i64,
    pub tick_millis: i64,
    pub remaining_jouletorq: std::sync::atomic::AtomicI64,
    pub remaining_tokentorq: std::sync::atomic::AtomicI64,
    pub remaining_robotorq: std::sync::atomic::AtomicI64,
    pub started_at_nanos: i64,
}

/// Shadow DistoVault: Universal Basic Disto (UBD) distribution
pub struct ShadowDistoVault {
    /// Total JTUs produced (lifetime)
    total_produced_micro_rt: std::sync::atomic::AtomicI64,
    
    /// Total JTUs distributed (lifetime)
    total_distributed_micro_rt: std::sync::atomic::AtomicI64,
    
    /// Active uniform distostreams (stream_id → stream)
    streams: DashMap<String, UniformDistoStream>,
    
    /// NATS client
    nats_client: async_nats::Client,
    
    /// ShortVault registry access
	short_vault_registry: Arc<crate::user_vaults::ShortVaultRegistry>,
}

impl ShadowDistoVault {
    pub fn new(
        nats_client: async_nats::Client,
        short_vault_registry: Arc<crate::user_vaults::ShortVaultRegistry>,
    ) -> Self {
        use std::sync::atomic::AtomicI64;
        
        Self {
            total_produced_micro_rt: AtomicI64::new(0),
            total_distributed_micro_rt: AtomicI64::new(0),
            streams: DashMap::new(),
            nats_client,
            short_vault_registry,
        }
    }

    /// Start listening for `vault.distostream.authorized` events and spawn streams
    pub async fn start_authorization_listener(
        self: Arc<Self>,
        client: async_nats::Client,
        default_tick_millis: i64,
    ) -> Result<()> {
        let mut sub = client.subscribe("vault.distostream.authorized").await?;

        tokio::spawn(async move {
            while let Some(msg) = sub.next().await {
                if let Ok(event) = serde_json::from_slice::<serde_json::Value>(&msg.payload) {
                    if let (Some(contract_id), Some(total), Some(duration)) = (
                        event.get("contract_id").and_then(|v| v.as_str()),
                        event.get("total_jtu_micro_rt").and_then(|v| v.as_i64()),
                        event.get("duration_seconds").and_then(|v| v.as_i64()),
                    ) {
                        let jtu_per_second = event
                            .get("jtu_per_second")
                            .and_then(|v| v.as_i64())
                            .unwrap_or_else(|| if duration > 0 { total / duration } else { total });

                        if let Err(err) = self
                            .start_uniform_stream(contract_id.to_string(), total, duration, jtu_per_second, default_tick_millis)
                            .await
                        {
                            tracing::error!(
                                error = %err,
                                contract_id = %contract_id,
                                "Failed to start uniform distostream",
                            );
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Create in-memory stream state and spawn a ticking task
    pub async fn start_uniform_stream(
        &self,
        contract_id: String,
        total_jtu_micro_rt: i64,
        duration_seconds: i64,
        jtu_per_second: i64,
        tick_millis: i64,
    ) -> Result<()> {
        use std::sync::atomic::{AtomicI64, Ordering};

        let stream_id = uuid::Uuid::new_v4().to_string();
        let remaining = AtomicI64::new(total_jtu_micro_rt);

        let stream = UniformDistoStream {
            stream_id: stream_id.clone(),
            contract_id: contract_id.clone(),
            total_jtu_micro_rt,
            duration_seconds,
            jtu_per_second,
            tick_millis,
            remaining_jtu_micro_rt: remaining,
            started_at: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        };

        self.total_produced_micro_rt.fetch_add(total_jtu_micro_rt, Ordering::SeqCst);
        self.streams.insert(stream_id.clone(), stream.clone());

        tracing::info!(
            stream_id = %stream_id,
            contract_id = %contract_id,
            total_jtu_micro_rt = total_jtu_micro_rt,
            duration_seconds = duration_seconds,
            jtu_per_second = jtu_per_second,
            tick_millis = tick_millis,
            "Uniform distostream started",
        );

        let registry = self.short_vault_registry.clone();
        let nats = self.nats_client.clone();
        let streams = self.streams.clone();

        tokio::spawn(async move {
            let tick_duration = std::time::Duration::from_millis(tick_millis as u64);
            let ticks = if duration_seconds > 0 {
                (duration_seconds * 1000) / tick_millis
            } else {
                1
            };

            for _ in 0..ticks {
                tokio::time::sleep(tick_duration).await;

                let per_second = jtu_per_second;
                if let Err(err) = Self::distribute_tick(&registry, &nats, &stream_id, per_second).await {
                    tracing::error!(
                        error = %err,
                        stream_id = %stream_id,
                        "UBD tick distribution failed",
                    );
                    break;
                }

                let remaining = streams
                    .get(&stream_id)
                    .map(|entry| entry.remaining_jtu_micro_rt.load(Ordering::SeqCst))
                    .unwrap_or(0);

                if remaining <= 0 {
                    tracing::info!(
                        stream_id = %stream_id,
                        "Uniform distostream completed (no remaining JTUs)",
                    );
                    break;
                }
            }

            streams.remove(&stream_id);
        });

        Ok(())
    }

    /// One tick of distribution: snapshot membership and distribute this tick's JTUs
    async fn distribute_tick(
        registry: &Arc<crate::user_vaults::ShortVaultRegistry>,
        nats: &async_nats::Client,
        stream_id: &str,
        jtu_per_second: i64,
    ) -> Result<()> {
        use std::sync::atomic::Ordering;

        // Get current membership at this tick
        let short_vaults = registry.get_all_short_vaults().await?;
        let member_count = short_vaults.len() as i64;

        if member_count == 0 {
            tracing::warn!(
                stream_id = %stream_id,
                "No ShortVaults to distribute to on this tick",
            );
            return Ok(());
        }

        let per_member_jtu = jtu_per_second / member_count;

        tracing::debug!(
            stream_id = %stream_id,
            tick_total = jtu_per_second,
            member_count = member_count,
            per_member = per_member_jtu,
            "UBD tick distribution starting",
        );

        let tasks: Vec<_> = short_vaults
            .into_iter()
            .map(|short_vault_id| {
                let registry = registry.clone();
                let nats = nats.clone();
                let stream_id = stream_id.to_string();
                async move {
                    registry.credit_short_vault(&short_vault_id, per_member_jtu).await?;

                    let event = serde_json::json!({
                        "event_type": "ubd_distributed",
                        "stream_id": stream_id,
                        "short_vault_id": short_vault_id,
                        "amount_micro_rt": per_member_jtu,
                        "timestamp": chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                    });

                    nats.publish("vault.ubd.distributed", serde_json::to_vec(&event)?.into())
                        .await?;

                    Ok::<_, anyhow::Error>(())
                }
            })
            .collect();

        futures::future::try_join_all(tasks).await?;

        tracing::debug!(
            stream_id = %stream_id,
            tick_total = jtu_per_second,
            "UBD tick distribution complete",
        );

        Ok(())
    }
}
```

---

## 5. User Vaults

### 5.1 ShortVault (User Savings)

```rust
// src/user_vaults/short_vault.rs

use std::sync::atomic::{AtomicI64, Ordering};
use anyhow::Result;

/// User ShortVault: First landing zone for ALL value
pub struct ShortVault {
    pub vault_id: String,
    pub user_id: String,
    
    /// Balance in micro RT (atomic for concurrent access)
    balance_micro_rt: AtomicI64,
}

impl ShortVault {
    pub fn new(vault_id: String, user_id: String) -> Self {
        Self {
            vault_id,
            user_id,
            balance_micro_rt: AtomicI64::new(0),
        }
    }
    
    /// Credit ShortVault (UBD, bearer bond redemption, etc.)
    pub fn credit(&self, amount_micro_rt: i64) -> Result<()> {
        self.balance_micro_rt.fetch_add(amount_micro_rt, Ordering::SeqCst);
        
        tracing::debug!(
            vault_id = %self.vault_id,
            amount = amount_micro_rt,
            balance_after = self.balance_micro_rt.load(Ordering::SeqCst),
                "ShortVault credited"
        );
        
        Ok(())
    }
    
    /// Transfer to Wallet (user-initiated)
    pub fn transfer_to_wallet(&self, amount_micro_rt: i64) -> Result<()> {
        let balance = self.balance_micro_rt.load(Ordering::SeqCst);
        
        if balance < amount_micro_rt {
            anyhow::bail!(
                "Insufficient balance: have {}, requested {}", 
                balance, 
                amount_micro_rt
            );
        }
        
        self.balance_micro_rt.fetch_sub(amount_micro_rt, Ordering::SeqCst);
        
        tracing::info!(
            vault_id = %self.vault_id,
            amount = amount_micro_rt,
                "ShortVault -> Wallet transfer"
        );
        
        Ok(())
    }
    
    /// Get current balance (lock-free)
    pub fn balance(&self) -> i64 {
        self.balance_micro_rt.load(Ordering::SeqCst)
    }
}

/// ShortVault registry (all user vaults)
pub struct ShortVaultRegistry {
    vaults: dashmap::DashMap<String, ShortVault>,
}

impl ShortVaultRegistry {
    pub fn new() -> Self {
        Self {
            vaults: dashmap::DashMap::new(),
        }
    }
    
    /// Create new ShortVault for user
    pub fn create_vault(&self, user_id: String) -> Result<String> {
        let vault_id = uuid::Uuid::new_v4().to_string();
        let vault = ShortVault::new(vault_id.clone(), user_id);
        
        self.vaults.insert(vault_id.clone(), vault);
        
        Ok(vault_id)
    }
    
    /// Credit specific ShortVault
    pub async fn credit_short_vault(&self, vault_id: &str, amount_micro_rt: i64) -> Result<()> {
        let vault = self.vaults
            .get(vault_id)
            .ok_or_else(|| anyhow::anyhow!("ShortVault not found: {}", vault_id))?;
        
        vault.credit(amount_micro_rt)
    }
    
    /// Get all ShortVault IDs
    pub async fn get_all_short_vaults(&self) -> Result<Vec<String>> {
        Ok(self.vaults.iter().map(|entry| entry.key().clone()).collect())
    }
}
```

---

## 6. Data & Event Schema

### 6.1 Motivation

Shift from single-field micro representation and JTU streaming to a **triple fixed‑point model** across physics (JouleTorq), throughput (TokenTorq), and monetary aggregate (RoboTorq). This enables deterministic multi-dimensional accounting, programmable allocation constraints, and precise provenance (certificates) without moving ledger objects.

### 6.2 Core Structures

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JouleTorqUnit {
        pub unit_id: String,          // {contract}-{ts}-{nonce}
        pub contract_id: String,
        pub digger_id: String,
        pub joules: f64,              // Physics measurement
        pub tokens_generated: i64,    // Derived token count
        pub timestamp_nanos: i64,
        pub parent_ingot_id: Option<String>,
        pub hash: String,             // SHA256 over canonical fields
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StakeEvent { /* as above */ }
pub struct IssuanceEvent {
    pub issuance_event_id: String,
    pub stream_id: String,
    pub contract_id: String,
    pub jouletorq_amount: i64,
    pub tokentorq_amount: i64,
    pub robotorq_amount: i64,
    pub member_ids: Vec<String>, // membership snapshot for deterministic split
    pub per_member_robotorq: i64, // derived; may omit physics/throughput per member
    pub provenance_cert_ids: Vec<String>,
    pub tick_index: i32,
    pub timestamp_nanos: i64,
    pub signature: Vec<u8>,
}
```

### 6.3 Signature Strategy

All StakeEvents and IssuanceEvents are signed (Falcon) over a canonical JSON ordering. Individual certificates remain separately signed upstream. Per-member signatures are not required; event-level signature + deterministic split suffices.

### 6.4 Phase3 Batch Payload (Certificates Only)

Canonical `vault.phase3.completed` payload now uses explicit JTU certificates:

```jsonc
{
    "event_type": "robotorqcert_batch_completed",
    "batch": {
        "batch_id": "batch-20251121-001",
        "created_at_nanos": 1732212345678900000,
        "certificates": [
            {
                "cert_id": "cert-abc123",
                "robotorq_proof_id": "proof-xyz789",
                "merkle_root": "4f7d...",
                "contract_ids": ["contract-A", "contract-B"],
                "timestamp_nanos": 1732212345678900000,
                "hash": "c0ffeec0ffee...",
                "status": "Digital",
                "bearer_bond_id": null
            }
        ]
    }
}
```

Stake semantics: Certificates stay resident; StakeEvents move derived triple balances between reserve and contracts referencing certificate provenance. Phase3 payload need not embed backed currency numbers—Vault derives them deterministically.

### 6.5 Migration Phases

1. Phase A: Introduce triple balance schema internally; legacy micro fields still published.
2. Phase B: Dual publication (legacy + triple) for reconciliation; signatures over both.
3. Phase C: Remove legacy single-field; rely solely on triple balances & certificate provenance.
4. Phase D: Optional optimization (aggregate commitments / range proofs) for large issuance streams.

### 6.6 Required Code Changes (Outline)

- Add model files: `joule_torq_unit.rs`, `joule_torq_certificate.rs`.
- Mint → Vault: unchanged for certificates; remove JTU list injection.
- StakeVault: implement triple balances + StakeEvent signing.
- DistoVault: implement IssuanceEvent generation (signed) with deterministic membership snapshot.
- Metrics: `vault_stake_events_total`, `vault_issuance_events_total`, `vault_triple_balance_reserve{dimension=..}`.
- Events: `vault.stake.event`, `vault.issuance.event`, deprecate `vault.robostake.*`.

## 7. Concurrency Model

### 6.1 Lock-Free Operations

```rust
// All balance operations use atomic integers (no locks)

// StakeVault allocation (concurrent)
async fn allocate_many(vault: &ShadowStakeVault, contracts: Vec<String>) -> Result<()> {
    let tasks: Vec<_> = contracts.into_iter()
        .map(|contract_id| vault.allocate(contract_id, 1000_000_000))
        .collect();
    
    futures::future::try_join_all(tasks).await?;
    Ok(())
}

// DistoVault uniform drip distribution (concurrent to ALL ShortVaults per tick)
async fn run_uniform_stream(
    disto_vault: std::sync::Arc<ShadowDistoVault>,
    nats_client: async_nats::Client,
) -> Result<()> {
    // Listens for `vault.distostream.authorized` and internally runs
    // futures::future::try_join_all per tick for max concurrency.
    let default_tick_millis = 1000; // 1s ticks (can be overridden by config)
    disto_vault
        .start_authorization_listener(nats_client, default_tick_millis)
        .await
}
```

### 6.2 DashMap for Concurrent Collections

```rust
// Lock-free concurrent HashMap
use dashmap::DashMap;

// Multiple threads can read/write simultaneously
let allocations: DashMap<String, i64> = DashMap::new();

// Thread-safe insert
allocations.insert("contract-1".to_string(), 1000);

// Thread-safe read
if let Some(amount) = allocations.get("contract-1") {
    println!("Allocated: {}", *amount);
}
```

---

## 8. NATS Integration

### 7.1 Event Topics (MVP)

```
# Mint → Vault batch completed
vault.phase3.completed       → Mint publishes RoboTorqBatch (event_type=robotorqcert_batch_completed)

# RoboStake lifecycle
vault.robostake.returned     → StakeVault records RoboStake returned from Mint

# Certificates
vault.cert.stored            → CertVault stores certificate

# Disto stream authorization
vault.distostream.authorized → CertVault authorizes DistoVault to produce JTUs

# UBD distribution
vault.ubd.distributed        → ShortVault credited (per tick / per member)
```

#### 8.1.1 Canonical `vault.phase3.completed` Payload (Refactored)

```jsonc
{
    "event_type": "robotorqcert_batch_completed",
    "batch": {
        "batch_id": "batch-20251121-001",
        "created_at_nanos": 1732212345678900000,
        "joule_torq_certificates": [
            {
                "jtu_id": "jtu-aaa111",
                "parent_cert_id": "cert-abc123",
                "parent_proof_id": "proof-xyz789",
                "merkle_leaf_hash": "f1c2...",
                "created_at_nanos": 1732212345678900000,
                "status": "Circulating"
            }
        ],
        "certificates": [
            {
                "cert_id": "cert-abc123",
                "robotorq_proof_id": "proof-xyz789",
                "merkle_root": "4f7d...",
                "contract_ids": ["contract-A", "contract-B"],
                "timestamp_nanos": 1732212345678900000,
                "hash": "c0ffeec0ffee...",
                "status": "Digital",
                "bearer_bond_id": null
            }
        ]
    }
}
```

Stake Semantics (Refactored): Numeric collateral field removed. Economic value is derived from the count & identity of `joule_torq_certificates` received. Transitional dual accounting may persist until Phase C cutover.

### 7.2 NATS Client Setup

```rust
// src/nats_client.rs

use async_nats::Client;
use anyhow::Result;

pub async fn connect_nats(url: &str) -> Result<Client> {
    let client = async_nats::connect(url).await?;
    
    tracing::info!("Connected to NATS at {}", url);
    
    Ok(client)
}

pub async fn subscribe_events(client: &Client) -> Result<()> {
    let mut sub = client.subscribe("vault.>").await?;
    
    tokio::spawn(async move {
        while let Some(msg) = sub.next().await {
            tracing::debug!(
                subject = %msg.subject,
                payload_len = msg.payload.len(),
                "Received vault event"
            );
            
            // Process event (event sourcing replay)
        }
    });
    
    Ok(())
}
```

---

## 9. MVP Implementation Plan

### Phase 1: Core Shadow Vaults (Week 1)

**Goals**:
- [x] Project structure (Cargo.toml, module layout)
- [ ] ShadowStakeVault implementation
  - [ ] Allocate RoboStake
  - [ ] Return RoboStake
  - [ ] Atomic balance tracking
- [ ] NATS client integration
- [ ] Event sourcing (publish events)

**Deliverable**: StakeVault can allocate/return RoboStake

### Phase 2: Certificate Storage (Week 2)

**Goals**:
- [ ] ShadowCertVault implementation
    - [ ] Store RoboTorqCertificates
    - [ ] Implement bearer bond APIs (merkle tree + SPHINCS+)
    - [ ] Implement bearer bond redemption APIs
    - [ ] Track off-grid inventory
- [ ] Integration with Mint (receive certificates directly)

**Deliverable**: Bearer bond structures and APIs exist in CertVault, but **Phase3 handler does not call** `issue_bearer_bond` or `redeem_bearer_bond` in the MVP path.

### Phase 3: UBD Distribution (Week 3)

**Goals**:
- [ ] ShadowDistoVault implementation
    - [ ] Distribute JTUs equally to all ShortVaults
    - [ ] Concurrent distribution (futures::join_all)
- [ ] ShortVaultRegistry implementation
    - [ ] Create user ShortVaults
    - [ ] Credit ShortVaults concurrently

**Deliverable**: UBD distribution functional

### Phase 4: Integration & Testing (Week 4)

**Goals**:
- [ ] End-to-end integration test
    - [ ] Allocate RoboStake → Digger → Refinery → Mint → Vaults
    - [ ] Mint returns RoboStake to StakeVault
    - [ ] DistoVault distributes UBD to all ShortVaults
- [ ] Bearer bond test (issue → redeem)
- [ ] Prometheus metrics
- [ ] Health checks

**Deliverable**: MVP vault system complete and tested

---

## Conclusion

The **Vault System MVP** is designed for maximum concurrency using:
- ✅ Tokio async runtime (concurrent I/O)
- ✅ Atomic operations (lock-free balance tracking)
- ✅ DashMap (concurrent HashMap)
- ✅ futures::try_join_all (parallel vault operations)

**Key Features**:
1. RoboStake travels through proof chain, returns to StakeVault
2. Certificates stored permanently in CertVault
3. Bearer bonds enable physical coins (merkle + SPHINCS+)
4. UBD distributes to ALL ShortVaults equally (first landing zone)
5. Event sourcing for complete audit trail

**Implementation**: 4 weeks to MVP, Rust-native, production-ready concurrency.

---

*"Rust's fearless concurrency meets RoboTorq's physics-based economics."* 🦀⚡💰
