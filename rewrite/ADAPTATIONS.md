# RoboTorq Reserve System – Rewrite Adaptations

This document consolidates lessons learned and design changes into a single, coherent narrative. It explains why the rewrite exists, what we changed, and how the updated architecture preserves the system’s economic invariants while improving safety, observability, and maintainability.

## Goals
- Preserve core economic invariants and auditability.
- Reduce complexity in concurrency and persistence layers.
- Standardize crypto primitives and message formats.
- Improve testability and CI feedback loops.
- Strengthen observability (metrics, tracing, logs) across services.

## Economic Invariants
- `1 TokenTorqIngot = 3,600 JouleTorqOre units`.
- `1 RoboTorq Certificate = 1,000 ingots = 3,600,000 Ore units`.

All aggregation, minting, signing, distribution, and redemption flows must enforce these invariants at compile-time (types), at runtime (validation), and via persistence (constraints + verifiable proofs).

## Unit Hierarchy Alignment
We map the processing pipeline directly onto the unit hierarchy to keep economic meaning intact:
- L0 `JouleTorqOre`: atomic work proof; hashed + signed.
- L1 `TokenTorqIngot`: 3,600 Ore units; Merkle branch root + signed.
- L2 `RoboTorq Certificate`: batch of 1,000 ingots; Merkle batch root + signed.
- L3 `RoboTorqUnits`: digitally distributed; signed distribution events.
- L4 `Bearer Bond`: physical bearer mapping; Merkle cert collection + signed.

## Architectural Adaptations

### 1) Concurrency & Safety
- Adopt idiomatic Rust patterns: clear ownership, no shared mutable state without explicit synchronization.
- Use bounded channels and backpressure for flow control.
- Prefer `tokio` tasks with explicit lifecycles; avoid unstructured spawning.
- Leverage domain-specific types to encode invariants (e.g., `OreCount`, `IngotCount`, `CertificateCount`).

### 2) Persistence Strategy
- Postgres as primary storage; SQLite for dev/tests where useful.
- Schema enforces aggregation ratios (3,600 and 1,000) with CHECK constraints.
- Idempotent writes protected by unique keys on cryptographic identities.
- Transaction boundaries aligned with unit-level operations (L0→L1→L2).

### 3) Crypto Primitives & Proofs
- Standardize hashing and signing across layers.
- Merkle proofs consistently encoded; roots stored with signatures.
- Deterministic serialization with versioned envelopes; forward-compatible.
 - Canonical bytes are produced in `domain-core` only. Adapters never construct canonical bytes; they receive `{ canonical_bytes, signing_time, key_id }` and perform signing/verification.

### 4) Observability
- Metrics: per-layer counters, latencies, error rates, queue depths.
- Tracing: propagate spans through ingestion→aggregation→minting→distribution.
- Logs: structured, leveled, and correlated to spans.
- Align with `ci/otlp-collector` and Grafana dashboards.

### 5) Testing & CI
- Unit tests for domain types and invariant checks in `commons`.
- Integration tests for persistence (including timeouts) and service boundaries.
- Property tests for aggregation and Merkle proofs.
- CI gates on cargo build/test, fmt, clippy, and coverage thresholds.

## Data and Message Model Adaptations
- Message Envelope: versioned, typed payloads; signatures attached to canonical byte representation.
- IDs: content-addressed where feasible (hash-based); human-readable aliases for ops.
- Serialization: explicit schema evolution policy; reject ambiguous decoders.

## Flow Rewrites

### L0 → L1 Aggregation
- Ingest atomic `JouleTorqOre` units.
- Validate hashes and signatures at the edge.
- Aggregate exactly 3,600 units into `TokenTorqIngot`; compute Merkle branch; sign.

### L1 → L2 Minting
- Aggregate exactly 1,000 ingots into a `RoboTorq Certificate`.
- Compute Merkle batch root; sign; persist transactional record with proofs.

### L2 → L3 Distribution
- Create digital issuance events backed 1:1 by certificates.
- Sign distribution events; record auditable ledger entries.

### L2 → L4 Bearer Mapping
- Curate Merkle collections of certificates mapped to physical instruments.
- Sign mapping; issue serialized proof package for custody.

## Guardrails Ensuring Invariants
- Type-level safety: distinct newtypes for counts to prevent unit confusion.
- Runtime checks: hard validation on aggregation sizes and digest matching.
- Persistence constraints: relational CHECK and FK constraints for ratios and lineage.
- Audit trails: every transformation carries a verifiable cryptographic proof.

## Implementation Notes (Rust)
- Modules under `crates/commons` for shared types, crypto, envelopes, and config.
- Mirror module paths in `src/**` per File Creation Rules; expand existing `mod.rs` instead of new siblings.
- Keep production logic separate from tests: prefer `tests/**` for integration and service tests.
- Document with doctests mirroring full module paths.

## Migration Strategy
- Incremental replacement of legacy components with feature flags.
- Dual-write mode during cutover where necessary, with reconciliation tasks.
- Backfill proofs for historical data where feasible, documented and auditable.

## Risks and Mitigations
- Concurrency deadlocks: bounded queues, explicit lifecycles, thorough instrumentation.
- Schema drift: migrations with invariant checks; rollback plans.
- Crypto consistency: test vectors; versioned envelopes; deterministic serialization.
- Performance regressions: load testing; latency budgets; profiling.

## Decision Record (Summarized)
- Adopt strict type safety for economic units.
- Centralize crypto and message envelope logic.
- Enforce invariants at schema and code levels.
- Standardize observability across layers.

## Next Steps
- Finalize `commons` invariants and type modules.
- Extend persistence tests for constraint violations and timeouts.
- Wire tracing propagation end-to-end.
- Update Grafana dashboards to reflect new metrics taxonomy.

---

This rewrite centers the system around economic clarity, cryptographic verifiability, and operational reliability. Each adaptation above directly supports preserving the TokenTorq/JouleTorq/RoboTorq invariants while making the system easier to reason about and evolve.
 
## Conflict Resolution Summary
- Real-time signatures vs logical time: use dual clocks. Workflows remain pure with logical time; signing requires explicit wall-time provided by assemblies/adapters and passed as data. Domain constructs canonical bytes from provided `SigningTime` but never calls the clock.
- Rigid 3,600/1,000 ratios vs corrections: keep strict happy-path invariants, but add governance-authorized Adjustment/Seizure workflows as explicit escape hatches with conspicuous audit events.
- Idempotency drift: derive keys from the causal event (stable envelope ID) or a canonical hash of the intent payload, not ad-hoc string formatting. Version keys explicitly.
- Snapshot pitfalls: make snapshotting a first-class intent with an idempotency key and persist an atomic `SnapshotWritten(latest_event_id)` marker alongside snapshot bytes.
- Adapter-domain dependency purity: expose a serde-friendly domain facade crate allowed for adapters; keep core domain small, pure, and transport-agnostic.

## Friction Points & Resolutions
**A. Adapters vs Domain Canonicalization**
- Risk: DTO duplication and drift if adapters rebuild domain objects to construct canonical bytes.
- Rule: canonicalization lives in `domain-core`. Adapters never construct canonical bytes; they operate on `{ canonical_bytes, signing_time, key_id }` only.

**B. Configurable Parameters vs Compile-time Invariants**
- Risk: dynamic parameters conflict with compile-time encoded invariants.
- Rule: parameters are frozen per workflow instance at start and updated via explicit governance events; types still encode ratios for happy-path constructors.

**C. Serial-Per-Key vs Multi-Layer Batching**
- Risk: serializing whole streams creates bottlenecks and state contention.
- Rule: treat batches as workflow keys (`BatchId`); each batch holds its partial state and progresses independently.

**D. Child Workflows Without Failure Semantics**
- Risk: lost `ChildCompleted`, out-of-order completion, unclear backoff ownership.
- Rule: parent persists a `pending_children` map; child emits `ChildCompleted`; parent resumes idempotently after replay; parent owns backoff, bounded retries, and reconciliation via `ProcessStore` queries.
# Next‑Generation Clean‑Slate Architecture (Rewrite Spec)

This document specifies the next‑generation, clean‑slate architecture with full decoupling and a simulator‑first design. It is intentionally technology‑agnostic at the top level; concrete adapter choices are examples, not requirements.

## Read This First (Freshman-Friendly Overview)
- Goal: build a system from small, replaceable parts that work together. Each part does one job, and parts talk to each other through clear messages.
- Why: swapping parts (like database or network) should be easy; testing should be predictable; crashes or retries shouldn’t break money rules.
- Big idea: keep “brain” (rules) separate from “hands” (doing stuff like saving to a DB or sending messages).

### The Pieces
- `domain-core`: the brain. Defines money units and rules (like 1 ingot = 3,600 ore). No database, no network.
- `domain-workflows`: how work happens over time. State machines that say “given this state and this event, move to that state and request these side-effects.” Pure logic.
- `ports`: the plug shapes. Traits (interfaces) that describe storage, messaging, crypto, metrics, and time. No actual code, just the contracts.
- `adapters/*`: the hands. Real code for SQLite/Postgres, HTTP, NATS/Kafka, Prometheus, etc. They implement the ports.
- `assemblies/*`: the wiring. Pick which adapters you want and connect them to workflows for a running service.
- `sim`: the practice arena. Runs everything in-memory with a fake clock so you can test behavior deterministically.

### How It Works (End-to-End)
1) A message arrives (wrapped in an envelope with IDs for tracking).
2) A worker (state machine) reads the current state and the message.
3) It decides the next state and writes down “intents” (requests for side-effects like “save this” or “emit that event”).
4) Adapters execute those intents safely (idempotent), with retries if needed.
5) The simulator uses the same steps but with in-memory adapters and a fake clock to reproduce scenarios.

## Target Architecture (Bird's‑Eye Summary)
- Components: `domain-core` (economic model, invariants), `domain-workflows` (pure state machines), `ports` (traits), `adapters/*` (impls), `assemblies/*` (wiring), `sim` (deterministic harness).
- Responsibilities: domain defines data and rules; workflows define transitions and intents; ports specify boundaries; adapters execute side-effects; assemblies compose deployments; sim validates end-to-end deterministically.
- Communication: message envelopes with correlation/causation/idempotency; workers consume events and produce intents executed by adapters; stores snapshot/process state.

## Architectural Principles & Constraints
- Pure domain logic; no IO in `domain-*`.
- Strict ports/adapters boundaries; traits only in `ports`.
- Determinism: workflows are pure functions over state + event + clock.
- Side-effect isolation via intents; adapters are idempotent.
- Testability: in-memory adapters and fake clocks must reproduce prod paths.
- Stability: trait contracts are versioned; domain invariants are single-sourced.
- Backward-compatibility: prioritizes forward progress; assemblies pin versions to manage upgrades.

| Decision | Requirement | Implemented Solution | Rationale | First Deliverable |
|---|---|---|---|---|
| Split responsibilities | Components must be swappable and independently testable | Focused crates: `domain-core`, `domain-workflows`, `ports`, `adapters/*`, `assemblies/*`, `sim` | Enables composition and independent evolution | Create `crates/domain-core` with economic types and invariant-safe constructors |
| Transport isolation | Domain must not depend on transport/observability | Transports and metrics as adapters over `ports` | Swap transports without touching domain/workers | Define `ports` traits crate with minimal dependencies |
| Single-source invariants | Economic rules must be centralized | Invariants + constructors in `domain-core` | Compile-time clarity and runtime validation | Implement invariant APIs + doctests in `domain-core` |
| Explicit workflows | Lifecycle must be deterministic and testable | State machines: `apply(state,event,ctx)->Transition` with intents | Deterministic transitions and testability | Scaffold `domain-workflows` with `Worker` trait and sample `minting` machine |
| Side-effect isolation | Pure logic must not perform IO | Effect intents executed by adapters | Clear separation of logic and effects | Add intent enums and executor adapter skeletons |
| Replaceable persistence | Storage must be swappable and simulatable | `ports::ProcessStore`, `LedgerStore`, `BatchStore` + in-memory impl | Replaceable storage backends | Define store traits + in-memory impl for tests |
| Adapter observability | Observability must be transport-agnostic | Workers emit events/counters to `MetricsSink` | Consistent telemetry boundaries | Implement `MetricsSink` adapter |
| Clear composition | Config must be localized at the edges | Per-adapter configs; composition in `assemblies/*` | Clear dependency boundaries | Create `assemblies/minting-cluster` wiring configs |
| Multi-layer testing | Domain and adapters must be testable in isolation | Doctests, unit tests, contract tests, sim scenarios | Faster iteration and reliability | Add test harness for `domain-workflows` + fake clock |
| Canonical messaging | Events must share envelope semantics | Single canonical envelope in `ports` | Uniform messaging across adapters | Define `MessageEnvelope` + `Event` traits in `ports` |
| Idempotent execution | At-least-once delivery must be safe | Intent idempotency keys + dedupe adapters | Safe retries and recovery | Add key derivation + adapter storage |
| Canonical crypto | Signatures must be verifiable across transports | Domain canonical bytes + merkle building; adapters manage keys | Correct, verifiable signatures | Implement canonicalization + `Signer/Verifier` traits |
| Scheduling/backpressure | Work must be controlled under load | Queue‑driven workers; `ScheduleRetryIn(Duration)` intent | Resilient under load | Add scheduler adapter with fake clock for sim |
| Error taxonomy | Errors must be portable across layers | `DomainError` + adapter mappings | Clean boundaries and clear semantics | Define error categories and mappings |
| Versioning discipline | Changes must be safe to roll out | Per‑crate semver; assemblies pin versions; stable traits | Safer, incremental changes | Establish crate boundaries and MSRV plan |
| Build surfaces | Features must be precise | Fine‑grained per‑adapter feature flags | Precise build surfaces | Introduce per‑adapter features |
| Docs shape | Architecture must be easy to learn | Domain‑first docs: invariants, workflows, intents, adapters, assemblies, sim | Clear mental model for new devs | Write `ARCHITECTURE.md` aligned with crates and workflows |

## Component Boundaries & Dependency Rules
- `domain-core`: value types, invariants, canonicalization; depends on nothing. Keep it tiny and pure.
- `domain-workflows`: state machines and intents; depends on `domain-core` and `ports` (traits only).
- `ports`: traits for storage, messaging, crypto, metrics, time, envelope; depends on nothing.
- `adapters/*`: concrete implementations; depend on `ports` only; never on `domain-*` to avoid cycles.
- `assemblies/*`: wiring; depend on `ports`, `adapters/*`, and `domain-*`.
- Never in domain: config, logging, tracing, persistence, HTTP.
 - Practical concession: provide a serde‑friendly `domain-core-facade` (DTOs only) that adapters may depend on for storage/IO. Keep `domain-core` itself pure and minimal; `domain-workflows` should not rely on the facade.

## Domain Model (Concise)
- Behavior & Governance: Certificates are minted from validated ingot batches, signed with canonical bytes, and become reserves backing digital units and bearer bonds. Minting is triggered by completion of ore→ingot batching proofs reaching invariant thresholds. Actors include Issuers (minting), Auditors (verification), Distributors (circulation), and Holders (consumers). Economic relationships enforce 1:1 backing and prevent over‑issuance.
- Units: `JouleTorqOre` (atomic), `TokenTorqIngot` (3,600 ore), `RoboTorqCertificate` (1,000 ingots = 3.6M ore).
- Actors: minting cluster (batch ore → ingots → certificates), distribution cluster (issue units/bonds), verification services (crypto proofs).
- Invariants: `ORE_PER_INGOT=3,600`, `INGOTS_PER_CERT=1,000`; conversions only via invariant constructors; merkle roots/signatures over canonical bytes.

## Governance Escape Hatches (Auditable)
- Adjustment workflow: allows fractional or corrective mint/burn under a cold multi‑sig governance key; always emits conspicuous audit events and distinct artifacts.
- Seizure workflow: supports court‑ordered actions with signed governance proofs; produces explicit lineage entries for external audit.
- Early closure: pending batches can be closed with a paired Adjustment event; happy‑path invariants remain intact for standard flows.
- Visibility: dashboards render these in a distinct color/channel; exports flag them for regulators.

## Workflow Definition
- Concurrency Guarantee (Serializable by Key):
	- Events for a given workflow process ID must be consumed in a single linear sequence.
	- The system guarantees no overlapping execution for the same key.
	- Adapters must enforce ordering or detect conflicts and surface them to the state machine as `Conflict` errors.
- API: `apply(state, event, ctx) -> Transition { new_state, intents }`.
- Start: triggered by a “start” event.
- Identity: one key per workflow (e.g., `TripleTorqId`).
- Concurrency: one workflow key is processed serially to avoid race conditions.
- Lifetime: continues until a terminal state; may spawn child workflows via an intent.
 - Batching keys: partial batches (e.g., L0→L1, L1→L2) are modeled as their own workflow keys (e.g., `BatchId`) to avoid unintended global serialization and to localize partial state.

## Simulator‑First Philosophy
- Determinism: fixed fake clock, deterministic RNG, serialized inputs.
- Reproducibility: scenario DSL encodes timelines, failures (drops, delays, reorders, duplicates), and recovery.
- Same code paths: sim and prod share the domain/workflows and ports; only adapters differ.

## Performance & Scaling (SLA Guardrails)
- Concurrency: scale workers horizontally by partitioning events by process key; each partition guarantees serial processing per key.
- Throughput: expected workflow start rate and event rate are assembly‑defined; workers consume from queues with backpressure.
- Scaling strategy: partition by key; workers are queue‑based subscribers; horizontal scaling increases partitions and consumers.
- Latency: retries use exponential backoff with caps; idempotency ensures safe replays.
- Storage: per‑workflow snapshot/process stores sized to event rates; dedupe tables keyed by idempotency.

## Extensibility & Modularity Rules
- New workflow: add a module in `domain-workflows` (or a crate if large). Define states, events, intents. If you need a new side-effect, add a new port trait and adapter.
- Assemblies: compose workers; ports stay modular (small traits for focused concerns).
- Domain crates must remain ignorant of new workflows (no workflow‑specific helpers or enums in `domain-core`).

## Error & Failure Semantics
- Categories: `DomainError` (validation/invariants), `Transient` (retriable), `Fatal` (non-retriable), `Conflict` (concurrency/idempotency).
- Retries: adapters retry using idempotency keys; state machines stay pure; only confirmed effects alter state.
- Idempotency: intent keys derived from domain IDs; adapters dedupe executions; design assumes at-least-once delivery.

## Observability & Developer Experience
- Tracing: adapters attach spans to intent execution; envelopes carry correlation/causation.
- Metrics: counters/gauges via `MetricsSink` per workflow and intent type.
- Debugging: local assemblies use in-memory adapters; sim offers a simple console to step scenarios.
- Dev story: run assemblies with feature flags for adapters; run sim with fake clock, same domain/workflows.

## Effect Execution Model (Transaction Boundaries)
- Effect batches: the state machine emits `EffectBatch { intents: Vec<Intent>, atomic: bool }`.
- Atomic groups: if `atomic=true`, adapters must persist intents via a durable outbox and commit all-or-none within a single transaction boundary.
- Non-atomic: each intent is independently idempotent; execution order is defined but effects may commit separately.
- Rule of thumb: DB write + message emit + audit log for the same business action should be grouped atomically; otherwise use outbox + relay.

Example (illustrative):
```rust
pub struct EffectBatch { pub intents: Vec<Intent>, pub atomic: bool }
``` 

## Kickoff Checklist
- Create `crates/domain-core` with economic types, constants, invariant constructors, and doctests.
- Create `crates/ports` with traits for messaging, storage, crypto, metrics, time, and envelope.
- Create `crates/domain-workflows` with `Worker` trait, `Transition` and intents; implement `minting` prototype.
- Create `crates/sim` with in-memory bus/store, fake clock, and a minimal scenario runner.
- Create `crates/adapters/sqlite` and `adapters/prometheus` minimal implementations behind features.
- Create `crates/assemblies/minting-cluster` wiring workers to adapters via configs.

---
This document is the single source of truth for the next‑generation clean‑slate architecture.

## Conceptual Topology (ASCII)

Stack Layers
```
+---------------------------+    Deployment (processes/containers)
|        Assemblies         |    Wire workflows to adapters via config
+---------------------------+
|         Adapters          |    Concrete IO (db, bus, http, metrics)
+---------------------------+
|           Ports           |    Traits/contracts (no IO)
+---------------------------+
|    Domain Workflows       |    State machines, intents (pure)
+---------------------------+
|        Domain Core        |    Types, invariants, canonical bytes (pure)
+---------------------------+
```

Component Diagram
```
Domain Core <---- Domain Workflows ----> Ports ----> Adapters ----> External Systems
								  ^                                    \
								  |                                     \-> Metrics/Tracing
							  Assemblies -------------------------------> Config/Wiring
```

Workflow Event Flow
```
Envelope(Event) --> Worker.apply(State, Event, Clock)
						-> Transition{ new_state, intents[] }
						-> AdapterExecutor executes intents (idempotent, ordered)
						-> Store snapshots/process state
						-> Emit Envelope(Event) to downstream workers
```

## Time & Clock Semantics (Authoritative & Logical Time)
- Ownership: assemblies provide time via a `ports::Clock` implementation.
- Dual time: domain/workflows use logical time for ordering; adapters provide wall time for externally verifiable artifacts (signatures, expiries).
- Monotonicity: logical time is hybrid logical clock (HLC) or Lamport; strictly non‑decreasing per process key.
- Backward wall‑clock: adapters detect and surface as `Transient` errors; logical time prevents reordering.
- Scheduling: `ScheduleRetryIn()` is defined relative to logical time, then mapped to wall time by adapters.
- Signing time: to-be-signed bytes take an explicit `SigningTime` provided by the assembly at mint time; domain remains pure by accepting `SigningTime` as data.

Example contracts (illustrative):
```rust
// ports::clock
pub struct ClockReading {
	pub wall_time: std::time::SystemTime,
	pub logical: u64, // HLC/Lamport ticks, monotonically increasing
}

pub trait Clock {
	fn now(&self) -> ClockReading;
}
```

Signing intent (illustrative):
```rust
// domain-workflows intent produced by minting workflow
pub struct SignCertificateIntent {
    pub cert_id: String,
    pub canonical_bytes: Vec<u8>,    // includes SigningTime supplied by assembly
    pub signing_time: std::time::SystemTime,
}

// assembly obtains wall time via ports::Clock and injects it
let t = clock.now().wall_time;
let bytes = domain::canonicalize_cert(&cert, t); // pure function given inputs
let intent = SignCertificateIntent { cert_id, canonical_bytes: bytes, signing_time: t };
```

## Idempotency Keys (Derived From Causal Event)
- Keys are derived from the causal event/envelope or a canonical hash of the intent payload, not ad‑hoc strings.
- Default: `IdempotencyKey = hash(envelope_id || intent_type || version)`.
- Stability: event IDs are versioned and stable; intent type strings are versioned; key derivation function is frozen.
- Same event + same intent → same key; different intents → different keys.

Example (illustrative):
```rust
pub struct EnvelopeId([u8; 32]);
pub struct IdempotencyKey([u8; 32]);

pub trait Intent {
	fn intent_type(&self) -> &'static str;
	fn version(&self) -> u32 { 1 }
}

pub fn derive_key_from_event(env_id: &EnvelopeId, intent: &impl Intent) -> IdempotencyKey {
	use blake3::Hasher;
	let mut h = Hasher::new();
	h.update(env_id.0.as_ref());
	h.update(intent.intent_type().as_bytes());
	h.update(&intent.version().to_be_bytes());
	IdempotencyKey(*h.finalize().as_bytes())
}
```

## Dedup Audit Events (Exactly‑Once Visibility)
- Adapters that drop duplicates must emit a canonical event into normal flow:
```
Deduped { idempotency_key, dropped_message_id, surviving_message_id }
```
- Visible in metrics, audit logs, sim runs, and troubleshooting.

## Snapshot Strategy & State Evolution
- Frequency: `SnapshotFrequency(N)` default (e.g., every 100 events) per workflow.
- Schema: versioned snapshots `{ version: u32, payload: bytes }`.
- Migration: `domain-workflows` exposes `migrate_snapshot(version, bytes)`; workflows may reject snapshots on failed migration (safe behavior).
- Purpose: performance, long‑running sagas, and schema evolution without 100k‑event replays.
- Atomicity: snapshotting is a first‑class intent with an idempotency key. Persist snapshot bytes and a `SnapshotWritten(latest_event_id)` marker in the same transaction for consistent recovery.

## Recovery Semantics (Dirty-Replay Tolerance)
- Durable execution markers: adapters persist `EffectStarted`, `EffectApplied`, and `EffectFailed` markers to resume safely after crashes.
- Resume rules: on restart, the executor scans markers and the outbox; incomplete effects are retried idempotently; completed ones are skipped.
- Crash scenarios: simulator provides scripted failures (crash during snapshot, emit, side-effect execution, signing) and validates recovery against invariants.

## Read-Model Rewind & Rebuild
- Rewind: read models support a `rewind_to(event_id)` operation and rebuild by reapplying events from snapshots or genesis.
- Checkpoints: periodic durable checkpoints enable faster rebuilds; checkpoints carry versioned schema and integrity hashes.
- Divergence detection: background jobs reconcile read-model projections against the authoritative ledger and emit alerts.

## Saga Composition (Child Workflows & Compensation)
- First‑class saga intents:
```rust
pub enum SagaIntent {
	StartChildWorkflow { key: String, workflow_type: String, args: Vec<u8> },
	AwaitChild { key: String },
	Compensate { key: String, reason: String },
}
```
- Enables ore → ingot → certificate → signature → attestations → distribution pipelines.
// Compensation patterns may be defined but implemented later.

### Parent–Child Coupling Rules
- Parent persists `pending_children: Map<ChildKey, AwaitedOutcome>` before emitting `StartChild`.
- Child emits `ChildCompleted { key, result }` into the same durable event stream.
- Parent `AwaitChild` is idempotent: on replay, it first checks `pending_children` and the `ProcessStore` for child outcome; if present, it resumes without requiring the event.
- Backoff ownership: parent owns retry/backoff schedule for awaiting; child owns its own retries for side-effects.

## Revocation, Burn, Expiry, and Key Rotation (Surface)
- Domain‑core canonical operations: `revoke_certificate`, `expire_certificate`, `burn_units`, `rotate_keys`.
- Workflows: `RevocationWorkflow`, `KeyRotationWorkflow`, `ExpirySweepWorkflow` defined (surface).
- Regulatory: required for audit readiness and compromise response.

## Key Management Lifecycle
- Key versioning: every signature carries `{ key_id, key_version }`; canonical bytes carry `canonical_bytes_version`.
- HSM/remote signing: define a `Signer` port that may be backed by HSM or remote signer; specify availability SLO and error taxonomy.
- Rotation windows: assemblies configure planned rotations; domain enforces grace periods where old keys verify and new keys sign.
- Fallback keys: documented emergency keys, sealed and auditable; use is an explicit governance event.
- Chain of authority: publish a signed key lineage object to let verifiers validate historical signatures across rotations.

## Event & Snapshot Versioning
- Version tags: `EventVersion(u32)`, `SnapshotVersion(u32)`.
- Encoding: choose a canonical encoding (e.g., CBOR or protobuf), document it, and keep stable.
- Tests: crate‑level tests ensure old payloads still deserialize; new fields are additive.
- Enables rolling upgrades and prevents silent corruption.

## Circuit Breakers & Bulkheads
- Per‑adapter safety: timeouts, error budgets, retry budgets, dedicated worker pools.
- Breaker events: emit `BreakerOpen { adapter_id, reason }` for visibility.
- Contract (illustrative):
```rust
pub trait CircuitBreaker {
	fn check(&self, adapter: &str) -> Result<(), BreakerOpen>;
}

pub struct BreakerOpen { pub adapter: String, pub reason: String }
```
- Simulator supports “breaker opened” scenarios.

## Read‑Side Consistency Model
- CQRS (recommended): read models updated by workflow events via `ports::ReadModel`, eventually consistent.
- Strong reads (critical paths): allow queries against the snapshot/process store with consistency guarantees.
- Contract (illustrative):
```rust
pub trait ReadModel {
	fn apply_event(&mut self, envelope: &[u8]);
	fn query(&self, q: &str) -> Vec<u8>; // implementors define schema
}
```

## Governance & Parameterization Layer
- Replace constants with `EconomicParameters { ore_per_ingot, ingots_per_cert, ... }`.
- Source of truth: signed governance snapshot and/or workflow‑driven updates.
- Versioned parameters object; domain‑core requires parameters explicitly from assemblies.
 - Freeze-on-start: each workflow instance snapshots `EconomicParameters` at creation; subsequent changes occur via explicit governance events and snapshot migration.

Example (illustrative):
```rust
pub struct EconomicParameters {
	pub ore_per_ingot: u32,
	pub ingots_per_cert: u32,
	pub version: u32,
}
```

## Top 5 Must‑Do Fixes (Practical Priorities)
1) Formal Clock port with logical time (HLC/Lamport) and clear scheduling rules.
2) Canonical idempotency key derivation per intent inside domain/workflows.
3) Event/snapshot versioning with migrations and serialization tests.
4) Dedup auditor events emitted by adapters for visibility and auditability.
5) Define revocation/burn/expiry workflows (surfaces) for regulatory readiness.

## Additional Refinements (Concise)
- Idempotency key format: use a structured, collision‑safe key (e.g., CBOR array or base62 of a stable hash). Keep `"{intent_type}:{business_key}:{version}"` for logs only.
- Scoped Intent pattern: define intents per workflow (`MyWorkflowIntent`) and convert to a minimal `GlobalIntent` to avoid a growing “god enum”.
- Snapshot migration fallback: if `migrate_snapshot` fails, load the last good snapshot, replay, and emit a prominent `SnapshotMigrationFailed` event.
- EconomicParameters placement: type lives in `domain-core`; assemblies inject values. Ensures pure tests and governance flexibility.
- Forward‑only sagas: begin without compensation; add only when regulation/business requires it to avoid premature complexity.
- Circuit breaker durability: store breaker state in a shared durable store for cross‑instance consistency; in‑memory is fine for single‑instance/partitioned deployments.
- Replay/export/integrity: support full genesis replay under different parameters; provide a canonical regulator‑verifiable ledger export; hash‑chain/sign every transition to survive total DB loss.
