## Mint Rust Rewrite – Plan (Ingot → Cert → Batch)

### 1. Goals

- Remove all `Phase3` / `RoboTorqUnit` terminology from Mint.
- Implement a Rust Mint that:
	- Consumes `TokenTorqIngot` from Refinery over NATS.
	- Stamps ingots into `RoboTorqCertificate`s.
	- Maintains an internal `RoboTorqProof` ledger (merkle + SPHINCS+).
	- Batches certificates into `RoboTorqBatch` and publishes them to Vault.
- Keep NATS contracts consistent with the Vault MVP docs
	(`robotorqcert_batch_completed` + `RoboTorqBatch`).

### 2. Phased Rewrite Plan

#### Phase 1 – Scaffold Rust Mint Crate

1. Create `src/mint/Cargo.toml` with:
	 - `tokio`, `async-nats`, `serde`, `serde_json`, `anyhow`, `uuid`, `sha2`, `oqs`, `tracing`.
2. Create crate layout described in `MINT_RUST_ARCHITECTURE.md`:
	 - `config.rs`, `nats_client.rs`.
	 - `models/` (`token_torq_ingot.rs`, `robotorq_certificate.rs`, `robotorq_batch.rs`, `robotorq_proof.rs`).
	 - `engine/` (`ingot_processor.rs`, `batcher.rs`, `ledger.rs`, `merkle.rs`).
	 - `handlers/` (`ingot_subscriber.rs`, `batch_publisher.rs`).
3. Wire a minimal `main.rs` that:
	 - Loads `MintConfig` from env.
	 - Connects to NATS.
	 - Starts the ingot subscriber task.

#### Phase 2 – Ingot Buffer → Certificate (1000 Ingots per Cert)

4. Implement `TokenTorqIngot` model (ported from Go) in Rust.
5. Implement `RoboTorqCertificate` + `CertStatus` exactly as in the architecture doc.
6. In `engine::ingot_processor`:
	 - Maintain `Vec<TokenTorqIngot>` buffer.
	 - On ingest, push ingot + strip stake to batch stake accumulator.
	 - When buffer length == `INGOTS_PER_CERT` (1000):
		 * Compute per-certificate merkle root over ingot hashes.
		 * Produce one `RoboTorqCertificate`.
		 * Clear buffer.
7. Implement a simple in-memory `Ledger`:
	 - `store_certificate(cert: RoboTorqCertificate)`.
	 - `pending_certificates()` for batcher.

#### Phase 3 – Two-Level Merkle (Ingots→Cert, Certs→Proof)

8. Implement `engine::merkle` utilities:
	 - `fn build_merkle_root(hashes: &[String]) -> String` (used twice).
9. Certificate merkle (Level 1): ingot hashes → certificate merkle root.
10. Proof merkle (Level 2): certificate hashes → proof merkle root.
11. Implement `RoboTorqProof` creation (NO stake field):
	 - Accumulate certificate hashes until proof interval condition (time/count).
	 - Build Level 2 merkle root, sign root, persist proof.
12. Extend `Ledger` to store proofs.

#### Phase 4 – Batching and Publishing to Vault

11. Implement `RoboTorqBatch` in `models::robotorq_batch` as in architecture doc.
13. Implement `engine::batcher`:
		- Maintain stake accumulator (already filled during ingestion).
		- `fn build_batch(certs: &[RoboTorqCertificate], stake_accum: i64) -> RoboTorqBatch`.
		- Return stake separately for publisher (`robostake_micro_rt = stake_accum`).
		- Reset accumulator post-publish.
		- Batching rule: fixed cert count OR time window.
13. Implement `handlers::batch_publisher`:
		- Periodically (or on threshold), pull certs from `Ledger`.
		- Build a `RoboTorqBatch`.
		- Publish to NATS on `vault.phase3.completed` with payload:

			```jsonc
			{
				"event_type": "robotorqcert_batch_completed",
				"robostake_micro_rt": <sum>,
				"batch": { /* RoboTorqBatch */ }
			}
			```

14. Ensure there is **no** `Phase3RoboTorqUnit` or similar naming in this crate;
		any historical references live only in comments or docs.

#### Phase 5 – Testing & Parity Checks

16. Add unit tests:
		- Ingot buffer accumulation: no certificate before 1000 ingots.
		- Certificate merkle root deterministic over same ingot set.
		- Proof merkle root deterministic over certificate hashes.
		- Stake accumulator equals published batch stake; certificates/proofs contain no stake field.
		- Hash stability (changing order changes merkle, same order same merkle).
16. Add integration test:
		- Simulate N ingots coming from Refinery.
		- Verify that certificates are created, archived, batched, and that a
			correctly-shaped `robotorqcert_batch_completed` event is published.
18. Compare behavior against legacy Mint (conceptual parity):
		- 1000 ingots → 1 certificate mapping validated.
		- Equivalent total RT produced for same ingot volume.
		- Stake only at batch top-level.
		- Merkle integrity across two levels holds.

### 3. Naming Rules

- Code in this crate MUST NOT introduce new `Phase3*` or `*RoboTorqUnit`
	identifiers.
- Public surface uses:
	- `TokenTorqIngot` (input).
	- `RoboTorqCertificate` (economic record).
	- `RoboTorqBatch` (Mint → Vault message).
	- `RoboTorqProof` (internal archival proof record; no stake field).

	### 4. Stake & Ingot Accumulation Summary

	- Each ingot carries `robostake_total_micro_rt`.
	- Mint strips stake → accumulates toward batch stake.
	- Ingots buffered until `INGOTS_PER_CERT = 1000` → one certificate.
	- Certificates hashed into proof merkle (no stake stored).
	- Batch publish includes only aggregated `robostake_micro_rt`.
	- Vault returns stake; economics from certificate count (1 cert = 1000 ingots = 1 RT).

