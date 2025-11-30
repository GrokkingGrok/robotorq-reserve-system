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
- Pin dependencies in workspace Cargo.toml with specific versions
- Implement sim_sleep helper for time dilation in simulation mode
- Clarify simulation legal notice for contributor awareness
- Add default implementations to RoboTorqService trait methods
- Set up CI pipeline with automated testing and linting

Deliverables:
- Stabilized template with pinned dependencies and CI
- Implemented time dilation for simulation testing
- Updated documentation and legal notices
- Template ready for persistence implementation without regressions

### Phase 2.1 — Persistence Strategy (Unified)
- Implement multi-backend repository abstraction supporting Postgres/SQLite/Memory
- Add health contributions, connection pooling, and timeout handling
- Implement database migrations with multi-backend support
- Standardize error mapping from persistence layer to service errors
- Add repository pattern for common data access operations

Deliverables:
- Multi-backend persistence module with repository abstraction
- Migration system supporting all configured backends
- Health checks and connection management
- Standardized persistence error types and mapping
- Examples and comprehensive tests

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
