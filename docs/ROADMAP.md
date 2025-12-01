# RoboTorq HTTP Service Template — Roadmap

This roadmap sequences work from the current template to a fully distributed, observable, secure foundation usable by Robot-Gateway, Refinery, Mint, Vault, and future services.

## Goals

- Single persistence strategy across all service types
- First-class NATS (JetStream) messaging with backpressure
- Strong observability (Prometheus + Grafana) and error taxonomy
- Security hooks for AuthN/Z and transport security
- Extensible template with domain plugins and consistent lifecycle

## Phased Plan

### Phase 1 — Lifecycle, Config, and Observability Baseline
- Traits: finalize `RoboTorqService` expectations (async init/shutdown, health, metrics).
- Config: define comprehensive `RoboTorqConfig` schema (HTTP, NATS, Persistence, Observability, Security, Crypto, Economic).
- HTTP: add middleware for request timing, status metrics; configurable CORS; body/timeout limits.
- Metrics: implement a minimal metrics registry interface and expose common counters/gauges.
- Docs/CI: `cargo doc`, examples crate, unit/integration tests; add basic Grafana dashboards.

Deliverables:
- Validated config loader with 7 comprehensive config layers
- Improved `HttpServer` with middleware, CORS, timeouts, body limits
- Metrics registry with common counters/gauges and `/metrics` consistency
- Docs: ARCHITECTURE.md, ROADMAP.md, example service
- Full test coverage and CI integration

### Phase 2.0 — Template Stabilization
Status: in progress

Completed:
- Normalize module paths in commons (removed #[path], removed glob re-exports); centralized schema version constants and gauges
- Metrics naming and labels aligned (service/component/version/subject), per INCONSISTENCIES audit

In progress:
- Pin dependencies in workspace Cargo.toml with specific versions
  - Add cargo-deny for license/duplicate checks
  - Generate and commit lockfile for reproducible CI builds
- Implement sim_sleep helper for time dilation in simulation mode
  - Feature-gate with `sim` and provide deterministic time source
- Clarify simulation legal notice for contributor awareness
  - Include repo-level NOTICE and per-crate README badge
- Add default implementations to RoboTorqService trait methods
  - Provide no-op health/metrics defaults and graceful shutdown hooks
- Set up CI pipeline with automated testing and linting
  - Rust: fmt, clippy, test; optional miri on nightly
  - Cache strategy for workspace builds

Deliverables:
- Stabilized template with pinned dependencies and baseline CI
- Implemented time dilation for simulation testing (sim_sleep + feature-gate)
- Updated documentation and legal notices for simulation mode
- Module path normalization and schema/version gauges consolidated (from INCONSISTENCIES)
- Template ready for persistence implementation without regressions

### Phase 2.1 — Persistence Strategy (Unified)
Scope: unify Postgres/SQLite/Memory backends behind a common abstraction, with health, timeouts, and standardized errors

Plan:
- Implement multi-backend repository abstraction supporting Postgres/SQLite/Memory
  - Use sqlx with feature flags: `postgres`, `sqlite`; `runtime-tokio`
  - Memory backend via in-process store for tests/examples
- Add health contributions, connection pooling, and timeout handling
  - Pooled connections (sqlx::Pool), per-op timeouts via tokio timeouts
- Implement database migrations with multi-backend support
  - Use sqlx migrate (avoid name collision with service “Refinery” crate)
  - Seed example migrations and a migration runner utility
- Standardize error mapping from persistence layer to service errors
  - Define PersistenceError enum and map driver errors
- Add repository pattern for common data access operations
  - Traits for read/write ops; typed IDs; pagination helpers

Deliverables:
- Multi-backend persistence module with repository abstraction
- Migration system (sqlx migrate) supporting configured backends
- Health checks and connection management with timeouts
- Standardized persistence error types and mapping
- Examples and comprehensive tests (including testcontainers for Postgres)

### Phase 3 — NATS & JetStream
- Client abstraction over `async-nats`: connect, publish, subscribe, request/reply.
- JetStream: idempotent stream/consumer setup utilities.
- Bounded channel bridge for backpressure between HTTP and NATS workers.
- Typed envelopes with trace IDs and version; retry/backoff policies.
- Observability: metrics for NATS ops, queue depths, acks/nacks; health contributions for connection/state.

Deliverables:
- NATS module, JetStream setup helpers, bounded queues, metrics & health integration

### Phase 4 — Security Hooks
- AuthN: JWT, API keys, optional mTLS.
- AuthZ: policy interface with allow/deny decisions; per-route configuration.
- TLS for HTTP and NATS; secrets management guidelines.

Deliverables:
- Security middleware, policy traits, configuration, tests

### Phase 5 — Extensibility and Plugins
- Route registration trait for domain services; health/metrics contributors.
- Cargo features to include/exclude domain modules (Gateway, Refinery, Mint, Vault).

Deliverables:
- Plugin architecture, example domain module integrations

### Phase 6 — Distributed Operation Readiness
- HA configurations: NATS clusters, persistence replication/failover.
- Graceful shutdown signaling across components; readiness flipping on dependency degradation.
- Backpressure strategies validated under load; rate limiting and circuit breakers.
- Advanced dashboards and alerting.

Deliverables:
- Deployment guides and terraform/scripts; load test results

## Cross-Cutting Standards

- **Subject Naming:** `rtq.<service>.<type>.*` (e.g., `rtq.gateway.cmd.*`, `rtq.mint.tx.*`).
- **Error Codes:** Stable error categories and codes for programmatic handling.
- **Tracing:** Propagate trace IDs across HTTP ↔ NATS; standard span names and fields.
- **Metrics Labels:** `service`, `component`, `version`, `subject` where applicable.

## Try It (Dev Workflow)

1. Run local NATS with JetStream and expose Prometheus/Grafana.
2. Start a sample service implementing `RoboTorqService`, publish a message, observe metrics.
3. Flip readiness during init; verify `/readyz` and `/healthz` behavior.

## Decision Log (to fill as we go)

- Persistence choice: Multi-backend support (Postgres, SQLite, Memory) with runtime configuration
- Persistence config: Comprehensive layer supporting connection pooling, SSL, backend-specific tuning
- AuthN/Z baseline: TBD (start with JWT + policy traits)
- Tracing sink: TBD (OpenTelemetry/OTLP)
