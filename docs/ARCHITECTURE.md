# RoboTorq HTTP Service Template — Architecture

This document describes the current architecture of the HTTP service template and the foundational components intended to support multiple service types (Robot-Gateway, Refinery, Mint, Vault, etc.). It focuses on lifecycle, configuration, observability, and messaging. Business/economic logic is deliberately out of scope here.

## Current State

- **HTTP Layer (Axum):**
  - Endpoints: `/healthz` (liveness), `/readyz` (readiness), `/metrics` (Prometheus).
  - `HttpServer` orchestrates binding, routing, permissive CORS in dev, and handler wiring.
  - `RoboTorqService` trait standardizes service lifecycle: `initialize`, `health_check`, `export_metrics`, `shutdown`.

- **Configuration:**
  - `RoboTorqConfig` exists and is passed into initialization. Endpoint configuration via `HttpServerConfig` (`HttpService`, `HttpEndpoint`s).
  - Defaults suitable for local development.

- **Observability:**
  - Prometheus text exposition via `/metrics` backed by a metrics handler.
  - Basic tracing/logging (to be expanded).

- **Shutdown:**
  - Async cooperative shutdown via `shutdown_service`; legacy TCP-based helper remains for basic servers.

- **Readiness/Liveness:**
  - Readiness is driven by an `Arc<AtomicBool>`; liveness reflects `health_check()` results.

## Design Principles

- **Uniform Lifecycle:** All services (Gateway, Refinery, Mint, Vault) implement the same trait-based lifecycle.
- **Single Persistence Strategy:** Adopt one persistence technology and pattern across services to reduce operational complexity.
- **Messaging First-Class:** NATS (with JetStream) forms the backbone for events, RPC, and streaming with explicit backpressure.
- **Observability Everywhere:** Metrics, tracing, and structured logs are mandatory and standardized.
- **Security-Ready:** AuthN/Z hooks and transport security are designed in, enabled per deployment phase.
- **Extensible:** Clear plugin points for domain-specific modules without changing the core template.

## Target Capabilities

1. **Persistence Layer (Unified):**
   - Choose and standardize a single storage approach (e.g., Postgres + SQLx, or FoundationDB, or RocksDB/LMDB for embedded). Provide a small repository abstraction layer with:
     - Connection management
     - Migrations
     - Transactions
     - Health contributions
   - Decision criteria: transactionality needs, consistency level, replication story, operational footprint, Windows dev friendliness.

2. **NATS Integration:**
   - `async-nats` client abstraction: connect, publish/subscribe, request/reply.
   - JetStream setup utilities: idempotent stream/consumer creation with retention and ack policies.
   - Bounded channel bridge between HTTP handlers and NATS I/O for backpressure.
   - Typed envelopes with trace IDs and versioning.

3. **Observability:**
   - Metrics registry with common labels (`service`, `component`, `version`).
   - Middleware for request timing and status metrics.
   - Tracing propagation across HTTP ↔ NATS.
   - Grafana dashboards for HTTP, NATS, and persistence.

4. **Error Taxonomy:**
   - Expand `InvariantError` into categories: Config, Startup, Bind, DependencyInit, RequestHandling, Messaging, Persistence, Shutdown.
   - Map to HTTP statuses for admin endpoints and include error codes for programmatic handling.

5. **Security Hooks:**
   - AuthN via JWT/API keys/mTLS.
   - Authorization policy interface.
   - TLS config for HTTP/NATS.

6. **Extensibility & Plugins:**
   - Trait-based route registration and health/metrics contributors for domain services.
   - Cargo features to toggle domains.

## Runtime Model

- **Startup:**
  - Load config → validate → initialize dependencies (persistence, NATS, crypto) → set readiness true.
- **Serving:**
  - HTTP requests handled via Axum with observability middleware.
  - NATS subscriptions serviced by background tasks using bounded queues to decouple producer/consumer speeds.
- **Shutdown:**
  - Set readiness false → stop accepting new work → drain queues → close clients → finalize.

## Configuration Model

- `HttpServerConfig`: service binding + endpoint paths.
- `RoboTorqConfig`: top-level app config including:
  - HTTP: address, port, CORS, TLS
  - NATS: servers, creds, TLS, JetStream streams/consumers
  - Persistence: DSN or path, pool sizes, timeouts
  - Observability: metrics enablement, tracing sinks

## Subject & Stream Conventions (NATS)

- Subjects follow service-oriented naming:
  - Gateway: `rtq.gateway.cmd.*`, `rtq.gateway.events.*`
  - Refinery: `rtq.refinery.cmd.*`, `rtq.refinery.events.*`
  - Mint: `rtq.mint.tx.*`, `rtq.mint.events.*`
  - Vault: `rtq.vault.audit.*`, `rtq.vault.events.*`
- JetStream streams group subjects per domain with retention and ack policies aligned to business criticality.

## Persistence Strategy (Unified)

Select one of:

- **Postgres (SQLx):** Strong transaction support, mature tooling, easy JetStream integration for outbox/inbox patterns.
- **RocksDB/LMDB:** Embedded, fast, simple ops; requires careful design for replication and backup.
- **FoundationDB:** Strong consistency, scalable, but operationally heavier.

Provide a thin repository abstraction and health contributors; include migrations and connection pooling.

## Security & Compliance

- Optional mTLS, JWT-based authentication, and authorization policies.
- Audit logging for administrative actions (init, shutdown, config changes).

## Deployment Modes

- **Local Dev:** Permissive CORS, no TLS, in-memory or local single-node persistence, single NATS.
- **Staging:** TLS, JetStream durable consumers, Postgres with migrations, locked-down metrics.
- **Production:** HA NATS cluster, replicated persistence, hardened auth, dashboards, alerts.

## Simulation Integration

To support fast, deterministic testing, services should integrate a time-dilation helper that accelerates prescribed waits when simulation mode is enabled.

- Time Dilation: Replace fixed sleeps with a helper that consults simulation config. When enabled, the effective wait time is scaled (e.g., 10x faster). When disabled, real wall-clock waits are used.
- Central Helper: Provide an async function (e.g., `sim_sleep(duration, cfg)`) that:
  - Checks `cfg.simulation.enabled` and `cfg.simulation.time_dilation`.
  - Adjusts the duration accordingly and calls `tokio::time::sleep`.
- Coverage: Use the helper anywhere waits are prescribed:
  - Initialization backoffs (DB, NATS)
  - NATS retry/jitter in workers
  - Persistence retry loops
  - Graceful shutdown drains
- Observability: Consider tagging metrics and spans with `mode="sim"` and `time_dilation` for clarity.

Example (pseudo-Rust):

```rust
pub async fn sim_sleep(dur: std::time::Duration, cfg: &RoboTorqConfig) {
    let eff = if cfg.simulation.enabled {
        let factor = cfg.simulation.time_dilation.max(1.0);
        let nanos = dur.as_nanos() as f64 / factor;
        std::time::Duration::from_nanos(nanos as u64)
    } else {
        dur
    };
    tokio::time::sleep(eff).await;
}
```
