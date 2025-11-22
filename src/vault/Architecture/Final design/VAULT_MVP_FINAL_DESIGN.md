# Vault System MVP – Final Design

**Version**: 1.1 (MVP – NATS, Single Node)
**Date**: November 21, 2025  
**Status**: Design Frozen for Implementation  
**Language**: Rust (tokio async runtime)  

---

## 1. MVP Scope (Tight)

This MVP defines **one end-to-end capability**:

> On Phase3 completion from Mint, the vault service returns RoboStake to the StakeVault, stores certificates in CertVault, and causes DistoVault to generate and sign JTUs and stream them as UBD into all ShortVaults, on a simple, configurable uniform schedule.

Out of scope for this MVP:
- Trust service implementation (only implied as a conceptual allocator)
- Any StakeVault-originated payments or slashing (StakeVault is **receive-only** here)
- Bearer bond issuance/redemption logic (only data structures/config hooks preserved)
- Demurrage rerouting, crisis mode, multi-node consensus
- Per-contract custom curves beyond a single **uniform schedule** parameter

---

## 2. Mint → Vault Interface (NATS)

### 2.1 Event Subject

Mint emits a single event on Phase3 completion:

- Subject: `vault.phase3.completed`

### 2.2 Event Payload

```jsonc
{
  "event_type": "robotorqcert_batch_completed",
  "robostake_micro_rt": 1000000000,

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
- Mint sends a single `RoboTorqBatch` containing N `RoboTorqCert`s. Vault must extract all certificates from `batch.certificates` and process them.
- **Critically**: `total_jtu_micro_rt` is **not** in this payload. It is derived inside the vault service from the certificates already stored in CertVault:
  - Economically, each fully-backed RoboTorqCertificate represents **1 RT = 3.6 million JTUs** of value.
  - CertVault therefore computes `total_jtu_micro_rt` by summing the JTU value of all fully-backed RoboTorqCertificates for this contract (e.g., `cert_count × 3_600_000`, or equivalently the joules in the RT units mapped into JTUs).
- `duration_seconds` is derived from vault config (`VAULT_DISTOSTREAM_DEFAULT_SECONDS`), and `jtu_per_second` is computed as `total_jtu_micro_rt / duration_seconds` inside the vault.

---

## 3. Core Components in MVP

These components live in the Rust `robotorq-vault` crate and are wired together by a Phase3 handler.

### 3.1 ShadowStakeVault (receive-only in MVP)

Responsibilities in MVP:
- Maintain `available_micro_rt` and `deployed_micro_rt` atomically.
- On `vault.phase3.completed`, **return** RoboStake via `return_allocation(contract_id, robostake_micro_rt)`.
- Emit `vault.robostake.returned` events for observability.

Not in MVP:
- No outbound payments from StakeVault.
- No slashing logic.

### 3.2 ShadowCertVault (backbone for certs & authorization)

Responsibilities in MVP:
- Store all `RoboTorqCertificate`s extracted from each `RoboTorqBatch` received from Mint.
- Emit `vault.cert.stored` per certificate.
- **Authorize DistoVault** to produce JTUs for a specific contract based on how many fully-backed RoboTorqCertificates exist for that contract (and their energy content), **without** recomputing Trust's allocation or torq math.

Authorization semantics:

1. For each `vault.phase3.completed` event, CertVault:
  - Reads the `RoboTorqBatch` from the payload.
  - Records all certificates from `batch.certificates` under their `cert_id`.
   - Computes `total_jtu_micro_rt` **from the certificates themselves**, treating each fully-backed RoboTorqCertificate as 1 RT = 3.6M JTUs (or by summing the joules in the corresponding RT units and converting to JTUs).
   - Derives a **uniform schedule**:
     - `duration_seconds` = from payload or default (config, default `60`).
     - `jtu_per_second = total_jtu_micro_rt / duration_seconds`.
2. CertVault then emits an **authorization event**:

   - Subject: `vault.distostream.authorized`

   - Payload:

   ```jsonc
   {
     "event_type": "distostream_authorized",
     "contract_id": "printer-coin-42",
     "total_jtu_micro_rt": 5400000000,
     "duration_seconds": 60,
     "jtu_per_second": 90000000,
     "schedule_type": "uniform",
     "starts_at": 1732224000000000000,   // optional; default = now
     "cert_ids": ["cert-001", "cert-002", "..."]
   }
   ```

In MVP, `schedule_type` is always `"uniform"` and the only parameters that matter are `total_jtu_micro_rt`, `duration_seconds`, and `starts_at`.

Bearer bonds:
- Structs like `BearerBondCert`, `BondStatus`, and `BearerBondKey` remain in the CertVault model for **future work only**.
- No bearer-bond-related methods (`issue_bearer_bond`, `redeem_bearer_bond`) are called from the Phase3 handler in this MVP.

### 3.3 ShadowDistoVault (UBD stream executor)

Responsibilities in MVP:
- Subscribe to `vault.distostream.authorized`.
- For each authorized schedule, create a short-lived in-memory stream record, then drip JTUs over time.
- Credit all ShortVaults equally at each tick.

Data model sketch:

```rust
struct UniformDistoStream {
    contract_id: String,
    total_jtu_micro_rt: i64,
    duration_seconds: i64,
    jtu_per_second: i64,
    starts_at_nanos: i64,
    produced_so_far: i64,
}
```

Execution model:

- On `distostream_authorized`:
  - Create `UniformDistoStream` and spawn a `tokio` task.
  - Task loop:
    - Every second (or smaller step), until `produced_so_far >= total_jtu_micro_rt`:
      - Compute `step_amount = min(jtu_per_second, total_jtu_micro_rt - produced_so_far)`.
      - Get current list of ShortVault IDs from `ShortVaultRegistry`.
      - Divide `step_amount` equally across members.
      - Call `credit_short_vault` for each using `try_join_all`.
      - Emit a `vault.ubd.distributed` event summarizing the step.
      - Update `produced_so_far += step_amount`.

Important: DistoVault **does not talk to CertVault or StakeVault directly** in MVP; it only reacts to:
- `vault.distostream.authorized` from CertVault
- The current ShortVault membership from `ShortVaultRegistry`.

### 3.4 ShortVault & ShortVaultRegistry

Responsibilities in MVP:
- `ShortVault`: atomic `balance_micro_rt`, `credit`, and `transfer_to_wallet`.
- `ShortVaultRegistry`:
  - Manage all user ShortVaults.
  - Provide `get_all_short_vaults()` for DistoVault.
  - Provide `credit_short_vault()` invoked by DistoVault tasks.

---

## 4. MVP Event Flow (Step-by-Step)

1. **Phase3 completion (Mint → Vault)**
  - Mint publishes `vault.phase3.completed` with:
  - `robostake_micro_rt` and a `RoboTorqBatch` containing N `RoboTorqCert`s.

2. **StakeVault: Return RoboStake**
   - Vault service handler calls:
     - `stake_vault.return_allocation(contract_id, robostake_micro_rt)`.
   - `ShadowStakeVault`:
     - Adds `robostake_micro_rt` back to `available_micro_rt`.
     - Decrements `deployed_micro_rt`.
     - Emits `vault.robostake.returned`.

3. **CertVault: Store Certificates & Authorize Disto**
     - For each certificate in `batch.certificates`:
       - `cert_vault.store_certificate(cert)`.
     - CertVault computes:
       - `total_jtu_micro_rt` from the certificates themselves, treating each fully-backed RoboTorqCertificate as 1 RT = 3.6M JTUs (or equivalently by summing their joules and mapping to JTUs).
       - `duration_seconds = payload.duration_seconds.unwrap_or(config.default_distostream_seconds)` (default `60`).
       - `jtu_per_second = total_jtu_micro_rt / duration_seconds`.
     - CertVault publishes `vault.distostream.authorized` with:
       - `contract_id`, `total_jtu_micro_rt`, `duration_seconds`, `jtu_per_second`, `schedule_type="uniform"`, `starts_at`, `cert_ids`.

4. **DistoVault: Execute Uniform UBD Stream**
   - On `vault.distostream.authorized`, DistoVault:
     - Creates `UniformDistoStream` in memory.
     - Spawns a `tokio::spawn` task that:
       - Each second, until done:
         - Computes `step_amount`.
         - Reads current ShortVault IDs.
         - Splits `step_amount` equally.
         - Credits each ShortVault.
         - Emits `vault.ubd.distributed` with:

           ```jsonc
           {
             "event_type": "ubd_tick",
             "contract_id": "printer-coin-42",
             "step_amount_micro_rt": 90000000,
             "member_count": 1000,
             "per_member_micro_rt": 90000,
             "timestamp": 1732224001000000000
           }
           ```

5. **ShortVaults: Receive Value**
- Each `ShortVault` simply updates `balance_micro_rt` atomically on each credit.
- Wallets can later transfer from ShortVault to Wallet at user request (MVP API defined but front-end may come later).

---

## 5. Configuration Knobs (MVP)

These knobs should be easy to configure via `config.rs` / env vars:

- `VAULT_DISTOSTREAM_DEFAULT_SECONDS` (int, default `60`)
- `VAULT_DISTOSTREAM_TICK_MILLIS` (int, default `1000` ms)
- `VAULT_NATS_URL` (string)

CertVault and DistoVault must read `VAULT_DISTOSTREAM_DEFAULT_SECONDS` so the schedule can be changed without code changes.

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
