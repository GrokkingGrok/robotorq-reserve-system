# RoboTorq HTTP Service Template — Architecture

This document describes the current architecture of the HTTP service template and the foundational components intended to support multiple service types (Robot-Gateway, Refinery, Mint, Vault, etc.). It focuses on lifecycle, configuration, observability, and messaging. Business/economic logic is deliberately out of scope here.

---

## Current State

### **HTTP Layer (Axum)**
- **Endpoints**:
  - `/healthz`: Liveness probe, tied to `health_check()` results.
  - `/readyz`: Readiness probe, backed by an `Arc<AtomicBool>`.
  - `/metrics`: Prometheus metrics exposition.
- **HttpServer**:
  - Orchestrates binding, routing, permissive CORS in development, and handler wiring.
  - Automatically sets up a metrics registry with config-derived labels (`service`, `component`, `version`).
  - Supports lifecycle hooks for service initialization, readiness, and shutdown.
- **RoboTorqService Trait**:
  - Standardizes service lifecycle:
    - `initialize`: Prepares the service with `RoboTorqConfig`.
    - `health_check`: Reports service health.
    - `export_metrics`: Exposes service-specific metrics.
    - `shutdown`: Handles graceful shutdown.

---

### **Configuration**
- **RoboTorqConfig**:
  - Top-level configuration for the application.
  - Includes simulation settings, HTTP server configuration, NATS, persistence, and observability.
- **HttpServerConfig**:
  - Defines service binding and endpoint paths.
  - Defaults are suitable for local development.

---

### **Observability**
- **Metrics**:
  - Prometheus text exposition via `/metrics`.
  - Includes HTTP middleware metrics (e.g., request counts, latencies) and service-specific metrics.
  - `http_health_requests_total` added for `/healthz` endpoint.
- **Tracing**:
  - Basic tracing/logging is in place, with plans to expand for distributed tracing.
- **Simulation Mode**:
  - Metrics and spans can be tagged with `mode="sim"` and `time_dilation` for clarity.

---

### **Shutdown**
- **Async Cooperative Shutdown**:
  - `shutdown_service` ensures services release resources cleanly.
  - Readiness is set to `false` before stopping new work.
  - Graceful shutdown drains queues and finalizes clients.

---

### **Readiness/Liveness**
- **Readiness**:
  - Driven by an `Arc<AtomicBool>`, set to `true` when the service is ready.
- **Liveness**:
  - Reflects the results of `health_check()`.

---

## Design Principles

1. **Uniform Lifecycle**: All services (Gateway, Refinery, Mint, Vault) implement the same trait-based lifecycle.
2. **Single Persistence Strategy**: Adopt one persistence technology and pattern across services to reduce operational complexity.
3. **Messaging First-Class**: NATS (with JetStream) forms the backbone for events, RPC, and streaming with explicit backpressure.
4. **Observability Everywhere**: Metrics, tracing, and structured logs are mandatory and standardized.
5. **Security-Ready**: AuthN/Z hooks and transport security are designed in, enabled per deployment phase.
6. **Extensible**: Clear plugin points for domain-specific modules without changing the core template.

---

## Target Capabilities

### 1. **Persistence Layer (Unified)**
- **Current State**: Not yet implemented.
- **Planned**:
  - Choose and standardize a single storage approach (e.g., Postgres + SQLx, or FoundationDB, or RocksDB/LMDB for embedded).
  - Provide a small repository abstraction layer with:
    - Connection management
    - Migrations
    - Transactions
    - Health contributions

---

### 2. **NATS Integration**
- **Current State**: Not yet implemented.
- **Planned**:
  - `async-nats` client abstraction: connect, publish/subscribe, request/reply.
  - JetStream setup utilities: idempotent stream/consumer creation with retention and ack policies.
  - Bounded channel bridge between HTTP handlers and NATS I/O for backpressure.
  - Typed envelopes with trace IDs and versioning.

---

### 3. **Observability**
- **Current State**:
  - Metrics registry with common labels (`service`, `component`, `version`).
  - Middleware for request timing and status metrics.
- **Planned**:
  - Tracing propagation across HTTP ↔ NATS.
  - Grafana dashboards for HTTP, NATS, and persistence.

---

### 4. **Error Taxonomy**
- **Current State**:
  - `InvariantError` exists but is not yet categorized.
- **Planned**:
  - Expand `InvariantError` into categories: Config, Startup, Bind, DependencyInit, RequestHandling, Messaging, Persistence, Shutdown.
  - Map to HTTP statuses for admin endpoints and include error codes for programmatic handling.

---

### 5. **Security Hooks**
- **Current State**: Not yet implemented.
- **Planned**:
  - AuthN via JWT/API keys/mTLS.
  - Authorization policy interface.
  - TLS config for HTTP/NATS.

---

### 6. **Extensibility & Plugins**
- **Current State**:
  - Trait-based route registration and health/metrics contributors for domain services.
- **Planned**:
  - Cargo features to toggle domains.

---

## Runtime Model

### **Startup**
- Load config → validate → initialize dependencies (persistence, NATS, crypto) → set readiness true.

### **Serving**
- HTTP requests handled via Axum with observability middleware.
- NATS subscriptions serviced by background tasks using bounded queues to decouple producer/consumer speeds.

### **Shutdown**
- Set readiness false → stop accepting new work → drain queues → close clients → finalize.

---

## Simulation Integration

### **Current State**
- **Time Dilation**:
  - `sim_sleep(duration, cfg)` adjusts sleep duration based on simulation speedup factor.
  - Feature-gated with `sim` for zero production overhead.
- **Deterministic Time**:
  - `DeterministicTime` struct provides programmatic control over simulated time (pause/resume, dynamic speedup).

### **Planned Enhancements**
- Distributed time synchronization for multi-service simulations.
- Observability tags for simulation mode.

---

## Deployment Modes

1. **Local Dev**:
   - Permissive CORS, no TLS, in-memory or local single-node persistence, single NATS.
2. **Staging**:
   - TLS, JetStream durable consumers, Postgres with migrations, locked-down metrics.
3. **Production**:
   - HA NATS cluster, replicated persistence, hardened auth, dashboards, alerts.

---

## Subject & Stream Conventions (NATS)

- **Subjects**:
  - Gateway: `rtq.gateway.cmd.*`, `rtq.gateway.events.*`
  - Refinery: `rtq.refinery.cmd.*`, `rtq.refinery.events.*`
  - Mint: `rtq.mint.tx.*`, `rtq.mint.events.*`
  - Vault: `rtq.vault.audit.*`, `rtq.vault.events.*`
- **Streams**:
  - Group subjects per domain with retention and ack policies aligned to business criticality.

---

## Persistence Strategy (Unified)

### **Current State**
- Not yet implemented.

### **Planned**
- Select one of:
  - **Postgres (SQLx)**: Strong transaction support, mature tooling, easy JetStream integration for outbox/inbox patterns.
  - **RocksDB/LMDB**: Embedded, fast, simple ops; requires careful design for replication and backup.
  - **FoundationDB**: Strong consistency, scalable, but operationally heavier.

---

## Security & Compliance

- Optional mTLS, JWT-based authentication, and authorization policies.
- Audit logging for administrative actions (init, shutdown, config changes).

---

## Summary

The RoboTorq HTTP Service Template is evolving to support distributed, highly concurrent systems with robust observability, lifecycle management, and simulation capabilities. The current focus is on completing simulation integration, observability, and NATS messaging, with persistence and security enhancements planned for future phases.