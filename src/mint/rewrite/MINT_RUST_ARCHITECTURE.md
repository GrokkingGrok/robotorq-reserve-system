## Mint Service – Rust Architecture (RoboTorqCertificate / RoboTorqBatch)

### 1. Scope

The Rust Mint service replaces the Go-based Phase3 Mint. It:

- Consumes `TokenTorqIngot` messages from Refinery over NATS.
- Stamps ingots into `RoboTorqCertificate`s (each represents exactly 1 RT = 3.6M JTUs; no amount field).
- Maintains a local proof ledger (hashes, merkle roots, archive units).
- Batches certificates into `RoboTorqBatch` and publishes them to Vault.
- Does **not** use `Phase3RoboTorqUnit` or "Phase 3" terminology.

### 2. Crate Layout

```text
src/mint/
	Cargo.toml
	src/
		main.rs              # Service entrypoint
		lib.rs               # Library exports
		config.rs            # MintConfig (env-driven)
		nats_client.rs       # NATS connection helpers

		models/
			mod.rs
			token_torq_ingot.rs    # Input from Refinery
			robotorq_certificate.rs# Mint-side RoboTorqCertificate
			robotorq_batch.rs      # Mint-side RoboTorqBatch
			archive_unit.rs        # Internal archival proof unit (merkle + signature)

		engine/
			mod.rs
			ingot_processor.rs     # Ingot → cert stamping logic
			batcher.rs             # Cert → batch aggregation
			ledger.rs              # Local storage of certs and archive units
			merkle.rs              # Merkle tree utilities

		handlers/
			mod.rs
			ingot_subscriber.rs    # NATS subscription for ingots
			batch_publisher.rs     # NATS publisher for RoboTorqBatch → Vault
```

### 3. Core Models

#### 3.1 `TokenTorqIngot` (input from Refinery)

Rust struct mirrors existing Go ingot model (fields not expanded here):

```rust
/// TokenTorqIngot: 3600-unit ingot produced by Refinery.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenTorqIngot {
		pub ingot_id: String,
		pub contract_id: String,
		pub joules_total: f64,
		pub robostake_total_micro_rt: i64,
		// ... additional ingot metadata / hashes as needed
}
```

#### 3.2 `RoboTorqCertificate` (Canonical Wire Shape)

Mint-side representation of a single RoboTorq certificate. Each certificate
is minted only after exactly **1000 ingots** have been accumulated. The 1000
ingots collectively represent **1 RT = 3.6M JTUs**; there is no amount field.
Certificates point back to a `RoboTorqProof` and cache its merkle root. Vault
receives the same logical shape inside `RoboTorqBatch`.

Constant:

```rust
pub const INGOTS_PER_CERT: usize = 1000;
```

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoboTorqCertificate {
	pub cert_id: String,
	/// Link to Mint-side proof (RoboTorqProof) retaining full merkle + signature
	pub robotorq_proof_id: String,
	/// Cached merkle root (duplicated from proof for Vault index/lookups)
	pub merkle_root: String,
	/// All contracts that contributed work (can be large; kept as vector)
	pub contract_ids: Vec<String>,
	/// Minting timestamp (unix nanos for precision)
	pub timestamp_nanos: i64,
	/// Deterministic SHA-256 over serialized certificate minus this field
	pub hash: String,
	pub status: CertStatus,
	pub bearer_bond_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CertStatus {
	Digital,
	OffGrid,
}
```

#### 3.3 `RoboTorqBatch` (Canonical Wire Shape)

Batch of certificates published to Vault. Each certificate inside has already
encapsulated 1000 ingots. The batch itself is purely a communication wrapper
plus aggregated stake.

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoboTorqBatch {
	pub batch_id: String,
	/// Creation timestamp (unix nanos)
	pub created_at_nanos: i64,
	pub certificates: Vec<RoboTorqCertificate>,
}
```

#### 3.4 `RoboTorqProof` (Internal Only – Not Sent to Vault)

Internal archival record. Certificates point to this via `robotorq_proof_id`.

IMPORTANT: Proofs deliberately contain **no RoboStake amount**. All RoboStake
arriving with ingots is stripped during ingestion and only aggregated for the
current batch’s top-level `robostake_micro_rt` field so it can be returned to
the Vault. Economics are derived by the Vault from certificate count; Mint
does not persist stake inside proofs or certificates.

Two-Level Merkle Structure:
1. Level 1 (Certificate Merkle): Each certificate computes its own merkle
	root from the 1000 ingot hashes it encapsulates.
2. Level 2 (Proof Merkle): A `RoboTorqProof` merkle root is built from the
	hashes of the certificates minted within the proof interval.

This layering enables granular audit (ingot → certificate) and aggregated
integrity (certificate → proof) without duplicating stake data.

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoboTorqProof {
    pub proof_id: String,
    pub merkle_root: String,
    pub tree_height: u32,
    pub contract_ids: Vec<String>,
    pub minted_at_nanos: i64,
    pub signature: Vec<u8>,      // SPHINCS+ signature
    pub public_key: Vec<u8>,     // Mint's SPHINCS+ public key
}
```

### 4. NATS Contracts (Mint Side)

- **Ingest**: `refinery.ingot.produced`
	- Payload: `TokenTorqIngot` JSON.

- **Emit to Vault**: `vault.phase3.completed`
		- Canonical payload (wire contract):

				```jsonc
				{
					"event_type": "robotorqcert_batch_completed",
					"robostake_micro_rt": 1000000000,
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

There is no `Phase3RoboTorqUnit` or "Phase 3" terminology in the Rust Mint; it
deals strictly with ingots, certificates, batches, and proofs.

### 5. Stake & Ingot Accumulation (Canonical Flow)

1. Refinery publishes `TokenTorqIngot` including `robostake_total_micro_rt`.
2. Mint ingot subscriber strips stake and adds it to the batch stake accumulator.
3. Ingots are buffered; once `INGOTS_PER_CERT == 1000` ingots collected:
	- Build per-certificate merkle root from those 1000 ingot hashes.
	- Create a single `RoboTorqCertificate` (no stake field).
	- Clear ingot buffer for next certificate.
4. Certificates generated during a proof interval contribute their hashes to the proof merkle structure.
5. When batch threshold (number of certificates or time) reached:
	- Compute `robostake_micro_rt = sum(stripped_ingot_stake)`.
	- Publish `vault.phase3.completed` with batch (certs) + aggregated stake.
	- Reset stake accumulator for next batch.
6. Vault stores certificates, returns stake to StakeVault using top-level `robostake_micro_rt`.

Rationale: Removing stake from proofs prevents ambiguity or duplication of economic state; Vault remains single source for value derivation.

