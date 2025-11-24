 # Rust Trust — MVP Plan

 ## Overview

 This document defines a minimal, safe MVP for a Rust-based `trust` service that implements three required behaviors:

 1) Load a special Genesis Contract on startup that does NOT require Robostake.
 2) The Genesis Contract lists the user's 3D printer as the authorized robot for that contract.
 3) Respond to Digger requests by sending the requested contract (including the Genesis Contract) to the Digger.

 The goal is a small, auditable, well-tested service that can operate in parallel with the existing Trust implementation for migration testing.

 ## Goals & Acceptance Criteria (MVP)

- **Genesis contract present**: On service start the configured genesis contract is loaded into the contract store and accessible via its `contract_id`.
- **No-RoboStake contract**: The genesis contract has `robostake_required: false` and can be accepted by diggers without stake.
- **Robot listed**: The genesis contract contains a `robot_id` referencing the user's 3D printer (example: `3d-printer-jon`); the field is present and discoverable.
- **Request/Reply contract API**: The trust service replies to contract requests from Diggers over NATS (request/reply or a `trust.contract.request` subject) with the full contract JSON.
- **Basic validation**: The contract schema is validated on load and on publish to digger.
- **Observability**: Exposes `/metrics` and `/health` endpoints for Prometheus and basic uptime checks.

## High-Level Architecture

Components:

- `main.rs` — bootstrap, config, metrics, and task orchestration.
- `config.rs` — environment-driven `TrustConfig::from_env()` with defaults for genesis path, NATS URL, subjects, etc.
- `models/contract.rs` — canonical `Contract` data model and JSON schema/serde definitions.
- `store/contract_store.rs` — in-memory store with optional on-disk persistence for contracts.
- `handlers/nats_handler.rs` — NATS subscription handler for incoming contract requests and RPC replies.
- `http/health_metrics.rs` — HTTP server exposing `/metrics` and `/health`.

## Recommended Project Layout

Create a simple, conventional Rust crate layout for the trust service. Example:

```
trust/
├── Cargo.toml
├── Dockerfile
├── README.md
├── config/
│   └── genesis_contract.json
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config.rs
   │   ├── metrics.rs
│   ├── models/
│   │   └── contract.rs
│   ├── store/
│   │   └── contract_store.rs
│   ├── handlers/
│   │   ├── nats_handler.rs
│   │   └── admin.rs
│   ├── http/
│   │   └── health_metrics.rs
│   └── tests/
│       └── integration.rs
└── tests/
  └── fixtures/
    └── genesis_contract.json
```

This layout mirrors other Rust services in the repo and keeps code modular and testable.

Message flow (MVP):

1. On startup, `contract_store` loads `GENESIS_CONTRACT` from configured file (or embedded asset).
2. `nats_handler` subscribes to `trust.contract.request` subject (or uses NATS request/reply).
3. When a Digger publishes a request for `contract_id`, `nats_handler` looks up the contract and replies with serialized `Contract` JSON.
4. All contract payloads are validated for schema and invariants (for Genesis, `robostake_required == false`).

## Contract Data Model (canonical MVP)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub contract_id: String,       // canonical unique id
    pub version: u32,
    pub title: String,
    pub description: Option<String>,
  pub robot_id: String,          // e.g. "3d-printer-jon"
  pub owner_name: Option<String>,// human owner name (e.g. "Jonathan Joseph Clark")
  pub owner_wallet_id: Option<String>, // owner's wallet id (populate when available)
    pub robostake_required: bool,  // MUST be false for the Genesis contract
    pub payload: serde_json::Value,// contract body (DSL or JSON args)
    pub created_at: DateTime<Utc>,
    pub signature: Option<String>, // optional signing for future
}
```

Sample Genesis contract JSON (example—replace `robot_id` with your 3D printer identifier):

```json
{
  "contract_id": "genesis-0001",
  "version": 1,
  "title": "Genesis: 3D Printer Contract",
  "description": "Initial genesis contract allowing 3D printer to act without robostake.",
  "robot_id": "3d-printer-jon",
  "owner_name": "Jonathan Joseph Clark",
  "owner_wallet_id": null,
  "robostake_required": false,
  "payload": { "actions": ["print-object"], "parameters": {} },
  "created_at": "2025-11-24T00:00:00Z",
  "signature": null
}
```

## NATS Subjects / API

- `trust.contract.request` — subject. Request body: `{ "contract_id": "..." }` and allow NATS reply semantics. The service replies with the contract JSON or an error structure `{ "error": "not_found" }`.
- `trust.contract.publish` — (optional) admin subject to insert/update contracts in store (authenticated in later phases).

Request/Reply example (NATS request semantics):

Requester sends to `trust.contract.request` with payload `{ "contract_id": "genesis-0001" }` and expects reply containing the `Contract` JSON.

## Security, Signing & Backwards Compatibility

- MVP focuses on functionality; signing is optional. If `TRUST_ENABLE_SIGNING=true`, the service will sign contracts using configured key and populate `signature`.
- Genesis contract with `robostake_required: false` is intentionally permissive — ensure operational controls (network, ACLs) keep it from unwanted use in production.
- For migration, use additional flags or namespaced subjects (e.g., `trust.contract.request.rust`) during parallel testing.

## Persistence & Durability

- MVP uses an in-memory store with optional on-disk JSON file persistence. `TRUST_CONTRACT_STORE_PATH` can be configured to write a simple file snapshot for restart recovery.

## Tests & Validation

- Unit tests:
  - Validate `Contract` (schema, required fields).
  - `contract_store::load_genesis()` loads file and validates invariants.
  - `nats_handler` lookup + reply logic (use an in-memory NATS client/mock).

- Integration tests (local):
  - Start a test NATS server (or use docker-compose), run `rust-trust`, send a request, assert reply contains the genesis contract and `robostake_required == false`.

## Implementation estimate (rough)

- Design & docs: 0.5 day (this doc)
- Config + models + store: 1 day
- NATS handler & request/reply flow: 1 day
- HTTP metrics/health + metrics: 0.5 day
- Tests (unit + simple integration): 1 day
- Buffer/edge-case polish & review: 0.5 day

Estimated total: ~4–5 workdays for a single engineer to deliver a tested MVP (parallelizable).

## Next steps (planned, but not executed)

1. Author `src/trust/src/config.rs` implementing `TrustConfig::from_env()` and safe defaults.
2. Add `TRUST_OWNER_NAME` / `TRUST_OWNER_WALLET_ID` env support and wire owner fields into loaded Genesis contract (populate when wallet exists).
3. Implement `models::Contract` and `store::ContractStore` with file-backed snapshot.
2. Implement `models::Contract` and `store::ContractStore` with file-backed snapshot.
3. Implement `handlers::NatsHandler` using the `async-nats` client and request/reply semantics.
4. Add tests and a docker-compose recipe for local integration testing.

---

Created as the MVP specification for a focused `rust-trust` implementation that loads a non-robostake Genesis Contract, lists the user's 3D printer as the robot, and serves contracts to Diggers on request.
