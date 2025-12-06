🌅 Horizon 0 — Substrate Only

Goal: produce deterministic substrate, simulator, adapters, assemblies, and demo workflows.

🧩 Epic 0 — Foundations: Ports, Types, and Core Primitives

**Phase 0 — Core Interfaces & Determinism Contracts**
- Sprint 0: Create `crates/ports` skeleton (structure, modules, docs; no logic).
- Sprint 1: Define `ProcessStore` trait (load/store, atomic boundaries, recovery markers).
	- Note: `ProcessStore` doc must require an atomic `store_events_and_commit_outbox(events, outbox_entries)` operation (single Postgres transaction) to prevent partial commit scenarios.
- Sprint 2: Define `MessageBus` trait (per-key FIFO, at-least-once, envelope routing).
- Sprint 3: Define `Clock` + `Randomness` traits (logical/HLC + wall-clock injection; deterministic RNG for sim).
	- Note: move `Randomness` early — deterministic RNG is required for Lamport tiebreakers, deterministic scheduling, and crypto stubs used in tests.
- Sprint 4: Define `MetricsSink`, `ReadModel`, `CircuitBreaker` interfaces (minimal traits + guarantees).
- Sprint 5: Determinism contracts (docs/tests: envelope ordering, causal chain rules, logical timestamp invariants, idempotency rules). Locks in correctness assumptions.
 - Sprint 6: Sketch Intent stubs early (ExternalEffect, SupervisorOps) so adapters and sim can exercise side-effects from day one.

**Phase 1 — Substrate-Core Types**
- Sprint 0: Implement `Envelope` (id, causation_id, correlation_id, timestamp(logical), metadata).
- Sprint 1: Implement `Intent` enum (Persist, Emit, ScheduleWake, Snapshot, ExternalEffect, SupervisorOps).
	- Document `ExternalEffect { id: String, payload: Vec<u8> }` as a first-class stub Intent usable by sim and adapters.
	- Expand `SupervisorOps` into explicit intents: `SpawnChild`, `ChildCompleted`, `CancelChild` (spawn/cancel semantics to be implemented in supervisor helper).
- Sprint 2: Implement `EffectBatch` + atomic/outbox semantics (grouping, ordering guarantees, validation).
- Sprint 3: Implement `Workflow` trait (serialize state/event, reducer API shape).
- Sprint 4: Implement Idempotency subsystem (Lamport + tiebreaker, dedupe key rules, causal hashing).
- Sprint 5: Implement Versioning primitives (snapshot version, event version, commit markers).

**Phase 2 — Substrate-Core Execution Engine**
- Sprint 0: Implement Reducer runner (apply-event pipeline: state transition + intent extraction).
- Sprint 1: Implement durable outbox (atomic write + durable delivery markers).
- Sprint 2: Implement recovery logic (replay events, handle incomplete batches, re-drive external intents).
- Sprint 3: Implement Snapshot intent + atomic marker (serialize state, persist snapshot+marker in same batch).
- Sprint 4: Implement Process Supervisor primitives (fanout/aggregate, retry/backoff, token bucket, durable semaphore).
- Sprint 5: Integrate executor orchestrator (loop: read messages → reduce → write batches → emit).

🔌 Epic 1 — Adapters: In-Memory + Postgres + NATS + Prometheus

**Phase 0 — In-Memory Adapters (for Sim)**
- Sprint 0: Implement `inmem::ProcessStore` (state map, event log, snapshot storage).
- Sprint 1: Implement `inmem::MessageBus` (per-key FIFO, deterministic scheduling hooks).
- Sprint 2: Implement `inmem::Clock` (fake deterministic clock + tick/advance API).
- Sprint 3: Implement `inmem::MetricsSink` (counters, gauges, histograms in-memory).
- Sprint 4: Integrate `Randomness` adapter (seeded deterministic RNG).
- Sprint 5: End-to-end adapter wiring tests (executor loop works using inmem ports).

**Phase 1 — Postgres + Prometheus**
- Sprint 0: Implement `postgres::ProcessStore` (tables + tx boundaries; outbox, commit markers, snapshot storage schema migrations).
- Sprint 1: Implement Postgres snapshot store (blob store + version metadata).
- Sprint 2: Implement Postgres dedupe subsystem (idempotency keys, causal hash constraints).
- Sprint 3: Implement `prometheus::MetricsSink` (exporter wiring, counters, gauges).
- Sprint 4: Integrate OTLP tracing hooks (span per intent/executor step).
- Sprint 5: Run PG integration tests (transactional boundaries, recovery from partial commits).

**Phase 2 — Messaging Adapters**
_Horizon 0 decision_: ship NATS (+ JetStream) only; Kafka is a Phase 2 stretch target. NATS makes strict per-key FIFO and fast iteration easier for H0.
- Sprint 0: Implement NATS message bus (key-based FIFO, subject mapping, durable consumers).
- Sprint 1: NATS ordering + backpressure (single-threaded consumption per process key).
- Sprint 2: Implement Kafka message bus (MVP: key-partition mapping, consumer groups).
- Sprint 3: Validate at-least-once semantics (redelivery handling, idempotency pipeline validation).
- Sprint 4: Stress test adapters (network drops, reorder, duplicate, reconnect).
- Sprint 5: Unified MessageBus conformance suite (guarantee adapters honor substrate semantics).

🧪 Epic 2 — Deterministic Simulator + Assemblies + Demo Workflows

**Phase 0 — Simulator Core**
- Sprint 0: Implement deterministic scheduler (priority queue, fixed ordering, seeded RNG).
- Sprint 1: Implement scenario DSL (inject delays, drops, reorders, duplicates, crashes).
- Sprint 2: Implement fake cluster runner (simulated workers, crash/restart cycles).
- Sprint 3: Add deterministic replay hash (hash of event sequence + state transitions).
- Sprint 4: Implement run diff tool (compare run logs, highlight divergence).
- Sprint 5: Integrate sim with inmem adapters (executor, bus, store).

**Phase 1 — Assemblies + Example Workflows**
- Sprint 0: Create `assemblies/example` (config loading, adapter wiring).
- Sprint 1: Implement `--sim` / `--prod` CLI (shared binary; swap adapters by config).
- Sprint 2: Demo workflow: counter (pure deterministic workflow, persist/emit).
- Sprint 3: Demo workflow: delay/timer (ScheduleWake intent + deterministic clock).
- Sprint 4: Demo workflow: parent/child (Supervisor helper, `pending_children` logic).
- Sprint 5: Add `rebuild-read-model` command (CLI: read model rebuild + checkpoints).

**Phase 2 — Validation, CI, Observability**
- Sprint 0: CI — `cargo fmt` / `clippy` / `test` (base pipeline).
- Sprint 1: Integration tests: outbox boundaries (crash during commit, partial delivery).
- Sprint 2: Snapshot round-trip suite (rebuild from snapshot must match replayed events).
- Sprint 3: Determinism CI matrix (replay same seed → compare deterministic replay hash).
- Sprint 4: Grafana dashboards (executor latency, outbox lag, queue depth, retries).
- Sprint 5: Horizon 0 release packaging (single binary, docker-compose for PG + NATS + Prometheus).

✔️ Result: Horizon 0 Achieved

After completing all epics, phases, and sprints above, Horizon 0 delivers:
- Full substrate-core
- All ports defined
- In-memory adapters
- Deterministic simulator
- Basic Postgres/NATS/Prometheus adapters
- Demo workflows
- Example assemblies
- Deterministic replay + diff tooling
- CI + observability + release packaging

Exactly what Horizon 0 is meant to deliver.