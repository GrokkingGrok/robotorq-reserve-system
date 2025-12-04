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
  - Error taxonomy: replace the legacy `InvariantError` with domain-specific enums
    (Config, Startup, Persistence, Messaging, Shutdown) and a `ServiceError` type
    for service boundary responses.
  - Provide feature-gated `From` conversions for common external error types (e.g., `sqlx::Error`, `async-nats::Error`) to keep the commons lightweight by default.
  - Map error categories to appropriate HTTP/admin status codes and error codes for operational handling.

Notes:
- The commons crate now includes a migration to the new error taxonomy. Services should prefer the new typed errors (e.g. `PersistenceError::Pool`) when implementing repositories.
- Legacy single-type `InvariantError` conversions were used during an earlier migration window and have been removed in the `rewrite-core` refactor. Services should now use domain-specific errors and map them to `ServiceError` at public boundaries; see the ARCHITECTURE and docs/ migration notes for details.

Deliverables:
- Decoupled `RoboTorqService` lifecycle, health, and metrics traits.
- Abstracted timekeeping with `TimeProvider` for production and simulation.
- Modular metrics registry with optional service-specific registries.
- Standardized error taxonomy for consistent error handling across layers.
- **All old code must be removed before moving to the next phase.**

### Phase 2.0.2 — Logging/Tracing

Status: Completed

Goal: establish consistent, structured logging and tracing across the template so services
are observable in development, CI, and production. The work provides an incremental migration
path: immediate debug improvements, JSON logs for CI, and optional OTLP export behind a feature flag.

Scope (summary of what was implemented):
- A single, well-documented logging initializer in `crates/commons::util::logging` with idempotent entry points.
- Standardized on `tracing`/`tracing-subscriber` and feature-gated `opentelemetry`/OTLP exporter.
- Instrumented core lifecycle boundaries (initialize/start/ready/shutdown), HTTP server lifecycle, background spawn points, and key shutdown/error paths.
- Lightweight utilities for test logging, lock-wait diagnostics, and task tracing.

Completed Deliverables:
- `crates/commons/src/util/logging/mod.rs` implemented with `init_test_logging()` and `init_prod_tracing(json: bool, otlp: Option<OtlpConfig>)` (idempotent initialization).
- `OtlpConfig` added and OTLP exporter gated behind a Cargo feature to avoid extra compile-time deps by default.
- `HttpServer` lifecycle instrumented (start, bind, ready, shutdown) with structured spans and fields.
- `spawn_traced()` helper implemented and used for key `tokio::spawn` sites in `commons` and examples.
- `log_if_waited()` lock-wait diagnostic helper added and used in critical shutdown/pool lock spots as examples.
- Middleware-based `TraceContext` injection (`insert_trace_context`) implemented and unit-tested (presence and idempotence).
- Examples and integration tests updated to use structured `tracing` (replaced `println!`), and a CI smoke workflow (`.github/workflows/logging-ci.yml`) was added to assert JSON fields.
- Code hygiene: ran tests and Clippy; commons tests pass and workspace clippy warnings were fixed.

Remaining / Pending Items:
- Full OTLP end-to-end verification (collector + exporter) in CI requires an external collector or test-side container; this is intentionally pending to avoid infra dependency in the default CI.
- A short CONTRIBUTING/README snippet documenting the logging/tracing policy and how to enable OTLP in CI is suggested (not yet added).
- The experimental global JSON formatter approach was abandoned (private API); the middleware approach is the supported, stable solution.

Acceptance criteria status:
- `crates/commons::util::logging` provides the two idempotent entry points — met.
- An example service emits JSON logs containing `service`, `component`, `level`, and `message` when `json=true` — met (examples and CI demo updated).
- Trace context (trace_id) is attached to HTTP request spans and exported to logs when available — met via middleware `TraceContext` injection.
- CI smoke test added to validate structured fields — added; note that OTLP end-to-end is still gated by external collector availability.

Next actions (suggested):
- Add a short `docs/` or `CONTRIBUTING.md` snippet describing how to enable OTLP and the `RUST_LOG` defaults for local vs CI.
- Decide whether to push the `rewrite-core` commits to the remote branch (I can push on your instruction).

OTLP prototype (status):
- Centralized OTLP wiring implemented in `crates/commons::util::logging` and gated behind the `otlp` Cargo feature.
- Example `crates/examples/src/bin/emit_traces.rs` emits a test span with attribute `example_id = "robotorq_emit_traces_test_001"` for smoke testing.
- Local collector compose: `ci/otlp-collector/docker-compose.yml` and pre-check script `scripts/run_otlp_precheck.py` exist to run a local end-to-end smoke test. A manual GitHub Actions workflow (`.github/workflows/otlp-e2e.yml`) uses the script and uploads `otel-collector.log` as an artifact.
- Recommended next steps: perform a `cargo tree` check for opentelemetry transitive versions; decide when to enable the e2e workflow on default branches; optionally add the short OTLP setup doc (`docs/OTLP_LOCAL_SETUP.md`) and a message envelope spec (`docs/MESSAGE_ENVELOPE.md`) for Phase 3 adapter work.

Notes:
- All changes were implemented to be minimal, feature-gated, and backwards-compatible. Tests in `crates/commons` pass and clippy was run and fixed.


### Phase 2.1 — Persistence Strategy (Unified)
Scope: unify Postgres/SQLite/Memory backends behind a common abstraction, with health, timeouts, and standardized errors
Plan (Option A — Commons-based persistence utilities; do not enact without follow-up):

- Goal: provide a trace-aware, transport-agnostic persistence abstraction implemented inside `crates/commons::util::persistence` so services can reuse a single, well-tested set of repository traits, backends, migration tooling, and error mappings.

- Key requirements (trace-first, non-invasive):
  - Repository methods accept a small `Context` or trace carrier so DB operations can create child spans and attach standard attributes (`db.system`, `db.name`, `db.statement` (truncated), `persistence.operation`, and `db.rows_affected` when available).
  - Backends must not depend on OTLP directly — use `commons` tracing helpers for span creation and propagation. OTLP remains feature-gated behind `commons/otlp`.
  - Per-op timeouts must cancel queries and return `PersistenceError::Timeout` with metric/span annotations.

- Proposed file layout (follow File Creation Rules — place under `crates/commons`):
  - `crates/commons/src/util/persistence/mod.rs` — public traits, `Context` type, `PersistenceError` enum, and re-exports.
  - `crates/commons/src/util/persistence/backends/memory/mod.rs` — in-memory backend used for fast unit tests and examples.
  - `crates/commons/src/util/persistence/backends/sqlite/mod.rs` — sqlite backend (in-memory option for tests).
  - `crates/commons/src/util/persistence/backends/postgres/mod.rs` — Postgres backend implemented with `sqlx::Pool` and timeouts.
  - `crates/commons/src/util/persistence/migrations.rs` — migration runner and instrumentation helpers.
  - Tests:
    - `crates/commons/tests/persistence/memory.rs` — unit tests for in-memory backend.
    - `crates/commons/tests/persistence/sqlite_integration.rs` — sqlite integration tests.
    - `crates/commons/tests/persistence/postgres_integration.rs` — Postgres integration with Testcontainers (branch-local CI).

- API & design notes:
  - `Context` shape: small, transport-agnostic holder for trace headers, optional deadline/tokio timeout and a few convenience helpers to start child spans. Keep implementation minimal so adapters (HTTP/NATS) can convert incoming headers into `Context` without pulling tracing heavy deps.
  - Repository trait pattern (example signature):
    - `async fn insert_item(&self, ctx: &Context, item: NewItem) -> Result<ItemId, PersistenceError>;`
  - All DB ops should start a child span (via `commons::util::tracing` helpers) and annotate errors before mapping to `PersistenceError`.

- Configuration & features:
  - Extend `RoboTorqConfig` with a `persistence` section: `driver`, `url`, `pool_size`, `connect_timeout`, `statement_timeout`, `migration_path`, and `enable_query_tracing: bool`.
  - Cargo features (per crate): `persistence-postgres`, `persistence-sqlite`, `persistence-memory`, `persistence-testcontainers` (optional). Keep `commons/otlp` as the only place enabling OTLP exporter.
  - Suggested runtime dependencies only in backend modules (feature-gated): `sqlx` (postgres/sqlite features), `testcontainers` (optional, dev-only), `tokio` runtime features.

- Tests & CI policy (branch-local first):
  - Unit tests: exercise memory backend and trait implementations (fast, deterministic).
  - SQLite integration: run in-memory sqlite tests in CI as quick smoke tests.
  - Postgres integration: use Testcontainers in branch-local workflows to validate behavior (connection pooling, migrations, timeouts). Keep Postgres integration off default CI until shielding/infra is accepted.
  - Tests must assert that spans include at least the `db.system` and an obfuscated `db.statement` attribute when tracing enabled.

- Migration runner:
  - Provide a simple migration runner (`migrations.rs`) that applies migrations from `migration_path`, exposes a traced span per migration, records durations, and returns structured results for health checks.

- Error taxonomy & mapping:
  - Define `PersistenceError` enum (e.g., `Connection`, `Query`, `Timeout`, `ConstraintViolation`, `NotFound`, `PoolExhausted`, `MigrationError`) and map driver errors into these canonical variants at the backend boundary.
  - Ensure mapping happens *before* propagating errors to service layers so public service APIs use `ServiceError` consistently.

- Deliverables (Option A):
  - `crates/commons::util::persistence` module with traits, `Context`, `PersistenceError`, and re-exports.
  - Implementations: memory, sqlite, postgres backends with tests.
  - Migration runner with tracing and health outputs.
  - Unit and integration tests (memory, sqlite, Postgres via Testcontainers) with assertions for trace attributes.
  - Docs: `docs/` snippets describing persistence config and tracing requirements.

- Prioritized checklist (suggested order; estimates):
  1. Design `Context` + repository trait scaffolding in `crates/commons/src/util/persistence` (1–2 days).
  2. Implement memory backend + unit tests and a simple example that shows traced DB ops (0.5–1 day).
  3. Implement sqlite backend (in-memory option) + tests (0.5–1 day).
  4. Implement Postgres backend (sqlx pool + timeouts) + basic tests (1–2 days).
  5. Migration runner + spans + health hooks (0.5–1 day).
  6. Integration tests with Testcontainers Postgres and branch-local CI workflow (1–2 days).
   7. Docs and config schema updates (0.5 day).

  #### Design refinements (from critique)

  These concise rules address common implementation questions and reduce future bikeshedding. They should be treated as required defaults for Option A and implemented or documented as part of the sprint work.

  - **Monotonic deadlines (Timekeeper integration):** `Context` deadlines MUST be monotonic instants (use `std::time::Instant` / `tokio::time::Instant` via the project's `TimeProvider`/timekeeper). For cross-process propagation, transmit a remaining timeout (milliseconds) in headers; the receiver converts that to a local monotonic `Instant = now_monotonic() + duration`. Do not rely on `SystemTime` for cancellations; keep `SystemTime` only for human-facing logs if needed.

  - **Structured DB span attributes (`DbAttributes`):** provide a small `DbAttributes` struct (or builder/macro) in `commons` so backends call a single helper to attach DB attributes. Example API:
    - `span.add_db_attributes(DbAttributes { system: "postgres", name: db_name, statement: obfuscated_sql, rows_affected: Some(n) })`
    - Centralize `db.statement` obfuscation and canonical attribute names to avoid stringly-typed keys and copy/paste bugs.

  - **Migration runner failure behavior (clarity):** the runner defaults to *fail-fast* — a failed migration aborts subsequent migrations; the runner detects and reports "dirty" states and returns structured results. Transactional guarantees are driver-dependent: Postgres migrations should be applied in a transaction per file where possible; SQLite may not support transactional changes for every operation — document per-DB caveats.

  - **Schema version validation (startup rule):** services MUST validate the DB schema version at startup (compare `current_db_version` vs `app_expected_version`); a mismatch yields a startup error and readiness=false. Document how to recover (migration steps) in admin docs.

  - **Statement obfuscation policy (stable defaults):** default obfuscation rules to prevent leaking sensitive data in production:
    - Truncate obfuscated statements to 512 bytes by default.
    - Strip or replace parameter values with placeholders (`?`) or tokens.
    - Remove SQL comments and normalize whitespace.
    - Allow full SQL logging only when `enable_query_tracing = true` (dev/CI opt-in).

  Map these small policies into the sprints: Timekeeper + Context in Sprint 1; `DbAttributes` + obfuscation helpers in Sprint 2; schema/version checks and `PersistenceHealth` in Sprint 3; migration-runner semantics & tests in Sprint 4; policy docs and final obfuscation polish in Sprint 5.

  #### Driver & test guidance

  These small conventions make driver injection and testing ergonomic and should be implemented alongside the Sprint work:

  - **`PersistenceDriver` trait & `DriverFactory`:** add a narrow trait in `crates/commons::util::persistence` exposing `async fn health(&self) -> PersistenceHealth` (and optional `shutdown()`). Provide `DriverFactory::build(config) -> Arc<dyn PersistenceDriver>` as the single injection point for services. Keep the trait small — domain repository methods remain typed and separate.

  - **`with_db_span` naming and behavior:** standardize the span helper as `with_db_span(...)` (or `with_db_span_async`) that every backend calls once per repository method. Responsibilities: start a `persistence.<operation>` span, attach `DbAttributes`, observe cancellation from `Context`, map backend errors into `PersistenceError`, and annotate spans on error.

  - **`is_ready` / readiness probe:** expose a cheap readiness helper that calls `driver.health().await` and returns a bool. Services should use this in their HTTP readiness endpoints so Kubernetes probes only pass when persistence and schema versions are acceptable.

  - **`persistence::test::memory_driver()` helper:** provide a test helper that returns an `Arc<dyn PersistenceDriver>` backed by the memory backend. Re-export under a test helper path so other crates can write integration-style tests without spinning up Testcontainers.

  These items should be captured in Sprint 1 design docs (trait shapes and helper names) and implemented incrementally in Sprints 2/3 (memory driver, factory, readiness, and Postgres wiring).

  #### Phase 2.1 Subphases (sprints)

To make progress predictable, the Phase 2.1 work is split into five short sprints. Each sprint maps to the prioritized checklist above and is small enough to complete in a day or two.

- **Phase 2.1.1 — Sprint 1: Design Context & Traits**
  - Scope: define the minimal `Context` type (trace headers, optional deadline/cancellation token), `PersistenceError` taxonomy, repository trait patterns, and `exec_with_span` helper API.
  - Deliverables: `Context` design document, example trait signatures, `PersistenceError` enum sketch, `exec_with_span` helper spec, and unit tests asserting expected contracts.
  - Estimate: 1–2 days.

- **Phase 2.1.2 — Sprint 2: Memory + SQLite Backends**
  - Scope: implement a fast in-memory backend and an in-memory SQLite backend wired to `Context` and `exec_with_span` helpers; provide unit tests and a small example demonstrating traced DB ops.
  - Deliverables: `memory` backend implementation, `sqlite` (in-memory) backend, unit tests under `crates/commons/tests/persistence/`, and an example binary showing traced DB operations.
  - Estimate: 0.5–1.5 days.

- **Phase 2.1.3 — Sprint 3: Postgres Backend & Pooling**
  - Scope: implement the Postgres backend using `sqlx::Pool`, add per-operation timeouts/cancellation mapping to `PersistenceError::Timeout`, and provide `PersistenceHealth` hooks exposing pool health metrics and readiness checks.
  - Deliverables: `postgres` backend module, pooling guidance in config, `PersistenceHealth` trait, mapping of driver errors to canonical `PersistenceError` variants, and unit tests.
  - Estimate: 1–2 days.

- **Phase 2.1.4 — Sprint 4: Migrations + Integration CI**
  - Scope: implement a traced migration runner, produce migrations health outputs, and add branch-local integration tests using Testcontainers for Postgres alongside a branch-local CI workflow to exercise migrations and pooled connections.
  - Deliverables: `migrations.rs` runner with tracing, integration tests under `crates/commons/tests/persistence/`, `scripts/` helpers for local reproduction, and a branch-local GitHub Actions workflow (kept off default CI).
  - Estimate: 1–2 days.

- **Phase 2.1.5 — Sprint 5: Docs, Instrumentation, and Polish**
  - Scope: finalize docs (config schema `RoboTorqConfig.persistence`, docs/persistence.md), formalize span naming and `db.statement` obfuscation policy, add `DriverFactory` guidance, and finish CI/test polish (clippy/tests/docs).
  - Deliverables: documentation snippets, sample config, query obfuscation guidance, span naming conventions, and a checklist for rolling Option A into feature branches.
  - Estimate: 0.5–1 day.

Notes:
 - This Option A proposal follows the project's File Creation Rules by placing code under `crates/commons/src/util/persistence` and keeping tests decoupled under `crates/commons/tests/**`.
 - No code or CI changes will be enacted by this update — this is a roadmap update only. Implementations and any CI adjustments will be created on `rewrite-core` or feature branches and will not be merged to `release/v0` without your instruction.

#### Gaps, Risks, and Suggested Improvements

- **Repository Trait Shape Risk:** repository traits that accept a large or tracing-heavy Context may leak implementation details into service layers. Keep `Context` minimal (trace headers, optional deadline) and provide helpers in `commons` to start spans. This reduces coupling and keeps backends swappable.
- **Cancellation & Deadlines Gap:** ensure `Context` supports cancellation/deadline semantics that map cleanly to drivers (e.g., sqlx query timeouts, cancellation tokens). Without this, timeouts may not cancel in-flight queries and can lead to resource leaks.
- **Migration Runner Guarantees:** migration runner must expose an idempotent, observable contract. Document expected transactional guarantees per DB (e.g., sqlite single-writer / Postgres transactional migrations) and record per-migration spans and results for auditability.
- **Query Obfuscation & Privacy Risk:** when tracing queries, avoid logging full SQL text in production. Implement an obfuscation/truncation strategy for `db.statement` and make full statements a CI/dev-only option behind `enable_query_tracing`.
- **Pooling & Resource Contention Risk:** different drivers have different pooling semantics; document pool sizing guidance and expose `pool.health` hooks. Provide a `PersistenceHealth` trait in `commons` that backends implement to expose pool metrics and simple readiness checks.
- **Driver Feature Divergence:** driver-specific features (e.g., Postgres LISTEN/NOTIFY, sqlite WAL) can encourage feature creep on repository traits. Keep domain-facing traits small and add driver-capability traits only when necessary behind feature flags.
- **Testing Strategy Gap:** rely on fast memory + in-memory sqlite unit tests for default CI; run Postgres Testcontainers in branch-local CI. Document how to reproduce integration failures locally with `scripts/` helpers.
- **Span Naming & Attributes Improvement:** standardize span names and attributes for DB ops (`persistence.<operation>`, `db.system`, `db.name`, `db.statement.obfuscated`, `db.rows_affected`) and add a helper `exec_with_span(ctx, name, attrs, async_op)` to reduce boilerplate and ensure consistent instrumentation.
- **Driver Factory & Injection Suggestion:** provide a small `DriverFactory` in `commons` responsible for creating typed backend instances from configuration. This centralizes connection setup, pooling, and instrumentation wiring.
- **Operational Risk — Migrations & Rollbacks:** migrations can cause downtime if long-running; provide a recommended strategy (online-friendly migrations, feature-flags for schema changes, per-migration timeouts) and ensure migrations are observable and cancellable.

#### Phase 2.1 — Remaining Tasks

- Postgres config applied: `ssl_mode`, `application_name`, and `search_path` wired; defaults and opt-ins documented.
- CRUD ops (Postgres): add traced, context-aware CRUD methods mirroring SQLite (`with_db_span`, obfuscation, timeouts).
- Schema validation: formalize version check at startup (compare `current_db_version` vs `expected_version`) and reflect result in readiness.
- Migration runner consistency: finalize fail-fast behavior, structured results, and per-DB caveats; ensure idempotence across both backends.
- Tests coverage: add targeted Postgres CRUD + timeout tests; extend obfuscation doctests; verify readiness flips correctly with schema mismatches.

---

#### Remaining Tasks

---

### Phase 2.2

- Postgres config verification: add tests for `ssl_mode`, `application_name`, and `search_path` handling in pool/session setup.
- Error taxonomy polish: map `sqlx::Error` into canonical `PersistenceError` variants (Timeout, PoolExhausted, ConstraintViolation).
- Per-op timeouts: enforce via `Context` and annotate spans; document defaults.
- Tests: integration coverage for config handling and timeouts.

---

### Phase 2.3

- Repository trait scaffolding: narrow, context-aware methods; keep domain types decoupled from drivers.
- Memory backend helper: fast test driver with `persistence::test::memory_driver()`.
- DriverFactory: single injection point from `PersistenceConfig`.
- Tests: trait contracts, memory backend semantics, span attributes.

---

### Phase 2.4

- Migration runner semantics: unified directory-based application, idempotence, structured results.
- Schema versioning: `schema_version` recording + startup validation; readiness reflects mismatch.
- Health output: minimal `PersistenceHealth` fields (version, migration status).
- Tests: idempotent re-run, dirty-state detection, mismatch readiness behavior.

---

### Phase 2.5

- Readiness wiring: `/readyz` depends on driver health + schema version.
- Pool health: minimal Postgres pool stats surfaced (size/available) — avoid over-engineering.
- CI strategy: default CI runs memory+SQLite; branch-local runs Postgres Testcontainers.
- Tests: readiness toggling under degradation; pool exhaustion path.

---

### Phase 2.6

- Obfuscation policy: finalize defaults (truncate/strip/normalize), dev-only full SQL behind flag.
- Span naming: standardize `persistence.<operation>` and `DbAttributes` helper.
- Docs: concise persistence config reference (SQLite default, Postgres toggle), migration usage.
- Clippy/coverage tidy-up.

---

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
