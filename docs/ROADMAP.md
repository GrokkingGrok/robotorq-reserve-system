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
Status: completed

Completed:
- Normalize module paths in commons (removed #[path], removed glob re-exports); centralized schema version constants and gauges
- Metrics naming and labels aligned (service/component/version/subject), per INCONSISTENCIES audit
- Add default implementations to RoboTorqService trait methods (async lifecycle, no-op health/metrics, graceful shutdown hooks)
- Implement ServiceMetricsContext with config-derived labels (derive from RoboTorqConfig instead of hardcoded values)
- Pin dependencies in workspace Cargo.toml with specific versions
- Add cargo-deny for license/duplicate checks
- Generate and commit lockfile for reproducible CI builds
- Implement sim_sleep helper for time dilation in simulation mode
  - Feature-gate with `sim` and provide deterministic time source
- Clarify simulation legal notice for contributor awareness
  - Include repo-level NOTICE and per-crate README badge
- Set up CI pipeline with automated testing and linting
  - Rust: fmt, clippy, test; optional miri on nightly
  - Cache strategy for workspace builds

Deliverables:
- Stabilized template with pinned dependencies and baseline CI
- Implemented time dilation for simulation testing (sim_sleep + feature-gate)
- Updated documentation and legal notices for simulation mode
- Module path normalization and schema/version gauges consolidated (from INCONSISTENCIES)
- Template ready for persistence implementation without regressions

### Phase 2.0.1 — Decoupling

Status: Completed — traits decomposed (`ServiceLifecycle`, `HealthContributor`, `MetricsContributor`), time abstraction (`TimeProvider`) implemented (production + simulation), metrics label/registry decoupling added (`MetricsLabelProvider`, optional per‑service registry). Legacy code removal scheduled and in-progress; all old helpers will be removed in follow-up cleanup.

Scope: Reduce tight coupling between core components to improve modularity and extensibility, ensuring a stable foundation for future phases.

Plan:
1. **Refactor `RoboTorqService` Trait**:
   - Split into smaller traits:
     - `ServiceLifecycle` for `initialize` and `shutdown`.
     - `HealthContributor` for `health_check`.
     - `MetricsContributor` for `export_metrics`.
   - Provide default implementations for optional methods to reduce boilerplate.

2. **Abstract Timekeeping**:
   - Introduce a `TimeProvider` trait with methods like `now()` and `sleep()`.
   - Implementations:
     - `SystemTimeProvider` for production.
     - `SimulatedTimeProvider` for simulation.
   - Replace direct calls to `sim_sleep` with injected `TimeProvider` instances.

3. **Decouple Metrics Registry**:
   - Allow services to optionally provide their own metrics registry.
   - Update `HttpServer` to accept a registry as a parameter, falling back to a default if none is provided.
   - Introduce a `MetricsLabelProvider` trait to decouple label derivation from `RoboTorqConfig`.

4. **Standardize Error Handling**:
  - Expand `InvariantError` into clear categories (Config, Startup, Persistence, Messaging, Shutdown).
  - Provide feature-gated `From` conversions for common external error types (e.g., `sqlx::Error`, `async-nats::Error`) to keep the commons lightweight by default.
  - Map error categories to appropriate HTTP/admin status codes and error codes for operational handling.

Notes:
- The commons crate now includes a migration to the new error taxonomy. Services should prefer the new typed errors (e.g. `PersistenceError::Pool`) when implementing repositories.
- Legacy single-type `InvariantError` conversions remain source-compatible during the migration window; the plan is to remove transient compatibility helpers in the next cleanup pass.

Deliverables:
- Decoupled `RoboTorqService` lifecycle, health, and metrics traits.
- Abstracted timekeeping with `TimeProvider` for production and simulation.
- Modular metrics registry with optional service-specific registries.
- Standardized error taxonomy for consistent error handling across layers.
- **All old code must be removed before moving to the next phase.**

### Phase 2.0.2 — Logging/Tracing

Goal: establish consistent, structured logging and tracing across the template so services
are observable in development, CI, and production. Provide an incremental migration path
that improves debuggability immediately while enabling distributed trace export (OTLP)
and standardized log formats (JSON for CI/aggregators, human text for local dev).

Scope:
- Add a single, well-documented logging initializer in `crates/commons::util::logging`.
- Standardize on `tracing`/`tracing-subscriber` for structured logs and `opentelemetry` for
  optional OTLP export.
- Instrument core lifecycle boundaries: initialize/start/ready/shutdown, HTTP handlers,
  background worker spawn points, and key error paths.
- Provide lightweight utilities for test logging, lock-wait diagnostics, and task tracing.

Deliverables:
- `crates/commons/src/util/logging/mod.rs` — `init_test_logging()` and `init_prod_tracing(json: bool, otlp: Option<OtlpConfig>)` helpers.
- Example `OtlpConfig` struct and feature gating for `opentelemetry-otlp` to keep commons minimal by default.
- Instrumentation added to `HttpServer` start/shutdown paths and `shutdown::run_service_shutdown` with structured fields (service, component, duration_ms, error).
- Lock-wait diagnostic helper `log_if_waited(mutex_name: &str, start: Instant, threshold: Duration)` used in critical Mutex/RwLock areas.
- Task instrumentation helpers: `spawn_traced(name: &str, fut)` to attach a span and log abort/reason on join/abort.
- Tests and examples showing JSON logs in CI and human logs locally; CI job step that asserts presence of key fields in exported logs.

Plan / Work items (incremental):
1. Draft `util::logging` API and add to `crates/commons` (idempotent init using `std::sync::Once`).
2. Add test helper and annotate unit/integration tests to call `init_test_logging()`.
3. Wire structured logging into `HttpServer` lifecycle (start, bind, ready, shutdown) and `run_service_shutdown` (start, stop call, shutdown call, duration).
4. Add `spawn_traced()` helper and replace key `tokio::spawn` sites in commons and examples.
5. Add lock-wait helper and instrument one or two hot spots (e.g., shutdown/pool locks) as examples.
6. Add feature-guarded OTLP exporter with a sample `OtlpConfig` and environment variable mapping; add a README snippet describing how to enable OTLP in CI.
7. Add CI smoke test that runs an example service, captures logs, and asserts required JSON fields (timestamp, level, service, span_context or trace_id).
8. Iterate: review telemetry gaps, add missing spans, and adopt sampling defaults for production.

Acceptance criteria:
- `crates/commons::util::logging` compiles and provides two public idempotent entry points: `init_test_logging()` and `init_prod_tracing(json: bool, otlp: Option<OtlpConfig>)`.
- At least one example service is instrumented and produces JSON logs with `service`, `component`, `level`, and `message` fields when `json=true`.
- Trace context (trace_id) is attached to HTTP request spans and exported to logs as `trace_id` when available.
- CI job validates the presence of structured fields in the example service's logs.

Risks & mitigations:
- Risk: Adding OTLP and exporters increases compile-time dependencies. Mitigation: gate OTLP behind a Cargo feature and default to disabled in the workspace.
- Risk: Log noise growth. Mitigation: adopt `RUST_LOG` defaults and sensible sampling/level policies; prefer `debug` for high-frequency spans and `info` for lifecycle events.

Estimated effort: 2–4 days of focused work (prototype + instrumentation + CI smoke tests).

Files to add/update (proposed):
- `crates/commons/src/util/logging/mod.rs` (new)
- `crates/commons/src/services/robotorq_service/mod.rs` — add lifecycle span calls at start/stop (small edits)
- `crates/commons/src/services/robotorq_service/shutdown/mod.rs` — add additional structured traces around lock wait and stop/shutdown calls
- `crates/examples/*` — update one example to enable JSON/OTLP and show CI-friendly logs

If this plan looks good I will implement step 1 (add `crates/commons/src/util/logging/mod.rs`) and instrument one lifecycle path (HttpServer start/shutdown) as a concrete example, then run tests and show sample logs for your review.
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
