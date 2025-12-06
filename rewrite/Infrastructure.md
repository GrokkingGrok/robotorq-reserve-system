# One Workflow. One Executor. Two Runtimes.

# General Deterministic Distributed Substrate

This document defines the general-purpose distributed runtime we are building: a unified program that runs deterministic workflows in both simulation and production, using the exact same executor, envelopes, and intent semantics. The product has two faces:
- A distributed logic runner (production): services wired to real adapters (Postgres/NATS/Prometheus/HSM) executing workflows with durability and observability.
- A deterministic simulator (primary early product): in-memory adapters + fake clock + scenario DSL to model, stress, and prove workflow behavior and recovery.

The key promise: write a workflow once; run it identically in sim and prod without code changes.

## Viability
- Outcome: deterministic workflow/state machine execution, distributed intent/executor architecture, fault-tolerant message routing, simulation and production running identical logic, reproducibility and strong correctness of distributed behavior.
- Proven approach: mirrors Temporal/Cadence (durable execution), Akka Persistence (event-sourced actors), CockroachDB deterministic testing, and ledger-style event-sourced recovery.

## Two-Layer Architecture
- Layer 1 — Substrate / Infrastructure
	- Contains: ports, adapters, envelopes & idempotency, executor + intent engine, snapshotting + recovery, logical + wall clock, simulator environment, assemblies (wiring).
	- Contains no domain logic.
	- Layer 2 — Domain Logic (Plug-in)
	- Later modules provide: domain-core (types, invariants, etc.), domain workflows (state machines using substrate executor).
	- Domain logic is pure and deterministic. Domain plugins import the substrate; substrate never imports domain logic.

## Substrate-Only Layout (Minimal Crates)
```
crates/
	ports/                             # traits only (no IO)
		src/
			lib.rs
			process_store.rs               # ProcessStore
			message_bus.rs                 # MessageBus
			clock.rs                       # Clock + readings
			metrics_sink.rs                # MetricsSink
			read_model.rs                  # optional ReadModel
			circuit_breaker.rs             # optional CircuitBreaker
			signer.rs                      # optional Signer stub

	substrate-core/                    # executor + envelopes + intents
		src/
			lib.rs
			envelope.rs                    # id, causation_id, correlation_id, timestamp
			intent.rs                      # Intent + EffectBatch { atomic } + ExternalEffect
			reducer.rs                     # apply(state, event) -> {state, intents}
			executor.rs                    # durable outbox, recovery markers
			idempotency.rs                 # derive from causal chain with total order (Lamport + tiebreaker)
			snapshot.rs                    # snapshot intent + atomic marker
			workflow.rs                    # Workflow plug-in trait

	adapters/                          # concrete IO implementations
		inmem/
			src/
				lib.rs
				process_store.rs
				message_bus.rs
				metrics_sink.rs
				clock.rs                     # fake deterministic clock
		postgres/
			src/
				lib.rs
				process_store.rs             # outbox + tx boundary
				snapshot_store.rs
				dedupe.rs
		sqlite/
			src/
				lib.rs
				process_store.rs
		nats/
			src/
				lib.rs
				message_bus.rs               # ordered delivery by process key (document FIFO trade-offs; prefer single-threaded consumption per key)
		kafka/
			src/
				lib.rs
				message_bus.rs               # partitioned by key
		prometheus/
			src/
				lib.rs
				metrics_sink.rs
		signer-hsm/
			src/
				lib.rs
				signer.rs                    # HSM/remote signer proxy (optional)

	sim/                               # deterministic simulator using inmem
		src/
			lib.rs
			scenario.rs                    # DSL: schedule, delays, drops, reorders
			scheduler.rs                   # deterministic scheduler
			runner.rs                      # simulate crashes + recovery

	assemblies/                        # wiring (config-driven)
		example/
			src/
				main.rs                      # CLI: --sim / --prod modes
				config.rs                    # adapter configs
				wiring.rs                    # compose ports + executor
```

### Substrate-Core Defines
- Process / workflow state abstraction (opaque bytes to substrate).
- Reducer function (generic domain plug-in): `apply(state, event) -> { state, intents }`.
- Intents: persist, emit message, retry, scheduled wake, snapshot.
- EffectBatch with atomic/outbox semantics.
- Envelope fields: `id`, `causation_id`, `correlation_id`, `timestamp (logical)`.
- Idempotency derived from causal chain.
- Versioning primitives for events and snapshots.
- Executor: atomic batches, recovery markers, outbox delivery.
 - Process Supervisor primitives: generic fanout/aggregate, retry-with-backoff helper, token-bucket rate limiter, durable semaphore/lock.

### Domain Workflow Plugin API
```rust
pub trait Workflow {
	type State: serde::Serialize + serde::de::DeserializeOwned;
	type Event: serde::Serialize + serde::de::DeserializeOwned;

	fn apply(state: &Self::State, event: &Self::Event) -> (Self::State, Vec<Intent>);
}
```
Domain logic is entirely separate from substrate.
Constraint: workflows must not read clocks inside `apply()`. Wall-time is injected as data by assemblies; scheduling happens via intents (e.g., `ScheduleWakeup(Duration)`).

### Ports Define
- `ProcessStore`, `MessageBus`, `Signer` (optional stub), `Clock`, `MetricsSink`, `ReadModel` (optional), `CircuitBreaker` (optional), `Randomness` (deterministic in sim).

### Adapters Provide
- Concrete implementations for DB, messaging, metrics, and optional signer/HSM.

### Sim Provides
- fake deterministic clock
- in-memory stores and bus
- deterministic scheduler
- scenario DSL (e.g., schedule events, inject delays, simulate crashes)
- same executor, same envelopes, same semantics as production
 - Guarantee: the simulator never introduces nondeterminism unless explicitly scheduled by the scenario DSL.
 - Built-in eventual projection runner for `ReadModel` to assert convergence of domain-declared projections.
 - Trace-deterministic logging: per-event structured trace logs, deterministic replay hash (hash of event sequence), and run diff tooling.

## Remove/Postpone (Substrate-Only)
- Remove/punt anything that assumes domain meaning: domain-core, domain-workflows, invariants, cryptographic proofs, merkle/canonicalization, ledger logic, batch workflows, audits, governance/admin semantics, domain-specific parent/child models (keep generic only).
- Keep only generic substrate concepts.

## Governance of Ambiguities (Rules Adopted)
- Canonicalization: when domain arrives later, canonical bytes generation lives in `domain-core`; adapters never construct canonical bytes. Adapters operate on `{ canonical_bytes, signing_time, key_id }` only.
- Time: dual clocks. Logical/HLC for workflow determinism; wall-time injected by assemblies into signing/canonicalization as data. Simulator injects deterministic wall-times.
- Envelope timestamps use logical time only. Adapters may attach wall-time metadata separately.
- Idempotency keys: derive from causal event/envelope or canonical hash; avoid ad-hoc strings.
- Effect transaction boundary: `EffectBatch { intents, atomic }` with durable outbox semantics for atomic groups.
- Snapshots: snapshotting is a first-class intent; persist `SnapshotWritten(latest_event_id)` atomically with snapshot bytes.
- Snapshots must be pure compression of prior events—not sources of new nondeterminism.
- Recovery: durable execution markers; resume rules retry incomplete effects idempotently.
- Read-model rebuild: checkpoints and `rewind_to(event_id)` semantics; divergence detection jobs.
- Batching keys: treat partial batches as workflow keys (e.g., `BatchId`) to localize state and avoid global serialization.
- Parent–child coupling: parent persists `pending_children`; child emits `ChildCompleted`; parent resumes idempotently and owns backoff.
 - Intent execution ordering (deterministic): for each event E: (1) `apply(state, E) -> (state', intents[])`; (2) group intents into batches; (3) for each batch, write all atomic effects then commit; (4) emit all external effects. Ordering is stable across runs.

## Staged Rollout
- Horizon 0 — Substrate Only
	- Build substrate-core, ports, adapters, sim, assemblies.
	- Outputs: deterministic sim, executor, envelopes, demo workflows (counter, delay, parent/child).

- Horizon 1 — Domain Prototyping (Sim Only)
	- Domain modules plug in.
	- Use pure attributes only.
	- Simulator validates correctness.

- Horizon 2 — Production
	- Swap adapters.
	- Same executor, workflows, semantics.
	- Production-ready cluster deployment.

## Deliverables & Milestones
These are concrete, user-visible artifacts and APIs that together form the general-purpose simulator/runner.

- D1: `crates/ports` API
	- Traits: `ProcessStore`, `MessageBus`, `Clock`, `MetricsSink`, `ReadModel` (optional), `CircuitBreaker` (optional), `Signer` (stub).
	- Versioned Rust docs + doctests demonstrating contract expectations.

- D2: `crates/substrate-core` executor
	- Types: `Envelope`, `Intent`, `EffectBatch { intents, atomic }`, `Transition`.
	- Engine: durable outbox, recovery markers, idempotency derivation, snapshot intent + atomic marker.
	- Public `Workflow` plug-in trait with example implementations (counter, delay, parent/child).
	- Include Process Supervisor primitives: fanout-aggregate, retry-with-backoff helper, token-bucket rate limiter, durable semaphore/lock.

- D3: Adapters (MVP set)
	- `adapters/inmem`: process store, message bus, metrics sink.
	- `adapters/postgres`: process store + outbox transactional boundary.
	- `adapters/nats`: message bus with ordered delivery by process key.
	- `adapters/prometheus`: metrics sink with runtime counters/gauges.

- D4: `crates/sim` deterministic simulator
	- Fake clock and deterministic scheduler.
	- Scenario DSL: schedule events, inject delays, drops, reorders, duplicates, crashes.
	- Reproducible seeds; repeatable run reports; integrity checks.
	- Trace-deterministic logging and a run diff tool.

- D5: `assemblies/example` wiring
	- Config-driven composition of ports/adapters/executor.
	- Two modes: `--sim` (inmem + fake clock) and `--prod` (postgres/nats/prometheus).
	- Minimal CLI with commands: `run-sim-scenario`, `run-worker`, `rebuild-read-model`.
	- Provide a well-tested generic parent/child workflow helper to avoid timeout/orphaning pitfalls.

- D6: Observability
	- OTLP exporters wired; baseline Grafana dashboards for executor latency, queue depth, retries, snapshot rate, outbox lag.
	- Tracing propagation across intent execution; correlation/causation IDs in structured logs from day 1; deterministic replay hash emitted per run.

- D7: CI/CD and Tests
	- Cargo fmt/clippy/test; integration tests for outbox boundaries, snapshot recovery, idempotency.
	- Property tests for deterministic replay; simulator crash-matrix tests.

- D8: Developer Docs
	- Architecture overview (substrate-first), plug-in guide for workflows, adapter authoring guide, simulator cookbook.
	- Quickstart: build, run sim scenarios, run prod worker locally with docker-compose.

- D9: Release Packaging
	- Single binary offering `sim` and `worker` subcommands.
	- Example configs and docker-compose for local prod adapters.

## Testing Strategy
- Unit: executor transitions, idempotency derivation, effect batch atomicity.
- Integration: ProcessStore timeout tests, durable outbox transaction boundaries, snapshot recovery.
- Property: envelope versioning, event ordering determinism, replay integrity.
- Simulation: crash scenarios (snapshot, emit, side-effect, signing), backpressure, duplicate deliveries.
 - CI guard: snapshot round-trip — replay from snapshot must match original event stream.
 - Determinism CI: compare deterministic replay hashes across identical seeds; fail on divergence.

## Risks & Mitigations
- Remove domain-specific risks (governance, economic invariants). Keep distributed systems risks only.

## Next Actions
- Scaffold `crates/substrate-core` and `crates/ports` with executor, envelopes, intents.
- Implement `crates/adapters/inmem` and `crates/sim` with fake clock and scenario DSL.
- Add durable outbox in `adapters/postgres`; wire `assemblies/example`.
- Establish CI tasks and minimal Grafana dashboards for substrate metrics.
