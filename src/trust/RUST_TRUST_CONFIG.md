 # Rust Trust — Config & Contract Schema

 This file lists environment variables, subject names, metric names and the minimal contract JSON schema for the `rust-trust` MVP described in `RUST_TRUST_MVP_PLAN.md`.

 ## Environment variables (recommended)

 - `TRUST_NATS_URL` (string)
   - Default: `nats://nats:4222`
   - NATS server used for subscriptions and replies.

 - `TRUST_CONTRACT_STORE_PATH` (string)
   - Default: empty (in-memory only)
   - File path to persist contract snapshots (optional).

 - `TRUST_GENESIS_CONTRACT_PATH` (string)
   - Default: `./config/genesis_contract.json`
   - Path to the JSON file loaded at startup as the Genesis Contract.

 - `TRUST_GENESIS_CONTRACT_ID` (string)
   - Default: `genesis-0001`
   - The `contract_id` used for the genesis contract; useful to override per-deployment.

 - `TRUST_ENABLE_SIGNING` (bool)
   - Default: `false`
   - If `true`, the service will sign contracts when publishing replies.

 - `TRUST_SIGNING_KEY_PATH` (string)
   - Default: empty
   - Private key path for signing.

 - `TRUST_METRICS_HOST` (string)
   - Default: `0.0.0.0`

 - `TRUST_METRICS_PORT` (u16)
   - Default: `8082`

 - `TRUST_LOG_LEVEL` (string)
   - Default: `info`

 ## NATS Subjects (MVP)

 - `trust.contract.request` — request/lookup for a contract by `contract_id`. Request payload: `{ "contract_id":"..." }`. Reply: `Contract` JSON or `{ "error":"not_found" }`.
 - `trust.contract.publish` — (optional admin) publish or update a contract in the store. Payload: `Contract` JSON.

## Recommended Project Layout

Keep the repository layout small and consistent with other Rust services in this repo. Example:

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
│   ├── models/
│   │   └── contract.rs
│   ├── store/
│   │   └── contract_store.rs
│   ├── handlers/
│   │   └── nats_handler.rs
│   └── http/
│       └── health_metrics.rs
└── tests/
  └── fixtures/
    └── genesis_contract.json
```


 ## Metric names (recommended)

 - `trust_contracts_loaded_total` (gauge)
 - `trust_contract_requests_total` (counter)
 - `trust_contract_request_errors_total` (counter)
 - `trust_contract_store_size` (gauge)
 - `trust_service_uptime_seconds` (gauge)

 ## Minimal Contract JSON schema (MVP)

 Fields (required in MVP):

 - `contract_id`: string
 - `version`: integer
 - `title`: string
 - `robot_id`: string
 - `robostake_required`: boolean
 - `payload`: object (free-form)
 - `created_at`: RFC3339 timestamp string
 - `signature`: string | null (optional)

 Example JSON (same as plan sample):

 ```json
 {
   "contract_id": "genesis-0001",
   "version": 1,
   "title": "Genesis: 3D Printer Contract",
   "robot_id": "3d-printer-jon",
   "robostake_required": false,
   "payload": { "actions": ["print-object"], "parameters": {} },
   "created_at": "2025-11-24T00:00:00Z",
   "signature": null
 }
 ```

 ## Backwards compatibility and migration notes

 - During migration run `rust-trust` alongside the existing Trust service and use a separate subject `trust.contract.request.rust` until confidence is achieved.
 - If Go Trust consumers expect floating/legacy fields, consider including deprecated compatibility fields in the published JSON only during migration (document clearly).

 ## Test fixtures

 - `tests/fixtures/genesis_contract.json` (use the sample above)
 - `tests/integration/request_contract.test.rs` — integration test that starts a real or in-memory NATS and asserts the reply contains expected fields.

 ## Implementation hints

 - Use `async-nats` for the NATS client (aligns with other Rust services in this repo).
 - Use `serde` + `schemars` for JSON (optionally generate JSON schema for runtime validation).
 - Keep `robostake_required` typed as `bool` in the model and expose integer micro-RT fields only when economic accounting becomes necessary — for the MVP this flag is enough.
