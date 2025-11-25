# Trust Configuration (env vars)

This file documents the runtime environment variables used by the `rust-trust` service.

Core connection and bootstrap
- `TRUST_NATS_URL` (or `NATS_URL` fallback): URL for the NATS server. Default: `nats://nats:4222`.
- `TRUST_GENESIS_CONTRACT_PATH`: Path to the genesis contract JSON file loaded at startup. Default: `config/genesis_contract.json`.

HTTP & Metrics
- `TRUST_HTTP_HOST`: HTTP host/address to bind for admin/health endpoints. Default: `0.0.0.0`.
- `TRUST_HTTP_PORT`: HTTP port for admin/health endpoints. Default: `8080`.
- `TRUST_METRICS_HOST`: Metrics host (Prometheus binding). Default: `0.0.0.0`.
- `TRUST_METRICS_PORT`: Metrics port. Default: `9092`.

Admin & plugins
- `TRUST_ADMIN_NATS_SUBJECT`: NATS subject for admin requests. Default: `trust.admin`.
- `TRUST_ALLOW_PLUGIN_REGISTRATION`: `true`/`false` to allow plugins to register themselves. Default: `false`.

Operational
- `TRUST_SHUTDOWN_GRACE_SECONDS`: Grace period in seconds to wait for background tasks to finish on shutdown. Default: `5`.
- `TRUST_LOG_LEVEL`: Logging level for the service (e.g., `info`, `debug`). Default: `info`.
- `TRUST_MAX_REQUEST_BYTES`: Maximum allowed bytes for a NATS contract request payload. Default: `1024`.

Owner metadata
- `TRUST_OWNER_NAME`: Optional owner name to include in default genesis contract metadata.
- `TRUST_OWNER_WALLET_ID`: Optional wallet identifier for the owner.

Simulation (feature-gated)
- If built with the `simulation` feature, the following apply:
  - `TRUST_SIMULATION_MODE`: `true`/`false` to enable simulation behaviors. Default: `false`.
  - `TRUST_TIME_COMPRESSION`: Float; speed-up factor for simulated time. Default: `1.0`.

Notes
- Use `TRUST_NATS_URL` to override NATS per-service; when omitted `NATS_URL` is used as a fallback.
- Changes to env vars require service restart.
- For production, secure admin subjects with NATS credentials and ACLs.
