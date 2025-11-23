# Vault System MVP – Final Design

**Version**: 1.1 (MVP – NATS, Single Node)
**Date**: November 21, 2025  
**Status**: Design Frozen for Implementation  
**Language**: Rust (tokio async runtime)  

---

## 1. MVP Scope (Tight)

This MVP defines **one end-to-end capability**:

> On Phase3 completion from Mint, the vault service stores newly minted RoboTorq certificates in CertVault (economic value returns ONLY as certificates), derives hierarchical backed currency totals (RoboTorq, TokenTorq remainder, JouleTorq remainder), authorizes a uniform issuance schedule, and causes DistoVault to generate and sign hierarchical IssuanceEvents streaming value equally to all ShortVaults.

Out of scope for this MVP:
- Trust service implementation (only implied as a conceptual allocator)
- Any reverse StakeVault return events (economic "return" enters via Mint certificate batches)
- Bearer bond issuance/redemption logic (only data structures/config hooks preserved)
- Demurrage rerouting, crisis mode, multi-node consensus
- Per-contract custom curves beyond a single **uniform schedule** parameter

---

## 2. Mint → Vault Interface (NATS)

### 2.1 Event Subject

Mint emits a single event on Phase3 completion:

- Subject: `vault.phase3.completed`

### 2.2 Event Payload (Phase3 Certificate Batch)

```jsonc
{
  "event_type": "robotorqcert_batch_completed",
  // NOTE: Legacy numeric robostake field removed; value is re-derived from certificates as whole RoboTorq units (R) with zero remainders at ingress.

  "batch": {
    "batch_id": "batch-0001",
    "created_at": 1732224000000000000,
    "certificates": [
      {
        "unit_id": "RT-20251121-123456.000000",
        "merkle_root": "abcdef1234...64chars...",
        "tree_height": 10,
        "robo_stake_total": 1000000.0,
        "contract_ids": ["printer-coin-42"],
        "merkle_proof_api": "/verify/proof/RT-20251121-123456.000000",
        "minted_at": 1732224000000000000
      }
      // ... N many RoboTorqCerts in the batch
    ]
  }
}
```

Notes:
- Mint sends a single `RoboTorqBatch` containing N `RoboTorqCertificate`s. Vault extracts all certificates and stores them in CertVault.
- Hierarchical representation: `robotorq` (whole certificates), `tokentorq_remainder` (<1000 ingots), `jouletorq_remainder` (<3600 ore units). At ingress, only whole RoboTorq increments; remainders are zero.
- Total hierarchical value: `total_robotorq = cert_count`; remainders zero. Canonical comparison / signing form: `total_jouletorq = total_robotorq * 3_600_000 + tokentorq_remainder * 3_600 + jouletorq_remainder`.
- `duration_seconds` derives from config (`VAULT_DISTOSTREAM_DEFAULT_SECONDS`). Per-tick issuance triple is computed deterministically by dividing remaining RoboTorq over ticks; fractional final tick becomes remainder fields.

---

## 3. Core Components in MVP

These components live in the Rust `robotorq-vault` crate and are wired together by a Phase3 handler.

### 3.1 ShadowStakeVault (allocation-only – robotic labor outbound)

Responsibilities in MVP:
- Maintain hierarchical reserve balances: `available_robotorq`, `available_tokentorq_remainder`, `available_jouletorq_remainder` atomically.
- (Optional stub this sprint) Produce signed `StakeEvent` allocations (Reserve → Contract) for approved robotic labor contracts.
- Does NOT process return flows; economic returns enter only as Mint Phase3 certificate batches into CertVault.

Not in MVP:
- No slashing logic.
- No Contract → Reserve StakeEvents.

### 3.2 ShadowCertVault (backbone for certificates & authorization)

Responsibilities in MVP:
- Store all `RoboTorqCertificate`s extracted from each `RoboTorqBatch` received from Mint.
- Emit `vault.cert.stored` per certificate.
- **Authorize DistoVault** to produce JTUs for a specific contract based on how many fully-backed RoboTorqCertificates exist for that contract (and their energy content), **without** recomputing Trust's allocation or torq math.

Authorization semantics:

1. For each `vault.phase3.completed` event, CertVault:
  - Reads the `RoboTorqBatch` from the payload.
  - Records all certificates from `batch.certificates` under their `cert_id`.
   - Computes hierarchical totals: `total_robotorq = cert_count` (remainders zero at authorization).
   - Converts to canonical jouletorq: `canonical_total_jouletorq = total_robotorq * 3_600_000`.
   - Derives a **uniform schedule**:
     - `duration_seconds` = config default (`60`) unless overridden.
     - Per-tick planned issuance derived by dividing remaining RoboTorq over remaining ticks; final fractional tick expressed via remainder fields.
2. CertVault then emits an **authorization event**:

   - Subject: `vault.distostream.authorized`

   - Payload:

   ```jsonc
   {
     "event_type": "distostream_authorized",
     "contract_id": "printer-coin-42",
     "robotorq_total": 1500,
     "tokentorq_remainder": 0,
     "jouletorq_remainder": 0,
     "duration_seconds": 60,
     "schedule_type": "uniform",
     "starts_at_nanos": 1732224000000000000,
     "provenance_cert_ids": ["cert-001", "cert-002", "..."]
   }
   ```

In MVP, `schedule_type` is always `"uniform"` and the only parameters that matter are `robotorq_total`, `duration_seconds`, and `starts_at_nanos` (remainders zero at authorization).

Bearer bonds:
- Structs like `BearerBondCert`, `BondStatus`, and `BearerBondKey` remain in the CertVault model for **future work only**.
- No bearer-bond-related methods (`issue_bearer_bond`, `redeem_bearer_bond`) are called from the Phase3 handler in this MVP.

### 3.3 ShadowDistoVault (UBD stream executor – hierarchical IssuanceEvents)

Responsibilities in MVP:
- Subscribe to `vault.distostream.authorized`.
- For each authorized schedule, create a short-lived in-memory stream record, then drip JTUs over time.
- Credit all ShortVaults equally at each tick.

Data model sketch:

```rust
struct UniformDistoStream {
  contract_id: String,
  robotorq_total: i64,
  tokentorq_remainder: i64,
  jouletorq_remainder: i64,
  duration_seconds: i64,
  tick_millis: i64,
  issued_robotorq_so_far: i64,
  issued_tokentorq_remainder: i64,
  issued_jouletorq_remainder: i64,
  starts_at_nanos: i64,
}
```

Execution model:

- On `distostream_authorized`:
  - Create `UniformDistoStream` and spawn a `tokio` task.
  - Task loop (per tick):
    - Compute per-tick hierarchical issuance; if <1 RoboTorq remains, issue fractional remainder via (T,J).
    - Snapshot ShortVault membership.
    - Split canonical jouletorq equally; reconstruct (R,T,J) per member via normalization.
    - Credit each ShortVault (currently flatten to micro representation internally; future upgrade stores full triple).
    - Emit `vault.ubd.distributed` summarizing the hierarchical tick.
    - Update issued counters; stop when all (R,T,J) issued.

Important: DistoVault **does not talk to CertVault or StakeVault directly** in MVP; it only reacts to:
- `vault.distostream.authorized` from CertVault
- The current ShortVault membership from `ShortVaultRegistry`.

### 3.4 ShortVault & ShortVaultRegistry

Responsibilities in MVP:
- `ShortVault`: atomic `balance_canonical_jouletorq` (flattened canonical jouletorq), `credit`, and `transfer_to_wallet`.
- `ShortVaultRegistry`:
  - Manage all user ShortVaults.
  - Provide `get_all_short_vaults()` for DistoVault.
  - Provide `credit_short_vault()` invoked by DistoVault tasks.

---

## 4. MVP Event Flow (Step-by-Step)

1. **Phase3 completion (Mint → Vault)**
  - Mint publishes `vault.phase3.completed` with:
  - A `RoboTorqBatch` containing N `RoboTorqCertificate`s (numeric robostake field removed; value implicit in certificate count).

2. **StakeVault: Return RoboStake**
   - Vault service handler calls:
    - (If allocation logic present) Allocation to labor uses hierarchical StakeEvents; economic return appears only as certificates (no `return_allocation` numeric call in MVP).
   - `ShadowStakeVault`:
    - Newly minted certificates increase `available_robotorq` at CertVault; StakeVault numeric micro fields are deprecated.
     - Emits `vault.robostake.returned`.

3. **CertVault: Store Certificates & Authorize Disto**
     - For each certificate in `batch.certificates`:
       - `cert_vault.store_certificate(cert)`.
     - CertVault computes:
      - Canonical total jouletorq from certificates: `canonical_total_jouletorq = cert_count * 3_600_000`.
       - `duration_seconds = payload.duration_seconds.unwrap_or(config.default_distostream_seconds)` (default `60`).
      - Per-tick canonical jouletorq: `canonical_jouletorq_per_second = canonical_total_jouletorq / duration_seconds`.
     - CertVault publishes `vault.distostream.authorized` with:
      - `contract_id`, `robotorq_total`, `tokentorq_remainder`, `jouletorq_remainder`, `duration_seconds`, `schedule_type="uniform"`, `starts_at_nanos`, `provenance_cert_ids`.

4. **DistoVault: Execute Uniform UBD Stream**
   - On `vault.distostream.authorized`, DistoVault:
     - Creates `UniformDistoStream` in memory.
     - Spawns a `tokio::spawn` task that each tick emits a hierarchical IssuanceEvent (R,T,J) split equally across members.
     - Example tick event:

       ```jsonc
       {
         "event_type": "ubd_tick",
         "contract_id": "printer-coin-42",
         "stream_id": "stream-123",
         "robotorq_issued": 10,
         "tokentorq_remainder_issued": 0,
         "jouletorq_remainder_issued": 0,
         "member_count": 1000,
         "per_member_canonical_jouletorq": 3600,
         "timestamp_nanos": 1732224001000000000
       }
       ```

5. **ShortVaults: Receive Value**
- Each `ShortVault` updates `balance_canonical_jouletorq` atomically per tick (flattened canonical jouletorq). Future upgrade stores (R,T,J) explicitly.
- Wallets can later transfer from ShortVault to Wallet at user request (unchanged semantics).

---

## 5. Configuration Knobs (MVP)

These knobs should be easy to configure via `config.rs` / env vars:

- `VAULT_DISTOSTREAM_DEFAULT_SECONDS` (int, default `60`)
- `VAULT_DISTOSTREAM_TICK_MILLIS` (int, default `1000` ms)
- `VAULT_NATS_URL` (string)

CertVault and DistoVault must read `VAULT_DISTOSTREAM_DEFAULT_SECONDS` so the schedule can be changed without code changes. Remainder normalization (enforcing 0 ≤ T < 1000; 0 ≤ J < 3600) occurs after each issuance tick.

---

## 6. Bearer Bonds – Future Only (Not in MVP)

For future phases only:
- Keep `BearerBondCert`, `BondStatus`, `BearerBondKey` in the CertVault model.
- Do **not** call `issue_bearer_bond` or `redeem_bearer_bond` in the Phase3 handler.
- Do **not** emit `vault.bond.issued` or `vault.bond.redeemed` in MVP.

This preserves forward-compatibility for physical coins without adding complexity to the first implementation.

---

## 7. Implementation Notes

- The existing `VAULT_MVP_ARCHITECTURE.md` describes the Rust module layout and example code for `ShadowStakeVault`, `ShadowCertVault`, `ShadowDistoVault` and `ShortVaultRegistry`. This **final design document** constrains behavior and NATS contracts for the MVP.
- When in doubt, the flow in **Section 4** is the source of truth for MVP behavior.
